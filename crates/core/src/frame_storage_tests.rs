// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use crate::{ColorKind, Gene, GenomeModifiers, GenomeProperties};

fn world() -> World {
    let mut parameters = WorldParameters::default();
    parameters.genetics.meta_mutation_rate = 0;
    World::with_parameters(
        WorldConfig {
            seed: 42,
            width: 1000,
            height: 1000,
            initial_attempts: 0,
        },
        parameters,
    )
    .unwrap()
}

fn genome(clade: &str) -> Genome {
    let gene = Gene::new(12.0, 0.0, ColorKind::Green.argb(), 0, [0; 36]).shared();
    let mut code = Genome::from_parts(
        &[gene],
        GenomeProperties {
            symmetry: 1,
            modifiers: GenomeModifiers {
                cream: 1,
                fallow: 1,
                spore: 1,
                adapt_spore: 1,
                ..GenomeModifiers::default()
            },
            ..GenomeProperties::default()
        },
    );
    code.set_clade_id(Some(clade.to_owned()));
    code
}

fn add(world: &mut World, center: [i32; 2], clade: &str) -> OrganismRef {
    world.paste_genome(genome(clade), center).unwrap().unwrap();
    Rc::clone(world.organisms.borrow().last().unwrap())
}

#[test]
fn frozen_candidates_keep_live_identity_after_removal_and_exclude_newborns() {
    let mut world = world();
    let removed = add(&mut world, [100, 100], "removed");
    let parent = add(&mut world, [300, 300], "parent");
    add(&mut world, [700, 700], "third");
    world.rebuild_spatial().unwrap();
    let frozen_bounds = removed.borrow().state.bounds.get();
    let parent_bounds = parent.borrow().state.bounds.get();
    let weak = Rc::downgrade(&removed);
    world.organisms.borrow_mut().remove(0);
    removed.borrow().state.bounds.update(|bounds| bounds.x = 800);
    removed.borrow().state.bounds.update(|bounds| bounds.y = 800);
    let moved_bounds = removed.borrow().state.bounds.get();
    assert_eq!(world.spatial.borrow().query(frozen_bounds), [0]);
    assert_eq!(world.spatial.borrow().query(parent_bounds), [1]);
    assert!(Rc::ptr_eq(&world.frame_objects.borrow()[1], &world.organisms.borrow()[0]));
    assert!(world.spatial.borrow().query(moved_bounds).is_empty());
    assert_eq!(
        world.with_frame_candidate(&removed, &parent, &parent.borrow(), None, |candidate| {
            candidate.state.bounds.get()
        }),
        moved_bounds
    );
    drop(removed);
    world.pause();
    assert!(
        weak.upgrade().is_some(),
        "Paused frames retain removed candidates"
    );
    parent.borrow_mut().state.children = 1;
    parent.borrow_mut().state.energy = 1000.0;
    let id = parent.borrow().id;
    world.spawn_child(id, true).unwrap().unwrap();
    let newborn = Rc::clone(world.organisms.borrow().last().unwrap());
    assert!(
        world.frame_objects.borrow()
            .iter()
            .all(|candidate| !Rc::ptr_eq(candidate, &newborn))
    );
    assert_eq!(world.frame_objects.borrow().len(), 3);
    world.rebuild_spatial().unwrap();
    assert!(
        weak.upgrade().is_none(),
        "Discarding the frozen frame releases removed objects"
    );
    assert!(
        world.frame_objects.borrow()
            .iter()
            .any(|candidate| Rc::ptr_eq(candidate, world.organisms.borrow().last().unwrap()))
    );
}

#[test]
fn copied_world_retains_membership_and_removed_frame_aliases_without_cross_world_sharing() {
    let mut original = world();
    add(&mut original, [100, 100], "removed");
    add(&mut original, [300, 300], "member");
    original.rebuild_spatial();
    original.organisms.borrow_mut().remove(0);
    let copied = original.clone();
    assert_eq!(original, copied);
    assert!(Rc::ptr_eq(&copied.organisms.borrow()[0], &copied.frame_objects.borrow()[1]));
    assert!(!Rc::ptr_eq(
        &copied.frame_objects.borrow()[0],
        &original.frame_objects.borrow()[0]
    ));
    copied.frame_objects.borrow()[0].borrow_mut().state.energy = 123.0;
    copied.frame_objects.borrow()[1]
        .borrow()
        .genome.as_ref().unwrap().borrow_mut()
        .set_clade_id(Some("copy".to_owned()));
    assert_ne!(original.frame_objects.borrow()[0].borrow().state.energy, 123.0);
    assert_eq!(
        copied.organisms.borrow()[0].borrow().genome.as_ref().unwrap().borrow().clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(),
        Some("copy")
    );
    assert_eq!(
        original.organisms.borrow()[0].borrow().genome.as_ref().unwrap().borrow().clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(),
        Some("member")
    );
}

#[test]
fn nested_birth_uses_held_parent_and_peer_and_preserves_identity_when_legacy_ids_wrap() {
    let mut world = world();
    world.next_id.set(i32::MAX);
    let first = add(&mut world, [100, 100], "first");
    let wrapped = add(&mut world, [300, 300], "wrapped");
    world.next_id.set(i32::MAX);
    let parent = add(&mut world, [700, 700], "parent");
    assert_eq!(first.borrow().id, i64::from(i32::MAX));
    assert_eq!(wrapped.borrow().id, i64::from(i32::MIN));
    assert_eq!(parent.borrow().id, first.borrow().id);
    assert!(!Rc::ptr_eq(&first, &parent));
    world.rebuild_spatial().unwrap();
    let child = world
        .construct_child_from(&parent, &mut parent.borrow_mut(), true, None)
        .unwrap()
        .unwrap();
    assert_eq!(child.borrow().genome.as_ref().unwrap().borrow().clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("parent"));

    let peer = Rc::clone(&wrapped);
    peer.borrow().state.bounds.set(IntRect {
        x: 1,
        y: 1,
        width: 998,
        height: 998,
    });
    world.rebuild_spatial().unwrap();
    let mut parent_state = parent.borrow_mut();
    let peer_state = peer.borrow_mut();
    parent_state.state.children = 1;
    parent_state.state.energy = 1000.0;
    let next_id = world.next_id.get();
    let result = world
        .construct_child_from(&parent, &mut parent_state, true, Some((&peer, &peer_state)))
        .unwrap();
    assert!(
        result.is_none(),
        "The held peer blocks every placement direction"
    );
    assert!(parent_state.saved_genome.is_some());
    assert_eq!(parent_state.state.energy, 1000.0);
    assert_eq!(world.next_id.get(), next_id);
}

#[test]
fn segment_rendering_preserves_java_integer_addition_at_wrapped_centers() {
    let mut world = world();
    let actor = add(&mut world, [100, 100], "render");
    {
        let mut actor = actor.borrow_mut();
        actor.state.angle = 0.0;
        crate::movement::calculate_bounds(&mut actor, true).unwrap();
        actor.state.integer_center = [i32::MAX, 0];
    }
    let segment = world.render_segments().unwrap()[0];
    assert_eq!(segment.x1, f64::from(i32::MAX - 1));
    assert_eq!(segment.x2, f64::from(i32::MIN));
}

#[test]
fn fast_overlap_excludes_only_self_and_reads_held_peers_after_membership_removal() {
    let mut world = world();
    let parent = add(&mut world, [100, 100], "parent");
    world.rebuild_spatial().unwrap();
    let parent_state = parent.borrow_mut();
    assert!(!world.fast_check_hit(&parent_state, &parent, &parent_state, None).unwrap());
    drop(parent_state);
    let peer = add(&mut world, [300, 300], "peer");
    peer.borrow_mut().id = parent.borrow().id;
    peer.borrow().state.bounds.set(parent.borrow().state.bounds.get());
    world.rebuild_spatial().unwrap();
    world.organisms.borrow_mut().remove(1);
    let parent_state = parent.borrow_mut();
    let peer_state = peer.borrow_mut();
    assert!(world.fast_check_hit(
        &parent_state,
        &parent,
        &parent_state,
        Some((&peer, &peer_state))
    ).unwrap());
    peer_state.state.bounds.update(|bounds| bounds.x = 800);
    assert!(!world.fast_check_hit(
        &parent_state,
        &parent,
        &parent_state,
        Some((&peer, &peer_state))
    ).unwrap());
}
