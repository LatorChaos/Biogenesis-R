// SPDX-License-Identifier: GPL-2.0-or-later

use super::draining_contacts::age_bark;
use crate::organism::Organism;
use crate::{
    Body, ColorKind, EyeState, Gene, GeneArrayRef, GeneticsError, Genome, GenomeProperties,
    GeometryError, OrganismState, WorldError,
};
use std::{cell::RefCell, rc::Rc};

fn genome() -> Genome {
    Genome::from_parts(
        &[Gene::new(8.0, 0.0, ColorKind::Bark.argb(), 0, [0; 36]).shared()],
        GenomeProperties {
            symmetry: 1,
            ..Default::default()
        },
    )
}

fn actor() -> Organism {
    Organism::unplaced(Rc::new(RefCell::new(genome())), OrganismState::default()).unwrap()
}

#[test]
fn signed_growth_storage_defers_access_and_survives_detached_copy() {
    let code = genome();
    for ratio in [i32::MIN, -1, 0, 1, 16, 17, 256, i32::MAX] {
        let mut body = Body::uncreated();
        let rebuilt = body.rebuild(&code, ratio, EyeState::default(), 0).unwrap();
        assert_eq!((body.segment_count, body.growth_ratio), (0, ratio));
        assert_eq!(rebuilt.growth_ratio, ratio);
        assert_eq!(body.clone().growth_ratio, ratio);
        assert!(body.arrays.start_x.is_none() && body.phenotype_colors.is_none());
        if !(1..=16).contains(&ratio) {
            assert_eq!(
                Body::at_growth(&code, ratio),
                Err(GeometryError::InvalidGrowth)
            );
        }
    }
}

#[test]
fn signed_growth_table_failure_keeps_rebuild_prefix_and_gene_precedence() {
    let code = genome();
    let mut body = Body::at_growth(&code, 16).unwrap();
    let old_segments = body.segment_values().unwrap();
    let old_size = body.size().unwrap();
    body.mass = 77.0;
    body.inertia = 88.0;
    body.update_effects = 0;
    assert_eq!(
        body.rebuild(&code, i32::MIN, EyeState::default(), 0),
        Err(GeometryError::GrowthIndex { index: i32::MAX }),
    );
    assert_eq!((body.growth_ratio, body.update_effects), (i32::MIN, 1));
    assert_eq!((body.mass, body.inertia), (0.0, 88.0));
    assert_eq!(body.segment_values().unwrap(), old_segments);
    assert_eq!(body.size().unwrap(), old_size);
    let null_gene = code.with_array(Some(GeneArrayRef::from_slots(vec![None])));
    assert_eq!(
        body.rebuild(&null_gene, i32::MIN, EyeState::default(), 0),
        Err(GeometryError::Genetics(GeneticsError::NullGene {
            index: 0
        })),
    );
}

#[test]
fn signed_growth_updates_preserve_values_before_failed_rebuild() {
    let mut growing = actor();
    growing.body.growth_ratio = 257;
    growing.body.mass = 77.0;
    growing.state.energy = 100.0;
    growing.state.age = 7;
    growing.state.active = true;
    growing.state.has_grown = 9;
    growing.state.velocity = [3.0, -4.0];
    assert_eq!(
        crate::movement::grow(&mut growing),
        Err(GeometryError::GrowthIndex { index: 255 })
    );
    assert_eq!((growing.body.growth_ratio, growing.body.mass), (256, 0.0));
    assert_eq!(growing.state.has_grown, 9);
    assert_eq!(growing.state.velocity, [3.0, -4.0]);

    let mut shrinking = actor();
    shrinking.body.growth_ratio = i32::MIN;
    shrinking.body.mass = 77.0;
    shrinking.state.energy = -100.0;
    shrinking.state.has_grown = 9;
    assert_eq!(
        crate::movement::grow(&mut shrinking),
        Err(GeometryError::GrowthIndex { index: i32::MIN })
    );
    assert_eq!(
        (shrinking.body.growth_ratio, shrinking.body.mass),
        (i32::MIN + 1, 0.0)
    );
    assert_eq!(shrinking.state.has_grown, 9);
}

#[test]
fn signed_growth_contact_failure_retains_color_store_and_array_precedence() {
    let mut peer = actor();
    peer.state.active = true;
    peer.body.growth_ratio = i32::MIN;
    peer.effects.photosynthesis = 91.0;
    assert_eq!(
        age_bark(&mut peer, 0),
        Err(WorldError::Geometry(GeometryError::GrowthIndex {
            index: i32::MAX
        }))
    );
    assert_eq!(peer.body.color_kind(0).unwrap(), ColorKind::OldBark);
    assert_eq!(peer.effects.photosynthesis, 91.0);
    peer.body.arrays.photo = None;
    assert_eq!(
        age_bark(&mut peer, 0),
        Err(WorldError::Geometry(GeometryError::MissingPrimitiveArray {
            field: "_mphoto"
        }))
    );
    assert_eq!(peer.effects.photosynthesis, 91.0);
}

#[test]
fn jade_photo_gate_defers_growth_access_and_preserves_rng_prefix() {
    for carbon_dioxide in [0.0, 10.0] {
        let world = crate::World::new(crate::WorldConfig {
            initial_attempts: 0,
            ..Default::default()
        })
        .unwrap();
        let code = Genome::from_parts(
            &[Gene::new(8.0, 0.0, ColorKind::Green.argb(), 0, [0; 36]).shared()],
            GenomeProperties {
                symmetry: 1,
                ..Default::default()
            },
        );
        let mut organism =
            Organism::unplaced(Rc::new(RefCell::new(code)), OrganismState::default()).unwrap();
        organism.state.world = world.association();
        organism
            .body
            .set_color(0, ColorKind::DarkGreen.color_ref())
            .unwrap();
        organism.body.set_photo(0, 1.0).unwrap();
        organism.body.growth_ratio = i32::MIN;
        organism.effects.jade_factor = 1;
        organism.effects.lower_symmetric = 10;
        organism.effects.photosynthesis = 10.0;
        organism.effects.colony_photosynthesis = 3.0;
        organism.effects.leaf_photo = 0.0;
        organism.effects.methanotrophy = 0.0;
        organism.state.energy = 7.0;
        world.oxygen.set(20.0);
        world.carbon_dioxide.set(carbon_dioxide);
        world.methane.set(0.0);
        *world.session.0.random.borrow_mut() = crate::JavaRandom::new(0);
        let mut expected_random = world.random_snapshot();
        expected_random.next_bounded(1).unwrap();
        let skipped = carbon_dioxide == 0.0;
        let expected_result = if skipped {
            expected_random.next_bounded(10).unwrap();
            Ok(())
        } else {
            Err(WorldError::Geometry(GeometryError::GrowthIndex {
                index: i32::MAX,
            }))
        };
        assert_eq!(world.jade_extra_effects(&mut organism), expected_result);
        assert_eq!(world.random_snapshot(), expected_random);
        assert_eq!(
            organism.effects.colony_photosynthesis,
            if skipped { 0.0 } else { 3.0 }
        );
        assert_eq!(organism.body.color_kind(0).unwrap(), ColorKind::Green);
        assert_eq!(organism.body.growth_ratio, i32::MIN);
        assert_eq!(organism.state.energy, 7.0);
        assert_eq!(
            (world.oxygen.get(), world.carbon_dioxide.get(), world.methane.get()),
            (20.0, carbon_dioxide, 0.0)
        );
    }
}
