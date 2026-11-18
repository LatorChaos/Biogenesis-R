// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use crate::legacy_spatial::decode_world_spatial;
use crate::object_stream::{ArrayData, FieldValue, JavaString, Node, StreamReader, class_schema};
use crate::{ColorKind, Gene, JavaColor, JavaRandom, World, WorldConfig, WorldError, WorldParameters};
use std::collections::HashMap;

#[path = "legacy_jts_streams.rs"]
mod jts_inputs;

fn document(bytes: &[u8]) -> Document {
    StreamReader::new(bytes, Limits::default()).unwrap().read_document().unwrap()
}
fn limits(document: &Document) -> Limits {
    Limits { handles: document.arena.nodes.len() + 16, ..Limits::default() }
}
fn nodes(document: &Document, class: &str) -> Vec<NodeId> {
    document.arena.nodes.iter().enumerate().filter_map(|(index, value)| {
        let Node::Object(value) = value else { return None };
        let Node::Descriptor(descriptor) = &document.arena.nodes[value.descriptor.0 as usize] else { panic!("Descriptor") };
        (class_schema(descriptor.class).name == class).then_some(NodeId(index as u32))
    }).collect()
}
fn worlds(document: &Document) -> Vec<(NodeId, WorldHandle)> {
    nodes(document, WORLD).into_iter().map(|id| {
        let owner = World::with_random(
            WorldConfig { initial_attempts: 0, width: 7000, height: 7000, ..WorldConfig::default() },
            WorldParameters::default(), JavaRandom::new(9245 + i64::from(id.0)),
        ).unwrap();
        (id, owner.handle())
    }).collect()
}
fn set(document: &mut Document, id: NodeId, owner: &str, name: &str, value: FieldValue) {
    let Node::Object(object) = &document.arena.nodes[id.0 as usize] else { panic!("Object") };
    let (slot, position) = object.slots.iter().enumerate().find_map(|(index, slot)| {
        let Node::Descriptor(descriptor) = &document.arena.nodes[slot.descriptor.0 as usize] else { panic!("Descriptor") };
        let schema = class_schema(descriptor.class);
        if schema.name != owner { return None }
        schema.fields.iter().position(|field| field.name == name).map(|position| (index, position))
    }).unwrap();
    let Node::Object(object) = &mut document.arena.nodes[id.0 as usize] else { unreachable!() };
    object.slots[slot].values[position] = value;
}
fn reference(document: &Document, id: NodeId, owner: &'static str, name: &'static str) -> Value {
    LegacyGraph::new(document, limits(document)).unwrap().reference(id, owner, name).unwrap()
}
fn handle(value: Value) -> NodeId { let Value::Handle(value) = value else { panic!("Handle") }; value }

#[derive(Default)]
pub(crate) struct Identities {
    forward: HashMap<(u8, u32), usize>,
    reverse: HashMap<(u8, usize), NodeId>,
}
impl Identities {
    pub(crate) fn bind(&mut self, kind: u8, source: Value, target: Option<usize>) {
        match (source, target) {
            (Value::Null, None) => {},
            (Value::Handle(node), Some(identity)) => {
                if let Some(previous) = self.forward.insert((kind, node.0), identity) { assert_eq!(previous, identity); }
                if let Some(previous) = self.reverse.insert((kind, identity), node) { assert_eq!(previous, node); }
            }
            _ => panic!("Reference nullness changed"),
        }
    }
}

pub(crate) fn assert_actor(graph: &LegacyGraph<'_>, id: NodeId, actor: &Organism, identities: &mut Identities) {
    let mut compared = 0;
    assert_eq!(actor.body.inertia.to_bits(), graph.double_bits(id, ORGANISM, "_I").unwrap(), "_I"); compared += 1;
    assert_eq!(actor.id, i64::from(graph.int(id, ORGANISM, "_ID").unwrap()), "_ID"); compared += 1;
    assert_eq!(actor.state.age, graph.int(id, ORGANISM, "_age").unwrap(), "_age"); compared += 1;
    assert_eq!(actor.effects.all_frozen, graph.boolean(id, ORGANISM, "_allfrozen").unwrap(), "_allfrozen"); compared += 1;
    assert_eq!(actor.effects.altruist, graph.boolean(id, ORGANISM, "_altruist").unwrap(), "_altruist"); compared += 1;
    assert_eq!(actor.effects.antiviral, graph.int(id, ORGANISM, "_antiviral").unwrap(), "_antiviral"); compared += 1;
    assert_eq!(actor.effects.can_dodge, graph.boolean(id, ORGANISM, "_candodge").unwrap(), "_candodge"); compared += 1;
    assert_eq!(actor.effects.can_move, graph.boolean(id, ORGANISM, "_canmove").unwrap(), "_canmove"); compared += 1;
    assert_eq!(actor.effects.can_react, graph.boolean(id, ORGANISM, "_canreact").unwrap(), "_canreact"); compared += 1;
    assert_eq!(actor.state.integer_center[0], graph.int(id, ORGANISM, "_centerX").unwrap(), "_centerX"); compared += 1;
    assert_eq!(actor.state.integer_center[1], graph.int(id, ORGANISM, "_centerY").unwrap(), "_centerY"); compared += 1;
    assert_eq!(actor.effects.clockwise, graph.boolean(id, ORGANISM, "_clockwise").unwrap(), "_clockwise"); compared += 1;
    assert_eq!(actor.effects.colony_photosynthesis.to_bits(), graph.double_bits(id, ORGANISM, "_colonyPhotosynthesis").unwrap(), "_colonyPhotosynthesis"); compared += 1;
    assert_eq!(actor.effects.create_lavender, graph.int(id, ORGANISM, "_createlavender").unwrap(), "_createlavender"); compared += 1;
    assert_eq!(actor.effects.crowded, graph.boolean(id, ORGANISM, "_crowded").unwrap(), "_crowded"); compared += 1;
    assert_eq!(actor.state.center[0].to_bits(), graph.double_bits(id, ORGANISM, "_dCenterX").unwrap(), "_dCenterX"); compared += 1;
    assert_eq!(actor.state.center[1].to_bits(), graph.double_bits(id, ORGANISM, "_dCenterY").unwrap(), "_dCenterY"); compared += 1;
    assert_eq!(actor.effects.dodge, graph.boolean(id, ORGANISM, "_dodge").unwrap(), "_dodge"); compared += 1;
    assert_eq!(actor.state.reproduction.early_energy, graph.int(id, ORGANISM, "_earlyReproduceEnergy").unwrap(), "_earlyReproduceEnergy"); compared += 1;
    assert_eq!(actor.state.energy.to_bits(), graph.double_bits(id, ORGANISM, "_energy").unwrap(), "_energy"); compared += 1;
    assert_eq!(actor.effects.fallow_version, graph.int(id, ORGANISM, "_fallowversion").unwrap(), "_fallowversion"); compared += 1;
    assert_eq!(actor.effects.forest_photo.to_bits(), graph.double_bits(id, ORGANISM, "_forestphoto").unwrap(), "_forestphoto"); compared += 1;
    assert_eq!(actor.state.temporary_color_frames, graph.int(id, ORGANISM, "_framesColor").unwrap(), "_framesColor"); compared += 1;
    assert_eq!(actor.effects.generation_battle, graph.boolean(id, ORGANISM, "_generationbattle").unwrap(), "_generationbattle"); compared += 1;
    assert_eq!(actor.effects.gold, graph.int(id, ORGANISM, "_gold").unwrap(), "_gold"); compared += 1;
    assert_eq!(actor.body.growth_ratio, graph.int(id, ORGANISM, "_growthRatio").unwrap(), "_growthRatio"); compared += 1;
    assert_eq!(actor.effects.has_dodged, graph.boolean(id, ORGANISM, "_hasdodged").unwrap(), "_hasdodged"); compared += 1;
    assert_eq!(actor.body.eyes.enabled, graph.boolean(id, ORGANISM, "_haseyes").unwrap(), "_haseyes"); compared += 1;
    assert_eq!(actor.body.eyes.good_vision, graph.boolean(id, ORGANISM, "_hasgoodvision").unwrap(), "_hasgoodvision"); compared += 1;
    assert_eq!(actor.effects.healing, graph.int(id, ORGANISM, "_healing").unwrap(), "_healing"); compared += 1;
    assert_eq!(actor.effects.indigo, graph.int(id, ORGANISM, "_indigo").unwrap(), "_indigo"); compared += 1;
    assert_eq!(actor.state.reproduction.infected_energy, graph.int(id, ORGANISM, "_infectedReproduceEnergy").unwrap(), "_infectedReproduceEnergy"); compared += 1;
    assert_eq!(actor.effects.is_consumer, graph.boolean(id, ORGANISM, "_isaconsumer").unwrap(), "_isaconsumer"); compared += 1;
    assert_eq!(actor.effects.is_fungus, graph.boolean(id, ORGANISM, "_isafungus").unwrap(), "_isafungus"); compared += 1;
    assert_eq!(actor.effects.is_killer, graph.boolean(id, ORGANISM, "_isakiller").unwrap(), "_isakiller"); compared += 1;
    assert_eq!(actor.effects.is_plant, graph.boolean(id, ORGANISM, "_isaplant").unwrap(), "_isaplant"); compared += 1;
    assert_eq!(actor.effects.is_auburn, graph.boolean(id, ORGANISM, "_isauburn").unwrap(), "_isauburn"); compared += 1;
    assert_eq!(actor.effects.is_blond, graph.boolean(id, ORGANISM, "_isblond").unwrap(), "_isblond"); compared += 1;
    assert_eq!(actor.effects.is_coral, graph.boolean(id, ORGANISM, "_iscoral").unwrap(), "_iscoral"); compared += 1;
    assert_eq!(actor.effects.is_enhanced, graph.boolean(id, ORGANISM, "_isenhanced").unwrap(), "_isenhanced"); compared += 1;
    assert_eq!(actor.effects.is_frozen, graph.boolean(id, ORGANISM, "_isfrozen").unwrap(), "_isfrozen"); compared += 1;
    assert_eq!(actor.effects.is_gray, graph.boolean(id, ORGANISM, "_isgray").unwrap(), "_isgray"); compared += 1;
    assert_eq!(actor.effects.is_infectious, graph.boolean(id, ORGANISM, "_isinfectious").unwrap(), "_isinfectious"); compared += 1;
    assert_eq!(actor.effects.is_injured, graph.boolean(id, ORGANISM, "_isinjured").unwrap(), "_isinjured"); compared += 1;
    assert_eq!(actor.effects.is_lime, graph.boolean(id, ORGANISM, "_islime").unwrap(), "_islime"); compared += 1;
    assert_eq!(actor.effects.is_only_c4, graph.boolean(id, ORGANISM, "_isonlyc4").unwrap(), "_isonlyc4"); compared += 1;
    assert_eq!(actor.effects.is_reproductive, graph.boolean(id, ORGANISM, "_isreproductive").unwrap(), "_isreproductive"); compared += 1;
    assert_eq!(actor.effects.is_silver, graph.boolean(id, ORGANISM, "_issilver").unwrap(), "_issilver"); compared += 1;
    assert_eq!(actor.effects.is_spike, graph.boolean(id, ORGANISM, "_isspike").unwrap(), "_isspike"); compared += 1;
    assert_eq!(actor.effects.jade_factor, graph.int(id, ORGANISM, "_jadefactor").unwrap(), "_jadefactor"); compared += 1;
    assert_eq!(actor.state.last_angle.to_bits(), graph.double_bits(id, ORGANISM, "_lastTheta").unwrap(), "_lastTheta"); compared += 1;
    assert_eq!(actor.effects.lavender, graph.int(id, ORGANISM, "_lavender").unwrap(), "_lavender"); compared += 1;
    assert_eq!(actor.effects.leaf_photo.to_bits(), graph.double_bits(id, ORGANISM, "_leafphoto").unwrap(), "_leafphoto"); compared += 1;
    assert_eq!(actor.effects.friend_length, graph.int(id, ORGANISM, "_lengthfriend").unwrap(), "_lengthfriend"); compared += 1;
    assert_eq!(actor.effects.lower_symmetric, graph.int(id, ORGANISM, "_lowersymmetric").unwrap(), "_lowersymmetric"); compared += 1;
    assert_eq!(actor.effects.maintenance.to_bits(), graph.double_bits(id, ORGANISM, "_maintenance").unwrap(), "_maintenance"); compared += 1;
    assert_eq!(actor.body.mass.to_bits(), graph.double_bits(id, ORGANISM, "_mass").unwrap(), "_mass"); compared += 1;
    assert_eq!(actor.state.reproduction.maximum_age, graph.int(id, ORGANISM, "_max_age").unwrap(), "_max_age"); compared += 1;
    assert_eq!(actor.effects.methanotrophy.to_bits(), graph.double_bits(id, ORGANISM, "_methanotrophy").unwrap(), "_methanotrophy"); compared += 1;
    assert_eq!(actor.state.children, graph.int(id, ORGANISM, "_nChildren").unwrap(), "_nChildren"); compared += 1;
    assert_eq!(actor.state.total_children, graph.int(id, ORGANISM, "_nTotalChildren").unwrap(), "_nTotalChildren"); compared += 1;
    assert_eq!(actor.state.total_infected, graph.int(id, ORGANISM, "_nTotalInfected").unwrap(), "_nTotalInfected"); compared += 1;
    assert_eq!(actor.state.total_kills, graph.int(id, ORGANISM, "_nTotalKills").unwrap(), "_nTotalKills"); compared += 1;
    assert_eq!(actor.state.virus_children, graph.int(id, ORGANISM, "_nVirusChildren").unwrap(), "_nVirusChildren"); compared += 1;
    assert_eq!(actor.state.parent_id, graph.int(id, ORGANISM, "_parentID").unwrap(), "_parentID"); compared += 1;
    assert_eq!(actor.effects.photosynthesis.to_bits(), graph.double_bits(id, ORGANISM, "_photosynthesis").unwrap(), "_photosynthesis"); compared += 1;
    assert_eq!(actor.effects.plague_version, graph.int(id, ORGANISM, "_plagueversion").unwrap(), "_plagueversion"); compared += 1;
    assert_eq!(actor.state.reproduction.energy, graph.int(id, ORGANISM, "_reproduceEnergy").unwrap(), "_reproduceEnergy"); compared += 1;
    assert_eq!(actor.effects.reproduce_late, graph.int(id, ORGANISM, "_reproducelate").unwrap(), "_reproducelate"); compared += 1;
    assert_eq!(actor.effects.segment_pre_touch_effects, graph.int(id, ORGANISM, "_segmentpretoucheffects").unwrap(), "_segmentpretoucheffects"); compared += 1;
    assert_eq!(actor.body.segment_count, graph.int(id, ORGANISM, "_segments").unwrap(), "_segments"); compared += 1;
    assert_eq!(actor.effects.sibling_battle, graph.boolean(id, ORGANISM, "_siblingbattle").unwrap(), "_siblingbattle"); compared += 1;
    assert_eq!(actor.effects.sky_version, graph.int(id, ORGANISM, "_skyversion").unwrap(), "_skyversion"); compared += 1;
    assert_eq!(actor.effects.spin, graph.int(id, ORGANISM, "_spin").unwrap(), "_spin"); compared += 1;
    assert_eq!(actor.effects.spore_time, graph.int(id, ORGANISM, "_sporetime").unwrap(), "_sporetime"); compared += 1;
    assert_eq!(actor.effects.spore_version, graph.int(id, ORGANISM, "_sporeversion").unwrap(), "_sporeversion"); compared += 1;
    assert_eq!(actor.effects.summer_inactivity, graph.int(id, ORGANISM, "_summerinactivity").unwrap(), "_summerinactivity"); compared += 1;
    assert_eq!(actor.state.angle.to_bits(), graph.double_bits(id, ORGANISM, "_theta").unwrap(), "_theta"); compared += 1;
    assert_eq!(actor.effects.friend_theta, graph.int(id, ORGANISM, "_thetafriend").unwrap(), "_thetafriend"); compared += 1;
    assert_eq!(actor.state.reproduction.countdown, graph.int(id, ORGANISM, "_timeToReproduce").unwrap(), "_timeToReproduce"); compared += 1;
    assert_eq!(actor.state.reproduction.maximum_countdown, graph.int(id, ORGANISM, "_timeToReproduceMax").unwrap(), "_timeToReproduceMax"); compared += 1;
    assert_eq!(actor.effects.transfers_energy, graph.boolean(id, ORGANISM, "_transfersenergy").unwrap(), "_transfersenergy"); compared += 1;
    assert_eq!(actor.body.update_effects, graph.int(id, ORGANISM, "_updateEffects").unwrap(), "_updateEffects"); compared += 1;
    assert_eq!(actor.effects.use_extra_effects, graph.boolean(id, ORGANISM, "_useextraeffects").unwrap(), "_useextraeffects"); compared += 1;
    assert_eq!(actor.effects.use_frame_movement, graph.boolean(id, ORGANISM, "_useframemovement").unwrap(), "_useframemovement"); compared += 1;
    assert_eq!(actor.effects.use_friend_effects, graph.int(id, ORGANISM, "_usefriendeffects").unwrap(), "_usefriendeffects"); compared += 1;
    assert_eq!(actor.effects.use_pre_touch_effects, graph.boolean(id, ORGANISM, "_usepretoucheffects").unwrap(), "_usepretoucheffects"); compared += 1;
    assert_eq!(actor.effects.yellow_count, graph.int(id, ORGANISM, "_yellowCounter").unwrap(), "_yellowCounter"); compared += 1;
    assert_eq!(actor.state.active, graph.boolean(id, ORGANISM, "active").unwrap(), "active"); compared += 1;
    assert_eq!(actor.state.alive, graph.boolean(id, ORGANISM, "alive").unwrap(), "alive"); compared += 1;
    assert_eq!(actor.state.angular_velocity.to_bits(), graph.double_bits(id, ORGANISM, "dtheta").unwrap(), "dtheta"); compared += 1;
    assert_eq!(actor.state.velocity[0].to_bits(), graph.double_bits(id, ORGANISM, "dx").unwrap(), "dx"); compared += 1;
    assert_eq!(actor.state.velocity[1].to_bits(), graph.double_bits(id, ORGANISM, "dy").unwrap(), "dy"); compared += 1;
    assert_eq!(actor.state.has_grown, graph.int(id, ORGANISM, "hasGrown").unwrap(), "hasGrown"); compared += 1;
    assert_eq!(actor.state.has_moved, graph.boolean(id, ORGANISM, "hasMoved").unwrap(), "hasMoved"); compared += 1;
    let bounds = actor.state.bounds.get();
    for (name, actual) in [("x", bounds.x), ("y", bounds.y), ("width", bounds.width), ("height", bounds.height)] {
        assert_eq!(actual, graph.int(id, RECTANGLE, name).unwrap()); compared += 1;
    }
    assert_eq!(actor.state.bounds.class_name(), ORGANISM);
    identities.bind(0, Value::Handle(id), Some(actor.state.bounds.identity()));
    for field in crate::organism_arrays::INT_FIELDS {
        let source = graph.reference(id, ORGANISM, field.name()).unwrap();
        let actual = actor.body.arrays.int(field).as_ref();
        identities.bind(1, source, actual.map(|value| value.identity()));
        if let (Value::Handle(node), Some(actual)) = (source, actual) {
            let Node::Array(array) = graph.node(node).unwrap() else { panic!("Array") };
            let ArrayData::Ints(expected) = &array.data else { panic!("Int array") };
            assert_eq!(actual.values(), *expected);
        }
        compared += 1;
    }
    for field in crate::organism_arrays::DOUBLE_FIELDS {
        let source = graph.reference(id, ORGANISM, field.name()).unwrap();
        let actual = actor.body.arrays.double(field).as_ref();
        identities.bind(2, source, actual.map(|value| value.identity()));
        if let (Value::Handle(node), Some(actual)) = (source, actual) {
            let Node::Array(array) = graph.node(node).unwrap() else { panic!("Array") };
            let ArrayData::Doubles(expected) = &array.data else { panic!("Double array") };
            assert_eq!(actual.bits(), *expected);
        }
        compared += 1;
    }
    for (name, actual) in [("_geneticCode", &actor.genome), ("_infectedGeneticCode", &actor.infected_genome), ("_savedGeneticCode", &actor.saved_genome)] {
        identities.bind(3, graph.reference(id, ORGANISM, name).unwrap(), actual.as_ref().map(|value| Rc::as_ptr(value) as usize));
        compared += 1;
    }
    for (name, actual) in [("_sizeRect", actor.body.size_rect.as_ref()), ("lastFrame", actor.state.last_frame.as_ref())] {
        let source = graph.reference(id, ORGANISM, name).unwrap();
        identities.bind(0, source, actual.map(RectangleRef::identity));
        if let (Value::Handle(node), Some(actual)) = (source, actual) {
            let actual = actual.get();
            assert_eq!([actual.x, actual.y, actual.width, actual.height], [
                graph.int(node, RECTANGLE, "x").unwrap(), graph.int(node, RECTANGLE, "y").unwrap(),
                graph.int(node, RECTANGLE, "width").unwrap(), graph.int(node, RECTANGLE, "height").unwrap(),
            ]);
        }
        compared += 1;
    }
    identities.bind(4, graph.reference(id, ORGANISM, "_color").unwrap(), actor.state.temporary_color.as_ref().map(JavaColor::identity)); compared += 1;
    identities.bind(5, graph.reference(id, ORGANISM, "_world").unwrap(), actor.state.world.identity()); compared += 1;
    let text = graph.reference(id, ORGANISM, "_nPosition").unwrap();
    identities.bind(6, text, actor.state.network_position.as_ref().map(|value| value.units().as_ptr() as usize)); compared += 1;
    if let (Value::Handle(node), Some(actual)) = (text, &actor.state.network_position) {
        assert_eq!(actual.units(), graph.string_units(node).unwrap());
    }
    let colors = graph.reference(id, ORGANISM, "_segColor").unwrap();
    identities.bind(7, colors, actor.body.phenotype_colors.as_ref().map(|value| value.identity())); compared += 1;
    if let (Value::Handle(node), Some(actual)) = (colors, &actor.body.phenotype_colors) {
        let Node::Array(array) = graph.node(node).unwrap() else { panic!("Array") };
        let ArrayData::References(expected) = &array.data else { panic!("Color array") };
        let actual = actual.slots();
        assert_eq!(actual.len(), expected.len());
        for (source, value) in expected.iter().zip(actual.iter()) {
            identities.bind(4, *source, value.as_ref().map(JavaColor::identity));
            if let (Value::Handle(node), Some(color)) = (*source, value) {
                assert_eq!(color.argb(), graph.int(node, "java.awt.Color", "value").unwrap() as u32);
            }
        }
    }
    assert_eq!(compared, 156);
    assert!(actor.creation_complete);
    assert_eq!(actor.body.eyes.consumer, actor.effects.is_consumer);
    assert_eq!(actor.body.eyes.fungus, actor.effects.is_fungus);
    assert_eq!(actor.body.eyes.infectious, actor.effects.is_infectious);
}

#[test]
fn source_world_batches_preserve_all_actor_fields_and_retained_tree_identity() {
    assert_eq!(jts_inputs::STREAMS.len(), 23);
    let mut retained_tree_only = false;
    for (name, bytes) in jts_inputs::STREAMS {
        let input = document(bytes);
        let limits = limits(&input);
        let roots = {
            let graph = LegacyGraph::new(&input, limits).unwrap();
            let decoded = decode_world_spatial(&graph).unwrap();
            retained_tree_only |= decoded.actors.nodes.len() > decoded.members.len();
            decoded.actors.nodes
        };
        let bindings = worlds(&input);
        let before: Vec<_> = bindings.iter().map(|(_, owner)| owner.alias().clone()).collect();
        let random: Vec<_> = bindings.iter().map(|(_, owner)| owner.session().random_snapshot()).collect();
        let staged = stage_actor_batch(&input, limits, &roots, &bindings, limits.allocated_bytes).unwrap();
        let graph = LegacyGraph::new(&input, limits).unwrap();
        let mut identities = Identities::default();
        assert_eq!(staged.actors.len(), roots.len(), "{name}");
        for (id, actor) in &staged.actors { assert_actor(&graph, *id, &actor.borrow(), &mut identities); }
        for (index, (_, owner)) in bindings.iter().enumerate() {
            assert_eq!(owner.alias(), before[index], "No World events during import");
            assert_eq!(owner.session().random_snapshot(), random[index]);
        }
        drop(graph);
        drop(input);
        drop(bindings);
        let StagedActorBatch { actors, context, worlds, .. } = staged;
        let actors: Vec<_> = actors.into_iter().map(|(_, actor)| actor).collect();
        assert_eq!(context.actors().len(), actors.len());
        for actor in actors {
            assert!(actor.borrow().state.world.resolve().unwrap().handle().same_owner(&worlds[0]));
        }
    }
    assert!(retained_tree_only);
}

#[test]
fn raw_signed_null_and_ieee_state_is_assigned_without_model_reconstruction() {
    let mut input = document(include_bytes!("../tests/fixtures/organism-array/array_signed_count-initial.ser"));
    let roots = nodes(&input, ORGANISM);
    let victim = roots.iter().copied().find(|id| LegacyGraph::new(&input, limits(&input)).unwrap().int(*id, ORGANISM, "_segments").unwrap() == -1).unwrap();
    assert_eq!(reference(&input, victim, ORGANISM, "_segColor"), Value::Null);
    set(&mut input, victim, ORGANISM, "_ID", FieldValue::Int(i32::MIN));
    set(&mut input, victim, ORGANISM, "_growthRatio", FieldValue::Int(i32::MIN));
    set(&mut input, victim, ORGANISM, "_geneticCode", FieldValue::Reference(Value::Null));
    let raw = [0x8000000000000000, 0x7ff0000000000001, 0xfff8123456789abc, 0x7ff0000000000000];
    let doubles: Vec<_> = crate::object_stream::class_schema(crate::object_stream::class_by_name(ORGANISM).unwrap()).fields
        .iter().filter(|field| field.kind == b'D').map(|field| field.name).collect();
    assert_eq!(doubles.len(), 16);
    for (index, name) in doubles.iter().enumerate() { set(&mut input, victim, ORGANISM, name, FieldValue::DoubleBits(raw[index % raw.len()])); }
    let bindings = worlds(&input);
    let staged = stage_actor_batch(&input, limits(&input), &roots, &bindings, Limits::default().allocated_bytes).unwrap();
    let graph = LegacyGraph::new(&input, limits(&input)).unwrap();
    let mut identities = Identities::default();
    for (id, actor) in &staged.actors { assert_actor(&graph, *id, &actor.borrow(), &mut identities); }
    let victim = staged.actors.iter().find(|(id, _)| *id == victim).unwrap().1.borrow();
    assert_eq!(victim.id, i64::from(i32::MIN));
    assert_eq!(victim.body.growth_ratio, i32::MIN);
    assert_eq!(victim.body.segment_count, -1);
    assert!(victim.genome.is_none() && victim.body.phenotype_colors.is_none());
    assert!(victim.body.arrays.int(crate::IntField::StartX).is_some());
}

#[test]
fn rectangle_and_world_closure_outlives_wire_maps_without_strong_actor_cycles() {
    let mut input = document(include_bytes!("../tests/fixtures/rectangle-ownership/rect_peer-step_0.ser"));
    let roots = {
        let graph = LegacyGraph::new(&input, limits(&input)).unwrap();
        let list = graph.required_reference(graph.world_root().unwrap(), WORLD, "_organisms").unwrap();
        graph.synchronized_list(list).unwrap().1.into_iter().map(handle).collect::<Vec<_>>()
    };
    assert_eq!(roots.len(), 2);
    let all = nodes(&input, ORGANISM);
    assert_eq!(all.len(), 3);
    let third = *all.iter().find(|id| !roots.contains(id)).unwrap();
    let original = nodes(&input, WORLD)[0];
    let foreign = NodeId(input.arena.nodes.len() as u32);
    let other = input.arena.nodes[original.0 as usize].clone();
    input.arena.nodes.push(other);
    set(&mut input, roots[0], ORGANISM, "_world", FieldValue::Reference(Value::Handle(foreign)));
    set(&mut input, roots[1], ORGANISM, "_world", FieldValue::Reference(Value::Null));
    set(&mut input, roots[0], ORGANISM, "_sizeRect", FieldValue::Reference(Value::Handle(third)));
    set(&mut input, third, ORGANISM, "lastFrame", FieldValue::Reference(Value::Handle(roots[0])));
    let bindings = worlds(&input);
    let staged = stage_actor_batch(&input, limits(&input), &roots, &bindings, Limits::default().allocated_bytes).unwrap();
    let first = Rc::clone(&staged.actors.iter().find(|(id, _)| *id == roots[0]).unwrap().1);
    let second = Rc::clone(&staged.actors.iter().find(|(id, _)| *id == roots[1]).unwrap().1);
    let third_actor = Rc::clone(&staged.actors.iter().find(|(id, _)| *id == third).unwrap().1);
    assert_eq!(second.borrow().id, 77);
    assert_eq!(third_actor.borrow().id, 77);
    assert!(!Rc::ptr_eq(&first, &second));
    let expected = bindings.iter().find(|(id, _)| *id == foreign).unwrap().1.alias().association().identity();
    drop(input);
    drop(bindings);
    let StagedActorBatch { actors, context, worlds, .. } = staged;
    drop(actors);
    assert_eq!(context.actors().len(), 3);
    assert!(first.borrow().state.world.resolve().is_ok());
    assert_eq!(first.borrow().state.world.identity(), expected);
    assert!(matches!(second.borrow().state.world.resolve(), Err(WorldError::MissingWorld)));
    let cell = first.borrow().body.size_rect.clone().unwrap();
    assert!(Rc::ptr_eq(&cell.actor_target().unwrap(), &third_actor));
    cell.update(|value| value.x = -948);
    assert_eq!(third_actor.borrow().state.bounds.get().x, -948);
    assert!(Rc::ptr_eq(&third_actor.borrow().state.last_frame.as_ref().unwrap().actor_target().unwrap(), &first));
    let weak = Rc::downgrade(&third_actor);
    drop(third_actor);
    assert!(weak.upgrade().is_some());
    drop(worlds);
    assert!(matches!(first.borrow().state.world.resolve(), Err(WorldError::ExpiredWorld)));
    drop(context);
    assert!(weak.upgrade().is_none());
}

#[test]
fn shared_genetics_color_and_text_aliases_remain_live_after_staging() {
    let mut input = document(include_bytes!("../tests/fixtures/gene_array/shared_slot_holder_replacement-authored.ser"));
    let roots = nodes(&input, ORGANISM);
    let victim = roots[0];
    let primary = handle(reference(&input, victim, ORGANISM, "_geneticCode"));
    let text = NodeId(input.arena.nodes.len() as u32);
    input.arena.nodes.push(Node::String(JavaString { units: vec![0, 0xd800, 0xffff], long: false }));
    set(&mut input, primary, "biogenesis.GeneticCode", "_cladeID", FieldValue::Reference(Value::Handle(text)));
    set(&mut input, victim, ORGANISM, "_nPosition", FieldValue::Reference(Value::Handle(text)));
    let gene_array = handle(reference(&input, primary, "biogenesis.GeneticCode", "_genes"));
    let Node::Array(array) = &input.arena.nodes[gene_array.0 as usize] else { panic!("Array") };
    let ArrayData::References(genes) = &array.data else { panic!("Gene array") };
    let gene = genes.iter().copied().find(|value| matches!(value, Value::Handle(_))).map(handle).unwrap();
    let color = reference(&input, gene, "biogenesis.Gene", "_color");
    set(&mut input, victim, ORGANISM, "_color", FieldValue::Reference(color));
    let colors = handle(reference(&input, victim, ORGANISM, "_segColor"));
    let Node::Array(array) = &mut input.arena.nodes[colors.0 as usize] else { panic!("Array") };
    let ArrayData::References(values) = &mut array.data else { panic!("Colors") };
    values[0] = color;
    let bindings = worlds(&input);
    let staged = stage_actor_batch(&input, limits(&input), &roots, &bindings, Limits::default().allocated_bytes).unwrap();
    let actor = Rc::clone(&staged.actors.iter().find(|(id, _)| *id == victim).unwrap().1);
    drop(input);
    let actor = actor.borrow();
    let primary = Rc::clone(actor.genome.as_ref().unwrap());
    let infected = Rc::clone(actor.infected_genome.as_ref().unwrap());
    let saved = Rc::clone(actor.saved_genome.as_ref().unwrap());
    let array = primary.borrow().gene_array().unwrap().clone();
    assert!(array.same_instance(infected.borrow().gene_array().unwrap()));
    assert!(!array.same_instance(saved.borrow().gene_array().unwrap()));
    assert!(primary.borrow().clade_id().unwrap().same_instance(actor.state.network_position.as_ref().unwrap()));
    assert_eq!(actor.state.network_position.as_ref().unwrap().units(), [0, 0xd800, 0xffff]);
    let temporary = actor.state.temporary_color.clone().unwrap();
    assert!(temporary.same_instance(primary.borrow().gene_ref(0).unwrap().borrow().color_handle().unwrap()));
    assert!(temporary.same_instance(actor.body.phenotype_colors.as_ref().unwrap().slots()[0].as_ref().unwrap()));
    let replacement = Gene::new_with_color(12.0, 0.0, ColorKind::Red.color_ref(), 0, [0; 36]).shared();
    array.set(0, Some(Rc::clone(&replacement))).unwrap();
    assert!(Rc::ptr_eq(&infected.borrow().gene_ref(0).unwrap(), &replacement));
    assert!(!Rc::ptr_eq(&saved.borrow().gene_ref(0).unwrap(), &replacement));
}

#[test]
fn malformed_bindings_and_cumulative_resource_failures_leave_existing_roots_unchanged() {
    let mut input = document(include_bytes!("../tests/fixtures/organism-array/array_signed_count-initial.ser"));
    let roots = nodes(&input, ORGANISM);
    let actor = roots[0];
    let bindings = worlds(&input);
    let before = bindings[0].1.alias().clone();
    let session = bindings[0].1.session();
    let random = session.random_snapshot();
    assert!(matches!(stage_actor_batch(&input, limits(&input), &roots, &[], Limits::default().allocated_bytes), Err(GraphError { kind: GraphErrorKind::Reference, .. })));
    let original_record = input.records[0].clone();
    input.records[0] = crate::object_stream::Record::Value(Value::Handle(actor));
    assert!(matches!(stage_actor_batch(&input, limits(&input), &roots, &bindings, Limits::default().allocated_bytes), Err(GraphError { kind: GraphErrorKind::Class, .. })));
    input.records[0] = original_record;
    let valid = stage_actor_batch(&input, limits(&input), &roots, &bindings, Limits::default().allocated_bytes).unwrap();
    assert!(matches!(stage_actor_batch(&input, limits(&input), &roots, &bindings, valid.allocated_bytes - 1), Err(GraphError { kind: GraphErrorKind::Budget, .. })));
    let graph = LegacyGraph::new(&input, limits(&input)).unwrap();
    let other_graph = LegacyGraph::new(&input, limits(&input)).unwrap();
    let mut genetics = GeneticsStager::new(&other_graph, Limits::default().allocated_bytes).unwrap();
    let mut primitive = PrimitiveStager::new(&graph, Limits::default().allocated_bytes).unwrap();
    assert!(matches!(stage_actor_batch_in(&graph, &roots, &bindings, &mut genetics, &mut primitive), Err(GraphError { kind: GraphErrorKind::Reference, .. })));
    drop(primitive); drop(genetics); drop(other_graph); drop(graph);
    let original_id = LegacyGraph::new(&input, limits(&input)).unwrap().int(actor, ORGANISM, "_ID").unwrap();
    set(&mut input, actor, ORGANISM, "_ID", FieldValue::DoubleBits(0));
    assert!(matches!(stage_actor_batch(&input, limits(&input), &roots, &bindings, Limits::default().allocated_bytes), Err(GraphError { field: Some("_ID"), kind: GraphErrorKind::Type, .. })));
    set(&mut input, actor, ORGANISM, "_ID", FieldValue::Int(original_id));
    let foreign = NodeId(input.arena.nodes.len() as u32);
    let foreign_source = input.arena.nodes[bindings[0].0.0 as usize].clone();
    input.arena.nodes.push(foreign_source);
    set(&mut input, actor, ORGANISM, "_world", FieldValue::Reference(Value::Handle(foreign)));
    assert!(matches!(stage_actor_batch(&input, limits(&input), &roots, &bindings, Limits::default().allocated_bytes), Err(GraphError { field: Some("_world"), kind: GraphErrorKind::Reference, .. })));
    let duplicate = [(bindings[0].0, bindings[0].1.clone()), (foreign, bindings[0].1.clone())];
    assert!(matches!(stage_actor_batch(&input, limits(&input), &roots, &duplicate, Limits::default().allocated_bytes), Err(GraphError { kind: GraphErrorKind::Reference, .. })));
    set(&mut input, actor, ORGANISM, "_world", FieldValue::Reference(Value::Handle(bindings[0].0)));
    // Each family fits one element, while their combined two elements exceed the same document budget
    let int_array = input.arena.nodes.iter().enumerate().find_map(|(index, value)| match value { Node::Array(array) if matches!(&array.data, ArrayData::Ints(_)) => Some(NodeId(index as u32)), _ => None }).unwrap();
    let color_array = input.arena.nodes.iter().enumerate().find_map(|(index, value)| {
        let Node::Array(array) = value else { return None };
        let Node::Descriptor(descriptor) = &input.arena.nodes[array.descriptor.0 as usize] else { return None };
        (class_schema(descriptor.class).name == "[Ljava.awt.Color;").then_some(NodeId(index as u32))
    }).unwrap();
    let color = nodes(&input, "java.awt.Color")[0];
    let Node::Array(array) = &mut input.arena.nodes[int_array.0 as usize] else { unreachable!() };
    array.data = ArrayData::Ints(vec![123]);
    let Node::Array(array) = &mut input.arena.nodes[color_array.0 as usize] else { unreachable!() };
    array.data = ArrayData::References(vec![Value::Handle(color)]);
    for field in crate::organism_arrays::INT_FIELDS { set(&mut input, actor, ORGANISM, field.name(), FieldValue::Reference(Value::Null)); }
    for field in crate::organism_arrays::DOUBLE_FIELDS { set(&mut input, actor, ORGANISM, field.name(), FieldValue::Reference(Value::Null)); }
    for field in ["_geneticCode", "_infectedGeneticCode", "_savedGeneticCode", "_color", "_sizeRect", "lastFrame"] { set(&mut input, actor, ORGANISM, field, FieldValue::Reference(Value::Null)); }
    set(&mut input, actor, ORGANISM, "x1", FieldValue::Reference(Value::Handle(int_array)));
    set(&mut input, actor, ORGANISM, "_segColor", FieldValue::Reference(Value::Handle(color_array)));
    let mut quota = limits(&input); quota.total_array_elements = 2;
    assert!(stage_actor_batch(&input, quota, &[actor], &bindings, quota.allocated_bytes).is_ok());
    quota.total_array_elements = 1;
    assert!(matches!(stage_actor_batch(&input, quota, &[actor], &bindings, quota.allocated_bytes), Err(GraphError { kind: GraphErrorKind::Budget, .. })));
    let Node::Object(object) = &input.arena.nodes[actor.0 as usize] else { unreachable!() };
    let descriptor = object.descriptor;
    let Node::Descriptor(value) = &mut input.arena.nodes[descriptor.0 as usize] else { unreachable!() };
    value.signatures.clear();
    assert!(matches!(stage_actor_batch(&input, limits(&input), &[actor], &bindings, Limits::default().allocated_bytes), Err(GraphError { kind: GraphErrorKind::Descriptor, .. })));
    assert_eq!(bindings[0].1.alias(), before);
    assert!(bindings[0].1.session().same_owner(&session));
    assert_eq!(session.random_snapshot(), random);
}
