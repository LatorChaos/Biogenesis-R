// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use crate::{ColorKind, EffectsState, Gene, GenomeProperties, RotatedSegment};

fn empty(session: SessionRef) -> World {
    World::with_session(
        WorldConfig { initial_attempts: 0, ..WorldConfig::default() }, session,
    ).unwrap()
}

fn session() -> SessionRef {
    SessionRef::new(WorldParameters::default(), JavaRandom::new(827364))
}

// These two World fixtures retain at most 136 distinct charged owners
fn cohort_limits() -> crate::object_stream::Limits {
    crate::object_stream::Limits { handles: 256, ..crate::object_stream::Limits::default() }
}

fn add(world: &World, color: ColorKind) -> OrganismRef {
    let code = Genome::from_parts(
        &[Gene::new(12.0, 0.0, color.argb(), 0, [0; 36]).shared()],
        GenomeProperties { symmetry: 1, ..GenomeProperties::default() },
    );
    let x = 1000 + 1000 * world.organisms.borrow().len() as i32;
    world.paste_genome(code, [x, 1000]).unwrap().unwrap();
    Rc::clone(world.organisms.borrow().last().unwrap())
}

#[test]
fn null_world_payments_keep_the_energy_guard_and_reached_zero_access() {
    let world = empty(session());
    let mut actor = Organism::pasted_candidate(None, OrganismState::default());
    actor.state.energy = 5.0;
    let before = (world.oxygen(), world.carbon_dioxide(), world.methane(), world.random_snapshot());
    assert_eq!(actor.use_energy(6.0), Ok(false));
    assert_eq!(world.use_decay_energy(&mut actor, 6.0), Ok(false));
    assert_eq!(actor.use_energy(0.0), Err(WorldError::MissingWorld));
    assert_eq!(world.use_decay_energy(&mut actor, 0.0), Err(WorldError::MissingWorld));
    assert_eq!(actor.state.energy.to_bits(), 5.0_f64.to_bits());
    assert_eq!(before, (world.oxygen(), world.carbon_dioxide(), world.methane(), world.random_snapshot()));

    let mut border = Organism::pasted_candidate(None, OrganismState::default());
    border.state.alive = false;
    border.state.bounds.set(IntRect { x: 0, y: 1, width: 2, height: 2 });
    border.state.velocity = [2.0, 3.0];
    border.state.angular_velocity = 4.0;
    assert_eq!(world.is_inside_world(&mut border), Err(WorldError::MissingWorld));
    assert_eq!(border.state.velocity, [-2.0, 3.0]);
    assert_eq!(border.state.angular_velocity, 4.0);
    border.state.bounds.set(IntRect { x: 0, y: 0, width: 2, height: 2 });
    assert_eq!(world.is_inside_world(&mut border), Ok(false));
    assert_eq!(border.state.velocity, [2.0, -3.0]);
    assert_eq!(border.state.angular_velocity, 0.0);

    let foreign = empty(world.session());
    foreign.oxygen.set(10.0);
    foreign.carbon_dioxide.set(2.0);
    foreign.methane.set(3.0);
    actor.state.world = foreign.association();
    assert_eq!(world.use_decay_energy(&mut actor, 2.0), Ok(true));
    assert_eq!((actor.state.energy, foreign.oxygen(), foreign.carbon_dioxide(), foreign.methane()), (3.0, 8.0, 2.0, 5.0));
    assert_eq!(before, (world.oxygen(), world.carbon_dioxide(), world.methane(), world.random_snapshot()));
}

#[test]
fn reverse_contact_uses_actual_worlds_without_whole_owner_borrowing() {
    for (present, energy, expected_error) in [
        (true, 5.0, None), (false, 0.1, None), (false, 5.0, Some(WorldError::MissingWorld)),
    ] {
        let shared = session();
        shared.0.parameters.borrow_mut().lifecycle.teal_energy_consumption = 0.25;
        let caller = empty(shared.clone());
        let query = empty(shared.clone());
        let actor_ref = add(&caller, ColorKind::Green);
        let peer_ref = add(&query, ColorKind::Teal);
        for (reference, vertical) in [(&actor_ref, false), (&peer_ref, true)] {
            let mut actor = reference.borrow_mut();
            actor.effects = EffectsState::default();
            actor.state.energy = if vertical { energy } else { 5.0 };
            actor.state.alive = true;
            actor.state.parent_id = 77;
            actor.id = 77;
            actor.body.mass = 1.0;
            actor.body.inertia = 1.0;
            actor.body.eyes.enabled = false;
            actor.state.center = [2000.0; 2];
            actor.state.integer_center = [2000; 2];
            actor.state.bounds.set(IntRect { x: 1994, y: 1994, width: 13, height: 13 });
            actor.state.velocity = if vertical { [0.0; 2] } else { [0.0, 1.0] };
            actor.body.set_physical_length(0, 2.0).unwrap();
            actor.body.set_photo(0, 0.0).unwrap();
            actor.body.set_rotated_segment(0, if vertical {
                RotatedSegment { start: [0, -5], end: [0, 5] }
            } else {
                RotatedSegment { start: [-5, 0], end: [5, 0] }
            }).unwrap();
        }
        actor_ref.borrow_mut().state.world = query.association();
        {
            let mut peer = peer_ref.borrow_mut();
            peer.state.world = if present { caller.association() } else { WorldAssociation::default() };
            peer.effects.can_react = true;
            peer.body.set_reaction(0, 35, 1).unwrap();
        }
        caller.oxygen.set(10.0);
        caller.carbon_dioxide.set(0.0);
        query.oxygen.set(20.0);
        query.carbon_dioxide.set(0.0);
        query.rebuild_spatial();
        let random = shared.random_snapshot();
        let result = caller.check_actor_contact_from(&actor_ref, &mut actor_ref.borrow_mut());
        match expected_error {
            None => assert!(Rc::ptr_eq(&result.unwrap().unwrap(), &actor_ref)),
            Some(error) => {
                assert_eq!(result, Err(error));
                assert_ne!(peer_ref.borrow().state.velocity, [0.0; 2]);
            }
        }
        assert_eq!((query.oxygen(), query.carbon_dioxide()), (20.0, 0.0));
        assert_eq!(actor_ref.borrow().state.energy, 5.0);
        assert_eq!(shared.random_snapshot(), random);
        if present {
            assert_eq!((caller.oxygen(), caller.carbon_dioxide(), peer_ref.borrow().state.energy), (9.75, 0.25, 4.75));
        } else {
            assert_eq!((caller.oxygen(), caller.carbon_dioxide(), peer_ref.borrow().state.energy), (10.0, 0.0, energy));
        }
    }
}

#[test]
fn graph_copy_preallocates_world_roots_and_copies_one_shared_session() {
    let shared = session();
    let mut first = empty(shared.clone());
    let second = empty(shared.clone());
    let left = add(&first, ColorKind::Green);
    let right = add(&second, ColorKind::Teal);
    left.borrow_mut().state.world = second.association();
    right.borrow_mut().state.world = first.association();
    let incoming = first.new_incoming_corridor();
    first.add_incoming_corridor(Rc::clone(&incoming));
    second.add_incoming_corridor(Rc::clone(&incoming));
    second.statistics.replace(Some(first.statistics_ref()));
    let original = [first.handle(), second.handle()];
    let limited = crate::object_stream::Limits { handles: original.len() - 1, ..crate::object_stream::Limits::default() };
    assert_eq!(
        first.retain_worlds(&original, limited, 16 * 1024 * 1024).unwrap_err().kind,
        crate::legacy_graph::GraphErrorKind::Budget,
    );
    first.retain_worlds(&original, cohort_limits(), 16 * 1024 * 1024).unwrap();
    let copied = first.clone();
    let copied_left = Rc::clone(&copied.organisms.borrow()[0]);
    let copied_second = copied_left.borrow().state.world.resolve().unwrap();
    let copied_right = Rc::clone(&copied_second.organisms.borrow()[0]);
    assert!(copied_right.borrow().state.world.resolve().unwrap().handle().same_owner(&copied.handle()));
    assert!(!copied.handle().same_owner(&first.handle()));
    assert!(!copied_second.handle().same_owner(&second.handle()));
    assert!(Rc::ptr_eq(
        &copied.incoming_corridors.borrow()[0],
        &copied_second.incoming_corridors.borrow()[0],
    ));
    assert!(!Rc::ptr_eq(&copied.incoming_corridors.borrow()[0], &incoming));
    assert!(Rc::ptr_eq(&copied.statistics_ref(), &copied_second.statistics_ref()));
    assert!(!Rc::ptr_eq(&copied.statistics_ref(), &first.statistics_ref()));
    assert!(copied.session().same_owner(&copied_second.session()));
    assert!(!copied.session().same_owner(&shared));
    assert_eq!(copied.random_snapshot(), shared.random_snapshot());
    copied.session.0.random.borrow_mut().next_i32();
    assert_ne!(copied.random_snapshot(), shared.random_snapshot());
    assert!(!Rc::ptr_eq(&left, &copied_left));
    assert!(!Rc::ptr_eq(&right, &copied_right));
}

#[test]
fn external_cohort_retains_complete_worlds_and_drops_without_birth_leases() {
    let shared = session();
    let mut first = empty(shared.clone());
    let second = empty(shared);
    let left = add(&first, ColorKind::Green);
    let right = add(&second, ColorKind::Teal);
    left.borrow_mut().state.world = second.association();
    right.borrow_mut().state.world = first.association();
    let before = first.lease.clone();
    let rejected = first.retain_worlds(&[first.handle(), second.handle()], crate::object_stream::Limits::default(), 0).unwrap_err();
    assert_eq!(rejected.kind, crate::legacy_graph::GraphErrorKind::Budget);
    assert_eq!(first.lease.is_some(), before.is_some());
    first.retain_worlds(&[first.handle(), second.handle()], cohort_limits(), 16 * 1024 * 1024).unwrap();
    let escaped = super::super::OrganismHandle(Rc::clone(&left), first.handle());
    let weak_first = Rc::downgrade(&first.state);
    let weak_second = Rc::downgrade(&second.state);
    drop(left);
    drop(right);
    drop(first);
    drop(second);
    assert!(weak_first.upgrade().is_some() && weak_second.upgrade().is_some());
    drop(escaped);
    assert!(weak_first.upgrade().is_none() && weak_second.upgrade().is_none());

    let ordinary = empty(session());
    for _ in 0..4 { drop(add(&ordinary, ColorKind::Green)); }
    assert!(ordinary.lease.is_none());
    assert_eq!(Rc::strong_count(&ordinary.state), 1);
}

#[test]
fn session_settings_new_game_reset_and_failed_start_keep_owner_order() {
    let shared = session();
    let mut first = empty(shared.clone());
    let second = empty(shared.clone());
    let mut settings = crate::SettingsState::new("en".to_owned());
    let mut draft = crate::SettingsDraft::new(&settings);
    draft.set_editor(crate::SettingKey::DrainSubsDivisor, "17".to_owned()).unwrap();
    draft.ok(&mut settings, |_| {}).unwrap();
    first.apply_settings_parameters(&settings);
    assert_eq!(second.parameters().atmosphere.drain_divisor, 17);
    let independent = World::with_parameters(
        WorldConfig { seed: 827364, initial_attempts: 0, ..WorldConfig::default() },
        first.parameters(),
    ).unwrap();
    assert_eq!(independent.session(), shared);
    assert!(!independent.session().same_owner(&shared));
    let old = first.handle();
    let original_random = shared.random_snapshot();
    let error = World::with_session(WorldConfig { width: -1, ..WorldConfig::default() }, shared.clone()).unwrap_err();
    assert_eq!(error.error, WorldError::InvalidDimensions);
    assert!(!error.entered_genesis);
    assert!(error.session.same_owner(&shared));
    assert_eq!(shared.random_snapshot(), original_random);
    assert_eq!(
        first.new_game(-1, 100, 0, first.parameters()),
        Err(NewGameError { error: WorldError::InvalidDimensions, entered_genesis: false }),
    );
    assert!(first.handle().same_owner(&old));
    assert!(first.session().same_owner(&shared));
    first.new_game(7000, 7000, 0, first.parameters()).unwrap();
    assert!(first.handle().same_owner(&old));
    assert!(first.session().same_owner(&shared));
    first.reset().unwrap();
    assert!(!first.handle().same_owner(&old));
    assert!(!first.session().same_owner(&shared));
    assert!(old.session().same_owner(&second.session()));
    assert_eq!(first.random_snapshot(), JavaRandom::new(first.seed()));
}

#[test]
fn retention_counts_null_history_slots_without_changing_the_list() {
    let mut world = empty(session());
    let history = crate::history::HistoryListRef::from_elements(vec![None; 4096]);
    let statistics = world.statistics_ref();
    statistics.borrow_mut().population_list = Some(history.clone());
    let identity = history.identity();
    let before = history.elements().clone();
    let capacity = history.elements().capacity();
    let mut limits = crate::object_stream::Limits {
        handles: 64, work: 512, ..crate::object_stream::Limits::default()
    };
    let supplied = [world.handle()];
    let rejected = world.retain_worlds(&supplied, limits, 16 * 1024 * 1024).unwrap_err();
    assert_eq!(rejected.kind, crate::legacy_graph::GraphErrorKind::Budget);
    assert!(world.lease.is_none());
    assert_eq!(*history.elements(), before);
    assert_eq!(history.elements().capacity(), capacity);
    limits.work = 8192;
    world.retain_worlds(&supplied, limits, 16 * 1024 * 1024).unwrap();
    assert_eq!(*history.elements(), before);
    assert_eq!(history.elements().capacity(), capacity);
    assert_eq!(history.identity(), identity);
    assert!(statistics.borrow().population_list.as_ref().unwrap().same_instance(&history));
    assert!(Rc::ptr_eq(&world.statistics_ref(), &statistics));
}
