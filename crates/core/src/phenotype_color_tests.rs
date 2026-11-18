// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use crate::{ColorArrayRef, ColorKind, Gene, Genome, GenomeProperties, JavaColor};

fn genome(colors: &[Option<JavaColor>]) -> Genome {
    let genes: Vec<_> = colors.iter().map(|color|
        Gene::new_with_color(12.0, 0.0, color.clone(), 0, [0; 36]).shared()
    ).collect();
    Genome::from_parts(&genes, GenomeProperties { symmetry: 1, ..GenomeProperties::default() })
}

fn world() -> World {
    World::new(WorldConfig { initial_attempts: 0, width: 4000, height: 4000, ..WorldConfig::default() }).unwrap()
}

fn member(world: &mut World, colors: &[Option<JavaColor>], x: i32) -> OrganismRef {
    world.paste_genome(genome(colors), [x, 1000]).unwrap().unwrap();
    Rc::clone(world.organisms.borrow().last().unwrap())
}

#[test]
fn clone_isolates_mutable_arrays_and_retains_aliases_inside_the_copy() {
    let mut world = world();
    let fixed = ColorKind::Green.color_ref();
    let a = member(&mut world, &[fixed.clone()], 1000);
    let b = member(&mut world, &[fixed], 2000);
    b.borrow_mut().body.phenotype_colors = a.borrow().body.phenotype_colors.clone();
    let copied = world.clone();
    let left = copied.organisms.borrow()[0].borrow().body.phenotype_colors.clone().unwrap();
    let right = copied.organisms.borrow()[1].borrow().body.phenotype_colors.clone().unwrap();
    let original = a.borrow().body.phenotype_colors.clone().unwrap();
    assert!(left.same_instance(&right));
    assert!(!left.same_instance(&original));
    let unknown = JavaColor::from_argb(0x00123456);
    left.replace(0, Some(unknown.clone())).unwrap();
    assert!(right.get(0).unwrap().unwrap().same_instance(&unknown));
    assert_eq!(original.get(0).unwrap().unwrap().argb(), ColorKind::Green.argb().unwrap());
    let snapshot = copied.organisms.borrow()[0].borrow().body.clone();
    snapshot.set_color(0, None).unwrap();
    assert!(left.get(0).unwrap().is_some());
}

#[test]
fn death_keeps_flags_and_literal_prefix_before_null_slot_failure() {
    let mut world = world();
    let a = member(&mut world, &[ColorKind::Green.color_ref(), ColorKind::Green.color_ref(), ColorKind::Green.color_ref()], 1000);
    let original_eye = JavaColor::from_argb(ColorKind::Eye.argb().unwrap());
    let tail = JavaColor::from_argb(0xff123456);
    let colors = ColorArrayRef::from_slots(vec![Some(original_eye.clone()), None, Some(tail.clone())]);
    a.borrow_mut().body.phenotype_colors = Some(colors.clone());
    let population = world.population();
    let error = world.die(&mut a.borrow_mut(), None).unwrap_err();
    assert_eq!(error, WorldError::Geometry(GeometryError::MissingPhenotypeColor { segment: 1 }));
    assert!(!a.borrow().state.alive && !a.borrow().state.active && a.borrow().state.has_moved);
    assert_eq!(world.population(), population);
    let eye = colors.get(0).unwrap().unwrap();
    assert!(!eye.same_instance(&original_eye));
    assert!(eye.same_instance(&ColorKind::Eye.color_ref().unwrap()));
    assert!(colors.get(1).unwrap().is_none());
    assert!(colors.get(2).unwrap().unwrap().same_instance(&tail));
}

#[test]
fn death_distinguishes_null_array_short_array_and_present_unknown_color() {
    for (array, expected) in [
        (None, Some(GeometryError::MissingColorArray)),
        (Some(ColorArrayRef::from_slots(vec![])), Some(GeometryError::ColorIndex { segment: 0, length: 0 })),
        (Some(ColorArrayRef::from_slots(vec![None])), Some(GeometryError::MissingPhenotypeColor { segment: 0 })),
        (Some(ColorArrayRef::from_slots(vec![Some(JavaColor::from_argb(0x00123456))])), None),
    ] {
        let mut world = world();
        let a = member(&mut world, &[ColorKind::Green.color_ref()], 1000);
        a.borrow_mut().body.phenotype_colors = array;
        let result = world.die(&mut a.borrow_mut(), None);
        assert_eq!(result.err(), expected.clone().map(WorldError::Geometry));
        if expected.is_none() {
            assert!(a.borrow().body.required_color(0).unwrap().same_instance(&ColorKind::Brown.color_ref().unwrap()));
        }
    }
}

#[test]
fn creation_publishes_nullable_color_prefix_before_eye_dereference_failure() {
    let eye = JavaColor::from_argb(ColorKind::Eye.argb().unwrap());
    let code = genome(&[Some(eye.clone()), None, ColorKind::Green.color_ref()]);
    let mut body = Body::uncreated();
    assert_eq!(body.create_arrays(&code), Err(GeometryError::MissingColor { gene: 1 }));
    assert_eq!(body.phenotype_colors.as_ref().unwrap().len(), 3);
    assert!(body.required_color(0).unwrap().same_instance(&eye));
    assert!(body.color(1).unwrap().is_none() && body.color(2).unwrap().is_none());
    assert!(body.eyes.enabled);
    assert_eq!(body.segment_count, 3);
    for (channel, owner) in body.arrays.reactions.iter().enumerate() {
        let owner = owner.as_ref().unwrap();
        assert_eq!(owner.len(), 3);
        assert_eq!(owner.values(), [code.gene_ref(0).unwrap().borrow().reactions()[channel], 0, 0]);
    }
    for field in [crate::IntField::StartX, crate::IntField::StartY, crate::IntField::EndX, crate::IntField::EndY,
        crate::IntField::X1, crate::IntField::Y1, crate::IntField::X2, crate::IntField::Y2] {
        assert!(body.arrays.int(field).is_none());
    }
    for field in crate::organism_arrays::DOUBLE_FIELDS { assert!(body.arrays.double(field).is_none()); }
}

#[test]
fn cached_no_eye_rebuild_does_not_dereference_late_null_gene_colors() {
    let code = genome(&[ColorKind::Green.color_ref()]);
    let mut body = Body::at_growth(&code, 16).unwrap();
    let colors = body.phenotype_colors.clone().unwrap();
    code.gene_ref(0).unwrap().borrow_mut().set_color_handle(None);
    let rebuilt = body.rebuild(&code, 8, body.eyes, 0).unwrap();
    assert!(rebuilt.phenotype_colors.as_ref().unwrap().same_instance(&colors));
    assert_eq!(rebuilt.color_kind(0).unwrap(), ColorKind::Green);
}

#[test]
fn ordered_draw_retains_prefix_paint_and_bypasses_unused_color_arrays() {
    let mut world = world();
    let a = member(&mut world, &[ColorKind::Green.color_ref(), ColorKind::Green.color_ref()], 1000);
    let mut actor = a.borrow_mut();
    actor.body.phenotype_colors = Some(ColorArrayRef::from_slots(vec![ColorKind::Eye.color_ref(), None]));
    actor.body.eyes.enabled = true;
    actor.state.temporary_color_frames = 1;
    actor.state.temporary_color = None;
    let mut paint = 0xff123456;
    let mut lines = Vec::new();
    assert_eq!(outgoing::emit_actor_segments(&actor, &mut paint, |line| lines.push(line)),
        Err(GeometryError::MissingPhenotypeColor { segment: 1 }));
    assert_eq!(lines.len(), 1);
    assert_eq!(paint, ColorKind::Eye.argb().unwrap());
    actor.body.phenotype_colors = None;
    actor.body.eyes.enabled = false;
    lines.clear();
    outgoing::emit_actor_segments(&actor, &mut paint, |line| lines.push(line)).unwrap();
    assert_eq!(lines.len(), 2);
    assert!(lines.iter().all(|line| line.rgb == (paint & 0xffffff)));
    actor.state.temporary_color_frames = -1;
    actor.state.alive = false;
    outgoing::emit_actor_segments(&actor, &mut paint, |_| {}).unwrap();
    assert_eq!(paint, ColorKind::Brown.argb().unwrap());
}

#[test]
fn world_draw_failure_exposes_completed_lines_without_mutating_model() {
    let mut world = world();
    let a = member(&mut world, &[ColorKind::Green.color_ref(), ColorKind::Green.color_ref()], 1000);
    {
        let mut actor = a.borrow_mut();
        actor.body.eyes.enabled = true;
        actor.state.temporary_color_frames = 2;
        actor.body.set_color(1, None).unwrap();
    }
    let before = world.clone();
    let WorldError::Drawing { commands, paint, error } = world.render_segments().unwrap_err() else {
        panic!("Expected ordered drawing failure");
    };
    assert_eq!(commands.len(), 1);
    assert_eq!(paint, 0xffffffff);
    assert_eq!(error, GeometryError::MissingPhenotypeColor { segment: 1 });
    assert_eq!(world, before);
}

#[test]
fn leaf_failure_retains_methanotrophy_before_null_temporary_dereference() {
    let mut world = world();
    let a = member(&mut world, &[ColorKind::Green.color_ref()], 1000);
    let mut actor = a.borrow_mut();
    actor.effects.methanotrophy = 2.0;
    actor.state.age = 0;
    actor.state.temporary_color_frames = 1;
    actor.state.temporary_color = None;
    world.methane.set(100.0);
    let energy = actor.state.energy;
    let error = world.leaf_extra_effects(&mut actor).unwrap_err();
    assert_eq!(error, WorldError::Geometry(GeometryError::MissingTemporaryColor));
    assert!(actor.state.energy > energy);
    assert!(world.methane.get() < 100.0);
}


#[test]
fn temporary_reference_assignment_copy_and_expiry_preserve_the_supplied_owner() {
    let mut world = world();
    let a = member(&mut world, &[ColorKind::Green.color_ref()], 1000);
    let color = JavaColor::from_argb(0x12123456);
    let mut actor = a.borrow_mut();
    actor.set_display_color(Some(color.clone()));
    assert_eq!(actor.state.temporary_color_frames, 10);
    assert!(actor.state.temporary_color.as_ref().unwrap().same_instance(&color));
    actor.set_display_color(None);
    assert_eq!(actor.state.temporary_color_frames, 10);
    assert!(actor.state.temporary_color.is_none());
    actor.state.temporary_color = Some(color.clone());
    actor.state.temporary_color_frames = -1;
    let copy = actor.copy_for_world(&mut crate::organism::GenomeCloner::default());
    assert!(copy.state.temporary_color.as_ref().unwrap().same_instance(&color));
    assert_eq!(copy.state.temporary_color_frames, -1);
    actor.state.temporary_color_frames = 1;
    drop(actor);
    world.pause();
    assert!(world.single_step().unwrap());
    assert_eq!(a.borrow().state.temporary_color_frames, 0);
    assert!(a.borrow().state.temporary_color.as_ref().unwrap().same_instance(&color));
}

#[test]
fn organism_rebuild_retains_array_color_and_rectangle_owners() {
    let mut world = world();
    let actor = member(&mut world, &[ColorKind::Green.color_ref()], 1000);
    let mut actor = actor.borrow_mut();
    let arrays = actor.body.arrays.clone();
    let colors = actor.body.phenotype_colors.clone().unwrap();
    let rectangle = actor.body.size_rect.clone().unwrap();
    let old_mass = actor.body.mass;
    actor.genome.as_ref().unwrap().borrow().gene_ref(0).unwrap()
        .borrow_mut().set_color_handle(None);
    actor.body.growth_ratio = 1;
    actor.rebuild_body().unwrap();
    assert_eq!(actor.body.mass, 12.0);
    assert!(actor.body.mass > old_mass);
    assert!(actor.body.phenotype_colors.as_ref().unwrap().same_instance(&colors));
    assert!(actor.body.size_rect.as_ref().unwrap().same_instance(&rectangle));
    assert_eq!(rectangle.get().width, 13);
    assert_eq!(actor.body.required_color(0).unwrap().argb(), ColorKind::Green.argb().unwrap());
    for field in crate::organism_arrays::INT_FIELDS {
        assert!(actor.body.arrays.int(field).as_ref().unwrap().same_instance(arrays.int(field).as_ref().unwrap()));
    }
    for field in crate::organism_arrays::DOUBLE_FIELDS {
        assert!(actor.body.arrays.double(field).as_ref().unwrap().same_instance(arrays.double(field).as_ref().unwrap()));
    }
}

#[test]
fn organism_rebuild_retains_completed_prefix_before_missing_endpoint_failure() {
    let mut world = world();
    let actor = member(&mut world, &[ColorKind::Green.color_ref()], 1000);
    let mut actor = actor.borrow_mut();
    let length = actor.body.arrays.length.clone().unwrap();
    let end_x = actor.body.arrays.end_x.clone().unwrap();
    let rectangle = actor.body.size_rect.clone().unwrap();
    let old_size = rectangle.get();
    let old_inertia = actor.body.inertia;
    actor.body.growth_ratio = 1;
    actor.body.update_effects = 0;
    actor.body.arrays.end_y = None;
    assert_eq!(actor.rebuild_body(), Err(GeometryError::MissingPrimitiveArray { field: "_endPointY" }));
    assert_eq!(length.get(0, "_m").unwrap(), 12.0);
    assert_eq!(end_x.get(0, "_endPointX").unwrap(), 12);
    assert_eq!(actor.body.arrays.read_int(crate::IntField::StartX, 0).unwrap(), 0);
    assert_eq!(actor.body.arrays.read_int(crate::IntField::StartY, 0).unwrap(), 0);
    assert_eq!(actor.body.mass, 12.0);
    assert_eq!(actor.body.growth_ratio, 1);
    assert_eq!(actor.body.update_effects, 1);
    assert_eq!(actor.body.inertia.to_bits(), old_inertia.to_bits());
    assert_eq!(rectangle.get(), old_size);
    assert!(actor.body.arrays.length.as_ref().unwrap().same_instance(&length));
    assert!(actor.body.arrays.end_x.as_ref().unwrap().same_instance(&end_x));
    assert!(actor.body.arrays.end_y.is_none());
}
