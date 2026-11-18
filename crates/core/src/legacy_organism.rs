// SPDX-License-Identifier: GPL-2.0-or-later

//! Constructor-free actor assembly from bounded, already parsed source fields

use crate::legacy_genetics::GeneticsStager;
use crate::legacy_graph::{GraphError, GraphErrorKind, LegacyGraph};
use crate::legacy_organism_arrays::PrimitiveStager;
use crate::legacy_rectangles::stage_actor_rectangles_in;
use crate::object_stream::{Document, Limits, NodeId, Value};
use crate::organism::{Organism, OrganismRef, OrganismState};
use crate::rectangles::RectangleActorContext;
use crate::{Body, EffectsState, EyeState, IntRect, RectangleRef, ReproductionState, WorldAssociation, WorldHandle};
use std::{cell::RefCell, mem::size_of, rc::Rc};

const ORGANISM: &str = "biogenesis.Organism";
const RECTANGLE: &str = "java.awt.Rectangle";
const WORLD: &str = "biogenesis.World";

/// Temporary node bindings and strong leases stay outside the imported actor owners
pub(crate) struct StagedActorBatch {
    pub actors: Vec<(NodeId, OrganismRef)>,
    pub context: RectangleActorContext,
    pub worlds: Vec<WorldHandle>,
    pub allocated_bytes: usize,
}

fn failure(node: Option<NodeId>, field: Option<&'static str>, kind: GraphErrorKind) -> GraphError {
    GraphError { node, field, kind }
}
fn bytes(count: usize, width: usize) -> Result<usize, GraphError> {
    count.checked_mul(width).ok_or(failure(None, None, GraphErrorKind::Budget))
}
fn reserve<T>(graph: &LegacyGraph<'_>, values: &mut Vec<T>, additional: usize) -> Result<(), GraphError> {
    let needed = values.len().checked_add(additional).ok_or(failure(None, None, GraphErrorKind::Budget))?;
    if needed > values.capacity() {
        let before = values.capacity();
        graph.allocate(bytes(needed - before, size_of::<T>())?)?;
        values.try_reserve_exact(additional).map_err(|_| failure(None, None, GraphErrorKind::Allocation))?;
        graph.allocate(bytes(values.capacity() - needed, size_of::<T>())?)?;
    }
    Ok(())
}
fn enqueue(graph: &LegacyGraph<'_>, id: NodeId, nodes: &mut Vec<NodeId>, seen: &mut [bool]) -> Result<(), GraphError> {
    graph.node(id)?;
    if !seen[id.0 as usize] {
        reserve(graph, nodes, 1)?;
        seen[id.0 as usize] = true;
        nodes.push(id);
    }
    Ok(())
}

/// Roots come from membership and stored-tree tokens; Rectangle references extend their closure
/// No World fields are replaced, and no event or simulation consumer runs at this boundary
pub(crate) fn stage_actor_batch(
    document: &Document,
    mut limits: Limits,
    roots: &[NodeId],
    world_bindings: &[(NodeId, WorldHandle)],
    maximum: usize,
) -> Result<StagedActorBatch, GraphError> {
    limits.allocated_bytes = limits.allocated_bytes.min(maximum);
    let graph = LegacyGraph::new(document, limits)?;
    let mut genetics = GeneticsStager::new(&graph, limits.allocated_bytes)?;
    let mut primitive = PrimitiveStager::new(&graph, limits.allocated_bytes)?;
    stage_actor_batch_in(&graph, roots, world_bindings, &mut genetics, &mut primitive)
}

/// Share identity tables with statistics and other typed holders from this same document
pub(crate) fn stage_actor_batch_in<'graph, 'document>(
    graph: &'graph LegacyGraph<'document>,
    roots: &[NodeId],
    world_bindings: &[(NodeId, WorldHandle)],
    genetics: &mut GeneticsStager<'graph, 'document>,
    primitive: &mut PrimitiveStager<'graph, 'document>,
) -> Result<StagedActorBatch, GraphError> {
    let limits = graph.limits();
    let root = graph.world_root()?;
    if !genetics.uses_graph(graph) || !primitive.uses_graph(graph) {
        return Err(failure(None, None, GraphErrorKind::Reference));
    }
    if roots.len() > limits.handles || world_bindings.len() > limits.handles {
        return Err(failure(None, None, GraphErrorKind::Budget));
    }
    let mut worlds = Vec::new();
    reserve(&graph, &mut worlds, world_bindings.len())?;
    for (index, (node, world)) in world_bindings.iter().enumerate() {
        genetics.object(*node, WORLD)?;
        graph.charge(index + 1)?;
        if world_bindings[..index].iter().any(|(previous, owner)| previous == node || owner.same_owner(world)) {
            return Err(failure(Some(*node), None, GraphErrorKind::Reference));
        }
        worlds.push(world.clone());
    }
    if !world_bindings.iter().any(|(node, _)| *node == root) {
        return Err(failure(Some(root), None, GraphErrorKind::Reference));
    }
    let mut seen = Vec::new();
    reserve(&graph, &mut seen, graph.arena().nodes.len())?;
    graph.charge(graph.arena().nodes.len())?;
    seen.resize(graph.arena().nodes.len(), false);
    let mut nodes = Vec::new();
    for id in roots { enqueue(&graph, *id, &mut nodes, &mut seen)?; }
    let mut position = 0;
    while position < nodes.len() {
        let id = nodes[position];
        genetics.object(id, ORGANISM)?;
        for field in ["_sizeRect", "lastFrame"] {
            if let Value::Handle(target) = graph.reference(id, ORGANISM, field)? {
                match graph.class_name(target)? {
                    ORGANISM => enqueue(&graph, target, &mut nodes, &mut seen)?,
                    RECTANGLE => genetics.object(target, RECTANGLE)?,
                    _ => return Err(failure(Some(id), Some(field), GraphErrorKind::Class)),
                }
            }
        }
        position += 1;
    }
    let mut actors = Vec::new();
    reserve(&graph, &mut actors, nodes.len())?;
    for id in nodes {
        let association = match graph.reference(id, ORGANISM, "_world")? {
            Value::Null => WorldAssociation::default(),
            Value::Handle(target) => {
                graph.charge(world_bindings.len())?;
                let (_, owner) = world_bindings.iter().find(|(node, _)| *node == target)
                    .ok_or(failure(Some(id), Some("_world"), GraphErrorKind::Reference))?;
                owner.alias().association()
            }
        };
        graph.allocate(size_of::<RefCell<Organism>>() + 2 * size_of::<usize>() + RectangleRef::allocation_bytes())?;
        let actor = Rc::new(RefCell::new(parts(graph, id, association, genetics, primitive)?));
        actor.borrow().state.bounds.bind_actor(&actor).map_err(|_| failure(Some(id), None, GraphErrorKind::Reference))?;
        actors.push((id, actor));
    }
    let (rectangles, _) = stage_actor_rectangles_in(&graph, &actors, limits.allocated_bytes)?;
    for state in rectangles.actors {
        let mut actor = state.actor.borrow_mut();
        actor.body.size_rect = state.size_rect;
        actor.state.last_frame = state.last_frame;
    }
    Ok(StagedActorBatch { actors, context: rectangles.context, worlds, allocated_bytes: graph.allocated_bytes() })
}

fn parts(
    graph: &LegacyGraph<'_>, id: NodeId, association: WorldAssociation,
    genetics: &mut GeneticsStager<'_, '_>, primitive: &mut PrimitiveStager<'_, '_>,
) -> Result<Organism, GraphError> {
    let int = |field| graph.int(id, ORGANISM, field);
    let double = |field| graph.double_bits(id, ORGANISM, field).map(f64::from_bits);
    let boolean = |field| graph.boolean(id, ORGANISM, field);
    let primitive = primitive.organism(id)?;
    let bounds = RectangleRef::actor(IntRect {
        x: graph.int(id, RECTANGLE, "x")?, y: graph.int(id, RECTANGLE, "y")?,
        width: graph.int(id, RECTANGLE, "width")?, height: graph.int(id, RECTANGLE, "height")?,
    });
    let effects = EffectsState {
        all_frozen: boolean("_allfrozen")?,
        altruist: boolean("_altruist")?,
        antiviral: int("_antiviral")?,
        can_dodge: boolean("_candodge")?,
        can_move: boolean("_canmove")?,
        can_react: boolean("_canreact")?,
        clockwise: boolean("_clockwise")?,
        colony_photosynthesis: double("_colonyPhotosynthesis")?,
        create_lavender: int("_createlavender")?,
        crowded: boolean("_crowded")?,
        dodge: boolean("_dodge")?,
        fallow_version: int("_fallowversion")?,
        forest_photo: double("_forestphoto")?,
        generation_battle: boolean("_generationbattle")?,
        gold: int("_gold")?,
        has_dodged: boolean("_hasdodged")?,
        healing: int("_healing")?,
        indigo: int("_indigo")?,
        is_consumer: boolean("_isaconsumer")?,
        is_fungus: boolean("_isafungus")?,
        is_killer: boolean("_isakiller")?,
        is_plant: boolean("_isaplant")?,
        is_auburn: boolean("_isauburn")?,
        is_blond: boolean("_isblond")?,
        is_coral: boolean("_iscoral")?,
        is_enhanced: boolean("_isenhanced")?,
        is_frozen: boolean("_isfrozen")?,
        is_gray: boolean("_isgray")?,
        is_infectious: boolean("_isinfectious")?,
        is_injured: boolean("_isinjured")?,
        is_lime: boolean("_islime")?,
        is_only_c4: boolean("_isonlyc4")?,
        is_reproductive: boolean("_isreproductive")?,
        is_silver: boolean("_issilver")?,
        is_spike: boolean("_isspike")?,
        jade_factor: int("_jadefactor")?,
        lavender: int("_lavender")?,
        leaf_photo: double("_leafphoto")?,
        friend_length: int("_lengthfriend")?,
        lower_symmetric: int("_lowersymmetric")?,
        maintenance: double("_maintenance")?,
        methanotrophy: double("_methanotrophy")?,
        photosynthesis: double("_photosynthesis")?,
        plague_version: int("_plagueversion")?,
        reproduce_late: int("_reproducelate")?,
        segment_pre_touch_effects: int("_segmentpretoucheffects")?,
        sibling_battle: boolean("_siblingbattle")?,
        sky_version: int("_skyversion")?,
        spin: int("_spin")?,
        spore_time: int("_sporetime")?,
        spore_version: int("_sporeversion")?,
        summer_inactivity: int("_summerinactivity")?,
        friend_theta: int("_thetafriend")?,
        transfers_energy: boolean("_transfersenergy")?,
        use_extra_effects: boolean("_useextraeffects")?,
        use_frame_movement: boolean("_useframemovement")?,
        use_friend_effects: int("_usefriendeffects")?,
        use_pre_touch_effects: boolean("_usepretoucheffects")?,
        yellow_count: int("_yellowCounter")?,
    };
    let body = Body {
        segment_count: primitive.segment_count, arrays: primitive.arrays,
        phenotype_colors: genetics.color_array(graph.reference(id, ORGANISM, "_segColor")?)?,
        size_rect: None, mass: double("_mass")?, inertia: double("_I")?,
        eyes: EyeState {
            enabled: boolean("_haseyes")?, good_vision: boolean("_hasgoodvision")?,
            consumer: effects.is_consumer, fungus: effects.is_fungus, infectious: effects.is_infectious,
        },
        update_effects: int("_updateEffects")?, growth_ratio: int("_growthRatio")?,
    };
    let state = OrganismState {
        age: int("_age")?,
        energy: double("_energy")?,
        temporary_color_frames: int("_framesColor")?,
        last_angle: double("_lastTheta")?,
        children: int("_nChildren")?,
        total_children: int("_nTotalChildren")?,
        total_infected: int("_nTotalInfected")?,
        total_kills: int("_nTotalKills")?,
        virus_children: int("_nVirusChildren")?,
        parent_id: int("_parentID")?,
        angle: double("_theta")?,
        active: boolean("active")?,
        alive: boolean("alive")?,
        angular_velocity: double("dtheta")?,
        has_grown: int("hasGrown")?,
        has_moved: boolean("hasMoved")?,
        world: association,
        network_position: genetics.text(graph.reference(id, ORGANISM, "_nPosition")?)?,
        center: [double("_dCenterX")?, double("_dCenterY")?],
        integer_center: [int("_centerX")?, int("_centerY")?],
        velocity: [double("dx")?, double("dy")?],
        temporary_color: genetics.color(graph.reference(id, ORGANISM, "_color")?)?,
        last_frame: None,
        bounds,
        reproduction: ReproductionState {
            energy: int("_reproduceEnergy")?, maximum_age: int("_max_age")?,
            early_energy: int("_earlyReproduceEnergy")?, infected_energy: int("_infectedReproduceEnergy")?,
            countdown: int("_timeToReproduce")?, maximum_countdown: int("_timeToReproduceMax")?,
        },
    };
    Ok(Organism::from_persistent_parts(
        int("_ID")?,
        genetics.genome(graph.reference(id, ORGANISM, "_geneticCode")?)?,
        genetics.genome(graph.reference(id, ORGANISM, "_infectedGeneticCode")?)?,
        genetics.genome(graph.reference(id, ORGANISM, "_savedGeneticCode")?)?,
        body, state, effects,
    ))
}

#[cfg(test)]
#[path = "legacy_organism_tests.rs"]
pub(crate) mod tests;
