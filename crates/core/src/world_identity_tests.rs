// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;

#[test]
fn signed_saved_dimensions_preserve_bits_without_weakening_new_game_validation() {
    let world = World::new(WorldConfig {
        seed: 827364,
        initial_attempts: 0,
        ..WorldConfig::default()
    })
    .unwrap();
    world.session.0.random.borrow_mut().next_i64();
    let random = world.random_snapshot();
    for (width, height) in [(i32::MIN, i32::MAX), (-1, 0), (0, -1)] {
        { let mut config = world.config.get(); config.width = width; world.config.set(config); }
        { let mut config = world.config.get(); config.height = height; world.config.set(config); }
        assert_eq!(world.dimensions(), (width, height));
        assert_eq!(world.clone().dimensions(), (width, height));
        let before = world.clone();
        let failure = world
            .new_game(width, height, 0, world.parameters().clone())
            .unwrap_err();
        assert!(!failure.entered_genesis);
        assert_eq!(failure.error, WorldError::InvalidDimensions);
        assert_eq!(world, before);
        assert_eq!(world.random_snapshot(), random);
    }
    world
        .new_game(i32::MAX, i32::MAX, 0, world.parameters().clone())
        .unwrap();
    assert_eq!(world.dimensions(), (i32::MAX, i32::MAX));
    assert_eq!(world.random_snapshot(), random);
}

#[test]
fn world_clone_retains_reference_relationships_without_sharing_mutable_data() {
    let world = World::new(WorldConfig {
        seed: 42,
        width: 1000,
        height: 1000,
        initial_attempts: 0,
    })
    .unwrap();
    let code = Genome::random(&mut JavaRandom::new(0));
    let shared = Rc::new(RefCell::new(code));
    let same_genes = {
        let code = shared.borrow();
        Rc::new(RefCell::new(Genome::from_parts(
            &code.genes().unwrap(),
            code.properties(),
        )))
    };
    for (genome, center) in [
        (Rc::clone(&shared), [100, 100]),
        (Rc::clone(&shared), [200, 200]),
        (Rc::clone(&same_genes), [300, 300]),
    ] {
        let organism = world
            .construct_pasted_shared(genome, center)
            .unwrap()
            .unwrap();
        world.organisms.borrow_mut().push(Rc::new(RefCell::new(organism)));
    }
    world.organisms.borrow()[0].borrow_mut().infected_genome = Some(Rc::clone(&same_genes));
    world.organisms.borrow()[2].borrow_mut().saved_genome = Some(Rc::clone(&shared));
    let copy = world.clone();
    assert_eq!(world, copy);
    let original_ref = Rc::clone(&world.organisms.borrow()[0]);
    let original = original_ref.borrow();
    let first_ref = Rc::clone(&copy.organisms.borrow()[0]);
    let first = first_ref.borrow();
    let second_ref = Rc::clone(&copy.organisms.borrow()[1]);
    let second = second_ref.borrow();
    let third_ref = Rc::clone(&copy.organisms.borrow()[2]);
    let third = third_ref.borrow();
    assert!(Rc::ptr_eq(first.genome.as_ref().unwrap(), second.genome.as_ref().unwrap()));
    assert!(Rc::ptr_eq(
        first.infected_genome.as_ref().unwrap(), third.genome.as_ref().unwrap()));
    assert!(Rc::ptr_eq(
        third.saved_genome.as_ref().unwrap(), first.genome.as_ref().unwrap()));
    assert!(!Rc::ptr_eq(original.genome.as_ref().unwrap(), first.genome.as_ref().unwrap()));
    assert!(!Rc::ptr_eq(first.genome.as_ref().unwrap(), third.genome.as_ref().unwrap()));
    assert!(Rc::ptr_eq(
        &first.genome.as_ref().unwrap().borrow().genes().unwrap()[0],
        &third.genome.as_ref().unwrap().borrow().genes().unwrap()[0]
    ));
    assert!(!Rc::ptr_eq(
        &first.genome.as_ref().unwrap().borrow().genes().unwrap()[0],
        &original.genome.as_ref().unwrap().borrow().genes().unwrap()[0]
    ));
    first
        .genome.as_ref().unwrap().borrow_mut()
        .set_clade_id(Some("copied-shared".to_owned()));
    assert_eq!(second.genome.as_ref().unwrap().borrow().clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("copied-shared"));
    assert_eq!(original.genome.as_ref().unwrap().borrow().clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("0"));
    assert_eq!(third.genome.as_ref().unwrap().borrow().clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("2"));
    first.genome.as_ref().unwrap().borrow().gene_ref(0).unwrap().borrow_mut().set_length(17.0);
    assert_eq!(third.genome.as_ref().unwrap().borrow().gene_ref(0).unwrap().borrow().length(), 17.0);
    assert_ne!(original.genome.as_ref().unwrap().borrow().gene_ref(0).unwrap().borrow().length(), 17.0);
    let detached = copy.inspect(0).unwrap().unwrap();
    detached.genotype.as_ref().unwrap().gene_ref(0).unwrap().borrow_mut().set_length(3.0);
    assert_eq!(first.genome.as_ref().unwrap().borrow().gene_ref(0).unwrap().borrow().length(), 17.0);
}

#[test]
fn world_copy_shares_immutable_text_across_field_families_and_replaces_only_the_edited_holder() {
    let world = World::new(WorldConfig { initial_attempts: 0, ..WorldConfig::default() }).unwrap();
    let genome = Genome::random(&mut JavaRandom::new(0));
    world.paste_genome(genome, [1000, 1000]).unwrap().unwrap();
    let actor = Rc::clone(world.organisms.borrow().last().unwrap());
    let text = crate::JavaText::from_units(vec![0xd800, 0, 65]);
    actor.borrow().genome.as_ref().unwrap().borrow_mut().set_clade_text(Some(text.clone()));
    actor.borrow_mut().state.network_position = Some(text.clone());
    let copy = world.clone();
    let copied_ref = Rc::clone(&copy.organisms.borrow()[0]);
    let copied = copied_ref.borrow();
    assert!(!Rc::ptr_eq(copied.genome.as_ref().unwrap(), actor.borrow().genome.as_ref().unwrap()));
    assert!(copied.genome.as_ref().unwrap().borrow().clade_id().unwrap().same_instance(&text));
    assert!(copied.state.network_position.as_ref().unwrap().same_instance(&text));
    copied.genome.as_ref().unwrap().borrow_mut().set_clade_id(Some("copy-only".to_owned()));
    assert!(actor.borrow().genome.as_ref().unwrap().borrow().clade_id().unwrap().same_instance(&text));
    assert!(copied.state.network_position.as_ref().unwrap().same_instance(&text));
    assert_eq!(text.units(), [0xd800, 0, 65]);
}

#[test]
fn actual_litter_placement_replaces_all_reserved_units_without_changing_other_units_or_rng_order() {
    let world = World::new(WorldConfig { initial_attempts: 0, ..WorldConfig::default() }).unwrap();
    let genome = Genome::random(&mut JavaRandom::new(0));
    world.paste_genome(genome, [3000, 3000]).unwrap().unwrap();
    let parent_ref = Rc::clone(world.organisms.borrow().last().unwrap());
    let mut child = world.construct_pasted_shared(Rc::clone(parent_ref.borrow().genome.as_ref().unwrap()), [1000, 1000]).unwrap().unwrap();
    let text = crate::JavaText::from_units(vec![0xd800, 0, 48, 49, 50, 51, 52, 53, 54, 55, 48, 49, 50, 51, 52, 53, 54, 55]);
    let mut parent = parent_ref.borrow_mut();
    parent.state.children = 2;
    parent.state.network_position = Some(text.clone());
    let mut expected_random = world.random_snapshot();
    let reserved = 48 + expected_random.bounded(8) as u16;
    assert!(world.place_child(&parent_ref, &mut parent, None, &mut child, false).unwrap());
    assert_eq!(world.random_snapshot(), expected_random);
    let positions = parent.state.network_position.as_ref().unwrap();
    assert!(!positions.same_instance(&text));
    assert_eq!(&positions.units()[..2], [0xd800, 0]);
    assert_eq!(positions.units().len(), 18);
    assert_eq!(positions.units().iter().filter(|&&unit| unit == 88).count(), 2);
    for digit in 48..=55 {
        assert_eq!(positions.units().iter().filter(|&&unit| unit == digit).count(), if digit == reserved { 0 } else { 2 });
    }
    assert_eq!(text.units()[2..], [48, 49, 50, 51, 52, 53, 54, 55, 48, 49, 50, 51, 52, 53, 54, 55]);
}

#[test]
fn world_copy_keeps_color_reference_sharing_and_replaces_only_the_edited_gene() {
    let world = World::new(WorldConfig { initial_attempts: 0, ..WorldConfig::default() }).unwrap();
    let genome = Genome::random(&mut JavaRandom::new(0));
    let color = crate::JavaColor::from_argb(0xff00_ff00);
    genome.gene_ref(0).unwrap().borrow_mut().set_color_handle(Some(color.clone()));
    world.paste_genome(genome, [1000, 1000]).unwrap().unwrap();
    let original = Rc::clone(world.organisms.borrow()[0].borrow().genome.as_ref().unwrap());
    let copy = world.clone();
    let copied = Rc::clone(copy.organisms.borrow()[0].borrow().genome.as_ref().unwrap());
    assert!(!Rc::ptr_eq(&original, &copied));
    assert!(!Rc::ptr_eq(&original.borrow().genes().unwrap()[0], &copied.borrow().genes().unwrap()[0]));
    assert!(original.borrow().gene_ref(0).unwrap().borrow().color_handle().unwrap().same_instance(&color));
    assert!(copied.borrow().gene_ref(0).unwrap().borrow().color_handle().unwrap().same_instance(&color));
    copied.borrow().gene_ref(0).unwrap().borrow_mut().set_color(Some(color.argb()));
    assert!(!copied.borrow().gene_ref(0).unwrap().borrow().color_handle().unwrap().same_instance(&color));
    assert!(original.borrow().gene_ref(0).unwrap().borrow().color_handle().unwrap().same_instance(&color));
    assert_eq!(original.borrow().gene_ref(0).unwrap().borrow().color(), copied.borrow().gene_ref(0).unwrap().borrow().color());
    assert_eq!(world.random_snapshot(), copy.random_snapshot());
    drop(world);
    assert_eq!(color.argb(), 0xff00_ff00);
}
