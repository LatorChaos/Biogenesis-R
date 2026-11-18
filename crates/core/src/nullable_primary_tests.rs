// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use crate::organism::same_genome;
use crate::{ColorKind, Gene, GenomeProperties, JavaText};

fn world() -> World {
    World::new(WorldConfig { initial_attempts: 0, ..WorldConfig::default() }).unwrap()
}

fn code(color: ColorKind) -> GenomeRef {
    Rc::new(RefCell::new(Genome::from_parts(
        &[Gene::new(8.0, 0.0, color.argb(), -1, [0; 36]).shared()],
        GenomeProperties { symmetry: 1, ..GenomeProperties::default() },
    )))
}

fn candidate(genome: Option<GenomeRef>) -> OrganismRef {
    Rc::new(RefCell::new(Organism::pasted_candidate(genome, OrganismState::default())))
}

#[test]
fn primary_holder_distinguishes_null_empty_and_reference_identity() {
    let mut null = Organism::pasted_candidate(None, OrganismState::default());
    assert!(null.snapshot().genotype.is_none());
    assert_eq!(null.create_for_paste(), Err(GeometryError::MissingGenome));
    assert!(!null.creation_complete);
    assert!(null.genome.is_none() && (null.body.segment_count == 0));
    let empty = Rc::new(RefCell::new(Genome::from_parts(&[], GenomeProperties::default())));
    let distinct = Rc::new(RefCell::new(empty.borrow().clone()));
    let some = Organism::pasted_candidate(Some(Rc::clone(&empty)), OrganismState::default());
    assert!(some.snapshot().genotype.unwrap().genes().unwrap().is_empty());
    assert!(same_genome(None, None));
    assert!(!same_genome(None, Some(&empty)));
    assert!(!same_genome(Some(&empty), None));
    assert!(same_genome(Some(&empty), Some(&empty)));
    assert!(!same_genome(Some(&empty), Some(&distinct)));
}

#[test]
fn world_clone_retains_null_edges_and_copies_shared_genomes_once() {
    let original = world();
    let shared = code(ColorKind::Green);
    shared.borrow_mut().set_clade_text(Some(JavaText::from_units(vec![0, 0xd800])));
    let distinct = Rc::new(RefCell::new(Genome::from_parts(
        &shared.borrow().genes().unwrap(), shared.borrow().properties(),
    )));
    let null = candidate(None);
    null.borrow_mut().infected_genome = Some(Rc::clone(&shared));
    null.borrow_mut().saved_genome = Some(Rc::clone(&distinct));
    let first = candidate(Some(Rc::clone(&shared)));
    let alias = candidate(Some(Rc::clone(&shared)));
    {
        let mut members = original.organisms.borrow_mut();
        members.clear();
        members.extend([Rc::clone(&null), first, alias]);
    }
    *original.frame_objects.borrow_mut() = vec![Rc::clone(&null), Rc::clone(&null)];
    let distinct_actor = Organism::pasted_candidate(Some(Rc::clone(&distinct)), Default::default());
    original.statistics_ref().borrow_mut().event_organism_born(&distinct_actor, &original.organisms.borrow()[1].borrow());
    original.statistics_ref().borrow_mut().event_organism_die(&original.organisms.borrow()[1].borrow(), None);
    let copied = original.clone();
    assert!(copied.organisms.borrow()[0].borrow().genome.is_none());
    assert!(Rc::ptr_eq(&copied.organisms.borrow()[0], &copied.frame_objects.borrow()[0]));
    assert!(Rc::ptr_eq(&copied.frame_objects.borrow()[0], &copied.frame_objects.borrow()[1]));
    assert!(!Rc::ptr_eq(&null, &copied.organisms.borrow()[0]));
    let cloned_shared = copied.organisms.borrow()[1].borrow().genome.clone().unwrap();
    let cloned_distinct = copied.organisms.borrow()[0].borrow().saved_genome.clone().unwrap();
    assert!(same_genome(Some(&cloned_shared), copied.organisms.borrow()[2].borrow().genome.as_ref()));
    assert!(same_genome(Some(&cloned_shared), copied.organisms.borrow()[0].borrow().infected_genome.as_ref()));
    assert!(same_genome(Some(&cloned_shared), copied.statistics_ref().borrow().get_last_dead_being()));
    assert!(same_genome(Some(&cloned_distinct), copied.statistics_ref().borrow().get_last_born_being()));
    assert!(!Rc::ptr_eq(&shared, &cloned_shared));
    assert!(!Rc::ptr_eq(&distinct, &cloned_distinct));
    assert!(!Rc::ptr_eq(&cloned_shared, &cloned_distinct));
    assert!(Rc::ptr_eq(&cloned_shared.borrow().genes().unwrap()[0], &cloned_distinct.borrow().genes().unwrap()[0]));
    assert!(shared.borrow().clade_id().unwrap().same_instance(cloned_shared.borrow().clade_id().unwrap()));
    cloned_shared.borrow().gene_ref(0).unwrap().borrow_mut().set_length(13.0);
    assert_eq!(cloned_distinct.borrow().gene_ref(0).unwrap().borrow().length(), 13.0);
    assert_eq!(shared.borrow().gene_ref(0).unwrap().borrow().length(), 8.0);
}

#[test]
fn null_record_and_infection_assignments_preserve_counters_and_references() {
    let world = world();
    let shared = code(ColorKind::Green);
    let mut recipient = Organism::pasted_candidate(Some(Rc::clone(&shared)), Default::default());
    recipient.state.world = world.association();
    recipient.infected_genome = Some(Rc::clone(&shared));
    recipient.saved_genome = Some(Rc::clone(&shared));
    let mut donor = Organism::pasted_candidate(None, Default::default());
    donor.state.total_infected = 4;
    donor.state.total_children = 6;
    donor.state.reproduction.energy = 73;
    let mut previous = Organism::pasted_candidate(Some(Rc::clone(&shared)), Default::default());
    previous.state.total_infected = 1;
    world.statistics_ref().borrow_mut().event_organism_infects(&recipient, Some(&previous));
    world.infect_from(&mut recipient, super::infection::Infector::Other(&mut donor), super::infection::InfectionKind::Direct).unwrap();
    assert_eq!(donor.state.total_infected, 5);
    assert!(recipient.infected_genome.is_none() && recipient.saved_genome.is_none());
    assert_eq!(recipient.state.reproduction.infected_energy, 73);
    let stats_owner = world.statistics_ref();
    let mut stats = stats_owner.borrow_mut();
    assert_eq!(stats.get_being_most_infections_number(), 5);
    assert!(stats.get_being_most_infections().is_none());
    assert!(same_genome(stats.get_last_infected_being(), Some(&shared)));
    stats.event_organism_born(&donor, &donor);
    assert!(stats.get_last_born_being().is_none() && stats.get_being_most_children().is_none());
    assert_eq!(stats.get_being_most_children_number(), 6);
    let null = Rc::new(RefCell::new(donor));
    stats.find_best_alive_beings(&[null]);
    assert_eq!(stats.get_alive_being_most_children_number(), 6);
    assert!(stats.get_alive_being_most_children().is_none());
    stats.event_time(0, 0.0, 0.0, 0.0).unwrap();
    assert_eq!(stats.get_average_births(), 1.0);
    assert_eq!(stats.get_average_infections(), 2.0);
}

#[test]
fn nullable_copy_and_inspection_distinguish_eligibility() {
    let world = world();
    let actor = candidate(None);
    world.organisms.borrow_mut().push(Rc::clone(&actor));
    let handle = OrganismHandle(Rc::clone(&actor), world.handle());
    assert_eq!(world.copy_genome(&handle), Some(None));
    assert_eq!(world.inspect_genome(&handle), Some(None));
    assert!(world.inspect_handle(&handle).unwrap().genotype.is_none());
    actor.borrow_mut().state.alive = false;
    assert_eq!(world.copy_genome(&handle), None);
    assert_eq!(world.inspect_genome(&handle), Some(None));
    world.organisms.borrow_mut().clear();
    assert_eq!(world.copy_genome(&handle), None);
    assert_eq!(world.inspect_genome(&handle), None);
}

#[test]
fn missing_primary_freeze_preserves_payment_and_prior_cache_changes() {
    let world = world();
    world.session.0.parameters.borrow_mut().lifecycle.sky_energy_consumption = 2.0;
    world.oxygen.set(100.0);
    let mut recipient = Organism::unplaced(code(ColorKind::Sky), Default::default()).unwrap();
    recipient.state.world = world.association();
    recipient.genome = None;
    recipient.effects.is_plant = false;
    let mut donor = Organism::unplaced(code(ColorKind::Green), Default::default()).unwrap();
    donor.state.world = world.association();
    donor.state.energy = 10.0;
    let result = world.frozen_from(&mut recipient, super::contact_state::ContactActor::Other(&mut donor));
    assert_eq!(result, Err(WorldError::Geometry(GeometryError::MissingGenome)));
    assert_eq!(donor.state.energy, 8.0);
    assert_eq!(recipient.body.color_argb(0).unwrap(), ColorKind::LightBlue.argb().unwrap());
    assert!(recipient.effects.is_injured);
    assert!(!recipient.effects.can_move);
    assert!(recipient.genome.is_none());
    let mut uncreated = Organism::pasted_candidate(None, Default::default());
    world.frozen_from(&mut uncreated, super::contact_state::ContactActor::Other(&mut donor)).unwrap();
    assert!(uncreated.genome.is_none());
}

#[test]
fn null_primary_keeps_cached_images_state_actions_and_statistics_failure() {
    let world = world();
    let genome = GenomeHandle::from_shared(code(ColorKind::Green));
    let handle = world.paste_genome_handle(&genome, [1000, 1000]).unwrap().unwrap();
    let before_segments = world.render_segments().unwrap();
    let before_image = world.organism_image(&handle, 1_000_000).unwrap();
    handle.0.borrow_mut().genome = None;
    assert_eq!(world.render_segments().unwrap(), before_segments);
    assert_eq!(world.organism_image(&handle, 1_000_000).unwrap(), before_image);
    assert!(world.tracking_center(&handle).is_some());
    let energy = handle.0.borrow().state.energy;
    world.carbon_dioxide.set(20.0);
    assert_eq!(world.act_on_organism(&handle, OrganismAction::Feed), Ok(true));
    assert_eq!(handle.0.borrow().state.energy, energy + 10.0);
    assert!(world.inspect_handle(&handle).unwrap().genotype.is_none());
    assert_eq!(world.refresh_statistics(&world.open_statistics()), Err(StatisticsError::MissingGenome { organism_id: handle.0.borrow().id }));
    assert_eq!(world.tick(), 0);

    let empty_world = World::new(WorldConfig { initial_attempts: 0, ..WorldConfig::default() }).unwrap();
    empty_world.population.set(9);
    let empty = candidate(None);
    {
        let mut actor = empty.borrow_mut();
        actor.state.alive = false;
        actor.state.active = false;
        actor.state.has_moved = false;
        actor.state.age = 77;
        actor.state.energy = 7.0;
        actor.effects.is_plant = true;
        actor.effects.is_gray = true;
        actor.effects.is_injured = true;
        actor.effects.can_dodge = true;
        actor.effects.friend_length = 99;
        actor.effects.spin = 31;
        actor.effects.healing = 12;
        actor.effects.antiviral = 13;
        actor.effects.create_lavender = 14;
        actor.effects.yellow_count = 15;
        actor.effects.indigo = 16;
        actor.effects.gold = 17;
        actor.effects.sibling_battle = true;
        actor.effects.altruist = true;
        actor.state.reproduction.early_energy = 18;
        actor.body.eyes.enabled = true;
        actor.body.update_effects = 0;
    }
    empty_world.organisms.borrow_mut().push(Rc::clone(&empty));
    let handle = OrganismHandle(Rc::clone(&empty), empty_world.handle());
    assert_eq!(empty_world.act_on_organism(&handle, OrganismAction::Revive), Err(WorldError::Geometry(GeometryError::MissingGenome)));
    let actor = empty.borrow();
    assert!(actor.state.alive && actor.state.active && actor.state.has_moved);
    assert_eq!(actor.state.age, 1);
    assert_eq!(actor.state.reproduction.maximum_age, empty_world.parameters().effects.max_age);
    assert_eq!(actor.state.energy, 7.0);
    assert!(!actor.effects.is_plant && !actor.effects.is_gray && !actor.effects.is_injured && !actor.effects.can_dodge);
    assert_eq!(actor.effects.friend_length, -1);
    assert_eq!((actor.effects.spin, actor.effects.healing, actor.effects.antiviral, actor.effects.create_lavender), (0, 0, 0, 0));
    assert_eq!((actor.effects.yellow_count, actor.effects.indigo, actor.effects.gold, actor.state.reproduction.early_energy), (0, 0, 0, 0));
    assert!(!actor.body.eyes.enabled);
    assert_eq!(actor.body.update_effects, 2);
    assert!(actor.effects.sibling_battle && actor.effects.altruist);
    assert!(actor.genome.is_none() && (actor.body.segment_count == 0));
    assert_eq!(empty_world.population(), 9);
}

#[test]
fn null_birth_rejection_keeps_source_side_effects_without_primary_preflight() {
    let mut parent = Organism::pasted_candidate(None, Default::default());
    parent.effects.spore_version = 1;
    parent.effects.spore_time = -7;
    parent.state.children = 4;
    let mut random = JavaRandom::new(71);
    let mut expected_random = random.clone();
    let result = crate::birth::prepare(&mut parent, Default::default(), true, &mut random, &crate::GeneticsParams::default()).unwrap();
    assert!(result.is_none());
    assert_eq!(parent.effects.spore_time, 7);
    assert_eq!(parent.state.children, 1);
    assert_eq!(parent.state.temporary_argb(), ColorKind::Flower.argb());
    assert_eq!(random.next_i64(), expected_random.next_i64());
    assert!(parent.genome.is_none());
}

fn contact_pair(actor: ColorKind, peer: ColorKind) -> (World, Organism, Organism) {
    let world = world();
    world.oxygen.set(1000.0);
    world.session.0.parameters.borrow_mut().lifecycle.pink_energy_consumption = 1.0;
    world.session.0.parameters.borrow_mut().lifecycle.cream_energy_consumption = 1.0;
    world.session.0.parameters.borrow_mut().lifecycle.lilac_energy_consumption = 1.0;
    let mut actor = Organism::unplaced(code(actor), Default::default()).unwrap();
    let mut peer = Organism::unplaced(code(peer), Default::default()).unwrap();
    actor.state.world = world.association();
    peer.state.world = world.association();
    actor.state.energy = 10.0;
    peer.state.energy = 20.0;
    (world, actor, peer)
}

#[test]
fn null_modifiers_do_not_preflight_pink_cream_or_lilac_branches() {
    type Touch = fn(&World, &mut Organism, &mut Organism, usize, usize) -> Result<(), WorldError>;
    let cases: [(ColorKind, ColorKind, Touch); 3] = [
        (ColorKind::Pink, ColorKind::White, World::touch_pink),
        (ColorKind::Cream, ColorKind::Green, World::touch_cream),
        (ColorKind::Lilac, ColorKind::Lilac, World::touch_lilac),
    ];
    for (actor_color, peer_color, touch) in cases {
        let (mut world, mut actor, mut peer) = contact_pair(actor_color, peer_color);
        actor.genome = None;
        peer.genome = None;
        peer.effects.dodge = false;
        peer.effects.lavender = 0;
        touch(&mut world, &mut actor, &mut peer, 0, 0).unwrap();
        assert!(peer.state.energy < 20.0, "{actor_color:?}");
        assert!(peer.effects.has_dodged);
        assert!(actor.genome.is_none() && peer.genome.is_none());
        if actor_color == ColorKind::Lilac {
            assert_eq!(actor.body.color_argb(0).unwrap(), ColorKind::DarkLilac.argb().unwrap());
            assert!(actor.effects.use_extra_effects && peer.effects.use_extra_effects);
        }
    }
}

#[test]
fn null_pink_reads_follow_fruit_payment_and_precede_old_bark_payment() {
    let (world, mut actor, mut peer) = contact_pair(ColorKind::Pink, ColorKind::Fruit);
    actor.genome = None;
    peer.effects.spore_version = 2;
    assert_eq!(world.touch_pink(&mut actor, &mut peer, 0, 0), Err(WorldError::Geometry(GeometryError::MissingGenome)));
    assert_eq!(actor.state.energy, 9.0);
    assert_eq!(peer.state.energy, 20.0);
    assert_eq!(peer.state.temporary_argb(), ColorKind::Yellow.argb());
    assert!(peer.effects.has_dodged);
    let (world, mut actor, mut peer) = contact_pair(ColorKind::Pink, ColorKind::OldBark);
    actor.genome = None;
    actor.effects.is_enhanced = true;
    peer.effects.is_enhanced = true;
    peer.state.active = true;
    peer.state.temporary_color_frames = 0;
    assert_eq!(world.touch_pink(&mut actor, &mut peer, 0, 0), Err(WorldError::Geometry(GeometryError::MissingGenome)));
    assert_eq!(actor.state.energy, 10.0);
    assert_eq!(peer.state.energy, 20.0);
    assert!(peer.state.temporary_argb().is_none());
    assert!(peer.effects.has_dodged);
    actor.effects.is_enhanced = false;
    world.touch_pink(&mut actor, &mut peer, 0, 0).unwrap();
    assert_eq!(actor.state.energy, 10.0);
}

#[test]
fn friendly_null_branches_keep_altruist_transfer_and_mint_short_circuits() {
    let (world, mut actor, mut peer) = contact_pair(ColorKind::Rose, ColorKind::Green);
    actor.genome = None;
    peer.genome = None;
    actor.state.energy = 20.0;
    peer.state.energy = 2.0;
    actor.body.growth_ratio = 1;
    actor.effects.transfers_energy = true;
    actor.effects.altruist = true;
    peer.effects.altruist = true;
    actor.effects.use_friend_effects = 0;
    let total = actor.state.energy + peer.state.energy;
    world.touch_friend_effects(&mut actor, &mut peer, 0, 0).unwrap();
    assert!(peer.state.energy > 2.0 && actor.state.energy < 20.0);
    assert_eq!(actor.state.energy + peer.state.energy, total);
    assert_eq!(peer.state.temporary_argb(), ColorKind::Cyan.argb());
    let (world, mut actor, mut peer) = contact_pair(ColorKind::Mint, ColorKind::Mint);
    actor.genome = None;
    peer.genome = None;
    peer.state.energy = 10.0;
    actor.effects.use_friend_effects = 2;
    world.session.0.parameters.borrow_mut().lifecycle.mint_energy_consumption = 2.0;
    peer.infected_genome = Some(code(ColorKind::Green));
    peer.saved_genome = peer.infected_genome.clone();
    world.touch_friend_effects(&mut actor, &mut peer, 0, 0).unwrap();
    assert!(peer.infected_genome.is_none() && peer.saved_genome.is_none());
    assert_eq!((actor.state.energy, peer.state.energy), (9.0, 9.0));
    let (world, mut actor, mut peer) = contact_pair(ColorKind::Mint, ColorKind::Brown);
    actor.genome = None;
    actor.effects.use_friend_effects = 2;
    peer.infected_genome = Some(code(ColorKind::Green));
    world.touch_friend_effects(&mut actor, &mut peer, 0, 0).unwrap();
    assert!(peer.infected_genome.is_some());
    assert_eq!((actor.state.energy, peer.state.energy), (10.0, 20.0));
}

#[test]
fn hatching_null_primary_reads_follow_guards_and_peer_modifiers() {
    let (world, mut actor, mut peer) = contact_pair(ColorKind::Blue, ColorKind::Cream);
    actor.genome = None;
    peer.genome = None;
    actor.effects.spin = 0;
    world.touch_hatch(&mut actor, &peer, 0).unwrap();
    peer.body.set_color_argb(0, ColorKind::Sky.argb().unwrap()).unwrap();
    peer.effects.is_plant = true;
    actor.effects.healing = 0;
    world.touch_hatch(&mut actor, &peer, 0).unwrap();
    peer.body.set_color_argb(0, ColorKind::Silver.argb().unwrap()).unwrap();
    peer.effects.is_plant = false;
    peer.state.active = false;
    world.touch_hatch(&mut actor, &peer, 0).unwrap();
    peer.body.set_color_argb(0, ColorKind::Coral.argb().unwrap()).unwrap();
    peer.state.active = true;
    actor.effects.is_infectious = true;
    world.touch_hatch(&mut actor, &peer, 0).unwrap();
    peer.body.set_color_argb(0, ColorKind::Lilac.argb().unwrap()).unwrap();
    peer.effects.is_plant = true;
    let genome = code(ColorKind::Lilac);
    let mut properties = genome.borrow().properties();
    properties.modifiers.modifies_lilac = true;
    genome.borrow_mut().set_properties(properties);
    peer.genome = Some(genome);
    world.touch_hatch(&mut actor, &peer, 0).unwrap();
    peer.body.set_color_argb(0, ColorKind::Red.argb().unwrap()).unwrap();
    assert_eq!(world.touch_hatch(&mut actor, &peer, 0), Err(WorldError::Geometry(GeometryError::MissingGenome)));
}
