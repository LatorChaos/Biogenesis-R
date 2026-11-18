// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;

fn empty_world() -> World {
    World::new(WorldConfig {
        initial_attempts: 0,
        ..WorldConfig::default()
    })
    .unwrap()
}

#[test]
fn genesis_counts_successes_but_retains_configured_minimum_population() {
    let world = World::new(WorldConfig {
        seed: -1,
        width: 32,
        height: 32,
        initial_attempts: 300,
    })
    .unwrap();
    // The accepted Java construction fixture places 88 of these 300 attempts
    assert_eq!(world.population(), 88);
    assert_eq!(world.population(), world.organisms.borrow().len() as i32);
    assert_eq!(world.corpse_count().unwrap(), 0);
    assert_eq!(
        world.statistics_ref().borrow().get_created_organisms(),
        world.population()
    );
    assert_eq!(
        world.statistics_ref().borrow().get_max_population(),
        world.population()
    );
    assert_eq!(world.statistics_ref().borrow().get_min_population(), 300);
    assert!(world.statistics_ref().borrow().get_last_born_being().is_none());
    assert_eq!(world.statistics_ref().borrow().get_average_births(), 0.0);
}

#[test]
fn population_is_an_independent_wrapping_counter_with_synchronous_extrema() {
    let world = empty_world();
    world.decrease_population();
    assert_eq!(world.population(), -1);
    assert_eq!(world.corpse_count().unwrap(), 1);
    assert_eq!(world.statistics_ref().borrow().get_min_population(), -1);
    world.population.set(i32::MAX);
    world.increase_population();
    assert_eq!(world.population(), i32::MIN);
    assert_eq!(world.corpse_count().unwrap(), i32::MIN);
    assert_eq!(world.statistics_ref().borrow().get_max_population(), 0);
    world.decrease_population();
    assert_eq!(world.population(), i32::MAX);
    assert_eq!(world.statistics_ref().borrow().get_min_population(), -1);
}

#[test]
fn clock_samples_completed_gas_reactions_on_each_256th_accepted_step() {
    let mut world = empty_world();
    assert!(!world.step().unwrap());
    assert_eq!((world.frame(), world.time(), world.tick()), (0, 0, 0));
    for _ in 0..255 {
        assert!(world.single_step().unwrap());
    }
    assert_eq!((world.frame(), world.time(), world.tick()), (255, 0, 255));
    assert!(
        world.statistics_ref().borrow()
            .get_carbon_dioxide_list()
            .unwrap()
            .is_empty()
    );
    assert!(world.single_step().unwrap());
    assert_eq!((world.frame(), world.time(), world.tick()), (0, 1, 256));
    assert_eq!(crate::statistics::tests::samples(world.statistics_ref().borrow().get_population_list()), [0.0]);
    assert_eq!(
        crate::statistics::tests::samples(world.statistics_ref().borrow().get_oxygen_list()),
        [world.oxygen()]
    );
    assert_eq!(
        crate::statistics::tests::samples(world.statistics_ref().borrow().get_carbon_dioxide_list()),
        [world.carbon_dioxide()]
    );
    assert_eq!(
        crate::statistics::tests::samples(world.statistics_ref().borrow().get_methane_list()),
        [world.methane()]
    );
    world.resume();
    assert!(!world.single_step().unwrap());
    for _ in 0..256 {
        assert!(world.step().unwrap());
    }
    assert_eq!((world.frame(), world.time(), world.tick()), (0, 2, 512));
    assert_eq!(crate::statistics::tests::samples(world.statistics_ref().borrow().get_population_list()), [0.0, 0.0]);
    world.reset().unwrap();
    assert_eq!(world, empty_world());
}

#[test]
fn world_clone_keeps_live_statistics_aliases_and_isolates_record_only_genotypes() {
    let world = empty_world();
    for (seed, center) in [(0, [100, 100]), (1, [300, 300])] {
        world
            .paste_genome(Genome::random(&mut JavaRandom::new(seed)), center)
            .unwrap()
            .unwrap();
    }
    let parent = Rc::clone(&world.organisms.borrow()[0]);
    let child = Rc::clone(&world.organisms.borrow()[1]);
    parent.borrow_mut().state.total_children = 7;
    world.statistics_ref().borrow_mut()
        .event_organism_born(&child.borrow(), &parent.borrow());
    world.rebuild_spatial();
    world.organisms.borrow_mut().remove(1);
    let copy = world.clone();
    assert_eq!(world, copy);
    assert!(Rc::ptr_eq(
        copy.statistics_ref().borrow().get_being_most_children().unwrap(), copy.organisms.borrow()[0].borrow().genome.as_ref().unwrap()));
    assert!(Rc::ptr_eq(
        copy.statistics_ref().borrow().get_last_born_being().unwrap(), copy.frame_objects.borrow()[1].borrow().genome.as_ref().unwrap()));
    assert!(!Rc::ptr_eq(
        copy.statistics_ref().borrow().get_last_born_being().unwrap(),
        world.statistics_ref().borrow().get_last_born_being().unwrap()
    ));
    world.rebuild_spatial();
    let record_only = world.clone();
    assert_eq!(record_only.frame_objects.borrow().len(), 1);
    record_only.statistics_ref().borrow()
        .get_last_born_being()
        .unwrap()
        .borrow_mut()
        .set_clade_id(Some("record-only-copy".to_owned()));
    assert_ne!(
        world.statistics_ref().borrow()
            .get_last_born_being()
            .unwrap()
            .borrow()
            .clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(),
        Some("record-only-copy")
    );
    assert_eq!(
        world.statistics_ref().borrow().get_being_most_children_number(),
        7
    );
}

#[test]
fn real_flower_paste_preserves_java_float_rounded_reproduction_thresholds() {
    use crate::{ColorKind, Gene, GenomeModifiers, GenomeProperties};
    let mut count = 0;
    for line in include_str!("../tests/fixtures/thresholds.tsv").lines() {
        let row: Vec<_> = line.split('\t').collect();
        if row[0] != "F" {
            continue;
        }
        assert_eq!(row.len(), 5);
        let length = f64::from_bits(u64::from_str_radix(row[1], 16).unwrap());
        let code = Genome::from_parts(
            &[Gene::new(length, 0.0, ColorKind::Flower.argb(), -1, [0; 36]).shared()],
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
        let world = World::new(WorldConfig {
            seed: 827364,
            width: 2_000_000_000,
            height: 2_000_000_000,
            initial_attempts: 0,
        })
        .unwrap();
        let id = world
            .paste_genome(code, [1_000_000_000, 1_000_000_000])
            .unwrap()
            .unwrap();
        let actor = world.inspect(id).unwrap().unwrap();
        assert_eq!(
            actor.effects.reproduce_late,
            row[2].parse::<i32>().unwrap(),
            "Flower length {length}"
        );
        assert_eq!(
            actor.state.reproduction.early_energy,
            row[3].parse::<i32>().unwrap()
        );
        assert_eq!(
            actor.state.reproduction.energy,
            row[4].parse::<i32>().unwrap()
        );
        count += 1;
    }
    assert_eq!(count, 9);
}
