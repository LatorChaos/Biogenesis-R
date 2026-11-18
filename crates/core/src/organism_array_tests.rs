// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use crate::legacy_graph::{GraphErrorKind, LegacyGraph};
use crate::legacy_organism_arrays::{PrimitiveRoot, PrimitiveStager, export_primitive_arrays};
use crate::object_stream::{Limits, Record, StreamReader, Value, encode_document};
use crate::{ColorKind, DoubleArrayRef, Gene, Genome, GenomeProperties, IntArrayRef};

fn world() -> World {
    World::new(WorldConfig {
        initial_attempts: 0,
        width: 4000,
        height: 4000,
        ..WorldConfig::default()
    })
    .unwrap()
}
fn code(color: ColorKind, count: usize) -> Genome {
    let genes: Vec<_> = (0..count)
        .map(|_| Gene::new_with_color(12.0, 0.0, color.color_ref(), 0, [0; 36]).shared())
        .collect();
    Genome::from_parts(
        &genes,
        GenomeProperties {
            symmetry: 1,
            ..GenomeProperties::default()
        },
    )
}
fn member(world: &mut World, color: ColorKind, count: usize, x: i32) -> OrganismRef {
    world
        .paste_genome(code(color, count), [x, 1000])
        .unwrap()
        .unwrap();
    Rc::clone(world.organisms.borrow().last().unwrap())
}
fn roots(document: &crate::object_stream::Document) -> Vec<Value> {
    document
        .records
        .iter()
        .map(|record| match record {
            Record::Value(value) => *value,
            _ => panic!("Primitive root record"),
        })
        .collect()
}

#[test]
fn primitive_graph_roundtrip_preserves_null_lengths_aliases_and_raw_double_state() {
    let shared = IntArrayRef::from_values(vec![i32::MIN, 0, i32::MAX]);
    let equal = IntArrayRef::from_values(shared.values());
    let empty = IntArrayRef::from_values(vec![]);
    let raw = [
        0_u64,
        0x8000000000000000,
        0x7ff0000000000000,
        0xfff0000000000000,
        0x7ff0000000000001,
        0xfff8123456789abc,
    ];
    let doubles = DoubleArrayRef::from_bits(raw.to_vec());
    let inputs = [
        PrimitiveRoot::Int(None),
        PrimitiveRoot::Int(Some(&shared)),
        PrimitiveRoot::Int(Some(&shared)),
        PrimitiveRoot::Int(Some(&equal)),
        PrimitiveRoot::Int(Some(&empty)),
        PrimitiveRoot::Double(Some(&doubles)),
        PrimitiveRoot::Double(Some(&doubles)),
        PrimitiveRoot::Double(None),
    ];
    let limits = Limits::default();
    let document = export_primitive_arrays(&inputs, limits, 1024 * 1024).unwrap();
    let values = roots(&document);
    let (staged, staged_double) = {
        let graph = LegacyGraph::new(&document, limits).unwrap();
        let mut stager = PrimitiveStager::new(&graph, 1024 * 1024).unwrap();
        assert!(stager.ints(values[0]).unwrap().is_none());
        let first = stager.ints(values[1]).unwrap().unwrap();
        let budget = stager.allocated_bytes();
        let repeated = stager.ints(values[2]).unwrap().unwrap();
        assert_eq!(stager.allocated_bytes(), budget);
        assert!(first.same_instance(&repeated));
        let distinct = stager.ints(values[3]).unwrap().unwrap();
        assert_eq!(first, distinct);
        assert!(!first.same_instance(&distinct));
        assert!(stager.ints(values[4]).unwrap().unwrap().is_empty());
        let floating = stager.doubles(values[5]).unwrap().unwrap();
        assert_eq!(floating.bits(), raw);
        assert!(floating.same_instance(&stager.doubles(values[6]).unwrap().unwrap()));
        assert!(stager.doubles(values[7]).unwrap().is_none());
        assert_eq!(
            stager.ints(values[5]).unwrap_err().kind,
            GraphErrorKind::Class
        );
        (first, floating)
    };
    drop(document);
    staged.replace(1, -7, "_startPointX").unwrap();
    assert_eq!(staged.values(), [i32::MIN, -7, i32::MAX]);
    assert_eq!(staged_double.bits(), raw);
    assert_eq!(shared.values(), [i32::MIN, 0, i32::MAX]);

    let wire_document = export_primitive_arrays(&inputs, limits, 1024 * 1024).unwrap();
    let wire = encode_document(&wire_document, limits).unwrap();
    let decoded = StreamReader::new(&wire, limits)
        .unwrap()
        .read_document()
        .unwrap();
    let graph = LegacyGraph::new(&decoded, limits).unwrap();
    let mut stager = PrimitiveStager::new(&graph, 1024 * 1024).unwrap();
    let after = stager.doubles(roots(&decoded)[5]).unwrap().unwrap();
    assert_eq!(
        after.bits(),
        [
            0,
            0x8000000000000000,
            0x7ff0000000000000,
            0xfff0000000000000,
            0x7ff8000000000000,
            0x7ff8000000000000
        ]
    );
    assert_eq!(doubles.bits(), raw);
}

#[test]
fn primitive_staging_and_export_budgets_fail_without_mutating_existing_owners() {
    let owner = IntArrayRef::from_values(vec![1, 2, 3]);
    let limits = Limits::default();
    let inputs = [
        PrimitiveRoot::Int(Some(&owner)),
        PrimitiveRoot::Int(Some(&owner)),
    ];
    let document = export_primitive_arrays(&inputs, limits, 1024 * 1024).unwrap();
    let graph = LegacyGraph::new(&document, limits).unwrap();
    assert_eq!(
        PrimitiveStager::new(&graph, 0).err().unwrap().kind,
        GraphErrorKind::Budget
    );
    assert_eq!(
        export_primitive_arrays(&inputs, limits, 0)
            .unwrap_err()
            .kind,
        GraphErrorKind::Budget
    );
    let mut tight = limits;
    tight.total_array_elements = 2;
    assert_eq!(
        export_primitive_arrays(&inputs, tight, 1024 * 1024)
            .unwrap_err()
            .kind,
        GraphErrorKind::Budget
    );
    let mut exact = limits;
    exact.total_array_elements = 3;
    assert!(export_primitive_arrays(&inputs, exact, 1024 * 1024).is_ok());
    assert_eq!(owner.values(), [1, 2, 3]);
    assert_eq!(roots(&document)[0], roots(&document)[1]);
}

#[test]
fn whole_world_clone_isolates_primitive_storage_and_preserves_cross_actor_field_aliases() {
    let mut world = world();
    let a = member(&mut world, ColorKind::Green, 1, 1000);
    let b = member(&mut world, ColorKind::Green, 1, 2000);
    let shared = IntArrayRef::from_values(vec![9, -8]);
    let doubles = DoubleArrayRef::from_bits(vec![0x8000000000000000, 0x7ff0000000000001]);
    a.borrow_mut().body.arrays.start_x = Some(shared.clone());
    a.borrow_mut().body.arrays.end_y = Some(shared.clone());
    b.borrow_mut().body.arrays.reactions[3] = Some(shared.clone());
    a.borrow_mut().body.arrays.length = Some(doubles.clone());
    b.borrow_mut().body.arrays.photo = Some(doubles.clone());
    *world.frame_objects.borrow_mut() = vec![Rc::clone(&b), Rc::clone(&a)];
    let copied = world.clone();
    let left = copied.organisms.borrow()[0]
        .borrow()
        .body
        .arrays
        .start_x
        .clone()
        .unwrap();
    assert!(
        left.same_instance(
            copied.organisms.borrow()[0]
                .borrow()
                .body
                .arrays
                .end_y
                .as_ref()
                .unwrap()
        )
    );
    assert!(
        left.same_instance(
            copied.organisms.borrow()[1].borrow().body.arrays.reactions[3]
                .as_ref()
                .unwrap()
        )
    );
    assert!(Rc::ptr_eq(&copied.frame_objects.borrow()[0], &copied.organisms.borrow()[1]));
    assert!(!left.same_instance(&shared));
    left.replace(0, 55, "_startPointX").unwrap();
    assert_eq!(
        copied.organisms.borrow()[1].borrow().body.reaction(0, 3).unwrap(),
        55
    );
    assert_eq!(shared.values(), [9, -8]);
    let floating = copied.organisms.borrow()[0]
        .borrow()
        .body
        .arrays
        .length
        .clone()
        .unwrap();
    assert!(
        floating.same_instance(
            copied.organisms.borrow()[1]
                .borrow()
                .body
                .arrays
                .photo
                .as_ref()
                .unwrap()
        )
    );
    assert!(!floating.same_instance(&doubles));
    floating.replace_bits(0, 0x7ff8123456789abc, "_m").unwrap();
    assert_eq!(doubles.bits(), [0x8000000000000000, 0x7ff0000000000001]);
    let detached = copied.organisms.borrow()[0].borrow().body.clone();
    assert!(
        detached
            .arrays
            .start_x
            .as_ref()
            .unwrap()
            .same_instance(detached.arrays.end_y.as_ref().unwrap())
    );
    assert!(
        !detached
            .arrays
            .start_x
            .as_ref()
            .unwrap()
            .same_instance(&left)
    );
}

#[test]
fn saved_signed_count_controls_source_loops_without_repairing_array_lengths() {
    for friendly in [false, true] {
        for count in [0, -1, i32::MIN] {
            let mut world = world();
            let a = member(&mut world, ColorKind::Green, 1, 1000);
            let b = member(&mut world, ColorKind::Green, 1, 2000);
            let mut actor = a.borrow_mut();
            let peer = b.borrow();
            actor.body.segment_count = count;
            let before = actor.snapshot();
            let random = world.random_snapshot();
            let gas = [world.oxygen.get(), world.carbon_dioxide.get()];
            let outcome = if friendly {
                world.touch_friend_reaction(&mut actor, &peer, 0, 0)
            } else {
                world.touch_reaction(&mut actor, &peer, 0, 0)
            };
            if count == i32::MIN {
                assert_eq!(
                    outcome,
                    Err(WorldError::Geometry(GeometryError::ColorIndex {
                        segment: i32::MAX as usize,
                        length: 1
                    }))
                );
            } else {
                assert_eq!(outcome, Ok(()));
            }
            assert_eq!(actor.snapshot(), before);
            assert_eq!(world.random_snapshot(), random);
            assert_eq!([world.oxygen.get(), world.carbon_dioxide.get()], gas);
            assert_eq!(actor.body.segment_count, count);
        }
    }
}

#[test]
fn bounds_bypasses_unused_sources_and_retains_rotated_write_prefix_on_late_failure() {
    let mut world = world();
    let a = member(&mut world, ColorKind::Green, 2, 1000);
    let mut actor = a.borrow_mut();
    actor.state.angle = 0.0;
    actor.state.last_angle = 0.0;
    actor.body.arrays.start_x = None;
    actor.body.arrays.start_y = None;
    actor.body.arrays.end_x = None;
    actor.body.arrays.end_y = None;
    actor.body.arrays.start_radius = None;
    actor.body.arrays.end_radius = None;
    let cached = actor.body.rotated_values().unwrap();
    crate::movement::calculate_bounds(&mut actor, false).unwrap();
    assert_eq!(actor.body.rotated_values().unwrap(), cached);
    actor.body.arrays.start_x = Some(IntArrayRef::from_values(vec![0, 2]));
    actor.body.arrays.start_y = Some(IntArrayRef::from_values(vec![0, 0]));
    actor.body.arrays.end_x = Some(IntArrayRef::from_values(vec![1, 3]));
    actor.body.arrays.end_y = Some(IntArrayRef::from_values(vec![0, 0]));
    actor.body.arrays.start_radius = Some(DoubleArrayRef::from_values(vec![0.0, 2.0]));
    actor.body.set_rotated_start(1, 0, -99).unwrap();
    actor.body.set_rotated_start(1, 1, -99).unwrap();
    let last_angle = actor.state.last_angle;
    let bounds = actor.state.bounds.get();
    assert_eq!(
        crate::movement::calculate_bounds(&mut actor, true),
        Err(GeometryError::MissingPrimitiveArray { field: "_m2" })
    );
    assert_eq!(actor.body.rotated_start(1, 0).unwrap(), 2);
    assert_eq!(actor.body.rotated_start(1, 1).unwrap(), 0);
    assert_eq!(actor.state.last_angle, last_angle);
    assert_eq!(actor.state.bounds.get(), bounds);
    assert!(actor.body.arrays.end_radius.is_none());
}

#[test]
fn mimic_channel_stores_observe_shared_rhs_and_keep_color_before_later_null_destination() {
    let mut world = world();
    let a = member(&mut world, ColorKind::Dark, 2, 1000);
    let b = member(&mut world, ColorKind::Red, 1, 2000);
    let mut actor = a.borrow_mut();
    let mut peer = b.borrow_mut();
    let genome = actor.genome.as_ref().unwrap().clone();
    let mut properties = genome.borrow().properties();
    properties.modifiers.mimic_all = true;
    genome.borrow_mut().set_properties(properties);
    actor.effects.segment_pre_touch_effects = 2;
    actor.effects.forest_photo = 0.0;
    actor.effects.colony_photosynthesis = 0.0;
    let shared = IntArrayRef::from_values(vec![0, 0]);
    actor.body.arrays.reactions[0] = Some(shared.clone());
    peer.body.arrays.reactions[1] = Some(shared.clone());
    peer.body.set_reaction(0, 0, 1).unwrap();
    actor.body.arrays.reactions[2] = None;
    let green = actor.body.arrays.reactions[1].clone().unwrap();
    let peer_color = peer.body.required_color(0).unwrap();
    assert_eq!(
        world.pre_touch_effects(&mut actor, &mut peer, 0, 0),
        Err(WorldError::Geometry(GeometryError::MissingPrimitiveArray {
            field: "_segblueReaction"
        }))
    );
    assert!(
        actor
            .body
            .required_color(0)
            .unwrap()
            .same_instance(&peer_color)
    );
    assert_eq!(shared.values(), [1, 0]);
    assert_eq!(green.get(0, "_seggreenReaction").unwrap(), 1);
    assert_eq!(green.get(1, "_seggreenReaction").unwrap(), 0);
    assert!(actor.body.arrays.reactions[2].is_none());
}

#[test]
fn paid_contact_helpers_propagate_missing_length_after_payment_before_tint_or_drain() {
    for actor_color in [ColorKind::Spike, ColorKind::Orange, ColorKind::Cream] {
        let mut world = world();
        let a = member(&mut world, actor_color, 1, 1000);
        let b = member(&mut world, ColorKind::Green, 1, 2000);
        let mut actor = a.borrow_mut();
        let mut peer = b.borrow_mut();
        actor.body.arrays.length = None;
        actor.effects.is_enhanced = false;
        actor.effects.is_plant = true;
        actor.state.energy = 100.0;
        peer.state.energy = 200.0;
        peer.effects.dodge = false;
        peer.effects.lavender = 0;
        actor.state.temporary_color = None;
        peer.state.temporary_color = None;
        world.oxygen.set(100.0);
        world.carbon_dioxide.set(0.0);
        world.session.0.parameters.borrow_mut().lifecycle.spike_energy_consumption = 1.0;
        world.session.0.parameters.borrow_mut().lifecycle.orange_energy_consumption = 1.0;
        world.session.0.parameters.borrow_mut().lifecycle.cream_energy_consumption = 1.0;
        let random = world.random_snapshot();
        let outcome = match actor_color {
            ColorKind::Spike => world.touch_spikepoint(&mut actor, &mut peer, 0, 0),
            ColorKind::Orange => world.touch_orange(&mut actor, &mut peer, 0, 0),
            ColorKind::Cream => world.touch_cream(&mut actor, &mut peer, 0, 0),
            _ => unreachable!(),
        };
        assert_eq!(
            outcome,
            Err(WorldError::Geometry(GeometryError::MissingPrimitiveArray {
                field: "_m"
            }))
        );
        assert_eq!((actor.state.energy, peer.state.energy), (99.0, 200.0));
        assert_eq!((world.oxygen.get(), world.carbon_dioxide.get()), (99.0, 1.0));
        assert!(actor.state.temporary_color.is_none() && peer.state.temporary_color.is_none());
        assert!(peer.effects.has_dodged);
        assert_eq!(world.random_snapshot(), random);
    }
}

#[test]
fn oversized_public_primitive_indices_never_alias_or_mutate_slot_zero() {
    let genome = code(ColorKind::Green, 1);
    let body = Body::at_growth(&genome, 16).unwrap();
    let oversized = if usize::BITS > 32 {
        1_usize.checked_shl(32).unwrap()
    } else {
        i32::MAX as usize + 1
    };
    let expected = GeometryError::PrimitiveIndexDomain { index: oversized };
    let before = body.clone();
    assert_eq!(body.physical_length(oversized), Err(expected));
    assert_eq!(body.segment(oversized), Err(expected));
    assert_eq!(body.rotated_segment(oversized), Err(expected));
    assert_eq!(body.set_photo(oversized, 99.0), Err(expected));
    assert_eq!(body.set_reaction(oversized, 0, 37), Err(expected));
    assert_eq!(body.set_rotated_start(oversized, 0, -123), Err(expected));
    assert_eq!(body, before);
    assert_eq!(body.photo(0).unwrap().to_bits(), 0);
    assert_eq!(body.reaction(0, 0).unwrap(), 0);
}
