// SPDX-License-Identifier: GPL-2.0-or-later

use biogenesis_core::{
    DEFAULT_COLORS, Gene, GeneticsParams, Genome, GenomeModifiers, GenomeProperties, JavaRandom,
};
use std::rc::Rc;

fn properties(rate: i32) -> GenomeProperties {
    GenomeProperties {
        symmetry: 3,
        mirror: 1,
        mutation_rate: rate,
        activity: 2,
        modifiers: GenomeModifiers::default(),
    }
}

#[test]
fn nullable_color_and_raw_explicit_schema_survive_copy_and_setters() {
    let blank = Gene::default();
    assert_eq!(blank.color(), None);
    assert_eq!(*blank.reactions(), [0; 36]);
    let reactions = std::array::from_fn(|index| index as i32 * 17 - 300);
    let mut gene = Gene::new(-7.25, -19.5, Some(0x1280ff03), -99, reactions);
    let mut copied = gene.clone();
    copied.set_length(123.75);
    copied.set_theta(1024.0);
    copied.set_branch(i32::MIN);
    copied.set_color(None);
    for index in 0..36 {
        copied.set_reaction(index, i32::MAX - index as i32);
    }
    assert_eq!(
        gene,
        Gene::new(-7.25, -19.5, Some(0x1280ff03), -99, reactions)
    );
    assert_eq!(gene.rgb(), 0x80ff03);
    assert_eq!(copied.color(), None);
    gene.set_reactions(*copied.reactions());
    assert_eq!(
        *gene.reactions(),
        std::array::from_fn(|index| i32::MAX - index as i32)
    );
}

#[test]
fn explicit_container_and_zero_mutation_inheritance_share_genes_but_clone_is_deep() {
    let handle = Gene::new(4.5, 1.25, Some(0xff00ff00), -1, [2; 36]).shared();
    let mut container = vec![handle.clone(), handle.clone()];
    let mut parent = Genome::from_parts(&container, properties(0));
    assert_eq!(parent.generation(), 0);
    assert_eq!(parent.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), None);
    assert!(!parent.update_clade());
    container.clear();
    parent.set_generation(i32::MAX);
    parent.set_clade_id(Some("root+1<2".into()));
    parent.set_update_clade(true);
    let mut params = GeneticsParams::default();
    params.min_mutation_rate = 0;
    params.max_mutation_rate = 0;
    params.meta_mutation_rate = 0;
    let child = Genome::inherit_with(&parent, &mut JavaRandom::new(42), &params).unwrap();
    assert_eq!(child.generation(), i32::MIN);
    assert_eq!(child.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("root+1<2"));
    assert!(!child.update_clade());
    assert!(Rc::ptr_eq(&parent.genes().unwrap()[0], &child.genes().unwrap()[0]));
    assert!(Rc::ptr_eq(&parent.genes().unwrap()[1], &child.genes().unwrap()[1]));
    handle.borrow_mut().set_reaction(35, -777);
    assert_eq!(parent.gene_ref(1).unwrap().borrow().reaction(35), -777);
    assert_eq!(child.gene_ref(0).unwrap().borrow().reaction(35), -777);
    let cloned = parent.clone();
    assert!(!Rc::ptr_eq(&cloned.genes().unwrap()[0], &parent.genes().unwrap()[0]));
    assert!(!Rc::ptr_eq(&cloned.genes().unwrap()[0], &cloned.genes().unwrap()[1]));
    cloned.gene_ref(0).unwrap().borrow_mut().set_reaction(35, 999);
    assert_eq!(parent.gene_ref(0).unwrap().borrow().reaction(35), -777);
    assert_eq!(cloned.gene_ref(1).unwrap().borrow().reaction(35), -777);
    assert_eq!(cloned.generation(), parent.generation());
    assert_eq!(cloned.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), parent.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref());
    assert_eq!(cloned.update_clade(), parent.update_clade());
    let color = parent.gene_ref(0).unwrap().borrow().color_handle().unwrap().clone();
    assert!(cloned.gene_ref(0).unwrap().borrow().color_handle().unwrap().same_instance(&color));
    assert!(cloned.gene_ref(1).unwrap().borrow().color_handle().unwrap().same_instance(&color));
    cloned.gene_ref(0).unwrap().borrow_mut().set_color(Some(color.argb()));
    assert!(!cloned.gene_ref(0).unwrap().borrow().color_handle().unwrap().same_instance(&color));
    assert!(cloned.gene_ref(1).unwrap().borrow().color_handle().unwrap().same_instance(&color));
    assert!(parent.gene_ref(0).unwrap().borrow().color_handle().unwrap().same_instance(&color));
}

#[test]
fn every_configured_color_can_be_selected_without_randomizing_branch() {
    for (index, &(_, rgb, _)) in DEFAULT_COLORS.iter().enumerate() {
        let mut weights = [0; 46];
        weights[index] = 19;
        let mut gene = Gene::default();
        gene.set_branch(-271);
        gene.randomize(&mut JavaRandom::new(index as i64), &weights)
            .unwrap();
        assert_eq!(gene.color(), Some(0xff000000 | rgb));
        assert_eq!(gene.branch(), -271);
        assert!((2.0..18.0).contains(&gene.length()));
        assert!((0.0..2.0 * std::f64::consts::PI).contains(&gene.theta()));
        assert!(
            gene.reactions()
                .iter()
                .all(|reaction| (0..6).contains(reaction))
        );
    }
}

#[test]
fn invalid_parameter_boundaries_do_not_consume_randomness_or_partially_mutate_gene() {
    let mut cases = Vec::new();
    let mut params = GeneticsParams::default();
    params.color_weights = [0; 46];
    cases.push(params.clone());
    params.color_weights = [1; 46];
    params.color_weights[3] = -1;
    cases.push(params.clone());
    params.color_weights = [i32::MAX; 46];
    cases.push(params);
    params = GeneticsParams::default();
    params.initial_complexity = 0;
    cases.push(params.clone());
    params.initial_complexity = 2;
    params.min_mutation_rate = 100;
    params.max_mutation_rate = 0;
    cases.push(params.clone());
    params.min_mutation_rate = i32::MIN;
    params.max_mutation_rate = i32::MAX;
    cases.push(params);
    for invalid in cases {
        let mut random = JavaRandom::new(7);
        assert!(Genome::random_with(&mut random, &invalid).is_err());
        assert_eq!(random, JavaRandom::new(7));
    }
    let mut gene = Gene::new(8.0, -1.0, Some(0x01020304), 17, [9; 36]);
    let original = gene.clone();
    let mut random = JavaRandom::new(7);
    assert!(gene.randomize(&mut random, &[0; 46]).is_err());
    assert_eq!(gene, original);
    assert_eq!(random, JavaRandom::new(7));
}

#[test]
fn lineage_suffixes_use_the_reference_marker_count_and_clear_the_pending_flag() {
    let handle = Gene::random(&mut JavaRandom::new(0)).shared();
    let parent = Genome::from_parts(&[handle.clone()], properties(0));
    let mut child = Genome::from_parts(
        &[handle.clone(), handle],
        GenomeProperties {
            symmetry: 5,
            ..properties(0)
        },
    );
    child.set_clade_id(Some("root+1".into()));
    child.set_update_clade(true);
    child.update_lineage(&parent, 71, 2).unwrap();
    assert_eq!(child.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("root+1+71>71"));
    assert!(!child.update_clade());
    child.set_update_clade(true);
    child.update_lineage(&parent, 72, 2).unwrap();
    assert_eq!(child.clade_id().map(|text| text.to_utf8().expect("Fixture clade uses valid UTF16")).as_deref(), Some("root+1+71>71"));
    let mut absent = Genome::from_parts(&[], properties(0));
    absent.set_update_clade(true);
    assert!(absent.update_lineage(&parent, 0, 25).is_err());
    assert!(!absent.update_clade());
}

#[test]
fn lossless_clade_aliases_survive_copy_and_inheritance_until_a_real_lineage_edit() {
    let text = biogenesis_core::JavaText::from_units(vec![0xd800, 43, 49, 0]);
    let gene = Gene::new(4.0, 0.0, Some(0xff00ff00), 0, [0; 36]).shared();
    let mut parent = Genome::from_parts(&[gene.clone()], properties(0));
    parent.set_clade_text(Some(text.clone()));
    assert!(parent.clade_id().unwrap().same_instance(&text));
    assert!(parent.clone().clade_id().unwrap().same_instance(&text));
    let mut params = GeneticsParams::default();
    params.min_mutation_rate = 0;
    params.max_mutation_rate = 0;
    params.meta_mutation_rate = 0;
    let inherited = Genome::inherit_with(&parent, &mut JavaRandom::new(42), &params).unwrap();
    assert!(inherited.clade_id().unwrap().same_instance(&text));
    let mut child = Genome::from_parts(&[gene.clone(), gene], GenomeProperties { symmetry: 5, ..properties(0) });
    child.set_clade_text(Some(text.clone()));
    child.set_update_clade(true);
    child.update_lineage(&parent, 71, 2).unwrap();
    assert_eq!(child.clade_id().unwrap().units(), [0xd800, 43, 49, 0, 43, 55, 49, 62, 55, 49]);
    assert!(!child.clade_id().unwrap().same_instance(&text));
    assert_eq!(text.units(), [0xd800, 43, 49, 0]);
    let edited = child.clade_id().unwrap().clone();
    child.set_update_clade(true);
    child.update_lineage(&parent, 72, 1).unwrap();
    assert!(child.clade_id().unwrap().same_instance(&edited));
    assert!(!child.update_clade());
    child.set_clade_id(Some(String::new()));
    assert_eq!(child.clade_id().unwrap().units(), []);
    child.set_clade_id(None);
    assert!(child.clade_id().is_none());
}
