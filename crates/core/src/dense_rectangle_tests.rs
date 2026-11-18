// SPDX-License-Identifier: GPL-2.0-or-later

use super::peer_model_document;
use crate::legacy_graph::{GraphErrorKind, LegacyGraph};
use crate::legacy_rectangles::{Retention, retain_actor_context, stage_actor_rectangles_in};
use crate::object_stream::{Document, FieldValue, JavaObject, Limits, Node, NodeId, Value, class_schema};
use crate::organism::{GenomeCloner, OrganismRef};
use crate::RectangleRef;
use std::{cell::RefCell, rc::Rc};

fn source_bindings(count: usize) -> (Document, Vec<(NodeId, OrganismRef)>) {
    let (_model, mut document, original, _) = peer_model_document();
    let source = original[0].1.borrow();
    let Node::Object(template) = document.arena.nodes[original[0].0.0 as usize].clone() else {
        panic!("Source actor object required");
    };
    let rectangle_slot = template.slots.iter().find(|slot| {
        let Node::Descriptor(descriptor) = &document.arena.nodes[slot.descriptor.0 as usize] else { panic!("Descriptor required"); };
        class_schema(descriptor.class).name == "java.awt.Rectangle"
    }).unwrap().clone();
    let actor_slot = template.slots.iter().position(|slot| {
        let Node::Descriptor(descriptor) = &document.arena.nodes[slot.descriptor.0 as usize] else { panic!("Descriptor required"); };
        class_schema(descriptor.class).name == "biogenesis.Organism"
    }).unwrap();
    let Node::Descriptor(descriptor) = &document.arena.nodes[template.slots[actor_slot].descriptor.0 as usize] else { panic!("Descriptor required"); };
    let fields = class_schema(descriptor.class).fields;
    let id_field = fields.iter().position(|field| field.name == "_ID").unwrap();
    let size_field = fields.iter().position(|field| field.name == "_sizeRect").unwrap();
    let last_field = fields.iter().position(|field| field.name == "lastFrame").unwrap();
    let plain_node = NodeId(document.arena.nodes.len() as u32);
    document.arena.nodes.push(Node::Object(JavaObject {
        descriptor: rectangle_slot.descriptor, slots: vec![rectangle_slot],
    }));
    let plain = RectangleRef::new(source.state.bounds.get());
    let first = document.arena.nodes.len();
    let mut genomes = GenomeCloner::default();
    let mut bindings = Vec::with_capacity(count);
    for index in 0..count {
        let mut actor = source.copy_for_world(&mut genomes);
        actor.id = 77;
        actor.body.size_rect = None;
        actor.state.last_frame = None;
        let actor = Rc::new(RefCell::new(actor));
        actor.borrow().state.bounds.bind_actor(&actor).unwrap();
        let mut wire = template.clone();
        wire.slots[actor_slot].values[id_field] = FieldValue::Int(77);
        wire.slots[actor_slot].values[size_field] = FieldValue::Reference(Value::Handle(NodeId((first + (index + 1) % count) as u32)));
        wire.slots[actor_slot].values[last_field] = FieldValue::Reference(Value::Handle(plain_node));
        let node = NodeId(document.arena.nodes.len() as u32);
        document.arena.nodes.push(Node::Object(wire));
        bindings.push((node, actor));
    }
    for index in 0..count {
        let next = bindings[(index + 1) % count].1.borrow().state.bounds.clone();
        let mut actor = bindings[index].1.borrow_mut();
        actor.body.size_rect = Some(next);
        actor.state.last_frame = Some(plain.clone());
    }
    (document, bindings)
}

fn assert_staging(count: usize, limits: Limits) {
    let (document, bindings) = source_bindings(count);
    let before = bindings[0].1.borrow().snapshot();
    let graph = LegacyGraph::new(&document, limits).unwrap();
    let (staged, _) = stage_actor_rectangles_in(&graph, &bindings, limits.allocated_bytes).unwrap();
    assert_eq!(staged.actors.len(), count);
    assert_eq!(staged.context.actors().len(), count);
    for (index, state) in staged.actors.iter().enumerate() {
        assert!(Rc::ptr_eq(&state.actor, &bindings[index].1));
        assert!(state.bounds.same_instance(&bindings[index].1.borrow().state.bounds));
        assert!(state.size_rect.as_ref().unwrap().same_instance(&bindings[(index + 1) % count].1.borrow().state.bounds));
        assert!(state.last_frame.as_ref().unwrap().same_instance(staged.actors[0].last_frame.as_ref().unwrap()));
    }
    assert!(!staged.actors[0].bounds.same_instance(&staged.actors[1].bounds));
    assert_eq!(bindings[0].1.borrow().snapshot(), before);
}

#[test]
fn dense_rectangle_staging_preserves_12500_source_owners_with_default_work() {
    assert_staging(12_500, Limits::default());
}

#[test]
fn rectangle_staging_and_retention_fit_a_finite_linear_work_allowance() {
    assert_staging(512, Limits { work: 1_000_000, ..Limits::default() });
}

#[test]
fn rectangle_indexes_keep_duplicate_identity_and_budget_rejections() {
    let (document, bindings) = source_bindings(3);
    let limits = Limits::default();
    for duplicate in 0..3 {
        let mut values = bindings.clone();
        let saved = values[1].1.borrow().state.bounds.clone();
        match duplicate {
            0 => values[1].0 = values[0].0,
            1 => values[1].1 = values[0].1.clone(),
            _ => values[1].1.borrow_mut().state.bounds = values[0].1.borrow().state.bounds.clone(),
        }
        let graph = LegacyGraph::new(&document, limits).unwrap();
        assert!(matches!(stage_actor_rectangles_in(&graph, &values, limits.allocated_bytes),
            Err(error) if error.kind == GraphErrorKind::Reference));
        bindings[1].1.borrow_mut().state.bounds = saved;
    }
    let graph = LegacyGraph::new(&document, Limits { work: 0, ..limits }).unwrap();
    assert!(matches!(stage_actor_rectangles_in(&graph, &bindings, limits.allocated_bytes),
        Err(error) if error.kind == GraphErrorKind::Budget));
    let graph = LegacyGraph::new(&document, limits).unwrap();
    assert!(matches!(stage_actor_rectangles_in(&graph, &bindings, 1),
        Err(error) if error.kind == GraphErrorKind::Budget));
    assert!(matches!(retain_actor_context(bindings.iter().map(|(_, actor)| actor.clone()).collect(), limits, 1),
        Err(error) if error.kind == GraphErrorKind::Budget));
}

#[test]
fn retained_actor_identity_deduplicates_and_rejects_missing_targets() {
    let (_document, bindings) = source_bindings(4);
    let limits = Limits::default();
    let mut retention = Retention::new(limits.allocated_bytes, limits).unwrap();
    assert!(retention.retained_actor(&bindings[0].1).unwrap());
    assert!(!retention.retained_actor(&bindings[0].1).unwrap());
    assert!(matches!(retention.actor(&bindings[0].1), Err(error) if error.kind == GraphErrorKind::Reference));
    let actors = || bindings.iter().map(|(_, actor)| actor.clone()).collect();
    let original = bindings[0].1.borrow().state.last_frame.clone();
    let expired = RectangleRef::actor(bindings[0].1.borrow().state.bounds.get());
    bindings[0].1.borrow_mut().state.last_frame = Some(expired);
    assert!(matches!(retain_actor_context(actors(), limits, limits.allocated_bytes),
        Err(error) if error.kind == GraphErrorKind::Reference));
    bindings[0].1.borrow_mut().state.last_frame = original;
    let foreign = bindings[3].1.borrow().state.bounds.clone();
    bindings[0].1.borrow_mut().state.last_frame = Some(foreign);
    let mut subset: Vec<_> = bindings[..3].iter().map(|(_, actor)| actor.clone()).collect();
    subset[2].borrow_mut().body.size_rect = Some(subset[0].borrow().state.bounds.clone());
    assert!(matches!(retain_actor_context(subset, limits, limits.allocated_bytes),
        Err(error) if error.kind == GraphErrorKind::Reference));
}

#[test]
fn small_actor_retention_uses_actual_closure_with_default_handle_limit() {
    let (_document, bindings) = source_bindings(3);
    let maximum = 1024 * 1024;
    let limits = Limits { allocated_bytes: maximum, ..Limits::default() };
    let actors = || bindings.iter().map(|(_, actor)| actor.clone()).collect();
    let context = retain_actor_context(actors(), limits, maximum).unwrap();
    assert_eq!(context.actors().len(), bindings.len());
    for (retained, (_, source)) in context.actors().iter().zip(&bindings) {
        assert!(Rc::ptr_eq(retained, source));
    }
    let lower_ceiling = Limits { handles: 8192, ..limits };
    let lower = retain_actor_context(actors(), lower_ceiling, maximum).unwrap();
    assert_eq!(context.retained_bytes(), lower.retained_bytes());
    for (quota, bytes) in [
        (Limits { handles: bindings.len(), ..limits }, maximum),
        (Limits { work: 1, ..limits }, maximum),
        (limits, context.retained_bytes() - 1),
    ] {
        assert!(matches!(retain_actor_context(actors(), quota, bytes),
            Err(error) if error.kind == GraphErrorKind::Budget));
    }
}
