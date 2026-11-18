// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use crate::{ColorKind, Gene, GeneRef, GenomeProperties};

fn world() -> World {
    World::new(WorldConfig {
        seed: 827364,
        width: 7000,
        height: 7000,
        initial_attempts: 0,
    })
    .unwrap()
}

fn genome(color: ColorKind, length: f64, symmetry: i32) -> (GenomeRef, GeneRef) {
    let gene = Gene::new(length, 0.0, color.argb(), 0, [0; 36]).shared();
    let code = Genome::from_parts(
        &[Rc::clone(&gene)],
        GenomeProperties {
            symmetry,
            ..GenomeProperties::default()
        },
    );
    (Rc::new(RefCell::new(code)), gene)
}

#[test]
fn candidate_keeps_real_genotype_and_defers_geometry_until_attempt() {
    let world = world();
    let code = Rc::new(RefCell::new(Genome::from_parts(
        &[],
        GenomeProperties {
            symmetry: 1,
            ..GenomeProperties::default()
        },
    )));
    let mut expected = world.random_snapshot();
    let angle = expected.next_f64() * PI * 2.0;
    let mut actor = world.new_pasted_candidate(Rc::clone(&code));
    assert!(Rc::ptr_eq(actor.genome.as_ref().unwrap(),  &code));
    assert_eq!(actor.state.angle.to_bits(), angle.to_bits());
    assert_eq!(world.random_snapshot(), expected);
    assert_eq!(code.borrow().generation(), 0);
    assert!(!actor.creation_complete);
    assert!((actor.body.segment_count == 0));
    assert_eq!(
        (actor.body.size().unwrap().width, actor.body.size().unwrap().height, actor.body.growth_ratio),
        (0, 0, 0)
    );
    assert_eq!(actor.state.bounds.get(), IntRect::default());
    assert!(actor.body.arrays.photo.is_none());
    let x = world.session.0.random.borrow_mut().next_bounded(100).unwrap();
    let y = world.session.0.random.borrow_mut().next_bounded(100).unwrap();
    expected.next_bounded(100).unwrap();
    expected.next_bounded(100).unwrap();
    assert!(
        world
            .attempt_paste(None, &mut actor, [1000 + x, 1000 + y])
            .unwrap()
    );
    assert_eq!(world.random_snapshot(), expected);
    assert_eq!(code.borrow().generation(), 1);
    assert_eq!(actor.state.parent_id, -1);
    assert_eq!(actor.body.growth_ratio, 16);
    assert!(actor.creation_complete);
    assert_eq!(world.next_id.get(), 1);
    assert!(world.organisms.borrow().is_empty());
}

#[test]
fn retries_draw_coordinates_after_one_orientation_and_preserve_candidate_identity() {
    let world = world();
    let (blocker, _) = genome(ColorKind::Green, 2000.0, 4);
    let blocker = world
        .construct_pasted_shared(blocker, [1050, 1050])
        .unwrap()
        .unwrap();
    world.add_organism(Rc::new(RefCell::new(blocker)), None);
    let (code, _) = genome(ColorKind::Green, 8.0, 1);
    let mut expected = world.random_snapshot();
    let angle = expected.next_f64() * PI * 2.0;
    let actor_ref = Rc::new(RefCell::new(world.new_pasted_candidate(Rc::clone(&code))));
    let identity = Rc::as_ptr(&actor_ref);
    for succeeds in [false, true] {
        let center = {
            let mut random = world.session.0.random.borrow_mut();
            [
                1000 + random.next_bounded(100).unwrap(),
                1000 + random.next_bounded(100).unwrap(),
            ]
        };
        assert_eq!(
            center,
            [
                1000 + expected.next_bounded(100).unwrap(),
                1000 + expected.next_bounded(100).unwrap()
            ]
        );
        assert_eq!(
            world
                .attempt_paste(Some(&actor_ref), &mut actor_ref.borrow_mut(), center)
                .unwrap(),
            succeeds
        );
        assert_eq!(world.random_snapshot(), expected);
        assert_eq!(Rc::as_ptr(&actor_ref), identity);
        assert_eq!(actor_ref.borrow().state.angle.to_bits(), angle.to_bits());
        assert_eq!(code.borrow().generation(), 1);
        if !succeeds {
            assert_eq!(world.next_id.get(), 1);
            let blocker_ref = Rc::clone(&world.organisms.borrow()[0]);
            let mut blocker = blocker_ref.borrow_mut();
            crate::movement::offset(&mut blocker, [3000.0, 3000.0], 0.0);
            crate::movement::calculate_bounds(&mut blocker, true).unwrap();
        }
    }
    assert_eq!(world.organisms.borrow().len(), 1);
    assert_eq!(actor_ref.borrow().id, 1);
    world.add_organism(Rc::clone(&actor_ref), None);
    assert_eq!(world.organisms.borrow().len(), 2);
    assert!(Rc::ptr_eq(&world.organisms.borrow()[1], &actor_ref));
}

#[test]
fn cheater_retry_replaces_genotype_arrays_and_retains_classification_and_lifecycle() {
    let world = world();
    let (code, gene) = genome(ColorKind::Eye, 8.0, 1);
    let mut actor = world.new_pasted_candidate(Rc::clone(&code));
    assert!(!world.attempt_paste(None, &mut actor, [0, 0]).unwrap());
    assert!(actor.body.eyes.enabled);
    actor.body.eyes.good_vision = true;
    actor.body.update_effects = 7;
    actor.effects.is_consumer = true;
    actor.effects.is_fungus = true;
    actor.effects.is_infectious = true;
    for index in actor.body.indices() { actor.body.set_photo(index, 123.0).unwrap(); }
    for index in actor.body.indices() { actor.body.set_rotated_segment(index, crate::RotatedSegment {
        start: [4, 5], end: [6, 7],
    }).unwrap(); }
    actor.state.age = 23;
    actor.state.active = false;
    actor.state.energy = 55.0;
    actor.state.temporary_color = (ColorKind::Gray.argb()).map(crate::JavaColor::from_argb);
    actor.state.temporary_color_frames = 6;
    actor.infected_genome = Some(Rc::clone(&code));
    actor.saved_genome = Some(Rc::clone(&code));
    let before = actor.state.clone();
    let gases = [world.oxygen.get(), world.carbon_dioxide.get(), world.methane.get()];
    let reactions = std::array::from_fn(|index| index as i32 + 10);
    {
        let mut gene = gene.borrow_mut();
        gene.set_color(ColorKind::Blue.argb());
        gene.set_reactions(reactions);
        gene.set_branch(1);
    }
    assert!(!world.attempt_paste(None, &mut actor, [3000, 3000]).unwrap());
    assert_eq!(actor.body.color_argb(0).unwrap(), ColorKind::Blue.argb().unwrap());
    assert_eq!(actor.body.reaction_values(0).unwrap(), reactions);
    assert!(actor.body.eyes.enabled && actor.body.eyes.good_vision);
    assert!(actor.body.eyes.consumer && actor.body.eyes.fungus && actor.body.eyes.infectious);
    assert_eq!(actor.body.update_effects, 7);
    assert_eq!(actor.body.photo_values().unwrap(), vec![0.0]);
    assert_eq!(
        actor.body.rotated_values().unwrap(),
        vec![crate::RotatedSegment {
            start: [0; 2],
            end: [0; 2]
        }]
    );
    assert_eq!(actor.state.center, before.center);
    assert_eq!(actor.state.integer_center, before.integer_center);
    assert_eq!(actor.state.bounds.get(), before.bounds.get());
    assert_eq!(
        actor.state.last_angle.to_bits(),
        before.last_angle.to_bits()
    );
    assert_eq!(actor.state.age, before.age);
    assert_eq!(actor.state.energy.to_bits(), before.energy.to_bits());
    assert!(!actor.state.active);
    assert_eq!(actor.state.temporary_argb(), before.temporary_argb());
    assert_eq!(
        actor.state.temporary_color_frames,
        before.temporary_color_frames
    );
    assert!(Rc::ptr_eq(actor.infected_genome.as_ref().unwrap(), &code));
    assert!(Rc::ptr_eq(actor.saved_genome.as_ref().unwrap(), &code));
    assert_eq!(world.next_id.get(), 0);
    assert_eq!([world.oxygen.get(), world.carbon_dioxide.get(), world.methane.get()], gases);
}

#[test]
fn held_member_repaste_excludes_only_exact_identity_before_any_borrow() {
    let world = world();
    let (code, _) = genome(ColorKind::Green, 12.0, 1);
    let first = world
        .construct_pasted_shared(code, [1000, 1000])
        .unwrap()
        .unwrap();
    let actor_ref = Rc::new(RefCell::new(first));
    world.add_organism(Rc::clone(&actor_ref), None);
    {
        let mut actor = actor_ref.borrow_mut();
        assert!(
            world
                .attempt_paste(Some(&actor_ref), &mut actor, [1000, 1000])
                .unwrap()
        );
        assert_eq!(actor.id, 1);
        assert_eq!(actor.genome.as_ref().unwrap().borrow().clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("0"));
    }
    assert_eq!((world.organisms.borrow().len(), world.population.get()), (1, 1));
    let (blocker, _) = genome(ColorKind::Green, 12.0, 1);
    let mut blocker = world
        .construct_pasted_shared(blocker, [2000, 2000])
        .unwrap()
        .unwrap();
    blocker.id = actor_ref.borrow().id;
    world.add_organism(Rc::new(RefCell::new(blocker)), None);
    assert!(
        !world
            .attempt_paste(Some(&actor_ref), &mut actor_ref.borrow_mut(), [2000, 2000])
            .unwrap()
    );
    assert_eq!(world.next_id.get(), 3);
}

#[test]
fn successful_attempt_initializes_before_caller_membership_and_uses_available_carbon() {
    let world = world();
    world.oxygen.set(-0.0);
    world.carbon_dioxide.set(0.25);
    let (code, _) = genome(ColorKind::Green, 8.0, 1);
    let mut actor = world.new_pasted_candidate(Rc::clone(&code));
    assert!(world.attempt_paste(None, &mut actor, [1000, 1000]).unwrap());
    assert_eq!(actor.id, 0);
    assert_eq!(code.borrow().clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("0"));
    assert_eq!(actor.state.energy.to_bits(), 0.25_f64.to_bits());
    assert_eq!(world.oxygen.get().to_bits(), 0.25_f64.to_bits());
    assert_eq!(world.carbon_dioxide.get().to_bits(), 0.0_f64.to_bits());
    assert_eq!(actor.state.reproduction.energy, 43);
    assert_eq!(actor.state.reproduction.maximum_countdown, 14);
    assert!(actor.effects.is_plant);
    assert_eq!(actor.body.update_effects, 2);
    assert_eq!((world.organisms.borrow().len(), world.population.get()), (0, 0));
    world.add_organism(Rc::new(RefCell::new(actor)), None);
    assert_eq!((world.organisms.borrow().len(), world.population.get()), (1, 1));
}

#[test]
fn creation_phase_survives_actor_clone_and_distinguishes_geometry_failure() {
    let world = world();
    let (code, gene) = genome(ColorKind::Green, 8.0, 1);
    let original = Rc::new(RefCell::new(world.new_pasted_candidate(Rc::clone(&code))));
    let mut cloner = crate::organism::OrganismCloner::default();
    let copied = cloner.copy(&original);
    assert!(!original.borrow().creation_complete && !copied.borrow().creation_complete);
    assert!(!Rc::ptr_eq(copied.borrow().genome.as_ref().unwrap(), &code));
    gene.borrow_mut().set_length(f64::NAN);
    assert_eq!(
        Body::at_growth(&code.borrow(), 16),
        Err(GeometryError::NonfiniteGene { gene: 0 }),
    );
    assert_eq!(
        world.attempt_paste(Some(&original), &mut original.borrow_mut(), [1000, 1000]),
        Ok(true),
    );
    assert!(original.borrow().creation_complete);
    assert!(!copied.borrow().creation_complete);
    assert_eq!(copied.borrow().genome.as_ref().unwrap().borrow().gene_ref(0).unwrap().borrow().length(), 8.0);
    assert_eq!(original.borrow().body.forward_count(), 1);
    assert!(original.borrow().body.arrays.length.as_ref().unwrap().get(0, "_m").unwrap().is_nan());
    let mut cloner = crate::organism::OrganismCloner::default();
    assert!(cloner.copy(&original).borrow().creation_complete);

    let first = Gene::new(8.0, 0.0, ColorKind::Green.argb(), 0, [0; 36]).shared();
    let second = Gene::new(8.0, 0.0, ColorKind::Green.argb(), 0, [0; 36]).shared();
    let colors = [first.borrow().color_handle().cloned().unwrap(),
                  second.borrow().color_handle().cloned().unwrap()];
    let code = Rc::new(RefCell::new(Genome::from_parts(
        &[Rc::clone(&first), Rc::clone(&second)],
        GenomeProperties { symmetry: 1, ..GenomeProperties::default() },
    )));
    let original = Rc::new(RefCell::new(world.new_pasted_candidate(Rc::clone(&code))));
    let mut cloner = crate::organism::OrganismCloner::default();
    let copied = cloner.copy(&original);
    assert!(!original.borrow().creation_complete && !copied.borrow().creation_complete);
    assert!(!Rc::ptr_eq(copied.borrow().genome.as_ref().unwrap(), &code));
    second.borrow_mut().set_branch(3);
    let before_publication = (
        world.next_id.get(), world.population.get(), world.organisms.borrow().len(),
        world.oxygen.get().to_bits(), world.carbon_dioxide.get().to_bits(), world.methane.get().to_bits(),
    );
    let before_random = world.random_snapshot();
    assert_eq!(
        world.attempt_paste(Some(&original), &mut original.borrow_mut(), [2000, 2000]),
        Err(WorldError::Geometry(GeometryError::PrimitiveIndex {
            field: "_endPointX", index: 2, length: 2,
        })),
    );
    assert!(original.borrow().creation_complete);
    assert!(!copied.borrow().creation_complete);
    assert_eq!(copied.borrow().genome.as_ref().unwrap().borrow().gene_ref(1).unwrap().borrow().branch(), 0);
    assert_eq!(copied.borrow().genome.as_ref().unwrap().borrow().gene_ref(1).unwrap().borrow().length(), 8.0);
    assert_eq!(world.random_snapshot(), before_random);
    assert_eq!(
        (world.next_id.get(), world.population.get(), world.organisms.borrow().len(),
         world.oxygen.get().to_bits(), world.carbon_dioxide.get().to_bits(), world.methane.get().to_bits()),
        before_publication,
    );
    let actor = original.borrow();
    let body = &actor.body;
    assert_eq!(body.forward_count(), 2);
    assert_eq!(body.photo_values().unwrap(), vec![0.0, 0.0]);
    assert_eq!(body.arrays.length.as_ref().unwrap().bits(),
               vec![1.44_f64.to_bits(), 1.44_f64.to_bits()]);
    assert_eq!(body.mass.to_bits(), 2.88_f64.to_bits());
    assert_eq!(body.inertia.to_bits(), 0.0_f64.to_bits());
    assert_eq!((body.size().unwrap().width, body.size().unwrap().height, body.size().unwrap()), (0, 0, IntRect::default()));
    assert_eq!(body.update_effects, 1);
    assert_eq!(body.eyes, crate::EyeState::default());
    assert_eq!(body.arrays.end_x.as_ref().unwrap().values(), vec![1, 0]);
    for (index, color) in colors.iter().enumerate() {
        assert!(body.color(index).unwrap().as_ref().unwrap().same_instance(color));
    }
    for reaction in &body.arrays.reactions {
        assert_eq!(reaction.as_ref().unwrap().values(), vec![0, 0]);
    }
    drop(actor);
    let mut cloner = crate::organism::OrganismCloner::default();
    assert!(cloner.copy(&original).borrow().creation_complete);
}

#[test]
fn empty_geometry_rebuild_and_preview_accept_the_same_real_genotype() {
    let genome = Genome::from_parts(
        &[],
        GenomeProperties {
            symmetry: 1,
            ..GenomeProperties::default()
        },
    );
    let mut body = Body::at_growth(&genome, 16).unwrap();
    assert!((body.segment_count == 0));
    assert_eq!((body.mass.to_bits(), body.inertia.to_bits()), (0, 0));
    assert!(
        body.rebuild(&genome, 1, body.eyes, body.update_effects)
            .unwrap()
            .segment_values().unwrap()
            .is_empty()
    );
    let preview = crate::GenomePreview::new(&genome, 80, 60).unwrap();
    assert!(preview.segments.is_empty() && preview.colors.is_empty());
    assert_eq!(preview.translation, [40, 30]);
    assert_eq!(preview.scale.to_bits(), 1.0_f64.to_bits());
    for growth in [0, 17, i32::from(u8::MAX)] {
        for result in [
            Body::at_growth(&genome, growth),
            Body::from_phenotype(&genome, growth, &[], crate::EyeState::default(), 0),
            Body::from_segment_state(&genome, growth, &[], &[], crate::EyeState::default(), 0),
        ] {
            assert_eq!(result, Err(GeometryError::InvalidGrowth));
        }
    }
    let raw = body.rebuild(&genome, 0, body.eyes, body.update_effects).unwrap();
    assert_eq!((raw.segment_count, raw.growth_ratio), (0, 0));
    assert_eq!(
        Body::from_segment_state(&genome, 0, &[], &[[0; 36]], crate::EyeState::default(), 0),
        Err(GeometryError::InvalidGrowth),
    );
    assert_eq!(
        Body::from_segment_state(&genome, 16, &[], &[[0; 36]], crate::EyeState::default(), 0),
        Err(GeometryError::ReactionLength),
    );
    let invalid = Genome::from_parts(
        &[],
        GenomeProperties {
            symmetry: 0,
            ..GenomeProperties::default()
        },
    );
    assert_eq!(
        Body::at_growth(&invalid, 16),
        Err(GeometryError::InvalidSymmetry)
    );
    assert_eq!(
        Body::at_growth(&invalid, 0),
        Err(GeometryError::InvalidSymmetry),
    );
    let nonfinite = Gene::new(f64::NAN, 0.0, ColorKind::Green.argb(), 0, [0; 36]).shared();
    let missing = Gene::new(8.0, 0.0, None, 0, [0; 36]).shared();
    let properties = GenomeProperties { symmetry: 1, ..GenomeProperties::default() };
    let nonfinite_code = Genome::from_parts(&[Rc::clone(&nonfinite)], properties);
    assert_eq!(
        Body::at_growth(&nonfinite_code, 0),
        Err(GeometryError::NonfiniteGene { gene: 0 }),
    );
    assert_eq!(
        Body::from_segment_state(&nonfinite_code, 0, &[0xff00ff00], &[], crate::EyeState::default(), 0),
        Err(GeometryError::NonfiniteGene { gene: 0 }),
    );
    assert_eq!(
        Body::from_phenotype(&nonfinite_code, 0, &[], crate::EyeState::default(), 0),
        Err(GeometryError::PhenotypeLength),
    );
    let missing_code = Genome::from_parts(&[Rc::clone(&nonfinite), Rc::clone(&missing)], properties);
    assert_eq!(
        Body::at_growth(&missing_code, 0),
        Err(GeometryError::MissingColor { gene: 1 }),
    );
    let finite = Gene::new(8.0, 0.0, ColorKind::Green.argb(), 0, [0; 36]).shared();
    let branch = Gene::new(8.0, 0.0, ColorKind::Green.argb(), 3, [0; 36]).shared();
    let branched = Genome::from_parts(&[finite, branch], properties);
    assert_eq!(
        Body::at_growth(&branched, 0),
        Err(GeometryError::InvalidBranch { gene: 1 }),
    );
    assert_eq!(
        Body::from_segment_state(&branched, 0, &[0xff00ff00; 2], &[], crate::EyeState::default(), 0),
        Err(GeometryError::InvalidBranch { gene: 1 }),
    );
    let allowed_null = Genome::from_parts(&[missing], properties);
    let allowed = Body::from_phenotype(&allowed_null, 16, &[0xff00ff00], crate::EyeState::default(), 0).unwrap();
    assert_eq!(allowed.color_argb(0), Ok(0xff00ff00));
    assert!(allowed_null.gene_ref(0).unwrap().borrow().color_handle().is_none());
}

#[test]
fn invalid_symmetry_is_rejected_before_creating_any_segment_array() {
    let (code, _) = genome(ColorKind::Green, 8.0, 9);
    let mut body = Body::uncreated();
    assert_eq!(
        body.create_arrays(&code.borrow()),
        Err(GeometryError::InvalidSymmetry)
    );
    assert!((body.segment_count == 0));
}
