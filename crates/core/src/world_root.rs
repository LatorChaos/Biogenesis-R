// SPDX-License-Identifier: GPL-2.0-or-later

//! Stable complete World ownership and explicit session identity

use super::*;
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use std::rc::Weak;

#[derive(Debug, PartialEq)]
pub(super) struct SessionState {
    pub(super) parameters: RefCell<WorldParameters>,
    pub(super) random: RefCell<JavaRandom>,
}

/// Shared runtime settings and random stream, without a backedge to any World
#[derive(Clone, Debug, PartialEq)]
pub struct SessionRef(pub(super) Rc<SessionState>);

impl SessionRef {
    pub fn new(parameters: WorldParameters, random: JavaRandom) -> Self {
        Self(Rc::new(SessionState {
            parameters: RefCell::new(parameters),
            random: RefCell::new(random),
        }))
    }

    pub fn parameters(&self) -> WorldParameters {
        self.0.parameters.borrow().clone()
    }

    pub fn random_snapshot(&self) -> JavaRandom {
        self.0.random.borrow().clone()
    }

    pub fn same_owner(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    pub fn apply_settings_parameters(&self, settings: &crate::SettingsState) {
        let source = settings.parameters();
        let mut target = self.0.parameters.borrow_mut();
        target.atmosphere = source.atmosphere;
        target.physics = source.physics;
        target.lifecycle = source.lifecycle;
        target.genetics = source.genetics.clone();
        target.effects = source.effects.clone();
        target.initial_energy = source.initial_energy;
        target.spore_energy_consumption = source.spore_energy_consumption;
    }
}

/// The complete payload has no external publication lease or whole-owner borrow
#[derive(Debug)]
pub struct WorldState {
    pub(super) config: Cell<WorldConfig>,
    pub(super) session: SessionRef,
    pub(super) organisms: crate::membership::Membership,
    pub(super) frame_objects: RefCell<Vec<OrganismRef>>,
    pub(super) rectangle_context: RefCell<Option<Rc<crate::rectangles::RectangleActorContext>>>,
    pub(super) incoming_corridors: RefCell<Vec<IncomingCorridorRef>>,
    pub(super) outgoing_corridors: RefCell<Vec<OutgoingCorridorRef>>,
    pub(super) corridors_exist: Cell<bool>,
    pub(super) spatial: RefCell<SpatialTree>,
    pub(super) next_id: Cell<i32>,
    pub(super) population: Cell<i32>,
    pub(super) frame: Cell<i32>,
    pub(super) statistics: RefCell<Option<Rc<RefCell<WorldStatistics>>>>,
    pub(super) counter_observer: RefCell<Weak<super::selection_counters::CounterState>>,
    pub(super) feeding_observer: RefCell<Weak<super::feeding_observer::FeedingState>>,
    pub(super) tick: Cell<u64>,
    pub(super) paused: Cell<bool>,
    pub(super) oxygen: Cell<f64>,
    pub(super) carbon_dioxide: Cell<f64>,
    pub(super) methane: Cell<f64>,
}

/// Nullable saved identity; an expired present target is a retention failure
#[derive(Clone, Default)]
pub struct WorldAssociation(Option<Weak<WorldState>>);

impl WorldAssociation {
    pub(crate) fn resolve(&self) -> Result<World, WorldError> {
        let weak = self.0.as_ref().ok_or(WorldError::MissingWorld)?;
        let state = weak.upgrade().ok_or(WorldError::ExpiredWorld)?;
        Ok(World { state, lease: None, rectangle_lease: None })
    }

    pub(crate) fn is_present(&self) -> bool {
        self.0.is_some()
    }

    pub(crate) fn target(&self) -> Result<Option<Rc<WorldState>>, WorldError> {
        self.0.as_ref().map(|weak| weak.upgrade().ok_or(WorldError::ExpiredWorld)).transpose()
    }

    pub(crate) fn identity(&self) -> Option<usize> {
        self.0.as_ref().map(|value| value.as_ptr() as usize)
    }

    pub(crate) fn retarget(&mut self, worlds: &HashMap<usize, Weak<WorldState>>) {
        if let Some(identity) = self.identity() {
            self.0 = Some(worlds.get(&identity).expect("Complete World copy map").clone());
        }
    }
}
impl PartialEq for WorldAssociation {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (None, None) => true,
            (Some(left), Some(right)) => Weak::ptr_eq(left, right),
            _ => false,
        }
    }
}
impl std::fmt::Debug for WorldAssociation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("WorldAssociation").field(&self.identity()).finish()
    }
}

#[derive(Debug)]
struct WorldCohort {
    worlds: Vec<Rc<WorldState>>,
    actors: Vec<OrganismRef>,
}

/// All saved owners are validated before the unpublished actor backedges are filled
pub(super) struct StoredWorldParts {
    pub width: i32,
    pub height: i32,
    pub next_id: i32,
    pub population: i32,
    pub frame: i32,
    pub corridors_exist: bool,
    pub oxygen: f64,
    pub carbon_dioxide: f64,
    pub methane: f64,
    pub spatial: SpatialTree,
}

/// Owned root; Clone retains the existing detached graph-copy contract
#[derive(Debug)]
pub struct World {
    state: Rc<WorldState>,
    lease: Option<Rc<WorldCohort>>,
    rectangle_lease: Option<Rc<crate::rectangles::RectangleActorContext>>,
}

impl Deref for World {
    type Target = WorldState;
    fn deref(&self) -> &Self::Target { &self.state }
}

/// Identity-retaining external handle, distinct from World::clone
#[derive(Clone, Debug)]
pub struct WorldHandle {
    state: Rc<WorldState>,
    lease: Option<Rc<WorldCohort>>,
    rectangle_lease: Option<Rc<crate::rectangles::RectangleActorContext>>,
}
impl PartialEq for WorldHandle {
    fn eq(&self, other: &Self) -> bool { Rc::ptr_eq(&self.state, &other.state) }
}
impl Eq for WorldHandle {}

impl WorldHandle {
    pub fn same_owner(&self, other: &Self) -> bool { self == other }
    pub fn session(&self) -> SessionRef { self.state.session.clone() }
    pub fn oxygen(&self) -> f64 { self.state.oxygen.get() }
    pub fn carbon_dioxide(&self) -> f64 { self.state.carbon_dioxide.get() }
    pub(crate) fn alias(&self) -> World {
        World { state: Rc::clone(&self.state), lease: self.lease.clone(), rectangle_lease: self.rectangle_lease.clone() }
    }
}

impl World {
    pub(super) fn stage_legacy_root(&self, parts: StoredWorldParts) -> Self {
        let mut config = self.config.get();
        config.width = parts.width;
        config.height = parts.height;
        Self {
            state: Rc::new(WorldState {
                config: Cell::new(config),
                session: self.session.clone(),
                organisms: crate::membership::Membership::pending(),
                frame_objects: RefCell::new(Vec::new()),
                rectangle_context: RefCell::new(None),
                incoming_corridors: RefCell::new(Vec::new()),
                outgoing_corridors: RefCell::new(Vec::new()),
                corridors_exist: Cell::new(parts.corridors_exist),
                spatial: RefCell::new(parts.spatial),
                next_id: Cell::new(parts.next_id),
                population: Cell::new(parts.population),
                counter_observer: RefCell::new(Weak::new()),
                feeding_observer: RefCell::new(Weak::new()),
                frame: Cell::new(parts.frame),
                statistics: RefCell::new(None),
                tick: Cell::new(self.tick.get()),
                paused: Cell::new(self.paused.get()),
                oxygen: Cell::new(parts.oxygen),
                carbon_dioxide: Cell::new(parts.carbon_dioxide),
                methane: Cell::new(parts.methane),
            }),
            lease: None,
            rectangle_lease: None,
        }
    }

    pub(super) fn fill_legacy_references(
        &self,
        members: Option<crate::shared_world_owners::SharedListRef>,
        statistics: Rc<RefCell<WorldStatistics>>,
        tree: crate::shared_world_owners::TreeRef,
        tokens: Vec<Weak<RefCell<Organism>>>,
        context: Rc<crate::rectangles::RectangleActorContext>,
    ) {
        debug_assert!(self.rectangle_context.borrow().is_none());
        self.organisms.initialize(members);
        self.statistics.replace(Some(statistics));
        self.spatial.borrow().initialize(tree, tokens);
        self.rectangle_context.replace(Some(context));
    }

    #[cfg(test)]
    pub(super) fn spatial_actor(&self, token: usize) -> OrganismRef {
        let spatial = self.spatial.borrow();
        if spatial.imported() { return spatial.token_actor(token).expect("A snapshot token resolves its actual Actor owner"); }
        Rc::clone(&self.frame_objects.borrow()[token])
    }

    fn logical_graph_actors(&self) -> Vec<OrganismRef> {
        let mut lists = Vec::new();
        if let Some(members) = self.organisms.shared_ref() { lists.push(members.clone()); }
        for history in self.statistics_ref().borrow().history_refs().into_iter().flatten() { lists.push(history.shared_ref().clone()); }
        crate::shared_world_owners::referenced_actors(&lists, std::slice::from_ref(self.spatial.borrow().shared_ref()))
    }

    /// External handles retain their old context; only current logical roots seed this handoff
    pub(super) fn retire_imported_actors(&self) -> Result<(), WorldError> {
        let context_slot = self.rectangle_context.borrow();
        let Some(context) = context_slot.as_ref() else { return Ok(()); };
        let actors = context.actors();
        let failed = || WorldError::ImportRetentionAllocation;
        let mut by_owner = HashMap::new();
        by_owner.try_reserve(actors.len()).map_err(|_| failed())?;
        let mut marked = Vec::new();
        marked.try_reserve_exact(actors.len()).map_err(|_| failed())?;
        marked.resize(actors.len(), false);
        let mut pending = Vec::new();
        pending.try_reserve_exact(actors.len()).map_err(|_| failed())?;
        for (index, actor) in actors.iter().enumerate() {
            by_owner.insert(Rc::as_ptr(actor) as usize, index);
        }
        let enqueue = |actor: &OrganismRef, marked: &mut [bool], pending: &mut Vec<usize>| {
            if let Some(index) = by_owner.get(&(Rc::as_ptr(actor) as usize)).copied() {
                if !marked[index] { marked[index] = true; pending.push(index); }
            }
        };
        let visit = |actor: &OrganismRef, marked: &mut [bool], pending: &mut Vec<usize>| {
            enqueue(actor, marked, pending);
            let actor = actor.borrow();
            for rectangle in [Some(&actor.state.bounds), actor.body.size_rect.as_ref(), actor.state.last_frame.as_ref()]
                .into_iter().flatten()
            {
                if let Some(target) = rectangle.actor_target() { enqueue(&target, marked, pending); }
            }
        };
        for actor in self.logical_graph_actors().iter().chain(self.frame_objects.borrow().iter()) {
            visit(actor, &mut marked, &mut pending);
        }
        for corridor in self.incoming_corridors.borrow().iter() {
            if let Some(actor) = &corridor.borrow().candidate { visit(actor, &mut marked, &mut pending); }
        }
        for corridor in self.outgoing_corridors.borrow().iter() {
            if let Some(actor) = &corridor.borrow().traveller { visit(actor, &mut marked, &mut pending); }
        }
        while let Some(index) = pending.pop() { visit(&actors[index], &mut marked, &mut pending); }
        let mut retained = Vec::new();
        retained.try_reserve_exact(marked.iter().filter(|value| **value).count()).map_err(|_| failed())?;
        retained.extend(actors.iter().zip(marked).filter_map(|(actor, marked)| marked.then(|| Rc::clone(actor))));
        let replacement = if retained.is_empty() { None } else {
            Some(Rc::new(crate::rectangles::RectangleActorContext::validated(
                retained, context.retained_bytes(), context.maximum_bytes(),
            )))
        };
        drop(context_slot);
        self.rectangle_context.replace(replacement);
        Ok(())
    }

    pub fn handle(&self) -> WorldHandle {
        WorldHandle {
            state: Rc::clone(&self.state),
            lease: self.lease.clone(),
            rectangle_lease: self.rectangle_lease.clone().or_else(|| self.rectangle_context.borrow().clone()),
        }
    }

    pub fn session(&self) -> SessionRef { self.session.clone() }

    pub fn random_snapshot(&self) -> JavaRandom { self.session.random_snapshot() }

    pub(crate) fn association(&self) -> WorldAssociation {
        WorldAssociation(Some(Rc::downgrade(&self.state)))
    }

    pub(crate) fn statistics_ref(&self) -> Rc<RefCell<WorldStatistics>> {
        Rc::clone(self.statistics.borrow().as_ref().expect("World statistics is filled before publication"))
    }

    pub(crate) fn empty_in_session(config: WorldConfig, session: SessionRef) -> Self {
        let parameters = session.parameters();
        let atmosphere = parameters.atmosphere;
        Self {
            state: Rc::new(WorldState {
                config: Cell::new(config),
                session,
                organisms: crate::membership::Membership::empty(0),
                frame_objects: RefCell::new(Vec::new()),
                rectangle_context: RefCell::new(None),
                incoming_corridors: RefCell::new(Vec::new()),
                outgoing_corridors: RefCell::new(Vec::new()),
                corridors_exist: Cell::new(false),
                spatial: RefCell::new(SpatialTree::empty()),
                next_id: Cell::new(0),
                population: Cell::new(0),
                counter_observer: RefCell::new(Weak::new()),
                feeding_observer: RefCell::new(Weak::new()),
                frame: Cell::new(0),
                statistics: RefCell::new(Some(WorldStatistics::new(
                    config.initial_attempts as i32,
                    atmosphere.initial_oxygen,
                    atmosphere.initial_carbon_dioxide,
                    atmosphere.initial_methane,
                ).into_shared())),
                tick: Cell::new(0),
                paused: Cell::new(true),
                oxygen: Cell::new(0.0),
                carbon_dioxide: Cell::new(0.0),
                methane: Cell::new(0.0),
            }),
            lease: None,
            rectangle_lease: None,
        }
    }

    pub(crate) fn actor_roots(&self) -> Vec<OrganismRef> {
        let mut actors = self.logical_graph_actors();
        actors.extend(self.frame_objects.borrow().iter().cloned());
        if let Some(context) = &self.rectangle_lease { actors.extend(context.actors().iter().cloned()); }
        if let Some(context) = self.rectangle_context.borrow().as_ref() { actors.extend(context.actors().iter().cloned()); }
        for corridor in self.incoming_corridors.borrow().iter() {
            if let Some(actor) = &corridor.borrow().candidate { actors.push(Rc::clone(actor)); }
        }
        for corridor in self.outgoing_corridors.borrow().iter() {
            if let Some(actor) = &corridor.borrow().traveller { actors.push(Rc::clone(actor)); }
        }
        actors
    }

    /// Retain one validated, complete imported World closure outside internal model owners
    pub(crate) fn retain_worlds(
        &mut self,
        supplied: &[WorldHandle],
        limits: crate::object_stream::Limits,
        maximum: usize,
    ) -> Result<(), crate::legacy_graph::GraphError> {
        self.retain_worlds_metered(supplied, limits, maximum, None)
    }

    pub(super) fn retain_worlds_metered(
        &mut self, supplied: &[WorldHandle], limits: crate::object_stream::Limits, maximum: usize,
        graph: Option<(&crate::legacy_graph::LegacyGraph<'_>, usize)>,
    ) -> Result<(), crate::legacy_graph::GraphError> {
        let failed = || crate::legacy_graph::GraphError {
            node: None, field: None, kind: crate::legacy_graph::GraphErrorKind::Budget,
        };
        if supplied.len() > limits.handles { return Err(failed()); }
        let mut roots = Vec::new();
        let mut actors = Vec::new();
        let mut retention_limits = limits;
        if let Some((graph, actor_count)) = graph {
            graph.allocate(WorldState::cohort_allocation_bytes())?;
            crate::shared_world_owners::reserve(graph, &mut roots, supplied.len())?;
            crate::shared_world_owners::reserve(graph, &mut actors, actor_count)?;
            retention_limits.work = graph.remaining_work();
        } else {
            roots.try_reserve_exact(supplied.len()).map_err(|_| failed())?;
        }
        roots.extend(supplied.iter().map(|value| Rc::clone(&value.state)));
        let (actors, work) = crate::legacy_rectangles::retain_worlds_with_work(&roots, &self.state, retention_limits, maximum, actors)?;
        if let Some((graph, _)) = graph { graph.charge(work)?; }
        self.lease = Some(Rc::new(WorldCohort { worlds: roots, actors }));
        Ok(())
    }

    pub(crate) fn use_actor_energy(
        &self,
        actor: &mut Organism,
        amount: f64,
    ) -> Result<bool, WorldError> {
        actor.use_energy(amount)
    }

    pub(crate) fn transfer_gas(&self, amount: f64, methane: bool) -> f64 {
        let destination = if methane { &self.methane } else { &self.carbon_dioxide };
        crate::atmosphere::transfer_cells(&self.oxygen, destination, amount)
    }
}

impl Clone for World {
    fn clone(&self) -> Self {
        let mut worlds = HashMap::<usize, World>::new();
        let mut sessions = HashMap::<usize, SessionRef>::new();
        let mut queue = vec![Rc::clone(&self.state)];
        if let Some(lease) = &self.lease { queue.extend(lease.worlds.iter().cloned()); }
        let mut originals = Vec::new();
        let mut actor_sources = Vec::new();
        let mut actor_seen = HashSet::new();
        while let Some(original) = queue.pop() {
            let identity = Rc::as_ptr(&original) as usize;
            if worlds.contains_key(&identity) { continue; }
            let session_key = Rc::as_ptr(&original.session.0) as usize;
            let session = sessions.entry(session_key).or_insert_with(|| SessionRef::new(
                original.session.parameters(), original.session.random_snapshot(),
            )).clone();
            let mut destination = World::empty_in_session(original.config.get(), session);
            Rc::get_mut(&mut destination.state).expect("Unpublished copy root").organisms = crate::membership::Membership::pending();
            worlds.insert(identity, destination);
            let source = World {
                state: Rc::clone(&original),
                lease: None,
                rectangle_lease: if Rc::ptr_eq(&original, &self.state) { self.rectangle_lease.clone() } else { None },
            };
            let mut actors = source.actor_roots();
            let mut seen = HashSet::new();
            while let Some(actor) = actors.pop() {
                if !seen.insert(Rc::as_ptr(&actor) as usize) { continue; }
                if actor_seen.insert(Rc::as_ptr(&actor) as usize) { actor_sources.push(Rc::clone(&actor)); }
                let actor = actor.borrow();
                if let Some(target) = actor.state.world.target().expect("Live World closure") {
                    queue.push(target);
                }
                for rectangle in [
                    Some(&actor.state.bounds), actor.body.size_rect.as_ref(), actor.state.last_frame.as_ref(),
                ].into_iter().flatten() {
                    if let Some(target) = rectangle.actor_target() { actors.push(target); }
                }
            }
            originals.push(original);
        }
        let mut actors = OrganismCloner::default();
        let mut incoming_owners = HashMap::new();
        let mut outgoing_owners = HashMap::new();
        let mut connection_owners = HashMap::new();
        let mut statistics_owners = HashMap::new();
        actors.worlds = worlds.iter().map(|(key, value)| (*key, Rc::downgrade(&value.state))).collect();
        let actor_pairs: Vec<_> = actor_sources.iter().map(|source| (Rc::clone(source), actors.copy(source))).collect();
        let owner_budget = crate::shared_world_owners::RuntimeCopyMeter::new();
        let mut lists = crate::shared_world_owners::SharedOwnerCopier::new(&owner_budget, &actor_pairs)
            .expect("Complete admitted actor copy map");
        for original in &originals {
            let destination = &worlds[&(Rc::as_ptr(original) as usize)];
            let members = original.organisms.shared_ref().map(|value| lists.list(value)).transpose().expect("Admitted shared membership copy");
            destination.organisms.initialize(members);
            destination.frame_objects.replace(original.frame_objects.borrow().iter().map(|actor| actors.copy(actor)).collect());
            destination.rectangle_context.replace(original.rectangle_context.borrow().as_ref().map(|context| Rc::new(context.copied(&mut actors))));
            destination.incoming_corridors.replace(incoming::copy_list(&original.incoming_corridors.borrow(), &mut actors, &mut incoming_owners));
            destination.outgoing_corridors.replace(outgoing::copy_list(&original.outgoing_corridors.borrow(), &mut actors, &mut outgoing_owners, &mut connection_owners));
            destination.corridors_exist.set(original.corridors_exist.get());
            let spatial = original.spatial.borrow();
            let tree = lists.tree(spatial.shared_ref()).expect("Admitted shared spatial copy");
            let tokens = spatial.token_bindings().into_iter().map(|actor| {
                actor.filter(|actor| actor_seen.contains(&(Rc::as_ptr(actor) as usize)))
                    .map_or_else(Weak::new, |actor| Rc::downgrade(&actors.copy(&actor)))
            }).collect();
            destination.spatial.replace(SpatialTree::from_shared(tree, tokens, spatial.imported()));
            destination.next_id.set(original.next_id.get());
            destination.population.set(original.population.get());
            destination.frame.set(original.frame.get());
            let original_statistics = Rc::clone(original.statistics.borrow().as_ref().expect("Complete statistics owner"));
            let statistics = statistics_owners.entry(Rc::as_ptr(&original_statistics)).or_insert_with(|| {
                let source = original_statistics.borrow();
                let histories = source.history_refs().map(|value| value.map(|value| {
                    crate::history::HistoryListRef::from_shared(lists.list(value.shared_ref()).expect("Admitted shared history copy"))
                }));
                source.copy_with_lists(actors.genome_cloner(), histories).into_shared()
            });
            destination.statistics.replace(Some(Rc::clone(statistics)));
            destination.tick.set(original.tick.get());
            destination.paused.set(original.paused.get());
            destination.oxygen.set(original.oxygen.get());
            destination.carbon_dioxide.set(original.carbon_dioxide.get());
            destination.methane.set(original.methane.get());
        }
        let rectangle_lease = self.rectangle_lease.as_ref().map(|context| Rc::new(context.copied(&mut actors)));
        let root = Rc::as_ptr(&self.state) as usize;
        let mut copied = worlds.remove(&root).expect("Copied root");
        copied.rectangle_lease = rectangle_lease;
        let strong_actors: HashSet<_> = std::iter::once(&copied).chain(worlds.values())
            .flat_map(World::actor_roots).map(|actor| Rc::as_ptr(&actor) as usize).collect();
        let detached_targets = actors.copied_actors().filter(|actor| !strong_actors.contains(&(Rc::as_ptr(actor) as usize)))
            .cloned().collect::<Vec<_>>();
        if !worlds.is_empty() || !detached_targets.is_empty() || self.lease.is_some() {
            copied.lease = Some(Rc::new(WorldCohort {
                worlds: worlds.into_values().map(|world| world.state).collect(),
                actors: detached_targets,
            }));
        }
        copied
    }
}

impl PartialEq for World {
    fn eq(&self, other: &Self) -> bool {
        fn actor_eq(
            left: &OrganismRef, right: &OrganismRef,
            seen: &mut HashSet<(usize, usize)>,
        ) -> bool {
            let left = left.borrow();
            let right = right.borrow();
            let mut ls = left.state.clone();
            let mut rs = right.state.clone();
            ls.world = WorldAssociation::default();
            rs.world = WorldAssociation::default();
            let targets = match (left.state.world.target(), right.state.world.target()) {
                (Ok(None), Ok(None)) => true,
                (Ok(Some(l)), Ok(Some(r))) => state_eq(&l, &r, seen),
                _ => left.state.world == right.state.world,
            };
            targets && left.id == right.id && left.genome == right.genome
                && left.infected_genome == right.infected_genome
                && left.saved_genome == right.saved_genome
                && left.body == right.body && ls == rs
                && left.effects == right.effects
                && left.creation_complete == right.creation_complete
        }
        fn actor_list(
            left: &[OrganismRef], right: &[OrganismRef],
            seen: &mut HashSet<(usize, usize)>,
        ) -> bool {
            left.len() == right.len() && left.iter().zip(right).all(|(l, r)| actor_eq(l, r, seen))
        }
        fn shared_list_eq(
            left: Option<&crate::shared_world_owners::SharedListRef>,
            right: Option<&crate::shared_world_owners::SharedListRef>,
            seen: &mut HashSet<(usize, usize)>,
        ) -> bool {
            crate::shared_world_owners::OwnerEquality::new(|left: &OrganismRef, right: &OrganismRef| actor_eq(left,right,seen)).list(left,right)
        }
        fn statistics_eq(left: &WorldState, right: &WorldState, seen: &mut HashSet<(usize, usize)>) -> bool {
            let left_slot = left.statistics.borrow(); let right_slot = right.statistics.borrow();
            let left = left_slot.as_ref().expect("Complete statistics").borrow();
            let right = right_slot.as_ref().expect("Complete statistics").borrow();
            left.eq_non_histories(&right) && left.history_refs().into_iter().zip(right.history_refs()).all(|(a,b)| {
                shared_list_eq(a.map(crate::history::HistoryListRef::shared_ref), b.map(crate::history::HistoryListRef::shared_ref), seen)
            })
        }
        fn outgoing_eq(
            left: &Rc<WorldState>, right: &Rc<WorldState>,
            seen: &mut HashSet<(usize, usize)>,
        ) -> bool {
            let left = left.outgoing_corridors.borrow();
            let right = right.outgoing_corridors.borrow();
            left.len() == right.len() && left.iter().zip(right.iter()).all(|(left, right)| {
                let left = left.borrow();
                let right = right.borrow();
                left.bounds == right.bounds && left.connection == right.connection
                    && match (&left.traveller, &right.traveller) {
                        (None, None) => true,
                        (Some(left), Some(right)) => actor_eq(left, right, seen),
                        _ => false,
                    }
            })
        }
        fn state_eq(
            left: &Rc<WorldState>, right: &Rc<WorldState>,
            seen: &mut HashSet<(usize, usize)>,
        ) -> bool {
            if !seen.insert((Rc::as_ptr(left) as usize, Rc::as_ptr(right) as usize)) { return true; }
            let equal = left.config.get() == right.config.get()
                && left.session == right.session
                && left.corridors_exist.get() == right.corridors_exist.get()
                && left.next_id.get() == right.next_id.get()
                && left.population.get() == right.population.get()
                && left.frame.get() == right.frame.get()
                && left.tick.get() == right.tick.get()
                && left.paused.get() == right.paused.get()
                && left.oxygen.get() == right.oxygen.get()
                && left.carbon_dioxide.get() == right.carbon_dioxide.get()
                && left.methane.get() == right.methane.get()
                && left.spatial.borrow().equal_with_actor(&right.spatial.borrow(), |left, right| actor_eq(left, right, seen))
                && statistics_eq(left, right, seen)
                && outgoing_eq(left, right, seen)
                && shared_list_eq(left.organisms.shared_ref(), right.organisms.shared_ref(), seen)
                && actor_list(&left.frame_objects.borrow(), &right.frame_objects.borrow(), seen);
            if !equal { return false; }
            match (&*left.rectangle_context.borrow(), &*right.rectangle_context.borrow()) {
                (None, None) => {}
                (Some(l), Some(r)) => {
                    if l.retained_bytes() != r.retained_bytes() || l.maximum_bytes() != r.maximum_bytes() || !actor_list(l.actors(), r.actors(), seen) {
                        return false;
                    }
                }
                _ => return false,
            }
            let l = left.incoming_corridors.borrow();
            let r = right.incoming_corridors.borrow();
            l.len() == r.len() && l.iter().zip(r.iter()).all(|(l, r)| {
                let l = l.borrow();
                let r = r.borrow();
                l.bounds == r.bounds && l.pending == r.pending
                    && match (&l.candidate, &r.candidate) {
                        (None, None) => true,
                        (Some(l), Some(r)) => actor_eq(l, r, seen),
                        _ => false,
                    }
            })
        }
        state_eq(&self.state, &other.state, &mut HashSet::new())
    }
}

impl WorldState {
    pub(crate) fn cohort_allocation_bytes() -> usize {
        std::mem::size_of::<WorldCohort>() + 2 * std::mem::size_of::<usize>()
    }

    pub(crate) fn charge_retained(
        self: &Rc<Self>,
        budget: &mut crate::legacy_rectangles::Retention,
        supplied: &[Rc<Self>],
        retained: &mut Vec<OrganismRef>,
    ) -> Result<(), crate::legacy_graph::GraphError> {
        use crate::legacy_graph::{GraphError, GraphErrorKind};
        use std::mem::size_of;
        let failure = |kind| GraphError { node: None, field: None, kind };
        let product = |count: usize, unit: usize| count.checked_mul(unit).ok_or_else(|| failure(GraphErrorKind::Budget));
        if !budget.first(b'W', Rc::as_ptr(self) as usize)? { return Err(failure(GraphErrorKind::Reference)); }
        budget.allocation::<WorldState>()?;
        budget.visit(supplied.len())?;
        if !supplied.iter().all(|world| world.session.same_owner(&self.session)) {
            return Err(failure(GraphErrorKind::Reference));
        }
        if budget.first(b'E', Rc::as_ptr(&self.session.0) as usize)? {
            budget.allocation::<SessionState>()?;
            if let Some(outgoing) = &self.session.0.parameters.borrow().outgoing {
                budget.text(Some(&outgoing.user_name))?;
            }
        }
        budget.bytes(product(self.frame_objects.borrow().capacity(), size_of::<OrganismRef>())?)?;
        budget.bytes(product(self.incoming_corridors.borrow().capacity(), size_of::<IncomingCorridorRef>())?)?;
        budget.bytes(product(self.outgoing_corridors.borrow().capacity(), size_of::<OutgoingCorridorRef>())?)?;
        if let Some(context) = self.rectangle_context.borrow().as_ref() {
            if budget.first(b'k', Rc::as_ptr(context) as usize)? {
                budget.allocation::<crate::rectangles::RectangleActorContext>()?;
                budget.bytes(product(context.storage_capacity(), size_of::<OrganismRef>())?)?;
            }
        }
        fn queue(
            pending: &mut Vec<OrganismRef>, actor: OrganismRef,
            budget: &mut crate::legacy_rectangles::Retention,
        ) -> Result<(), GraphError> {
            budget.visit(1)?;
            let before = pending.capacity();
            pending.try_reserve(1).map_err(|_| GraphError { node: None, field: None, kind: GraphErrorKind::Allocation })?;
            let added = pending.capacity().checked_sub(before).and_then(|n| n.checked_mul(size_of::<OrganismRef>()))
                .ok_or(GraphError { node: None, field: None, kind: GraphErrorKind::Budget })?;
            budget.bytes(added)?;
            pending.push(actor);
            Ok(())
        }
        enum Owner {
            List(crate::shared_world_owners::SharedListRef),
            Tree(crate::shared_world_owners::TreeRef),
            Boundable(crate::shared_world_owners::BoundableRef),
            Envelope(crate::shared_world_owners::EnvelopeRef),
        }
        fn push_owner(pending: &mut Vec<Owner>, value: Owner, budget: &mut crate::legacy_rectangles::Retention) -> Result<(), GraphError> {
            budget.visit(1)?;
            let before = pending.capacity();
            pending.try_reserve(1).map_err(|_| GraphError { node:None,field:None,kind:GraphErrorKind::Allocation })?;
            budget.bytes(pending.capacity().checked_sub(before).and_then(|n|n.checked_mul(size_of::<Owner>())).ok_or(GraphError { node:None,field:None,kind:GraphErrorKind::Budget })?)?;
            pending.push(value); Ok(())
        }
        fn slot(value: &crate::shared_world_owners::SharedSlot, owners: &mut Vec<Owner>, actors: &mut Vec<OrganismRef>, budget: &mut crate::legacy_rectangles::Retention) -> Result<(), GraphError> {
            use crate::shared_world_owners::SharedSlot;
            budget.visit(1)?;
            match value {
                SharedSlot::Null => {},
                SharedSlot::Double(value) => if budget.first(b'B',value.identity())? { budget.allocation::<u64>()?; },
                SharedSlot::Actor(actor) => queue(actors,Rc::clone(actor),budget)?,
                SharedSlot::Boundable(value) => push_owner(owners,Owner::Boundable(value.clone()),budget)?,
                SharedSlot::Envelope(value) => push_owner(owners,Owner::Envelope(value.clone()),budget)?,
            }
            Ok(())
        }
        let mut pending = Vec::new();
        let mut owners = Vec::new();
        if let Some(members) = self.organisms.shared_ref() { push_owner(&mut owners,Owner::List(members.clone()),budget)?; }
        let statistics = Rc::clone(self.statistics.borrow().as_ref().expect("Complete World statistics"));
        if budget.first(b'V',Rc::as_ptr(&statistics) as usize)? {
            budget.allocation::<RefCell<WorldStatistics>>()?;
            let value = statistics.borrow();
            for genome in value.retained_genomes() { budget.genome(genome)?; }
            for history in value.history_refs().into_iter().flatten() { push_owner(&mut owners,Owner::List(history.shared_ref().clone()),budget)?; }
        }
        let spatial = self.spatial.borrow();
        budget.bytes(product(spatial.token_capacity(),size_of::<Weak<RefCell<Organism>>>())?)?;
        push_owner(&mut owners,Owner::Tree(spatial.shared_ref().clone()),budget)?;
        while let Some(owner) = owners.pop() {
            use crate::shared_world_owners::{BoundableValue,SharedListRef};
            budget.visit(1)?;
            match owner {
                Owner::List(value) => {
                    if value.wire_class_name()!="java.util.ArrayList" && budget.first(b'h',value.identity())? { budget.bytes(SharedListRef::wrapper_allocation())?; }
                    if !budget.first(b'H',value.backing_identity())? { continue; }
                    budget.bytes(SharedListRef::backing_allocation())?;
                    let values=value.slots();budget.bytes(product(values.capacity(),size_of::<crate::shared_world_owners::SharedSlot>())?)?;
                    for value in values.iter() { slot(value,&mut owners,&mut pending,budget)?; }
                },
                Owner::Tree(value) => {
                    if !budget.first(b't',value.identity())? { continue; }
                    budget.bytes(crate::shared_world_owners::TreeRef::allocation())?;
                    let value=value.value();
                    if let Some(root)=&value.root { push_owner(&mut owners,Owner::Boundable(root.clone()),budget)?; }
                    if let Some(pending_list)=&value.pending { push_owner(&mut owners,Owner::List(pending_list.clone()),budget)?; }
                },
                Owner::Boundable(value) => {
                    if !budget.first(b'b',value.identity())? { continue; }
                    budget.bytes(crate::shared_world_owners::BoundableRef::allocation())?;
                    match &*value.value() {
                        BoundableValue::Item { bounds,item } => {
                            if let Some(bounds)=bounds { push_owner(&mut owners,Owner::Envelope(bounds.clone()),budget)?; }
                            slot(item,&mut owners,&mut pending,budget)?;
                        },
                        BoundableValue::Node { bounds,children,.. } => {
                            if let Some(bounds)=bounds { push_owner(&mut owners,Owner::Envelope(bounds.clone()),budget)?; }
                            if let Some(children)=children { push_owner(&mut owners,Owner::List(children.clone()),budget)?; }
                        },
                    }
                },
                Owner::Envelope(value) => if budget.first(b'e',value.identity())? { budget.bytes(crate::shared_world_owners::EnvelopeRef::allocation())?; },
            }
        }
        drop(spatial);
        for actor in self.frame_objects.borrow().iter() {
            queue(&mut pending, Rc::clone(actor), budget)?;
        }
        if let Some(context) = self.rectangle_context.borrow().as_ref() {
            for actor in context.actors() { queue(&mut pending, Rc::clone(actor), budget)?; }
        }
        for corridor in self.incoming_corridors.borrow().iter() {
            if let Some(actor) = &corridor.borrow().candidate { queue(&mut pending, Rc::clone(actor), budget)?; }
        }
        for corridor in self.outgoing_corridors.borrow().iter() {
            if let Some(actor) = &corridor.borrow().traveller { queue(&mut pending, Rc::clone(actor), budget)?; }
        }
        while let Some(actor) = pending.pop() {
            if !budget.retained_actor(&actor)? { continue; }
            queue(retained, Rc::clone(&actor), budget)?;
            let actor_ref = actor;
            let actor = actor_ref.borrow();
            if !actor.state.bounds.actor_target().is_some_and(|target| Rc::ptr_eq(&target, &actor_ref)) {
                return Err(failure(GraphErrorKind::Reference));
            }
            if let Some(target) = actor.state.world.target().map_err(|_| failure(GraphErrorKind::Reference))? {
                budget.visit(supplied.len())?;
                if !supplied.iter().any(|world| Rc::ptr_eq(world, &target)) {
                    return Err(failure(GraphErrorKind::Reference));
                }
            }
            for cell in [Some(&actor.state.bounds), actor.body.size_rect.as_ref(), actor.state.last_frame.as_ref()]
                .into_iter().flatten()
            {
                if cell.is_actor() {
                    let target = cell.actor_target().ok_or_else(|| failure(GraphErrorKind::Reference))?;
                    queue(&mut pending, target, budget)?;
                }
            }
        }
        for corridor in self.incoming_corridors.borrow().iter() {
            budget.visit(1)?;
            if budget.first(b'U', Rc::as_ptr(corridor) as usize)? {
                budget.allocation::<RefCell<incoming::IncomingCorridor>>()?;
                let corridor = corridor.borrow();
                budget.bytes(product(corridor.pending.capacity(), size_of::<Option<GenomeRef>>())?)?;
                for genome in &corridor.pending { budget.genome(genome.as_ref())?; }
            }
        }
        for corridor in self.outgoing_corridors.borrow().iter() {
            budget.visit(1)?;
            if budget.first(b'O', Rc::as_ptr(corridor) as usize)? {
                budget.allocation::<RefCell<outgoing::OutgoingCorridor>>()?;
                let corridor = corridor.borrow();
                if budget.first(b'Q', Rc::as_ptr(&corridor.connection) as usize)? {
                    budget.allocation::<RefCell<outgoing::ConnectionCore>>()?;
                    let connection = corridor.connection.borrow();
                    budget.bytes(product(connection.pending.storage_capacity(), size_of::<outgoing::PendingTransfer>())?)?;
                    for genome in connection.pending.retained_genomes() { budget.genome(Some(genome))?; }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "world_root_tests.rs"]
mod tests;
