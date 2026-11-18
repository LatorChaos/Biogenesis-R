use super::*;
use crate::{ColorKind, GenomeModifiers, GenomeProperties, OrganismAction};

fn empty_world() -> World {
    World::new(WorldConfig {
        seed: 827364,
        initial_attempts: 0,
        ..WorldConfig::default()
    })
    .unwrap()
}

fn plain_handle() -> GenomeHandle {
    GenomeHandle::from_owned(Genome::from_parts(
        &[Gene::new(12.0, 0.0, ColorKind::Green.argb(), 0, [0; 36]).shared()],
        GenomeProperties {
            symmetry: 1,
            ..GenomeProperties::default()
        },
    ))
}

fn complete_properties() -> GenomeProperties {
    GenomeProperties {
        symmetry: 3,
        mirror: 1,
        mutation_rate: 47,
        activity: 2,
        modifiers: GenomeModifiers {
            cream: 3,
            fallow: 4,
            spore: 12,
            adapt_spore: 70,
            plague: true,
            disperse_children: true,
            generation_battle: true,
            sibling_battle: true,
            altruist: true,
            familial: true,
            social: true,
            peaceful: true,
            passive: true,
            clockwise: true,
            mimic_all: true,
            modifies_pink: true,
            modifies_lilac: true,
            modifies_sky: true,
            selfish: true,
        },
    }
}

#[test]
fn drafts_clone_every_slot_and_all_fields_without_live_aliases() {
    let world = empty_world();
    let original = Gene::new(
        -3.5,
        -0.0,
        Some(0x80123456),
        999,
        std::array::from_fn(|i| i as i32 * 7 - 90),
    )
    .shared();
    let mut code = Genome::from_parts(
        &[Rc::clone(&original), Rc::clone(&original)],
        complete_properties(),
    );
    code.set_generation(41);
    code.set_clade_id(Some("root+3-5>8<9".to_owned()));
    code.set_update_clade(true);
    let source = GenomeHandle::from_owned(code);
    let before_random = world.random_snapshot();
    let mut draft = world.new_genome_draft(Some(&source)).unwrap();
    assert_eq!(draft.properties(), complete_properties());
    assert_eq!(
        draft.genes(),
        vec![original.borrow().clone(), original.borrow().clone()]
    );
    assert_eq!(draft.genes()[0].theta().to_bits(), (-0.0_f64).to_bits());
    assert!(!Rc::ptr_eq(&draft.genome.genes().unwrap()[0], &draft.genome.genes().unwrap()[1]));
    assert!(
        draft
            .genome
            .genes().unwrap()
            .iter()
            .all(|slot| !Rc::ptr_eq(slot, &original))
    );
    let mut batch = draft.genes();
    batch[0].set_length(9.5);
    batch[0].set_reactions(std::array::from_fn(|i| i as i32));
    assert_eq!(draft.genes()[0].length(), -3.5);
    draft.replace_gene(0, batch.remove(0)).unwrap();
    assert_eq!(draft.genes()[1].length(), -3.5);
    assert_eq!(original.borrow().length(), -3.5);
    assert_eq!(source.detached_clone().generation(), 41);
    assert_eq!(world.random_snapshot(), before_random);
}

#[test]
fn empty_defaults_fixed_insert_delete_and_invalid_indices_are_pure() {
    let world = empty_world();
    world.session.0.parameters.borrow_mut().genetics.min_mutation_rate = 25;
    world.session.0.parameters.borrow_mut().genetics.max_mutation_rate = 75;
    let before_random = world.random_snapshot();
    let mut draft = world.new_genome_draft(None).unwrap();
    assert!(draft.genes().is_empty());
    assert_eq!(
        draft.properties(),
        GenomeProperties {
            symmetry: 2,
            mirror: 0,
            mutation_rate: 50,
            activity: 2,
            modifiers: GenomeModifiers {
                cream: 2,
                fallow: 2,
                spore: 4,
                adapt_spore: 4,
                ..GenomeModifiers::default()
            },
        }
    );
    assert_eq!(world.genetics_parameters().min_mutation_rate, 25);
    draft.insert_gene(0).unwrap();
    draft.insert_gene(1).unwrap();
    let fixed = Gene::new(2.0, 0.0, Some(0xff00ff00), -1, [0; 36]);
    assert_eq!(draft.genes(), vec![fixed.clone(), fixed]);
    let mut second = draft.genes()[1].clone();
    second.set_branch(999);
    second.set_reactions(std::array::from_fn(|i| i as i32 - 18));
    draft.replace_gene(1, second.clone()).unwrap();
    draft.insert_gene(0).unwrap();
    assert_eq!(draft.genes()[2], second);
    draft.remove_gene(0).unwrap();
    assert_eq!(draft.genes()[1].branch(), 999);
    let before = draft.genes();
    assert_eq!(
        draft.insert_gene(3),
        Err(DraftError::InvalidIndex { index: 3, count: 2 })
    );
    assert_eq!(
        draft.remove_gene(2),
        Err(DraftError::InvalidIndex { index: 2, count: 2 })
    );
    assert_eq!(
        draft.replace_gene(2, Gene::default()),
        Err(DraftError::InvalidIndex { index: 2, count: 2 })
    );
    assert_eq!(draft.genes(), before);
    assert_eq!(
        draft.preview(100, 100),
        Err(GeometryError::InvalidBranch { gene: 1 })
    );
    draft.clear();
    assert!(draft.commit().is_none());
    assert_eq!(world.random_snapshot(), before_random);
}

#[test]
fn commit_resets_lineage_retains_local_genes_and_empty_commit_keeps_clipboard() {
    let world = empty_world();
    let source = plain_handle();
    source.0.borrow_mut().set_generation(41);
    source.0.borrow_mut().set_clade_id(Some("root".to_owned()));
    source.0.borrow_mut().set_update_clade(true);
    let mut draft = source.draft().unwrap();
    draft.set_properties(complete_properties());
    let local_gene = Rc::clone(&draft.genome.genes().unwrap()[0]);
    let committed = draft.commit().unwrap();
    assert_ne!(committed, source);
    assert!(Rc::ptr_eq(&committed.0.borrow().genes().unwrap()[0], &local_gene));
    assert!(!Rc::ptr_eq(&local_gene, &source.0.borrow().genes().unwrap()[0]));
    let code = committed.detached_clone();
    assert_eq!(code.properties(), complete_properties());
    assert_eq!(code.generation(), 0);
    assert_eq!(code.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), None);
    assert!(!code.update_clade());
    assert_eq!(source.detached_clone().generation(), 41);
    let mut clipboard = Some(source.clone());
    if let Some(replacement) = world.new_genome_draft(None).unwrap().commit() {
        clipboard = Some(replacement);
    }
    assert_eq!(clipboard, Some(source));
}

#[test]
fn shared_paste_and_copy_use_object_identity_instead_of_legacy_ids() {
    let world = empty_world();
    let source = plain_handle();
    let first = world
        .paste_genome_handle(&source, [100, 100])
        .unwrap()
        .unwrap();
    let second = world
        .paste_genome_handle(&source, [300, 100])
        .unwrap()
        .unwrap();
    assert!(Rc::ptr_eq(first.0.borrow().genome.as_ref().unwrap(), second.0.borrow().genome.as_ref().unwrap()));
    assert_eq!(world.copy_genome(&first), Some(Some(source.clone())));
    assert_eq!(world.copy_genome(&second), Some(Some(source.clone())));
    assert_eq!(source.detached_clone().generation(), 1);
    assert_eq!(source.detached_clone().clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("0"));
    let equal_distinct = GenomeHandle::from_owned(source.detached_clone());
    assert_ne!(source, equal_distinct);
    assert_eq!(source.detached_clone(), equal_distinct.detached_clone());
    let third = world
        .paste_genome_handle(&equal_distinct, [500, 100])
        .unwrap()
        .unwrap();
    third.0.borrow_mut().id = first.0.borrow().id;
    assert_eq!(world.copy_genome(&third), Some(Some(equal_distinct.clone())));
    first.0.borrow_mut().infected_genome = Some(Rc::clone(&equal_distinct.0));
    assert_eq!(world.copy_genome(&first), Some(Some(source.clone())));
    world.act_on_organism(&first, OrganismAction::Kill).unwrap();
    assert!(world.copy_genome(&first).is_none());
    assert_eq!(world.inspect_genome(&first), Some(Some(source.clone())));
    world.rebuild_spatial();
    world.organisms.borrow_mut()
        .retain(|member| !Rc::ptr_eq(member, &second.0));
    assert!(world.copy_genome(&second).is_none());
    assert!(world.inspect_genome(&second).is_none());
    assert!(source.preview(100, 100).is_ok());
    let other = empty_world();
    assert!(other.copy_genome(&third).is_none());
    assert!(other.inspect_genome(&third).is_none());
}

#[test]
fn clipboard_survives_reset_replacement_and_detached_mutation() {
    let mut world = empty_world();
    let clipboard = plain_handle();
    let first = world
        .paste_genome_handle(&clipboard, [100, 100])
        .unwrap()
        .unwrap();
    let mut detached = clipboard.detached_clone();
    detached.set_generation(900);
    detached.gene_ref(0).unwrap().borrow_mut().set_length(18.0);
    assert_eq!(clipboard.detached_clone().gene_ref(0).unwrap().borrow().length(), 12.0);
    world.reset().unwrap();
    assert!(world.inspect_genome(&first).is_none());
    assert_eq!(clipboard.detached_clone().generation(), 1);
    let replacement = empty_world();
    let pasted = replacement
        .paste_genome_handle(&clipboard, [300, 100])
        .unwrap()
        .unwrap();
    assert_eq!(replacement.copy_genome(&pasted), Some(Some(clipboard.clone())));
    assert_eq!(clipboard.detached_clone().clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("0"));
    assert_eq!(format!("{clipboard:?}"), "GenomeHandle(..)");
}

#[test]
fn failed_paste_retains_shared_generation_and_orientation_draw_without_spending() {
    let world = empty_world();
    let blocker = plain_handle();
    world
        .paste_genome_handle(&blocker, [3500, 3500])
        .unwrap()
        .unwrap();
    let source = plain_handle();
    let before = (
        world.carbon_dioxide.get().to_bits(),
        world.oxygen.get().to_bits(),
        world.methane.get().to_bits(),
        world.next_id.get(),
        world.organisms.borrow().len(),
    );
    let mut expected_random = world.random_snapshot();
    expected_random.next_f64();
    assert_eq!(
        world.paste_genome_handle(&source, [3500, 3500]).unwrap(),
        None
    );
    assert_eq!(world.random_snapshot(), expected_random);
    assert_eq!(
        (
            world.carbon_dioxide.get().to_bits(),
            world.oxygen.get().to_bits(),
            world.methane.get().to_bits(),
            world.next_id.get(),
            world.organisms.borrow().len()
        ),
        before
    );
    assert_eq!(source.detached_clone().generation(), 1);
    assert_eq!(source.detached_clone().clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), None);
    assert!(
        world
            .paste_genome_handle(&source, [5000, 5000])
            .unwrap()
            .is_some()
    );
    assert_eq!(source.detached_clone().clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("1"));
}

#[test]
fn random_world_action_uses_recorded_java_genome_suffix_before_orientation() {
    let mut parameters = WorldParameters::default();
    let mut seed = None;
    let mut tail = None;
    let mut selected = false;
    let mut weights = 0;
    for line in include_str!("../tests/fixtures/genetics.tsv").lines() {
        let row: Vec<_> = line.split('\t').collect();
        if row[0] == "C" {
            selected = row[1] == "random-complexity-1";
            if selected {
                assert_eq!(row[2], "genome-random");
                seed = Some(row[3].parse::<i64>().unwrap());
                parameters.genetics.initial_complexity = row[4].parse().unwrap();
                parameters.genetics.min_mutation_rate = row[5].parse().unwrap();
                parameters.genetics.max_mutation_rate = row[6].parse().unwrap();
                parameters.genetics.meta_mutation_rate = row[7].parse().unwrap();
                parameters.genetics.clade_complexity = row[8].parse().unwrap();
            }
        } else if selected && row[0] == "W" {
            let index = row[1].parse::<usize>().unwrap();
            assert_eq!(row[2], crate::DEFAULT_COLORS[index].0);
            parameters.genetics.color_weights[index] = row[3].parse().unwrap();
            weights += 1;
        } else if selected && row[0] == "R" {
            tail = Some((
                row[1].parse::<i64>().unwrap(),
                row[2].parse::<i32>().unwrap(),
                u64::from_str_radix(row[3], 16).unwrap(),
            ));
        } else if selected && row[0] == "E" {
            break;
        }
    }
    assert_eq!(weights, 46);
    let (next_long, next_int, next_double) = tail.unwrap();
    let world = World::with_parameters(
        WorldConfig {
            seed: seed.unwrap(),
            initial_attempts: 0,
            ..WorldConfig::default()
        },
        parameters,
    )
    .unwrap();
    let actor = world.random_genome_at([3500, 3500]).unwrap().unwrap();
    // Java nextLong consumes the same two generator steps as constructor nextDouble
    let low = next_long as i32;
    let high = (next_long.wrapping_sub(i64::from(low)) >> 32) as u32;
    let orientation_fraction = (((u64::from(high) >> 6) << 27) + (u64::from(low as u32) >> 5))
        as f64
        / (1_u64 << 53) as f64;
    assert_eq!(
        actor.0.borrow().state.angle.to_bits(),
        (orientation_fraction * PI * 2.0).to_bits()
    );
    let mut actual = world.random_snapshot();
    assert_eq!(actual.next_bounded(10000).unwrap(), next_int);
    assert_eq!(actual.next_f64().to_bits(), next_double);
    assert_eq!(world.population(), 1);
}

#[test]
fn estimates_preserve_current_settings_and_java_wrapping_arithmetic() {
    let world = empty_world();
    let mut draft = world.new_genome_draft(None).unwrap();
    assert_eq!(
        world.genome_draft_estimates(&draft).unwrap(),
        GenomeEstimates {
            segments: 0,
            energy: 40,
            life: 6
        }
    );
    draft.insert_gene(0).unwrap();
    draft.insert_gene(1).unwrap();
    world.session.0.parameters.borrow_mut().effects.max_age = 99;
    world.session.0.parameters.borrow_mut().effects.age_divisor = 3;
    assert_eq!(
        world.genome_draft_estimates(&draft).unwrap(),
        GenomeEstimates {
            segments: 4,
            energy: 52,
            life: 100
        }
    );
    // Explicit raw controls protect Java integer behavior without claiming valid editor input
    draft.set_properties(GenomeProperties {
        symmetry: i32::MIN / 2,
        ..draft.properties()
    });
    world.session.0.parameters.borrow_mut().effects.age_divisor = -1;
    assert_eq!(
        world.genome_draft_estimates(&draft).unwrap(),
        GenomeEstimates {
            segments: i32::MIN,
            energy: i32::MIN + 40,
            life: 99_i32.wrapping_add(i32::MIN),
        }
    );
    world.session.0.parameters.borrow_mut().effects.age_divisor = 0;
    assert!(matches!(
        world.genome_draft_estimates(&draft),
        Err(WorldError::Arithmetic(_))
    ));
    world.session.0.parameters.borrow_mut().genetics.min_mutation_rate = i32::MAX;
    world.session.0.parameters.borrow_mut().genetics.max_mutation_rate = i32::MAX;
    assert_eq!(world.new_genome_draft(None).unwrap().properties().mutation_rate, -1);
}

#[test]
fn raw_drafts_commit_without_repairs_and_report_distinct_preview_xml_and_paste_errors() {
    let world = empty_world();
    let before_random = world.random_snapshot();
    let mut draft = world.new_genome_draft(None).unwrap();
    draft.insert_gene(0).unwrap();
    draft.insert_gene(1).unwrap();
    let mut gene = draft.genes()[1].clone();
    gene.set_branch(2);
    gene.set_reaction(35, 900);
    draft.replace_gene(1, gene).unwrap();
    assert!(draft.preview(100, 100).is_ok());
    let handle = draft.commit().unwrap();
    assert_eq!(handle.detached_clone().gene_ref(1).unwrap().borrow().reaction(35), 900);
    assert_eq!(world.random_snapshot(), before_random);
    // Source rejects the branch after creation without publishing an organism
    let before_publication = (
        world.next_id.get(), world.population.get(), world.organisms.borrow().len(),
        world.oxygen.get().to_bits(), world.carbon_dioxide.get().to_bits(), world.methane.get().to_bits(),
    );
    let mut expected_random = world.random_snapshot();
    expected_random.next_f64();
    assert_eq!(world.paste_genome_handle(&handle, [3500, 3500]), Ok(None));
    assert_eq!(world.random_snapshot(), expected_random);
    assert_eq!(
        (world.next_id.get(), world.population.get(), world.organisms.borrow().len(),
         world.oxygen.get().to_bits(), world.carbon_dioxide.get().to_bits(), world.methane.get().to_bits()),
        before_publication,
    );
    let mut raw = handle.draft().unwrap();
    let mut gene = raw.genes()[0].clone();
    gene.set_color(None);
    raw.replace_gene(0, gene).unwrap();
    assert_eq!(
        raw.preview(100, 100),
        Err(GeometryError::MissingColor { gene: 0 })
    );
    let committed = raw.commit().unwrap();
    assert_eq!(committed.detached_clone().gene_ref(0).unwrap().borrow().color(), None);
    assert!(matches!(
        crate::encode_genome_xml(&committed.detached_clone()),
        Err(crate::XmlError::UnsupportedColor {
            gene: 0,
            argb: None
        })
    ));
    assert!(matches!(
        world.paste_genome_handle(&committed, [4000, 4000]),
        Err(WorldError::Geometry(GeometryError::MissingColor {
            gene: 0
        }))
    ));
    assert_eq!(handle.detached_clone().gene_ref(0).unwrap().borrow().color(), Some(0xff00ff00));
}

#[test]
fn angle_display_preserves_half_boundaries_raw_bits_and_rng() {
    let world = empty_world();
    let before_random = world.random_snapshot();
    let mut draft = world.new_genome_draft(None).unwrap();
    draft.insert_gene(0).unwrap();
    for (raw, expected) in [
        (0x3f81df46a2529d38, 0),
        (0x3f81df46a2529d39, 1),
        (0x3f81df46a2529d3a, 1),
        (0xbf81df46a2529d39, 0),
        (0xbf81df46a2529d3a, 359),
        (0x3faf46bb9c109323, 4),
        (0x3fb893011f31982e, 5),
        (0x8000000000000000, 0),
        (0x7ff8000000000042, 0),
        (0x7ff0000000000000, 359),
        (0xfff0000000000000, 0),
    ] {
        let mut gene = draft.genes()[0].clone();
        gene.set_theta(f64::from_bits(raw));
        draft.replace_gene(0, gene).unwrap();
        assert_eq!(draft.gene_angle_degrees(0).unwrap(), expected, "{raw:016x}");
        assert_eq!(draft.genes()[0].theta().to_bits(), raw);
    }
    draft.set_gene_angle_degrees(0, 270).unwrap();
    assert_eq!(
        draft.genes()[0].theta().to_bits(),
        (270.0 * PI / 180.0).to_bits()
    );
    let before = draft.genes()[0].theta().to_bits();
    for degrees in [-1, 360, i32::MIN, i32::MAX] {
        assert_eq!(
            draft.set_gene_angle_degrees(0, degrees),
            Err(DraftError::InvalidAngleDegrees { degrees })
        );
        assert_eq!(draft.genes()[0].theta().to_bits(), before);
    }
    assert_eq!(
        draft.gene_angle_degrees(1),
        Err(DraftError::InvalidIndex { index: 1, count: 1 })
    );
    assert_eq!(world.random_snapshot(), before_random);
}

#[test]
fn draft_raster_keeps_source_centering_and_enforces_combined_budget() {
    let draft = plain_handle().draft().unwrap();
    for (width, height, left, right, row) in [(100, 100, 44, 56, 50), (320, 240, 154, 166, 120)] {
        let image = draft.preview_image(width, height, 1024 * 1024).unwrap();
        assert_eq!((image.width, image.height), (width as u32, height as u32));
        assert_eq!(image.rgb.len(), width as usize * height as usize * 3);
        for (index, pixel) in image.rgb.chunks_exact(3).enumerate() {
            let x = index % width as usize;
            let y = index / width as usize;
            let expected = if y == row && (left..=right).contains(&x) { [0, 255, 0] } else { [0, 0, 0] };
            assert_eq!(pixel, expected);
        }
    }
    assert!(matches!(draft.preview_image(100, 100, 30_000),
        Err(PreviewImageError::Image(OrganismImageError::BudgetExceeded { .. }))));
    assert!(matches!(draft.preview_image(0, 100, usize::MAX),
        Err(PreviewImageError::Geometry(GeometryError::InvalidViewport))));
    assert_eq!(draft.genes()[0].length(), 12.0);
    assert_eq!(draft.genes()[0].color(), ColorKind::Green.argb());
}
