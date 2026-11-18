// Biogenesis (C) 2006-2010 Joan Queralt Molina; Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

//! OutCorridor transfer ordering and explicit presentation advance, independent of transport I/O

use crate::organism::primary_genome;

use super::*;
use std::{
    collections::{HashMap, VecDeque},
    num::NonZeroUsize,
};

/// Caller-supplied identity and bounded pending ownership for each connection
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutgoingConfig {
    pub user_name: crate::JavaText,
    pub pending_per_connection: NonZeroUsize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutgoingFailure {
    Unconfigured,
    MissingClade,
    QueueFull,
    Allocation,
    TicketsExhausted,
}

/// Failed admission retains the actual attempted reference without snapshotting its contents
#[derive(Clone, Debug)]
pub struct OutgoingError {
    pub kind: OutgoingFailure,
    attempted: Option<GenomeRef>,
    connection: Option<ConnectionRef>,
}

impl OutgoingError {
    pub(crate) fn attempted(&self) -> Option<&GenomeRef> {
        self.attempted.as_ref()
    }
    pub(crate) fn connection(&self) -> Option<&ConnectionRef> {
        self.connection.as_ref()
    }
}

impl PartialEq for OutgoingError {
    fn eq(&self, other: &Self) -> bool {
        fn same<T>(left: &Option<Rc<T>>, right: &Option<Rc<T>>) -> bool {
            match (left, right) {
                (None, None) => true,
                (Some(left), Some(right)) => Rc::ptr_eq(left, right),
                _ => false,
            }
        }
        self.kind == other.kind
            && same(&self.attempted, &other.attempted)
            && same(&self.connection, &other.connection)
    }
}
impl Eq for OutgoingError {}

pub(crate) type ConnectionRef = Rc<RefCell<ConnectionCore>>;
pub(crate) type OutgoingCorridorRef = Rc<RefCell<OutgoingCorridor>>;

/// One simulation-thread owner retains queued and claimed references until terminal completion
#[derive(Debug, PartialEq)]
pub(crate) struct ConnectionCore {
    pub state: i32,
    pub pending: PendingTransfers,
}

impl ConnectionCore {
    pub(crate) fn new(state: i32, capacity: NonZeroUsize) -> Self {
        Self {
            state,
            pending: PendingTransfers::new(capacity),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TransferOutcome {
    Acknowledged,
    Failed,
    Cancelled,
}

#[derive(Debug, PartialEq)]
pub(crate) struct PendingTransfer {
    pub ticket: u64,
    genome: GenomeRef,
    claimed: bool,
}

impl PendingTransfer {
    pub(crate) fn genome(&self) -> &GenomeRef {
        &self.genome
    }
    pub(crate) fn claimed(&self) -> bool {
        self.claimed
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct RejectedTransfer {
    pub reason: OutgoingFailure,
    pub genome: GenomeRef,
}

#[derive(Debug, PartialEq)]
pub(crate) struct PendingTransfers {
    capacity: NonZeroUsize,
    next_ticket: Option<u64>,
    entries: VecDeque<PendingTransfer>,
}

impl PendingTransfers {
    pub(crate) fn storage_capacity(&self) -> usize { self.entries.capacity() }
    pub(crate) fn retained_genomes(&self) -> impl Iterator<Item = &GenomeRef> {
        self.entries.iter().map(|entry| &entry.genome)
    }

    fn new(capacity: NonZeroUsize) -> Self {
        Self {
            capacity,
            next_ticket: Some(0),
            entries: VecDeque::new(),
        }
    }

    pub(crate) fn entries(&self) -> impl Iterator<Item = &PendingTransfer> {
        self.entries.iter()
    }

    pub(crate) fn is_full(&self) -> bool {
        self.entries.len() >= self.capacity.get()
    }

    pub(crate) fn try_enqueue(&mut self, genome: GenomeRef) -> Result<u64, RejectedTransfer> {
        let reason = if self.is_full() {
            Some(OutgoingFailure::QueueFull)
        } else if self.next_ticket.is_none() {
            Some(OutgoingFailure::TicketsExhausted)
        } else if self.entries.try_reserve(1).is_err() {
            Some(OutgoingFailure::Allocation)
        } else {
            None
        };
        if let Some(reason) = reason {
            return Err(RejectedTransfer { reason, genome });
        }
        let ticket = self.next_ticket.expect("Checked available ticket");
        self.next_ticket = ticket.checked_add(1);
        self.entries.push_back(PendingTransfer {
            ticket,
            genome,
            claimed: false,
        });
        Ok(ticket)
    }

    pub(crate) fn claim_next(&mut self) -> Option<u64> {
        let entry = self.entries.iter_mut().find(|entry| !entry.claimed)?;
        entry.claimed = true;
        Some(entry.ticket)
    }

    /// Borrow the live genotype only when serialization actually begins
    pub(crate) fn genome(&self, ticket: u64) -> Option<&GenomeRef> {
        self.entries
            .iter()
            .find(|entry| entry.ticket == ticket)
            .map(|entry| &entry.genome)
    }

    /// Explicit terminal completion releases ownership independently of presentation capacity
    pub(crate) fn finish(&mut self, ticket: u64, _outcome: TransferOutcome) -> Option<GenomeRef> {
        let index = self
            .entries
            .iter()
            .position(|entry| entry.ticket == ticket)?;
        self.entries.remove(index).map(|entry| entry.genome)
    }

    fn copy_for_world(&self, actors: &mut OrganismCloner) -> Self {
        Self {
            capacity: self.capacity,
            next_ticket: self.next_ticket,
            entries: self
                .entries
                .iter()
                .map(|entry| PendingTransfer {
                    ticket: entry.ticket,
                    genome: actors.genome_cloner().copy(&entry.genome),
                    claimed: entry.claimed,
                })
                .collect(),
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct OutgoingCorridor {
    pub bounds: IntRect,
    pub connection: ConnectionRef,
    pub traveller: Option<OrganismRef>,
}

impl OutgoingCorridor {
    pub(crate) fn can_send(&self) -> bool {
        self.traveller.is_none()
    }
}

/// Ordered Java graphics operations, captured after each corridor's presentation mutation
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DrawCommand {
    Outline { bounds: IntRect, argb: u32 },
    Fill { bounds: IntRect, argb: u32 },
    Line(RenderSegment),
}

pub(super) fn copy_list(
    original: &[OutgoingCorridorRef],
    actors: &mut OrganismCloner,
    corridors: &mut HashMap<*const RefCell<OutgoingCorridor>, OutgoingCorridorRef>,
    connections: &mut HashMap<*const RefCell<ConnectionCore>, ConnectionRef>,
) -> Vec<OutgoingCorridorRef> {
    original
        .iter()
        .map(|reference| {
            if let Some(existing) = corridors.get(&Rc::as_ptr(reference)) {
                return Rc::clone(existing);
            }
            let corridor = reference.borrow();
            let connection_key = Rc::as_ptr(&corridor.connection);
            let connection = Rc::clone(connections.entry(connection_key).or_insert_with(|| {
                let connection = corridor.connection.borrow();
                Rc::new(RefCell::new(ConnectionCore {
                    state: connection.state,
                    pending: connection.pending.copy_for_world(actors),
                }))
            }));
            let result = Rc::new(RefCell::new(OutgoingCorridor {
                bounds: corridor.bounds,
                connection,
                traveller: corridor.traveller.as_ref().map(|actor| actors.copy(actor)),
            }));
            corridors.insert(Rc::as_ptr(reference), Rc::clone(&result));
            result
        })
        .collect()
}

fn rectangle(output: &mut Vec<DrawCommand>, color: &mut u32, bounds: IntRect, incoming: bool) {
    *color = if incoming { 0xffff_0000 } else { 0xff00_00ff };
    output.push(DrawCommand::Outline {
        bounds,
        argb: *color,
    });
    *color = if incoming {
        crate::ColorKind::Maroon.argb().unwrap()
    } else {
        crate::ColorKind::LightBlue.argb().unwrap()
    };
    output.push(DrawCommand::Fill {
        bounds: IntRect {
            x: bounds.x.wrapping_add(1),
            y: bounds.y.wrapping_add(1),
            width: bounds.width.wrapping_sub(1),
            height: bounds.height.wrapping_sub(1),
        },
        argb: *color,
    });
}

pub(super) fn emit_actor_segments(
    actor: &Organism,
    draw_color: &mut u32,
    mut emit: impl FnMut(RenderSegment),
) -> Result<(), crate::GeometryError> {
    let temporary = actor.state.temporary_color_frames > 0;
    let eyes = actor.body.eyes.enabled;
    // Graphics.setColor(null) preserves the current paint
    if temporary && !eyes {
        if let Some(color) = &actor.state.temporary_color {
            *draw_color = color.argb();
        }
    } else if !temporary && !actor.state.alive && !eyes {
        *draw_color = crate::ColorKind::Brown.argb().unwrap();
    }
    for index in actor.body.indices() {
        let color = if temporary && eyes {
            let phenotype = actor.body.required_color(index)?;
            if phenotype.argb() == crate::ColorKind::Eye.argb().unwrap() {
                Some(phenotype)
            } else {
                actor.state.temporary_color.clone()
            }
        } else if !temporary && (actor.state.alive || eyes) {
            actor.body.color(index)?
        } else {
            None
        };
        if let Some(color) = color {
            *draw_color = color.argb();
        }
        let line = crate::physics::Line::from_offsets(
            [actor.body.rotated_start(index, 0)?, actor.body.rotated_start(index, 1)?],
            [actor.body.rotated_end(index, 0)?, actor.body.rotated_end(index, 1)?], actor.state.integer_center,
        );
        emit(RenderSegment {
            x1: line.start[0], y1: line.start[1], x2: line.end[0], y2: line.end[1],
            rgb: *draw_color & 0x00ff_ffff,
            alpha: (*draw_color >> 24) as u8,
            organism_id: actor.id,
        });
    }
    Ok(())
}

fn draw_actor(actor: &Organism, output: &mut Vec<DrawCommand>, draw_color: &mut u32) -> Result<(), crate::GeometryError> {
    emit_actor_segments(actor, draw_color, |line| output.push(DrawCommand::Line(line)))
}

fn draw_failure(error: WorldError, commands: Vec<DrawCommand>, paint: u32) -> WorldError {
    match error {
        WorldError::Geometry(error) => WorldError::Drawing { error, commands, paint },
        error => error,
    }
}

impl World {
    pub(super) fn new_connection_core(&self, state: i32) -> Result<ConnectionRef, WorldError> {
        let capacity = self.session.0.parameters.borrow().outgoing
            .as_ref()
            .ok_or_else(|| outgoing_error(OutgoingFailure::Unconfigured, None, None))?
            .pending_per_connection;
        Ok(Rc::new(RefCell::new(ConnectionCore::new(
            state,
            capacity,
        ))))
    }

    pub(super) fn new_outgoing_corridor(
        &self,
        connection: ConnectionRef,
    ) -> OutgoingCorridorRef {
        Rc::new(RefCell::new(OutgoingCorridor {
            bounds: self.corridor_bounds(),
            connection,
            traveller: None,
        }))
    }

    pub(super) fn add_outgoing_corridor(&self, corridor: OutgoingCorridorRef) {
        self.outgoing_corridors.borrow_mut().push(corridor);
        self.corridors_exist.set(true);
    }

    pub(super) fn add_corridors(
        &self,
        incoming: IncomingCorridorRef,
        outgoing: OutgoingCorridorRef,
    ) {
        self.add_incoming_corridor(incoming);
        self.add_outgoing_corridor(outgoing);
    }

    /// Remove first rectangle matches and return ordered dirty regions without advancing paint
    pub(super) fn remove_corridors(
        &self,
        incoming: &IncomingCorridorRef,
        outgoing: &OutgoingCorridorRef,
    ) -> [IntRect; 2] {
        {
            let bounds = incoming.borrow().bounds;
            let mut corridors = self.incoming_corridors.borrow_mut();
            if let Some(index) = corridors.iter().position(|entry| entry.borrow().bounds == bounds) {
                corridors.remove(index);
            }
        }
        {
            let bounds = outgoing.borrow().bounds;
            let mut corridors = self.outgoing_corridors.borrow_mut();
            if let Some(index) = corridors.iter().position(|entry| entry.borrow().bounds == bounds) {
                corridors.remove(index);
            }
        }
        let incoming_bounds = {
            let mut corridor = incoming.borrow_mut();
            corridor.bounds.width = corridor.bounds.width.wrapping_add(1);
            corridor.bounds.height = corridor.bounds.height.wrapping_add(1);
            corridor.bounds
        };
        let outgoing_bounds = {
            let mut corridor = outgoing.borrow_mut();
            corridor.bounds.width = corridor.bounds.width.wrapping_add(1);
            corridor.bounds.height = corridor.bounds.height.wrapping_add(1);
            corridor.bounds
        };
        [incoming_bounds, outgoing_bounds]
    }

    pub(super) fn check_outgoing(&self, actor: &Organism) -> Option<OutgoingCorridorRef> {
        self.outgoing_corridors.borrow().iter()
            .find(|corridor| {
                corridor
                    .borrow()
                    .bounds
                    .contains(actor.state.integer_center)
            })
            .map(Rc::clone)
    }

    pub(super) fn send_outgoing(
        &self,
        corridor: &OutgoingCorridorRef,
        actor_ref: &OrganismRef,
        actor: &mut Organism,
    ) -> Result<bool, WorldError> {
        let mut corridor = corridor.borrow_mut();
        if corridor.traveller.is_some() || corridor.connection.borrow().state != 51 {
            return Ok(false);
        }
        {
            let parameters = self.session.0.parameters.borrow();
            let config = parameters.outgoing.as_ref()
                .ok_or_else(|| outgoing_error(OutgoingFailure::Unconfigured, None, None))?;
            let mut genome = primary_genome(&actor.genome)?.borrow_mut();
            let clade = genome
                .clade_id()
                .ok_or_else(|| outgoing_error(OutgoingFailure::MissingClade, None, None))?;
            if !clade.units().contains(&u16::from(b':')) {
                let mut prefixed = config.user_name.units().to_vec();
                prefixed.push(u16::from(b':'));
                prefixed.extend_from_slice(clade.units());
                genome.set_clade_text(Some(crate::JavaText::from_units(prefixed)));
            }
        }
        let admission = corridor
            .connection
            .borrow_mut()
            .pending
            .try_enqueue(Rc::clone(primary_genome(&actor.genome)?));
        if let Err(rejected) = admission {
            return Err(outgoing_error(
                rejected.reason,
                Some(rejected.genome),
                Some(Rc::clone(&corridor.connection)),
            ));
        }
        corridor.traveller = Some(Rc::clone(actor_ref));
        actor.use_energy(
            actor.state.energy)?;
        actor.state.alive = false;
        self.decrease_population();
        Ok(true)
    }

    fn draw_corridor(
        corridor: &OutgoingCorridorRef,
        output: &mut Vec<DrawCommand>,
        color: &mut u32,
    ) -> Result<(), WorldError> {
        let mut corridor = corridor.borrow_mut();
        rectangle(output, color, corridor.bounds, false);
        if let Some(reference) = corridor.traveller.as_ref().map(Rc::clone) {
            let mut actor = reference.borrow_mut();
            if actor.body.growth_ratio < 16 {
                actor.body.growth_ratio = actor.body.growth_ratio.wrapping_add(1);
                actor.state.angle += PI / 6.0;
                actor.rebuild_body().map_err(WorldError::Geometry)?;
                crate::movement::calculate_bounds(&mut actor, true)
                    .map_err(WorldError::Geometry)?;
                draw_actor(&actor, output, color)?;
            } else {
                corridor.traveller = None;
            }
        }
        Ok(())
    }

    pub(super) fn draw_outgoing(
        &self,
        corridor: &OutgoingCorridorRef,
    ) -> Result<Vec<DrawCommand>, WorldError> {
        let mut output = Vec::new();
        let mut color = 0xffff_ffff;
        if let Err(error) = Self::draw_corridor(corridor, &mut output, &mut color) {
            return Err(draw_failure(error, output, color));
        }
        Ok(output)
    }

    fn corridor_presentation(&self) -> Result<(Vec<DrawCommand>, u32), WorldError> {
        let mut output = Vec::new();
        let mut color = 0xffff_ffff;
        if self.corridors_exist.get() {
            for corridor in self.incoming_corridors.borrow().iter() {
                rectangle(&mut output, &mut color, corridor.borrow().bounds, true);
            }
            for corridor in self.outgoing_corridors.borrow().iter() {
                if let Err(error) = Self::draw_corridor(corridor, &mut output, &mut color) {
                    return Err(draw_failure(error, output, color));
                }
            }
        }
        Ok((output, color))
    }

    /// Advance corridors once and retain final paint for subsequent ordinary segments
    pub fn advance_corridor_presentation(&self) -> Result<(Vec<DrawCommand>, u32), WorldError> {
        self.corridor_presentation()
    }

    /// Advance travelling presentation once and capture commands in source World.draw order
    pub fn advance_presentation(&self) -> Result<Vec<DrawCommand>, WorldError> {
        let (mut output, mut color) = self.corridor_presentation()?;
        let count = match self.organisms.len() {
            Ok(count) => count,
            Err(error) => return Err(WorldError::DrawingList { error, commands: output, paint: color }),
        };
        for index in 0..count {
            let actor = match self.organisms.actor(index) {
                Ok(actor) => actor,
                Err(error) => return Err(WorldError::DrawingList { error, commands: output, paint: color }),
            };
            if let Err(error) = draw_actor(&actor.borrow(), &mut output, &mut color) {
                return Err(WorldError::Drawing { error, commands: output, paint: color });
            }
        }
        Ok(output)
    }
}

fn outgoing_error(
    kind: OutgoingFailure,
    attempted: Option<GenomeRef>,
    connection: Option<ConnectionRef>,
) -> WorldError {
    WorldError::Outgoing(OutgoingError {
        kind,
        attempted,
        connection,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColorKind, Gene, GenomeProperties};

    fn world() -> World {
        World::with_parameters(
            WorldConfig {
                seed: 827364,
                width: 7000,
                height: 7000,
                initial_attempts: 0,
            },
            WorldParameters {
                outgoing: Some(OutgoingConfig {
                    user_name: "Local".into(),
                    pending_per_connection: NonZeroUsize::new(1).unwrap(),
                }),
                ..Default::default()
            },
        )
        .unwrap()
    }

    fn actor(world: &mut World, center: [i32; 2]) -> OrganismRef {
        let genome = Genome::from_parts(
            &[Gene::new(8.0, 0.0, ColorKind::Green.argb(), 0, [0; 36]).shared()],
            GenomeProperties {
                symmetry: 1,
                ..Default::default()
            },
        );
        world.paste_genome(genome, center).unwrap().unwrap();
        Rc::clone(world.organisms.borrow().last().unwrap())
    }

    fn corridor(world: &mut World, state: i32) -> OutgoingCorridorRef {
        let connection = world.new_connection_core(state).unwrap();
        let corridor = world.new_outgoing_corridor(connection);
        corridor.borrow_mut().bounds = IntRect {
            x: 900,
            y: 900,
            width: 200,
            height: 200,
        };
        corridor
    }

    #[test]
    fn real_queue_rejection_preserves_prefix_and_exposes_exact_attempt_without_post_send_effects() {
        let mut world = world();
        let actor_ref = actor(&mut world, [1000, 1000]);
        let sentinel = actor(&mut world, [2000, 2000]);
        let corridor = corridor(&mut world, 51);
        let connection = Rc::clone(&corridor.borrow().connection);
        let ticket = connection
            .borrow_mut()
            .pending
            .try_enqueue(Rc::clone(sentinel.borrow().genome.as_ref().unwrap()))
            .unwrap();
        let mut actor = actor_ref.borrow_mut();
        actor.genome.as_ref().unwrap().borrow_mut().set_clade_id(Some("root".into()));
        let before = (
            actor.state.energy.to_bits(),
            actor.state.alive,
            world.population.get(),
            world.oxygen.get().to_bits(),
            world.carbon_dioxide.get().to_bits(),
        );
        let error = world
            .send_outgoing(&corridor, &actor_ref, &mut actor)
            .unwrap_err();
        let WorldError::Outgoing(error) = error else {
            panic!("Wrong admission error")
        };
        assert_eq!(error.kind, OutgoingFailure::QueueFull);
        assert!(Rc::ptr_eq(error.attempted().unwrap(), actor.genome.as_ref().unwrap()));
        assert!(Rc::ptr_eq(error.connection().unwrap(), &connection));
        assert_eq!(actor.genome.as_ref().unwrap().borrow().clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("Local:root"));
        assert_eq!(
            (
                actor.state.energy.to_bits(),
                actor.state.alive,
                world.population.get(),
                world.oxygen.get().to_bits(),
                world.carbon_dioxide.get().to_bits()
            ),
            before
        );
        assert!(corridor.borrow().traveller.is_none());
        let removed = connection
            .borrow_mut()
            .pending
            .finish(ticket, TransferOutcome::Cancelled)
            .unwrap();
        assert!(Rc::ptr_eq(&removed, sentinel.borrow().genome.as_ref().unwrap()));
        assert!(
            world
                .send_outgoing(&corridor, &actor_ref, &mut actor)
                .unwrap()
        );
        assert_eq!(actor.genome.as_ref().unwrap().borrow().clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("Local:root"));
    }

    #[test]
    fn pending_and_claimed_transfers_keep_live_genotype_until_explicit_terminal_completion() {
        let mut world = world();
        let actor_ref = actor(&mut world, [1000, 1000]);
        let genome = Rc::clone(actor_ref.borrow().genome.as_ref().unwrap());
        let mut queue = PendingTransfers::new(NonZeroUsize::new(1).unwrap());
        let ticket = queue.try_enqueue(Rc::clone(&genome)).unwrap();
        assert!(!queue.entries().next().unwrap().claimed());
        assert_eq!(queue.claim_next(), Some(ticket));
        assert_eq!(queue.claim_next(), None);
        genome
            .borrow_mut()
            .set_clade_id(Some("changed-before-serialization".into()));
        assert!(Rc::ptr_eq(queue.genome(ticket).unwrap(), &genome));
        assert_eq!(
            queue.genome(ticket).unwrap().borrow().clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(),
            Some("changed-before-serialization")
        );
        let rejected = queue.try_enqueue(Rc::clone(&genome)).unwrap_err();
        assert_eq!(rejected.reason, OutgoingFailure::QueueFull);
        assert!(Rc::ptr_eq(&rejected.genome, &genome));
        assert!(queue.entries().next().unwrap().claimed());
        let completed = queue.finish(ticket, TransferOutcome::Acknowledged).unwrap();
        assert!(Rc::ptr_eq(&completed, &genome));
        assert!(queue.finish(ticket, TransferOutcome::Failed).is_none());
        assert!(queue.try_enqueue(genome).is_ok());
    }

    #[test]
    fn clone_detaches_worlds_but_retains_duplicate_corridors_connections_travellers_and_queue_aliases()
     {
        let mut world = world();
        let actor_ref = actor(&mut world, [1000, 1000]);
        let incoming = world.new_incoming_corridor();
        incoming
            .borrow_mut()
            .receive(Some(Rc::clone(actor_ref.borrow().genome.as_ref().unwrap())));
        world.add_incoming_corridor(incoming);
        let outgoing = corridor(&mut world, 51);
        let connection = Rc::clone(&outgoing.borrow().connection);
        let second = world.new_outgoing_corridor(Rc::clone(&connection));
        world.add_outgoing_corridor(Rc::clone(&outgoing));
        world.add_outgoing_corridor(Rc::clone(&outgoing));
        world.add_outgoing_corridor(second);
        world.rebuild_spatial();
        world
            .send_outgoing(&outgoing, &actor_ref, &mut actor_ref.borrow_mut())
            .unwrap();
        connection.borrow_mut().pending.claim_next();
        let copied = world.clone();
        let first_ref = Rc::clone(&copied.outgoing_corridors.borrow()[0]);
        let first = first_ref.borrow();
        assert!(Rc::ptr_eq(
            &copied.outgoing_corridors.borrow()[0],
            &copied.outgoing_corridors.borrow()[1]
        ));
        assert!(Rc::ptr_eq(
            &first.connection,
            &copied.outgoing_corridors.borrow()[2].borrow().connection
        ));
        assert!(!Rc::ptr_eq(&first.connection, &connection));
        assert!(Rc::ptr_eq(
            first.traveller.as_ref().unwrap(),
            &copied.organisms.borrow()[0]
        ));
        assert!(Rc::ptr_eq(&copied.frame_objects.borrow()[0], &copied.organisms.borrow()[0]));
        let genome = Rc::clone(copied.organisms.borrow()[0].borrow().genome.as_ref().unwrap());
        assert!(Rc::ptr_eq(
            first
                .connection
                .borrow()
                .pending
                .entries()
                .next()
                .unwrap()
                .genome(),
            &genome
        ));
        assert!(
            first
                .connection
                .borrow()
                .pending
                .entries()
                .next()
                .unwrap()
                .claimed()
        );
        assert!(Rc::ptr_eq(
            copied.incoming_corridors.borrow()[0].borrow().pending[0]
                .as_ref()
                .unwrap(),
            &genome
        ));
        genome.borrow_mut().set_clade_id(Some("clone-only".into()));
        assert_ne!(
            actor_ref.borrow().genome.as_ref().unwrap().borrow().clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(),
            Some("clone-only")
        );
    }

    #[test]
    fn first_containing_corridor_shadows_later_capacity_and_contains_wrapped_edges_exactly() {
        let mut world = world();
        let actor_ref = actor(&mut world, [1000, 1000]);
        let first = corridor(&mut world, 50);
        let second = corridor(&mut world, 51);
        world.add_outgoing_corridor(Rc::clone(&first));
        world.add_outgoing_corridor(Rc::clone(&second));
        assert!(first.borrow().can_send());
        assert!(Rc::ptr_eq(
            &world.check_outgoing(&actor_ref.borrow()).unwrap(),
            &first
        ));
        assert!(
            !world
                .send_outgoing(&first, &actor_ref, &mut actor_ref.borrow_mut())
                .unwrap()
        );
        assert!(second.borrow().can_send());
        first.borrow_mut().traveller = Some(Rc::clone(&actor_ref));
        assert!(Rc::ptr_eq(
            &world.check_outgoing(&actor_ref.borrow()).unwrap(),
            &first
        ));
        let bounds = IntRect {
            x: i32::MAX - 1,
            y: 0,
            width: 5,
            height: 1,
        };
        assert!(bounds.contains([i32::MAX, 0]));
        assert!(!bounds.contains([i32::MIN, 0]));
        assert!(!IntRect { width: 0, ..bounds }.contains([i32::MAX - 1, 0]));
        assert!(
            !IntRect {
                width: -1,
                ..bounds
            }
            .contains([i32::MAX - 1, 0])
        );
        assert!(
            !IntRect {
                x: 0,
                y: 0,
                width: 2,
                height: 2
            }
            .contains([2, 1])
        );
    }

    #[test]
    fn scarce_oxygen_transfer_returns_before_contacts_breath_and_age_then_frame_removes_only_membership()
     {
        let mut world = world();
        let actor_ref = actor(&mut world, [1000, 1000]);
        let outgoing = corridor(&mut world, 51);
        world.add_outgoing_corridor(Rc::clone(&outgoing));
        {
            let mut actor = actor_ref.borrow_mut();
            actor.body.growth_ratio = 1;
            actor.state.velocity = [0.25, 0.0];
            actor.state.angular_velocity = 0.0;
            actor.state.age = 1;
            actor.state.energy = 40.0;
            actor.state.active = true;
        }
        world.oxygen.set(1.0);
        let population = world.population.get();
        let before_age = actor_ref.borrow().state.age;
        world.single_step().unwrap();
        assert!(world.organisms.borrow().is_empty());
        assert_eq!(world.population.get(), population - 1);
        assert!(Rc::ptr_eq(&world.frame_objects.borrow()[0], &actor_ref));
        assert!(Rc::ptr_eq(
            outgoing.borrow().traveller.as_ref().unwrap(),
            &actor_ref
        ));
        assert_eq!(actor_ref.borrow().state.age, before_age);
        assert_eq!(actor_ref.borrow().state.energy, 39.0);
        assert!(!actor_ref.borrow().state.alive);
        assert_eq!(actor_ref.borrow().body.growth_ratio, 1);
    }

    #[test]
    fn paint15_draws16_then_next_paint_releases_without_releasing_pending_ownership() {
        let mut world = world();
        let actor_ref = actor(&mut world, [1000, 1000]);
        let outgoing = corridor(&mut world, 51);
        world.add_outgoing_corridor(Rc::clone(&outgoing));
        world
            .send_outgoing(&outgoing, &actor_ref, &mut actor_ref.borrow_mut())
            .unwrap();
        actor_ref.borrow_mut().body.growth_ratio = 15;
        world.organisms.borrow_mut().clear();
        world.single_step().unwrap();
        assert_eq!(actor_ref.borrow().body.growth_ratio, 15);
        let before = (
            world.tick.get(),
            world.random_snapshot(),
            world.oxygen.get().to_bits(),
            world.carbon_dioxide.get().to_bits(),
        );
        let angle = actor_ref.borrow().state.angle;
        let first = world.advance_presentation().unwrap();
        assert_eq!(actor_ref.borrow().body.growth_ratio, 16);
        assert_eq!(
            actor_ref.borrow().state.angle.to_bits(),
            (angle + PI / 6.0).to_bits()
        );
        assert!(!outgoing.borrow().can_send());
        assert!(
            first
                .iter()
                .any(|item| matches!(item, DrawCommand::Line(_)))
        );
        let second = world.draw_outgoing(&outgoing).unwrap();
        assert_eq!(second.len(), 2);
        assert!(outgoing.borrow().can_send());
        assert_eq!(
            (
                world.tick.get(),
                world.random_snapshot(),
                world.oxygen.get().to_bits(),
                world.carbon_dioxide.get().to_bits()
            ),
            before
        );
        assert_eq!(
            outgoing
                .borrow()
                .connection
                .borrow()
                .pending
                .entries()
                .count(),
            1
        );
    }

    #[test]
    fn presentation_captures_duplicate_occurrences_before_member_draw_and_preserves_color_state() {
        let mut world = world();
        let actor_ref = actor(&mut world, [1000, 1000]);
        let incoming = world.new_incoming_corridor();
        let outgoing = corridor(&mut world, 51);
        world.add_corridors(incoming, Rc::clone(&outgoing));
        world.add_outgoing_corridor(Rc::clone(&outgoing));
        actor_ref.borrow().genome.as_ref().unwrap().borrow().genes().unwrap()[0]
            .borrow_mut()
            .set_length(200.0);
        world
            .send_outgoing(&outgoing, &actor_ref, &mut actor_ref.borrow_mut())
            .unwrap();
        {
            let mut actor = actor_ref.borrow_mut();
            actor.body.growth_ratio = 14;
            actor.state.temporary_color_frames = 10;
            actor.state.temporary_color = None;
        }
        let draw = world.advance_presentation().unwrap();
        assert_eq!(actor_ref.borrow().body.growth_ratio, 16);
        assert_eq!(draw.len(), 9);
        assert!(matches!(
            draw[0],
            DrawCommand::Outline {
                argb: 0xffff0000,
                ..
            }
        ));
        assert!(matches!(
            draw[1],
            DrawCommand::Fill {
                argb: 0xff800000,
                ..
            }
        ));
        assert!(matches!(
            draw[2],
            DrawCommand::Outline {
                argb: 0xff0000ff,
                ..
            }
        ));
        assert!(matches!(
            draw[3],
            DrawCommand::Fill {
                argb: 0xff000080,
                ..
            }
        ));
        let lines = draw
            .iter()
            .filter_map(|item| {
                if let DrawCommand::Line(line) = item {
                    Some(line)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(lines.len(), 3);
        assert!(lines.iter().all(|line| line.rgb == 0x000080));
        assert_ne!(
            (lines[0].x1, lines[0].y1, lines[0].x2, lines[0].y2),
            (lines[1].x1, lines[1].y1, lines[1].x2, lines[1].y2)
        );
        assert_eq!(lines[1], lines[2]);
    }

    #[test]
    fn genesis_retains_connection_queue_and_traveller_but_reset_constructs_fresh_owners() {
        let mut world = world();
        let actor_ref = actor(&mut world, [1000, 1000]);
        let outgoing = corridor(&mut world, 51);
        world.add_outgoing_corridor(Rc::clone(&outgoing));
        world
            .send_outgoing(&outgoing, &actor_ref, &mut actor_ref.borrow_mut())
            .unwrap();
        let random = world.random_snapshot();
        world.genesis().unwrap();
        assert_eq!(world.random_snapshot(), random);
        assert!(world.organisms.borrow().is_empty());
        assert!(Rc::ptr_eq(&world.outgoing_corridors.borrow()[0], &outgoing));
        assert!(Rc::ptr_eq(
            outgoing.borrow().traveller.as_ref().unwrap(),
            &actor_ref
        ));
        assert_eq!(
            outgoing
                .borrow()
                .connection
                .borrow()
                .pending
                .entries()
                .count(),
            1
        );
        world.reset().unwrap();
        assert!(world.outgoing_corridors.borrow().is_empty() && !world.corridors_exist.get());
        assert_eq!(
            outgoing
                .borrow()
                .connection
                .borrow()
                .pending
                .entries()
                .count(),
            1
        );
    }

    #[test]
    fn unconfigured_world_and_null_clade_fail_before_admission_without_inventing_identity() {
        let blank = World::new(WorldConfig {
            initial_attempts: 0,
            ..Default::default()
        })
        .unwrap();
        assert!(matches!(
            blank.new_connection_core(51),
            Err(WorldError::Outgoing(OutgoingError {
                kind: OutgoingFailure::Unconfigured,
                ..
            }))
        ));
        let mut world = world();
        let actor_ref = actor(&mut world, [1000, 1000]);
        let outgoing = corridor(&mut world, 51);
        actor_ref.borrow().genome.as_ref().unwrap().borrow_mut().set_clade_id(None);
        let before = actor_ref.borrow().snapshot();
        assert!(matches!(
            world.send_outgoing(&outgoing, &actor_ref, &mut actor_ref.borrow_mut()),
            Err(WorldError::Outgoing(OutgoingError {
                kind: OutgoingFailure::MissingClade,
                ..
            }))
        ));
        assert_eq!(actor_ref.borrow().snapshot(), before);
        assert_eq!(
            outgoing
                .borrow()
                .connection
                .borrow()
                .pending
                .entries()
                .count(),
            0
        );
    }

    #[test]
    fn false_corridor_flag_skips_rectangles_and_presentation_mutations_but_draws_members() {
        let mut world = world();
        let actor_ref = actor(&mut world, [1000, 1000]);
        let outgoing = corridor(&mut world, 51);
        let incoming = world.new_incoming_corridor();
        world.add_corridors(incoming, Rc::clone(&outgoing));
        world
            .send_outgoing(&outgoing, &actor_ref, &mut actor_ref.borrow_mut())
            .unwrap();
        actor_ref.borrow_mut().body.growth_ratio = 15;
        world.corridors_exist.set(false);
        let before = actor_ref.borrow().snapshot();
        let draw = world.advance_presentation().unwrap();
        assert_eq!(draw.len(), 1);
        assert!(matches!(draw[0], DrawCommand::Line(_)));
        assert_eq!(actor_ref.borrow().snapshot(), before);
        assert!(!outgoing.borrow().can_send());
        world.corridors_exist.set(true);
        assert_eq!(world.advance_presentation().unwrap().len(), 6);
        assert_eq!(actor_ref.borrow().body.growth_ratio, 16);
    }

    #[test]
    fn empty_actor_color_changes_are_shared_by_pure_and_ordered_member_rendering() {
        for (timer, tint, expected) in [
            (0, None, ColorKind::Brown.argb().unwrap()),
            (10, ColorKind::Red.argb(), ColorKind::Red.argb().unwrap()),
        ] {
            let mut world = world();
            let empty = Genome::from_parts(
                &[],
                GenomeProperties {
                    symmetry: 1,
                    ..Default::default()
                },
            );
            world.paste_genome(empty, [1000, 1000]).unwrap().unwrap();
            {
                let actor_ref = Rc::clone(&world.organisms.borrow()[0]);
                let mut actor = actor_ref.borrow_mut();
                assert!((actor.body.segment_count == 0));
                actor.state.alive = false;
                actor.state.temporary_color_frames = timer;
                actor.state.temporary_color = (tint).map(crate::JavaColor::from_argb);
            }
            let actor_ref = actor(&mut world, [2000, 2000]);
            actor_ref.borrow_mut().state.temporary_color_frames = 10;
            actor_ref.borrow_mut().state.temporary_color = None;
            let pure = world.render_segments().unwrap();
            assert_eq!(pure.len(), 1);
            assert_eq!(pure[0].rgb, expected & 0x00ff_ffff);
            let ordered = world.advance_presentation().unwrap();
            assert_eq!(ordered, vec![DrawCommand::Line(pure[0])]);
        }
    }
    #[test]
    fn outgoing_prefix_preserves_raw_units_and_retains_the_prefixed_owner_on_retry() {
        let mut world = world();
        let actor_ref = actor(&mut world, [1000, 1000]);
        let sentinel = actor(&mut world, [2000, 2000]);
        let corridor = corridor(&mut world, 51);
        let connection = Rc::clone(&corridor.borrow().connection);
        connection.borrow_mut().pending.try_enqueue(Rc::clone(sentinel.borrow().genome.as_ref().unwrap())).unwrap();
        let text = crate::JavaText::from_units(vec![0xd800, 0, 65]);
        actor_ref.borrow().genome.as_ref().unwrap().borrow_mut().set_clade_text(Some(text.clone()));
        actor_ref.borrow_mut().state.network_position = Some(text.clone());
        let random = world.random_snapshot();
        let mut actor = actor_ref.borrow_mut();
        let error = world.send_outgoing(&corridor, &actor_ref, &mut actor).unwrap_err();
        assert!(matches!(error, WorldError::Outgoing(error) if error.kind == OutgoingFailure::QueueFull));
        let prefixed = actor.genome.as_ref().unwrap().borrow().clade_id().unwrap().clone();
        assert_eq!(prefixed.units(), [76, 111, 99, 97, 108, 58, 0xd800, 0, 65]);
        assert!(!prefixed.same_instance(&text));
        assert!(actor.state.network_position.as_ref().unwrap().same_instance(&text));
        let error = world.send_outgoing(&corridor, &actor_ref, &mut actor).unwrap_err();
        assert!(matches!(error, WorldError::Outgoing(error) if error.kind == OutgoingFailure::QueueFull));
        assert!(actor.genome.as_ref().unwrap().borrow().clade_id().unwrap().same_instance(&prefixed));
        assert_eq!(world.random_snapshot(), random);
        assert_eq!(text.units(), [0xd800, 0, 65]);
    }

    #[test]
    fn removal_matches_first_bounds_without_comparing_pending_candidates_or_travellers() {
        let mut world = world();
        let first_actor = actor(&mut world, [1000, 1000]);
        let argument_actor = actor(&mut world, [2000, 2000]);
        let first_input = world.new_incoming_corridor();
        let argument_input = world.new_incoming_corridor();
        let tail_input = world.new_incoming_corridor();
        let input_bounds = IntRect { x: 10, y: 20, width: 30, height: 40 };
        first_input.borrow_mut().bounds = input_bounds;
        argument_input.borrow_mut().bounds = input_bounds;
        tail_input.borrow_mut().bounds = IntRect { x: 3000, y: 3000, width: 5, height: 5 };
        first_input.borrow_mut().receive(None);
        let code = Rc::clone(argument_actor.borrow().genome.as_ref().unwrap());
        argument_input.borrow_mut().receive(Some(Rc::clone(&code)));
        first_input.borrow_mut().candidate = Some(Rc::clone(&first_actor));
        argument_input.borrow_mut().candidate = Some(Rc::clone(&argument_actor));
        let first_output = corridor(&mut world, 51);
        let argument_output = corridor(&mut world, 52);
        let tail_output = corridor(&mut world, 51);
        tail_output.borrow_mut().bounds.x = 4000;
        first_output.borrow_mut().traveller = Some(Rc::clone(&first_actor));
        argument_output.borrow_mut().traveller = Some(Rc::clone(&argument_actor));
        let connection = Rc::clone(&argument_output.borrow().connection);
        let ticket = connection.borrow_mut().pending.try_enqueue(Rc::clone(&code)).unwrap();
        world.add_corridors(Rc::clone(&first_input), Rc::clone(&first_output));
        world.add_corridors(Rc::clone(&argument_input), Rc::clone(&argument_output));
        world.add_corridors(Rc::clone(&tail_input), Rc::clone(&tail_output));
        let before = (
            world.random_snapshot(), world.tick.get(), world.frame.get(), world.population.get(),
            world.oxygen.get().to_bits(), world.carbon_dioxide.get().to_bits(), world.methane.get().to_bits(),
            first_actor.borrow().snapshot(), argument_actor.borrow().snapshot(),
        );

        let repaint = world.remove_corridors(&argument_input, &argument_output);

        assert_eq!(repaint, [
            IntRect { x: 10, y: 20, width: 31, height: 41 },
            IntRect { x: 900, y: 900, width: 201, height: 201 },
        ]);
        {
            let incoming = world.incoming_corridors.borrow();
            let outgoing = world.outgoing_corridors.borrow();
            assert_eq!((incoming.len(), outgoing.len()), (2, 2));
            assert!(Rc::ptr_eq(&incoming[0], &argument_input) && Rc::ptr_eq(&incoming[1], &tail_input));
            assert!(Rc::ptr_eq(&outgoing[0], &argument_output) && Rc::ptr_eq(&outgoing[1], &tail_output));
        }
        assert_eq!(first_input.borrow().bounds, input_bounds);
        assert_eq!(first_output.borrow().bounds, IntRect { x: 900, y: 900, width: 200, height: 200 });
        assert_eq!(argument_input.borrow().bounds, repaint[0]);
        assert_eq!(argument_output.borrow().bounds, repaint[1]);
        assert_eq!(first_input.borrow().pending.len(), 1);
        assert!(first_input.borrow().pending[0].is_none());
        assert!(Rc::ptr_eq(argument_input.borrow().pending[0].as_ref().unwrap(), &code));
        assert!(Rc::ptr_eq(first_input.borrow().candidate.as_ref().unwrap(), &first_actor));
        assert!(Rc::ptr_eq(argument_input.borrow().candidate.as_ref().unwrap(), &argument_actor));
        assert!(Rc::ptr_eq(first_output.borrow().traveller.as_ref().unwrap(), &first_actor));
        assert!(Rc::ptr_eq(argument_output.borrow().traveller.as_ref().unwrap(), &argument_actor));
        assert_eq!(connection.borrow().state, 52);
        assert!(Rc::ptr_eq(connection.borrow().pending.genome(ticket).unwrap(), &code));
        assert!(world.corridors_exist.get());
        assert_eq!((
            world.random_snapshot(), world.tick.get(), world.frame.get(), world.population.get(),
            world.oxygen.get().to_bits(), world.carbon_dioxide.get().to_bits(), world.methane.get().to_bits(),
            first_actor.borrow().snapshot(), argument_actor.borrow().snapshot(),
        ), before);
    }

    #[test]
    fn removal_consumes_one_duplicate_alias_then_expands_even_after_both_lists_are_empty() {
        let mut world = world();
        let incoming = world.new_incoming_corridor();
        incoming.borrow_mut().bounds = IntRect { x: 11, y: 12, width: 3, height: 4 };
        incoming.borrow_mut().receive(None);
        let outgoing = corridor(&mut world, 51);
        world.add_corridors(Rc::clone(&incoming), Rc::clone(&outgoing));
        world.add_corridors(Rc::clone(&incoming), Rc::clone(&outgoing));
        let random = world.random_snapshot();
        for (input_size, output_size, remaining) in [( [4, 5], 201, 1), ([5, 6], 202, 0), ([6, 7], 203, 0)] {
            assert_eq!(world.remove_corridors(&incoming, &outgoing), [
                IntRect { x: 11, y: 12, width: input_size[0], height: input_size[1] },
                IntRect { x: 900, y: 900, width: output_size, height: output_size },
            ]);
            assert_eq!(world.incoming_corridors.borrow().len(), remaining);
            assert_eq!(world.outgoing_corridors.borrow().len(), remaining);
            if remaining == 1 {
                assert!(Rc::ptr_eq(&world.incoming_corridors.borrow()[0], &incoming));
                assert!(Rc::ptr_eq(&world.outgoing_corridors.borrow()[0], &outgoing));
            }
            assert!(world.corridors_exist.get());
            assert_eq!(incoming.borrow().pending.len(), 1);
            assert!(incoming.borrow().pending[0].is_none());
            assert_eq!(world.random_snapshot(), random);
        }
    }

    #[test]
    fn unmatched_removal_preserves_members_and_flag_while_expanding_signed_extents_with_wrapping() {
        for flag in [false, true] {
            let mut world = world();
            let retained_input = world.new_incoming_corridor();
            retained_input.borrow_mut().bounds = IntRect { x: 50, y: 60, width: 70, height: 80 };
            let retained_output = corridor(&mut world, 51);
            world.add_corridors(Rc::clone(&retained_input), Rc::clone(&retained_output));
            world.corridors_exist.set(flag);
            let incoming = world.new_incoming_corridor();
            incoming.borrow_mut().bounds = IntRect {
                x: i32::MAX, y: i32::MIN, width: i32::MAX, height: i32::MAX,
            };
            let outgoing = corridor(&mut world, 51);
            outgoing.borrow_mut().bounds = IntRect {
                x: i32::MIN, y: i32::MAX, width: -1, height: 0,
            };
            let random = world.random_snapshot();

            assert_eq!(world.remove_corridors(&incoming, &outgoing), [
                IntRect { x: i32::MAX, y: i32::MIN, width: i32::MIN, height: i32::MIN },
                IntRect { x: i32::MIN, y: i32::MAX, width: 0, height: 1 },
            ]);

            assert_eq!(world.incoming_corridors.borrow().len(), 1);
            assert_eq!(world.outgoing_corridors.borrow().len(), 1);
            assert!(Rc::ptr_eq(&world.incoming_corridors.borrow()[0], &retained_input));
            assert!(Rc::ptr_eq(&world.outgoing_corridors.borrow()[0], &retained_output));
            assert_eq!(retained_input.borrow().bounds, IntRect { x: 50, y: 60, width: 70, height: 80 });
            assert_eq!(retained_output.borrow().bounds, IntRect { x: 900, y: 900, width: 200, height: 200 });
            assert_eq!(world.corridors_exist.get(), flag);
            assert_eq!(world.random_snapshot(), random);
        }
    }


    mod removal_source {
        use super::*;
        use std::collections::BTreeMap;

        type Row = [String; 8];
        const FIXTURE: &str = include_str!("../tests/fixtures/corridor_removal.tsv");

        fn role<T>(owners: &[(&'static str, Rc<RefCell<T>>)], value: Option<&Rc<RefCell<T>>>) -> &'static str {
            match value {
                None => "~",
                Some(value) => owners.iter().find(|(_, owner)| Rc::ptr_eq(owner, value))
                    .map_or("unlabeled", |(name, _)| *name),
            }
        }

        fn names<T>(values: &[Rc<RefCell<T>>], owners: &[(&'static str, Rc<RefCell<T>>)]) -> String {
            if values.is_empty() { return "~".into(); }
            values.iter().map(|value| role(owners, Some(value))).collect::<Vec<_>>().join(",")
        }

        fn rect(bounds: IntRect) -> String {
            format!("{},{},{},{}", bounds.x, bounds.y, bounds.width, bounds.height)
        }

        struct Replay<'a> {
            case: &'static str,
            world: World,
            inputs: Vec<(&'static str, IncomingCorridorRef)>,
            outputs: Vec<(&'static str, OutgoingCorridorRef)>,
            genomes: Vec<(&'static str, GenomeRef)>,
            actors: Vec<(&'static str, OrganismRef)>,
            rows: &'a mut Vec<Row>,
        }

        impl<'a> Replay<'a> {
            fn new(case: &'static str, rows: &'a mut Vec<Row>) -> Self {
                Self { case, world: world(), inputs: Vec::new(), outputs: Vec::new(),
                    genomes: Vec::new(), actors: Vec::new(), rows }
            }

            fn emit(&mut self, kind: &str, stage: &str, cells: [String; 5]) {
                let [object, field, value, extra1, extra2] = cells;
                self.rows.push([kind.into(), self.case.into(), stage.into(), object, field, value, extra1, extra2]);
            }

            fn genome(&mut self, name: &'static str) -> GenomeRef {
                let mut code = Genome::from_parts(
                    &[Gene::new(12.0, 0.0, ColorKind::Green.argb(), -1, [0; 36]).shared()],
                    GenomeProperties { symmetry: 1, ..Default::default() },
                );
                code.set_clade_id(Some(name.into()));
                let code = Rc::new(RefCell::new(code));
                self.genomes.push((name, Rc::clone(&code)));
                // These rows declare supplied inputs, not derived runtime coverage
                self.emit("GENOME", "setup", [name.into(), "recipe".into(),
                    "green-length12-theta0-branch-(-1)".into(),
                    "symmetry=1;mirror=0;mutation=0;activity=0".into(),
                    "all flags=false; reactions=0".into()]);
                code
            }

            fn traveller(&mut self, name: &'static str, code: &GenomeRef) -> OrganismRef {
                let actor = Rc::new(RefCell::new(self.world.new_pasted_candidate(Rc::clone(code))));
                self.actors.push((name, Rc::clone(&actor)));
                actor
            }

            fn input(&mut self, name: &'static str, bounds: IntRect) -> IncomingCorridorRef {
                let corridor = self.world.new_incoming_corridor();
                corridor.borrow_mut().bounds = bounds;
                self.inputs.push((name, Rc::clone(&corridor)));
                corridor
            }

            fn output(&mut self, name: &'static str, bounds: IntRect, traveller: Option<OrganismRef>) -> OutgoingCorridorRef {
                // Source supplies null; this inert required Rust owner never sends or completes a transfer
                let connection = self.world.new_connection_core(0).unwrap();
                let corridor = self.world.new_outgoing_corridor(connection);
                corridor.borrow_mut().bounds = bounds;
                corridor.borrow_mut().traveller = traveller;
                self.outputs.push((name, Rc::clone(&corridor)));
                corridor
            }

            fn observe(&mut self, stage: &str, inputs: &[IncomingCorridorRef], outputs: &[OutgoingCorridorRef]) {
                let list = ["world".into(), "in-list".into(),
                    names(&self.world.incoming_corridors.borrow(), &self.inputs),
                    names(&self.world.outgoing_corridors.borrow(), &self.outputs),
                    self.world.corridors_exist.get().to_string()];
                self.emit("STATE", stage, list);
                let population = ["world".into(), "population".into(), self.world.population.get().to_string(),
                    format!("{:016x}", self.world.carbon_dioxide.get().to_bits()),
                    format!("{:016x}", self.world.oxygen.get().to_bits())];
                self.emit("STATE", stage, population);
                let methane = ["world".into(), "methane".into(), format!("{:016x}", self.world.methane.get().to_bits()),
                    self.world.organisms.borrow().len().to_string(), String::new()];
                self.emit("STATE", stage, methane);
                for reference in inputs {
                    let corridor = reference.borrow();
                    let queue = if corridor.pending.is_empty() { "~".into() } else {
                        corridor.pending.iter().map(|code| role(&self.genomes, code.as_ref())).collect::<Vec<_>>().join(",")
                    };
                    let cells = [role(&self.inputs, Some(reference)).into(), "rect".into(), rect(corridor.bounds),
                        format!("{}#{}", queue, corridor.pending.len()),
                        format!("{},~", role(&self.actors, corridor.candidate.as_ref()))];
                    self.emit("OBJECT", stage, cells);
                }
                for reference in outputs {
                    let corridor = reference.borrow();
                    let cells = [role(&self.outputs, Some(reference)).into(), "rect".into(), rect(corridor.bounds),
                        "~".into(), format!("~,{}", role(&self.actors, corridor.traveller.as_ref()))];
                    self.emit("OBJECT", stage, cells);
                }
            }

            fn remove(&mut self, stage: &str, input: &IncomingCorridorRef, output: &OutgoingCorridorRef) {
                let regions = self.world.remove_corridors(input, output);
                for (index, name) in [role(&self.inputs, Some(input)), role(&self.outputs, Some(output))].iter().enumerate() {
                    self.emit("REPAINT", stage, [(*name).into(), "order".into(), index.to_string(),
                        rect(regions[index]), String::new()]);
                }
                self.observe(&format!("{stage}-after"), &[Rc::clone(input)], &[Rc::clone(output)]);
            }

            fn frame_and_draw(&mut self, inputs: &[IncomingCorridorRef], outputs: &[OutgoingCorridorRef], code: &GenomeRef) {
                self.observe("before-frame", inputs, outputs);
                *self.world.session.0.random.borrow_mut() = JavaRandom::new(827364);
                let before = self.world.organisms.borrow().clone();
                assert!(self.world.single_step().unwrap());
                let after = self.world.organisms.borrow().clone();
                let added: Vec<_> = after.iter().filter(|actor| !before.iter().any(|old| Rc::ptr_eq(old, actor))).cloned().collect();
                assert_eq!((before.len(), added.len(), after.len()), (0, 1, 1));
                assert!(Rc::ptr_eq(added[0].borrow().genome.as_ref().unwrap(), code));
                self.actors.push(("admitted-0", Rc::clone(&added[0])));
                // This fixture projects membership; it does not intercept an admission callback
                let admission = ["admitted-0".into(), "genome".into(),
                    role(&self.genomes, added[0].borrow().genome.as_ref()).into(),
                    before.len().to_string(), after.len().to_string()];
                self.emit("ADMIT", "world-time", admission);
                self.observe("after-frame", inputs, outputs);
                let mut probe = self.world.new_pasted_candidate(Rc::clone(code));
                probe.state.integer_center = [101, 101];
                probe.state.center = [101.0, 101.0];
                let hit = self.world.check_outgoing(&probe);
                let cells = ["checkHitCorridor".into(), role(&self.outputs, hit.as_ref()).into(),
                    "center".into(), "101,101".into(), self.world.outgoing_corridors.borrow().len().to_string()];
                self.emit("HIT", "after-frame", cells);
                let commands = self.world.advance_presentation().unwrap();
                let input_bounds = inputs[1].borrow().bounds;
                let output_bounds = outputs[1].borrow().bounds;
                let mut input_calls = 0;
                let mut output_calls = 0;
                let mut edge = None;
                let mut fill = None;
                for command in commands {
                    match command {
                        DrawCommand::Outline { bounds, argb } if bounds == input_bounds => {
                            input_calls += 1;
                            assert!(!bounds.contains([100, 100]) && !bounds.contains([102, 102]));
                            assert_eq!(argb, 0xffff_0000);
                        }
                        DrawCommand::Outline { bounds, argb } => {
                            assert_eq!(bounds, output_bounds);
                            assert_eq!([bounds.x, bounds.y], [100, 100]);
                            assert!(102 < bounds.x + bounds.width && 102 < bounds.y + bounds.height);
                            output_calls += 1;
                            edge = Some(argb);
                        }
                        DrawCommand::Fill { bounds, argb } => {
                            assert!(!bounds.contains([100, 100]));
                            if bounds.contains([102, 102]) { fill = Some(argb); }
                        }
                        DrawCommand::Line(line) => {
                            // Member lines lie outside both fixed Source pixel probes, including a one-pixel margin
                            assert!(line.x1.max(line.x2) < 99.0 || line.y1.max(line.y2) < 99.0
                                || line.x1.min(line.x2) > 103.0 || line.y1.min(line.y2) > 103.0);
                        }
                    }
                }
                // Counter and pixel equivalents come from actual ordered commands, not Graphics callbacks
                self.emit("DRAW", "after-frame", ["world".into(), "corridor-calls".into(),
                    format!("{input_calls},{output_calls}"), "edge-rgb".into(), format!("{:08x}", edge.unwrap())]);
                let cells = ["world".into(), "fill-rgb".into(), format!("{:08x}", fill.unwrap()),
                    "in-alias-count".into(), self.world.incoming_corridors.borrow().len().to_string()];
                self.emit("DRAW", "after-frame", cells);
            }
        }

        fn replay() -> Vec<Row> {
            let mut rows = Vec::new();
            {
                let mut r = Replay::new("equal-bounds-first-match-and-repeat", &mut rows);
                let a = r.genome("code-a");
                let b = r.genome("code-b");
                let b2 = r.genome("code-b2");
                let ib = IntRect { x: 40, y: 50, width: 20, height: 30 };
                let ia = r.input("in-a", ib);
                let input = r.input("in-b", ib);
                ia.borrow_mut().receive(Some(Rc::clone(&a)));
                input.borrow_mut().receive(Some(Rc::clone(&b)));
                input.borrow_mut().receive(Some(b2));
                let ta = r.traveller("traveller-a", &a);
                let tb = r.traveller("traveller-b", &b);
                let ob = IntRect { x: 100, y: 110, width: 20, height: 30 };
                let oa = r.output("out-a", ob, Some(ta));
                let output = r.output("out-b", ob, Some(tb));
                r.world.add_corridors(Rc::clone(&ia), Rc::clone(&oa));
                r.world.add_corridors(Rc::clone(&input), Rc::clone(&output));
                r.observe("before", &[ia, Rc::clone(&input)], &[oa, Rc::clone(&output)]);
                *r.world.session.0.random.borrow_mut() = JavaRandom::new(7712301);
                r.remove("remove-b", &input, &output);
                let probe = r.world.session.0.random.borrow_mut().next_i64();
                r.emit("RNG", "remove-b", ["probe".into(), "random".into(), "next-long".into(),
                    format!("{:x}", probe as u64), format!("{:x}", probe as u64)]);
                r.remove("repeat-b", &input, &output);
                r.remove("repeat-absent-b", &input, &output);
            }
            {
                let mut r = Replay::new("duplicate-aliases-and-repeated-removal", &mut rows);
                let code = r.genome("alias-code");
                let input = r.input("in-alias", IntRect { x: 70, y: 80, width: 11, height: 12 });
                input.borrow_mut().receive(Some(Rc::clone(&code)));
                let traveller = r.traveller("alias-traveller", &code);
                let output = r.output("out-alias", IntRect { x: 170, y: 180, width: 13, height: 14 }, Some(traveller));
                r.world.add_corridors(Rc::clone(&input), Rc::clone(&output));
                r.world.add_corridors(Rc::clone(&input), Rc::clone(&output));
                r.observe("before", &[Rc::clone(&input)], &[Rc::clone(&output)]);
                for stage in ["alias-remove-first", "alias-remove-second", "alias-remove-absent"] {
                    r.remove(stage, &input, &output);
                }
            }
            for (case, incoming, outgoing, stage) in [
                ("absent-members-do-not-set-sticky-flag", IntRect { x: 5, y: 6, width: 7, height: 8 },
                    IntRect { x: 15, y: 16, width: 17, height: 18 }, "remove-absent"),
                ("i32-width-height-increment-wrap", IntRect { x: 1, y: 2, width: i32::MAX, height: i32::MAX },
                    IntRect { x: 3, y: 4, width: i32::MAX, height: i32::MAX }, "remove-wrap"),
            ] {
                let mut r = Replay::new(case, &mut rows);
                let (in_name, out_name) = if stage == "remove-absent" { ("in-absent", "out-absent") } else { ("in-wrap", "out-wrap") };
                let input = r.input(in_name, incoming);
                let output = r.output(out_name, outgoing, None);
                r.observe("before", &[Rc::clone(&input)], &[Rc::clone(&output)]);
                r.remove(stage, &input, &output);
            }
            {
                let mut r = Replay::new("frame-first-equal-removed-retained-alias-active", &mut rows);
                let removed_code = r.genome("removed-code");
                let retained_code = r.genome("retained-code");
                let ib = IntRect { x: 10, y: 10, width: 40, height: 40 };
                let removed = r.input("in-removed", ib);
                let retained = r.input("in-retained", ib);
                removed.borrow_mut().receive(Some(removed_code));
                retained.borrow_mut().receive(Some(Rc::clone(&retained_code)));
                let ob = IntRect { x: 100, y: 100, width: 40, height: 40 };
                let removed_out = r.output("out-removed", ob, None);
                let retained_out = r.output("out-retained", ob, None);
                r.world.add_corridors(Rc::clone(&removed), Rc::clone(&removed_out));
                r.world.add_corridors(Rc::clone(&retained), Rc::clone(&retained_out));
                r.world.add_corridors(Rc::clone(&retained), Rc::clone(&retained_out));
                let inputs = [removed, Rc::clone(&retained)];
                let outputs = [removed_out, Rc::clone(&retained_out)];
                r.observe("before-remove", &inputs, &outputs);
                r.remove("remove-equal-first", &retained, &retained_out);
                r.frame_and_draw(&inputs, &outputs, &retained_code);
            }
            rows
        }

        fn compare_fixture(text: &str) {
            let mut lines = text.lines();
            assert_eq!(lines.next(), Some("kind\tcase\tstage\tobject\tfield\tvalue\textra1\textra2"));
            let rows: Vec<Row> = lines.map(|line| line.split('\t').map(str::to_owned).collect::<Vec<_>>()
                .try_into().expect("Eight-column frozen removal row")).collect();
            let mut ledger = BTreeMap::new();
            for row in &rows { *ledger.entry(row[0].as_str()).or_insert(0) += 1; }
            assert_eq!(ledger, BTreeMap::from([("GENOME", 6), ("LIST", 36), ("REPAINT", 18),
                ("STATE", 48), ("OBJECT", 40), ("RNG", 1), ("ADMIT", 1), ("HIT", 1), ("DRAW", 2), ("SUMMARY", 1)]));
            assert_eq!(rows.last().unwrap(), &["SUMMARY", "all", "terminal", "oracle", "cases", "5", "records", "153"].map(str::to_owned));
            // LIST36 are Source-only callbacks; SUMMARY/header are metadata, not runtime observations
            let expected: Vec<_> = rows.iter().filter(|row| row[0] != "LIST" && row[0] != "SUMMARY").collect();
            let actual = replay();
            assert_eq!(actual.len(), 108 + 3 + 6);
            assert_eq!(actual.len(), expected.len());
            for (actual, expected) in actual.iter().zip(expected) {
                assert_eq!(actual, expected, "Source removal row differs");
            }
        }

        #[test]
        fn source_corridor_removal_replay_covers_frame_and_draw() {
            compare_fixture(FIXTURE);
        }

        #[test]
        fn removal_replay_rejects_changed_repaint_and_retained_queue() {
            for (before, after) in [
                ("40,50,21,31", "40,50,22,31"),
                ("retained-code#1", "removed-code#1"),
            ] {
                let changed = FIXTURE.replacen(before, after, 1);
                assert_ne!(changed, FIXTURE);
                assert!(std::panic::catch_unwind(|| compare_fixture(&changed)).is_err());
            }
        }
    }

}
