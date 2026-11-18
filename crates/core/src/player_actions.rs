// Copyright (C) 2006-2010 Joan Queralt Molina
// Color Mod (C) 2012-2022 MarcoDBAA
// SPDX-License-Identifier: GPL-2.0-or-later

//! Selected-organism commands from VisibleWorld.java:190-386

use super::*;

/// Cloning retains the selected object's identity without cloning its model state
#[derive(Clone)]
pub struct OrganismHandle(pub(super) OrganismRef, pub(super) WorldHandle);

/// Current living-member scalars with shared identities, without phenotype or genome copies
#[derive(Clone, Debug)]
pub struct LivingOrganismSummary {
    pub handle: OrganismHandle,
    pub id: i64,
    pub center: [f64; 2],
    pub bounds: IntRect,
    pub velocity: [f64; 2],
    pub age_ticks: i32,
    pub energy: f64,
    pub mass: f64,
    pub children: i32,
    pub kills: i32,
    pub infections: i32,
    pub consumer: bool,
    pub killer: bool,
    pub temporary_color: Option<u32>,
    pub temporary_color_frames: i32,
    pub genotype_available: bool,
    pub preview_genome: Option<GenomeHandle>,
}

impl fmt::Debug for OrganismHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("OrganismHandle(..)")
    }
}

impl PartialEq for OrganismHandle {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for OrganismHandle {}

impl OrganismHandle {
    /// Share the retained owner; unlike World::clone, this does not copy model state
    pub fn owner_world(&self) -> World {
        self.1.alias()
    }

    /// Apply the selected callback with its original owner and the currently viewed World
    pub fn act_in_visible_world(
        &self,
        visible: &World,
        action: OrganismAction,
    ) -> Result<bool, WorldError> {
        self.owner_world().act_on_organism_in_view(self, action, visible)
    }
}

/// A frontend command with the original alive or dead callback guard
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrganismAction {
    Feed,
    Weaken,
    Kill,
    Revive,
    Disperse,
    Reproduce,
    Rejuvenate,
}

impl World {
    /// Visit current living members once in source membership order without changing model state
    /// Optional genotype handles share identity and support the existing no-RNG preview path
    pub fn visit_living_organisms(
        &self,
        include_preview_genomes: bool,
        mut visit: impl FnMut(LivingOrganismSummary),
    ) -> Result<(), WorldError> {
        let owner = self.handle();
        for index in 0..self.organisms.len()? {
            let reference = self.organisms.actor(index)?;
            let actor = reference.borrow();
            if !actor.state.alive {
                continue;
            }
            let summary = LivingOrganismSummary {
                handle: OrganismHandle(Rc::clone(&reference), owner.clone()),
                id: actor.id,
                center: actor.state.center,
                bounds: actor.state.bounds.get(),
                velocity: actor.state.velocity,
                age_ticks: actor.state.age,
                energy: actor.state.energy,
                mass: actor.body.mass,
                children: actor.state.total_children,
                kills: actor.state.total_kills,
                infections: actor.state.total_infected,
                consumer: actor.effects.is_consumer,
                killer: actor.effects.is_killer,
                temporary_color: actor.state.temporary_argb(),
                temporary_color_frames: actor.state.temporary_color_frames,
                genotype_available: actor.genome.is_some(),
                preview_genome: if include_preview_genomes {
                    actor.genome.as_ref().map(|genome| GenomeHandle::from_shared(Rc::clone(genome)))
                } else {
                    None
                },
            };
            drop(actor);
            visit(summary);
        }
        Ok(())
    }

    /// Pick the first containing live member, otherwise the last containing corpse
    pub fn pick_organism(&self, position: [i32; 2]) -> Result<Option<OrganismHandle>, WorldError> {
        Ok(self.find_organism_from_position(position)?.map(|actor| OrganismHandle(actor, self.handle())))
    }

    /// Read a living member's center without cloning its phenotype or genome
    pub fn tracking_center(&self, handle: &OrganismHandle) -> Option<[i32; 2]> {
        if !self.contains_handle(handle) {
            return None;
        }
        let actor = handle.0.borrow();
        actor.state.alive.then_some(actor.state.integer_center)
    }

    /// Inspect a current member without exposing mutable model ownership
    pub fn inspect_handle(&self, handle: &OrganismHandle) -> Option<OrganismSnapshot> {
        self.contains_handle(handle)
            .then(|| handle.0.borrow().snapshot())
    }

    pub(super) fn contains_handle(&self, handle: &OrganismHandle) -> bool {
        self.organisms.contains(&handle.0)
    }

    /// Return false when the source alive guard rejects the action
    /// A true result means callback acceptance, including partial or absent energy payment
    /// Foreign and removed handles fail before any mutation
    pub fn act_on_organism(
        &self,
        handle: &OrganismHandle,
        action: OrganismAction,
    ) -> Result<bool, WorldError> {
        self.act_on_organism_in_view(handle, action, self)
    }

    fn act_on_organism_in_view(
        &self,
        handle: &OrganismHandle,
        action: OrganismAction,
        visible: &World,
    ) -> Result<bool, WorldError> {
        if !self.contains_handle(handle) {
            return Err(WorldError::InvalidOrganismHandle);
        }
        let reference = Rc::clone(&handle.0);
        let mut actor = reference.borrow_mut();
        let needs_live = !matches!(action, OrganismAction::Revive | OrganismAction::Disperse);
        if actor.state.alive != needs_live {
            return Ok(false);
        }
        match action {
            OrganismAction::Feed => {
                let amount = crate::java_math::min(10.0, visible.carbon_dioxide.get());
                // Source Feed uses current-world gases and the selected object's energy
                visible.carbon_dioxide.set(visible.carbon_dioxide.get() + amount);
                actor.state.energy += amount;
                visible.oxygen.set(visible.oxygen.get() + amount);
            }
            OrganismAction::Weaken => {
                let amount = actor.state.energy / 2.0;
                actor.use_energy(amount)?;
            }
            OrganismAction::Kill => self.die(&mut actor, None)?,
            OrganismAction::Revive => self.revive_member(&mut actor, visible)?,
            OrganismAction::Disperse => {
                let amount = actor.state.energy;
                actor.use_energy(amount)?;
            }
            OrganismAction::Reproduce => self.reproduce_from(&reference, &mut actor, None)?,
            OrganismAction::Rejuvenate => actor.state.age = 0,
        }
        Ok(true)
    }

    fn revive_member(&self, actor: &mut Organism, visible: &World) -> Result<(), WorldError> {
        for index in actor.body.indices() {
            let destination = actor.body.phenotype_colors.clone();
            let code = primary_genome(&actor.genome)?.borrow();
            let value = code.gene_for_segment(index)?.borrow().color_handle().cloned();
            crate::Body::store_color(destination, index, value)?;
        }
        actor.state.active = true;
        actor.state.alive = true;
        actor.state.has_moved = true;
        actor.state.age = 1;
        actor.state.reproduction.maximum_age =
            self.session.0.parameters.borrow().effects.max_age.wrapping_add(
                actor.body.segment_count / self.session.0.parameters.borrow().effects.age_divisor,
            );
        let effects = &mut actor.effects;
        effects.is_plant = false;
        effects.is_consumer = false;
        effects.is_fungus = false;
        effects.is_enhanced = false;
        effects.is_killer = false;
        effects.is_gray = false;
        effects.is_reproductive = false;
        effects.is_infectious = false;
        effects.is_coral = false;
        effects.is_auburn = false;
        effects.is_blond = false;
        effects.is_silver = false;
        effects.segment_pre_touch_effects = 0;
        effects.transfers_energy = false;
        effects.friend_length = -1;
        effects.friend_theta = -1;
        effects.jade_factor = 0;
        effects.sky_version = 0;
        effects.fallow_version = 0;
        effects.plague_version = 0;
        effects.reproduce_late = 0;
        effects.can_dodge = false;
        effects.dodge = false;
        effects.has_dodged = true;
        effects.is_frozen = false;
        effects.all_frozen = false;
        effects.is_injured = false;
        let genome = actor.genome.as_ref().map(|code| code.borrow());
        effects.apply(
            genome.as_deref(),
            &mut actor.body,
            actor.state.age,
            &mut actor.state.reproduction,
            &self.session.0.parameters.borrow().effects,
        )?;
        visible.increase_population();
        Ok(())
    }
}

#[cfg(test)]
mod living_summary_tests {
    use super::*;

    #[test]
    fn living_visit_reads_actor_values_once_without_copying_or_changing_the_world() {
        let world = World::new(WorldConfig {
            seed: 827364, width: 1200, height: 800, initial_attempts: 0,
        }).unwrap();
        let code = GenomeHandle::from_owned(Genome::from_parts(
            &[crate::Gene::new(12.0, 0.0, crate::ColorKind::Green.argb(), 0, [0; 36]).shared()],
            crate::GenomeProperties { symmetry: 1, mutation_rate: 0, ..crate::GenomeProperties::default() },
        ));
        let first = world.paste_genome_handle(&code, [200, 200]).unwrap().unwrap();
        let second = world.paste_genome_handle(&code, [500, 400]).unwrap().unwrap();
        first.0.borrow_mut().state.total_children = 3;
        second.0.borrow_mut().state.total_children = 12;
        second.0.borrow_mut().state.total_kills = 7;
        second.0.borrow_mut().state.total_infected = 2;
        second.0.borrow_mut().state.age = 513;
        let random = world.random_snapshot();
        let limits = crate::object_stream::Limits::default();
        let before = world.encode_legacy_file_bytes(limits, limits.allocated_bytes).unwrap();
        let mut rows = Vec::new();
        world.visit_living_organisms(true, |row| rows.push(row)).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].handle, first);
        assert_eq!(rows[1].handle, second);
        assert_eq!([rows[0].children, rows[1].children], [3, 12]);
        assert_eq!([rows[1].kills, rows[1].infections, rows[1].age_ticks], [7, 2, 513]);
        assert_eq!(rows[0].preview_genome, Some(code.clone()));
        assert_eq!(rows[1].preview_genome, Some(code));
        assert_eq!(world.random_snapshot(), random);
        assert_eq!(world.encode_legacy_file_bytes(limits, limits.allocated_bytes).unwrap(), before);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColorKind, Gene, GenomeProperties, ReproductionState};

    fn world() -> World {
        World::new(WorldConfig {
            seed: 827364,
            initial_attempts: 0,
            ..WorldConfig::default()
        })
        .unwrap()
    }

    fn member(world: &mut World, colors: &[u32], center: [i32; 2]) -> OrganismHandle {
        let genes: Vec<_> = colors
            .iter()
            .map(|color| Gene::new(12.0, 0.0, Some(*color), 0, [0; 36]).shared())
            .collect();
        let genome = Genome::from_parts(
            &genes,
            GenomeProperties {
                symmetry: 1,
                ..GenomeProperties::default()
            },
        );
        world.paste_genome(genome, center).unwrap().unwrap();
        let reference = Rc::clone(world.organisms.borrow().last().unwrap());
        let point = {
            let actor = reference.borrow();
            [actor.state.bounds.get().x, actor.state.bounds.get().y]
        };
        let handle = world.pick_organism(point).unwrap().unwrap();
        assert!(Rc::ptr_eq(&handle.0, &reference));
        handle
    }

    #[test]
    fn handles_keep_identity_and_reject_foreign_or_removed_members_even_with_retained_tree_refs() {
        let mut world = world();
        let first = member(&mut world, &[0xff00ff00], [100, 100]);
        let second = member(&mut world, &[0xff00ff00], [300, 100]);
        second.0.borrow_mut().id = first.0.borrow().id;
        assert_eq!(first, first.clone());
        assert_ne!(first, second);
        let copied = world.clone();
        let bounds = first.0.borrow().state.bounds.get();
        let copied_handle = copied.pick_organism([bounds.x, bounds.y]).unwrap().unwrap();
        assert_ne!(first, copied_handle);
        assert!(copied.inspect_handle(&first).is_none());
        assert_eq!(
            copied.act_on_organism(&first, OrganismAction::Feed),
            Err(WorldError::InvalidOrganismHandle)
        );
        world.rebuild_spatial();
        world.organisms.borrow_mut().remove(0);
        assert!(Rc::ptr_eq(&world.frame_objects.borrow()[0], &first.0));
        let before = world.clone();
        assert!(world.inspect_handle(&first).is_none());
        assert_eq!(
            world.act_on_organism(&first, OrganismAction::Revive),
            Err(WorldError::InvalidOrganismHandle)
        );
        assert_eq!(world, before);
        assert!(world.inspect_handle(&second).is_some());
    }

    #[test]
    fn feed_preserves_additive_co2_negative_gases_and_signed_zero() {
        let mut world = world();
        let handle = member(&mut world, &[0xff00ff00], [100, 100]);
        for (carbon, oxygen, energy, expected) in [
            (20.0, 2.0, 3.0, [30.0, 12.0, 13.0]),
            (3.0, 1.0, 3.0, [6.0, 4.0, 6.0]),
            (-4.0, 1.0, 3.0, [-8.0, -3.0, -1.0]),
            (-0.0, -0.0, -0.0, [-0.0, -0.0, -0.0]),
        ] {
            world.carbon_dioxide.set(carbon);
            world.oxygen.set(oxygen);
            handle.0.borrow_mut().state.energy = energy;
            assert_eq!(
                world.act_on_organism(&handle, OrganismAction::Feed),
                Ok(true)
            );
            let actual = [
                world.carbon_dioxide.get(),
                world.oxygen.get(),
                handle.0.borrow().state.energy,
            ];
            assert_eq!(actual.map(f64::to_bits), expected.map(f64::to_bits));
        }
    }

    #[test]
    fn weaken_and_disperse_accept_partial_or_absent_payment_without_claiming_full_energy_change() {
        let mut world = world();
        let handle = member(&mut world, &[0xff00ff00], [100, 100]);
        handle.0.borrow_mut().state.energy = 8.0;
        world.oxygen.set(1.0);
        world.carbon_dioxide.set(20.0);
        assert_eq!(
            world.act_on_organism(&handle, OrganismAction::Weaken),
            Ok(true)
        );
        assert_eq!(handle.0.borrow().state.energy, 7.0);
        assert_eq!((world.oxygen.get(), world.carbon_dioxide.get()), (0.0, 21.0));
        assert_eq!(
            world.act_on_organism(&handle, OrganismAction::Weaken),
            Ok(true)
        );
        assert_eq!(handle.0.borrow().state.energy, 7.0);
        assert_eq!(
            world.act_on_organism(&handle, OrganismAction::Kill),
            Ok(true)
        );
        world.oxygen.set(2.0);
        assert_eq!(
            world.act_on_organism(&handle, OrganismAction::Disperse),
            Ok(true)
        );
        assert_eq!(handle.0.borrow().state.energy, 5.0);
        assert_eq!((world.oxygen.get(), world.carbon_dioxide.get()), (0.0, 23.0));
    }

    #[test]
    fn source_alive_guards_preserve_complete_state_and_kill_does_not_repeat_events() {
        let mut world = world();
        let handle = member(&mut world, &[0xff00ff00], [100, 100]);
        for action in [OrganismAction::Revive, OrganismAction::Disperse] {
            let before = world.clone();
            assert_eq!(world.act_on_organism(&handle, action), Ok(false));
            assert_eq!(world, before);
        }
        world
            .act_on_organism(&handle, OrganismAction::Kill)
            .unwrap();
        for action in [
            OrganismAction::Feed,
            OrganismAction::Weaken,
            OrganismAction::Kill,
            OrganismAction::Reproduce,
            OrganismAction::Rejuvenate,
        ] {
            let before = world.clone();
            assert_eq!(world.act_on_organism(&handle, action), Ok(false));
            assert_eq!(world, before);
        }
    }

    #[test]
    fn revive_restores_source_flags_and_colors_without_life_initialization_or_geometry_rebuild() {
        let mut world = world();
        let handle = member(
            &mut world,
            &[
                0xff00ff00,
                ColorKind::Eye.argb().unwrap(),
                ColorKind::Purple.argb().unwrap(),
            ],
            [100, 100],
        );
        world
            .act_on_organism(&handle, OrganismAction::Kill)
            .unwrap();
        let (body, rotated, infected, random) = {
            let mut actor = handle.0.borrow_mut();
            actor.state.energy = 17.0;
            actor.state.velocity = [2.0, -3.0];
            actor.state.angular_velocity = 0.125;
            actor.state.reproduction = ReproductionState {
                early_energy: 999,
                energy: 777,
                infected_energy: -15,
                countdown: 8,
                maximum_countdown: 19,
                maximum_age: -123,
            };
            actor.infected_genome = Some(Rc::clone(actor.genome.as_ref().unwrap()));
            actor.saved_genome = Some(Rc::clone(actor.genome.as_ref().unwrap()));
            actor.effects.is_consumer = true;
            actor.effects.is_fungus = true;
            actor.effects.is_enhanced = true;
            actor.effects.is_killer = true;
            actor.effects.is_gray = true;
            actor.effects.is_reproductive = true;
            actor.effects.is_infectious = true;
            actor.effects.is_coral = true;
            actor.effects.is_auburn = true;
            actor.effects.is_blond = true;
            actor.effects.is_silver = true;
            actor.effects.is_frozen = true;
            actor.effects.all_frozen = true;
            actor.effects.is_injured = true;
            actor.effects.is_spike = true;
            actor.effects.use_extra_effects = true;
            actor.effects.can_move = true;
            actor.body.set_photo(2, -99.0).unwrap();
            (
                actor.body.clone(),
                actor.body.rotated_values().unwrap(),
                Rc::clone(actor.genome.as_ref().unwrap()),
                world.random_snapshot(),
            )
        };
        assert_eq!(
            world.act_on_organism(&handle, OrganismAction::Revive),
            Ok(true)
        );
        let actor = handle.0.borrow();
        assert!(actor.state.alive && actor.state.active && actor.state.has_moved);
        assert_eq!(actor.state.age, 1);
        assert_eq!(actor.state.energy, 17.0);
        assert_eq!(actor.state.velocity, [2.0, -3.0]);
        assert_eq!(actor.state.angular_velocity, 0.125);
        assert_eq!(
            (
                actor.state.reproduction.energy,
                actor.state.reproduction.infected_energy,
                actor.state.reproduction.countdown,
                actor.state.reproduction.maximum_countdown
            ),
            (777, -15, 8, 19)
        );
        assert_eq!(actor.state.reproduction.maximum_age, 6);
        assert!(Rc::ptr_eq(
            actor.infected_genome.as_ref().unwrap(),
            &infected
        ));
        assert!(Rc::ptr_eq(actor.saved_genome.as_ref().unwrap(), &infected));
        assert!(actor.effects.is_plant && actor.body.eyes.enabled);
        assert!(
            !actor.effects.is_consumer && !actor.effects.is_fungus && !actor.effects.is_infectious
        );
        assert!(!actor.effects.is_frozen && !actor.effects.all_frozen && !actor.effects.is_injured);
        assert!(
            actor.effects.is_spike && actor.effects.use_extra_effects && actor.effects.can_move
        );
        assert_eq!(actor.body.photo(2).unwrap(), -99.0);
        assert_eq!(
            (actor.body.indices()).map(|index| actor.body.color_argb(index).unwrap())
                .collect::<Vec<_>>(),
            [
                0xff00ff00,
                ColorKind::Eye.argb().unwrap(),
                ColorKind::Purple.argb().unwrap()
            ]
        );
        assert_eq!(actor.body.mass.to_bits(), body.mass.to_bits());
        assert_eq!(actor.body.inertia.to_bits(), body.inertia.to_bits());
        assert_eq!(actor.body.growth_ratio, body.growth_ratio);
        assert_eq!(actor.body.rotated_values().unwrap(), rotated);
        assert_eq!(world.random_snapshot(), random);
        assert_eq!(world.population(), 1);
    }

    #[test]
    fn forced_reproduction_uses_the_current_held_parent_and_rejuvenate_only_resets_age() {
        let mut world = world();
        let handle = member(&mut world, &[0xff00ff00], [100, 100]);
        handle.0.borrow_mut().state.energy = 500.0;
        assert_eq!(
            world.act_on_organism(&handle, OrganismAction::Reproduce),
            Ok(true)
        );
        assert!(world.organisms.borrow().len() > 1);
        assert_eq!(
            handle.0.borrow().state.total_children as usize,
            world.organisms.borrow().len() - 1
        );
        handle.0.borrow_mut().state.age = i32::MIN;
        let before = handle.0.borrow().snapshot();
        let random = world.random_snapshot();
        assert_eq!(
            world.act_on_organism(&handle, OrganismAction::Rejuvenate),
            Ok(true)
        );
        let mut expected = before;
        expected.state.age = 0;
        assert_eq!(world.inspect_handle(&handle).unwrap(), expected);
        assert_eq!(world.random_snapshot(), random);
    }

    #[test]
    fn raw_missing_gene_colors_and_inconsistent_empty_genomes_fail_without_color_normalization() {
        let mut world = world();
        let handle = member(&mut world, &[0xff00ff00], [100, 100]);
        world
            .act_on_organism(&handle, OrganismAction::Kill)
            .unwrap();
        handle.0.borrow().genome.as_ref().unwrap().borrow().genes().unwrap()[0]
            .borrow_mut()
            .set_color(None);
        let energy = handle.0.borrow().state.energy;
        assert_eq!(
            world.act_on_organism(&handle, OrganismAction::Revive),
            Err(WorldError::Geometry(GeometryError::MissingPhenotypeColor {
                segment: 0
            }))
        );
        assert!(handle.0.borrow().state.alive && handle.0.borrow().state.active);
        assert_eq!(handle.0.borrow().state.age, 1);
        assert_eq!(handle.0.borrow().state.energy, energy);
        assert!(handle.0.borrow().body.color(0).unwrap().is_none());
        handle.0.borrow_mut().state.alive = false;
        handle.0.borrow_mut().state.active = false;
        handle.0.borrow_mut().genome = Some(Rc::new(RefCell::new(Genome::from_parts(
            &[],
            GenomeProperties {
                symmetry: 1,
                ..GenomeProperties::default()
            },
        ))));
        assert_eq!(
            world.act_on_organism(&handle, OrganismAction::Revive),
            Err(WorldError::Geometry(GeometryError::Genetics(
                crate::GeneticsError::ZeroGeneDivisor
            )))
        );
        assert!(!handle.0.borrow().state.alive);
        assert_eq!(world.population(), 0);
    }

    #[test]
    fn retained_selection_alias_preserves_owner_identity_and_strict_foreign_rejection() {
        let mut original = world();
        let actor = member(&mut original, &[0xff00ff00], [100, 100]);
        let alias = actor.owner_world();
        assert!(alias.handle().same_owner(&original.handle()));
        let mut visible = world();
        let collision = member(&mut visible, &[0xff00ff00], [100, 100]);
        collision.0.borrow_mut().id = actor.0.borrow().id;
        assert_ne!(actor, collision);
        assert!(visible.inspect_handle(&actor).is_none());
        assert_eq!(visible.act_on_organism(&actor, OrganismAction::Feed), Err(WorldError::InvalidOrganismHandle));
        drop(original);
        assert!(alias.inspect_handle(&actor).is_some());
        assert!(actor.act_in_visible_world(&visible, OrganismAction::Rejuvenate).unwrap());
        alias.organisms.borrow_mut().clear();
        assert!(actor.owner_world().inspect_handle(&actor).is_none());
        let before = visible.clone();
        assert_eq!(actor.act_in_visible_world(&visible, OrganismAction::Revive), Err(WorldError::InvalidOrganismHandle));
        assert_eq!(visible, before);
    }

    #[test]
    fn retained_selected_callbacks_separate_visible_gases_and_population_from_actor_owner() {
        use crate::statistics::tests::{Observation, observed_field};

        let original = world();
        let visible = world();
        let make_actor = |world: &World| {
            let code = crate::GenomeHandle::from_owned(Genome::from_parts(
                &[Gene::new(18.0, 0.0, Some(0xff00ff00), 0, [0; 36]).shared()],
                GenomeProperties { symmetry: 1, ..GenomeProperties::default() },
            ));
            world.paste_genome_handle(&code, [256, 256]).unwrap().unwrap()
        };
        let actor = make_actor(&original);
        let other = make_actor(&visible);
        original.carbon_dioxide.set(1000.0);
        original.oxygen.set(1000.0);
        visible.carbon_dioxide.set(2000.0);
        visible.oxygen.set(2000.0);
        actor.0.borrow_mut().state.energy = 80.0;
        let loaded_before = visible.inspect_handle(&other).unwrap();
        assert_eq!(original.inspect_handle(&actor).unwrap().id, loaded_before.id);
        let counts = |world: &World| {
            let statistics = world.statistics_ref();
            let statistics = statistics.borrow();
            ["createdOrganisms", "birthSum", "birthLastTime", "deathSum", "deathLastTime"].map(|field| {
                match observed_field(&statistics, field) {
                    Observation::Int(value) => i64::from(value),
                    Observation::Long(value) => value,
                    _ => panic!("Expected an integer Source counter"),
                }
            })
        };
        assert_eq!(counts(&original), [1, 0, 0, 0, 0]);
        assert_eq!(counts(&visible), [1, 0, 0, 0, 0]);
        // Literal snapshots from the original Java retained-selection observations
        for (stage, action, energy, alive, age, co2, oxygen, old_population, current_population, deaths) in [
            ("feed", OrganismAction::Feed, 90.0, true, 0, 1000.0, 1000.0, 1, 1, 0),
            ("weaken", OrganismAction::Weaken, 45.0, true, 0, 1045.0, 955.0, 1, 1, 0),
            ("kill", OrganismAction::Kill, 45.0, false, 0, 1045.0, 955.0, 0, 1, 1),
            ("revive", OrganismAction::Revive, 45.0, true, 1, 1045.0, 955.0, 0, 2, 1),
            ("kill-before-disperse", OrganismAction::Kill, 45.0, false, 1, 1045.0, 955.0, -1, 2, 2),
            ("disperse", OrganismAction::Disperse, 0.0, false, 1, 1090.0, 910.0, -1, 2, 2),
        ] {
            assert!(actor.act_in_visible_world(&visible, action).unwrap(), "{stage}");
            let selected = original.inspect_handle(&actor).unwrap();
            assert_eq!(selected.state.energy, energy, "{stage}");
            assert_eq!((selected.state.alive, selected.state.age, selected.state.total_children), (alive, age, 0), "{stage}");
            assert_eq!((original.carbon_dioxide.get(), original.oxygen.get()), (co2, oxygen), "{stage}");
            assert_eq!((visible.carbon_dioxide.get(), visible.oxygen.get()), (2010.0, 2010.0), "{stage}");
            assert_eq!((original.population.get(), visible.population.get()), (old_population, current_population), "{stage}");
            assert_eq!((original.organisms.borrow().len(), visible.organisms.borrow().len()), (1, 1), "{stage}");
            assert_eq!(counts(&original), [1, 0, 0, deaths, deaths], "{stage}");
            assert_eq!(counts(&visible), [1, 0, 0, 0, 0], "{stage}");
            assert_eq!(visible.inspect_handle(&other).unwrap(), loaded_before, "{stage}");
            assert!(visible.inspect_handle(&actor).is_none(), "{stage}");
        }
    }

}

#[cfg(test)]
#[path = "player_actions_source_tests.rs"]
pub(super) mod source_tests;
