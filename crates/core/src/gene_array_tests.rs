// SPDX-License-Identifier: GPL-2.0-or-later
use crate::legacy_genetics::{GeneticsStager, export_genomes};
use crate::legacy_graph::{GraphErrorKind, LegacyGraph};
use crate::object_stream::{
    FieldValue, Limits, Node, Record, StreamReader, class_schema, encode_document,
};
use crate::*;
use std::{cell::RefCell, rc::Rc};
fn properties() -> GenomeProperties {
    GenomeProperties {
        symmetry: 1,
        modifiers: GenomeModifiers {
            cream: 1,
            fallow: 1,
            spore: 1,
            adapt_spore: 1,
            ..Default::default()
        },
        ..Default::default()
    }
}
fn raw(array: Option<GeneArrayRef>) -> crate::organism::GenomeRef {
    Rc::new(RefCell::new(Genome::from_raw_array(
        array,
        properties(),
        -17,
        Some(JavaText::from_units(vec![0, 0xd800])),
        true,
    )))
}
fn limits() -> Limits {
    Limits {
        handles: 4096,
        array_elements: 1024,
        total_array_elements: 8192,
        records: 4096,
        allocated_bytes: 8 << 20,
        work: 2_000_000,
        ..Default::default()
    }
}

#[test]
fn nullable_fixed_gene_slots_preserve_identity_and_signed_access_order() {
    let gene = Gene::default().shared();
    let array =
        GeneArrayRef::from_slots(vec![Some(Rc::clone(&gene)), None, Some(Rc::clone(&gene))]);
    let mut first = Genome::from_raw_array(Some(array.clone()), properties(), 0, None, false);
    let peer = first.with_array(Some(array.clone()));
    assert_eq!(first.gene_count(), Ok(3));
    assert!(first.gene_slot(1).unwrap().is_none());
    assert_eq!(
        first.gene_slot(-1),
        Err(GeneticsError::GeneIndex {
            index: -1,
            length: 3
        })
    );
    assert_eq!(first.gene_ref(1), Err(GeneticsError::NullGene { index: 1 }));
    if usize::BITS > 32 {
        let valid = Genome::from_parts(
            &[Gene::new(2.0, 0.0, ColorKind::Green.argb(), 0, [0; 36]).shared()],
            properties(),
        );
        let index = usize::try_from(1u64 << 32).unwrap();
        assert_eq!(
            valid.gene_ref(index),
            Err(GeneticsError::GeneIndexDomain { index })
        );
        assert_eq!(
            valid.gene_for_segment(index),
            Err(GeneticsError::GeneIndexDomain { index })
        );
    }
    array.set(1, Some(Rc::clone(&gene))).unwrap();
    assert!(Rc::ptr_eq(&peer.gene_ref(1).unwrap(), &gene));
    let copied = Genome::from_nullable_parts(&array.slots(), properties());
    assert!(!copied.gene_array().unwrap().same_instance(&array));
    assert!(Rc::ptr_eq(&copied.gene_ref(1).unwrap(), &gene));
    first.set_gene_array(None);
    assert_eq!(first.gene_count(), Err(GeneticsError::NullGeneArray));
    assert_eq!(first.gene_slot(i32::MIN), Err(GeneticsError::NullGeneArray));
    assert_eq!(peer.gene_count(), Ok(3));
    assert!(first.clone().gene_array().is_none());
    let empty = Genome::from_parts(&[], properties());
    assert_eq!(
        empty.gene_slot(0),
        Err(GeneticsError::GeneIndex {
            index: 0,
            length: 0
        })
    );
    let modulo = empty.gene_for_segment(0).unwrap_err();
    assert_eq!(modulo, GeneticsError::ZeroGeneDivisor);
    assert_eq!(
        GeometryError::from(modulo),
        GeometryError::Genetics(GeneticsError::ZeroGeneDivisor)
    );
    assert_eq!(
        WorldError::from(modulo),
        WorldError::Geometry(GeometryError::Genetics(GeneticsError::ZeroGeneDivisor))
    );
    assert!(matches!(
        WorldError::from(GeneticsError::InvalidGeneCount),
        WorldError::InvalidParameters(_)
    ));
}

#[test]
fn java_clone_and_snapshot_copy_have_distinct_nullable_contracts() {
    let gene = Gene::default().shared();
    let code = Genome::from_nullable_parts(
        &[Some(Rc::clone(&gene)), Some(Rc::clone(&gene)), None],
        properties(),
    );
    assert!(matches!(
        code.java_clone(),
        Err(GeneticsError::NullGene { index: 2 })
    ));
    let copied = code.clone();
    assert!(copied.gene_slot(2).unwrap().is_none());
    assert!(!Rc::ptr_eq(
        &copied.gene_ref(0).unwrap(),
        &copied.gene_ref(1).unwrap()
    ));
    assert!(!Rc::ptr_eq(&copied.gene_ref(0).unwrap(), &gene));
    let empty = Genome::from_nullable_parts(&[], properties());
    assert_eq!(empty.java_clone().unwrap().gene_count(), Ok(0));
    assert!(matches!(
        Genome::from_raw_array(None, properties(), 0, None, false).java_clone(),
        Err(GeneticsError::NullGeneArray)
    ));
}

#[test]
fn world_gene_copy_preserves_array_and_gene_aliases_without_retaining_original_mutability() {
    let gene = Gene::default().shared();
    let array =
        GeneArrayRef::from_slots(vec![Some(Rc::clone(&gene)), None, Some(Rc::clone(&gene))]);
    let first = raw(Some(array.clone()));
    let peer = raw(Some(array.clone()));
    let null = raw(None);
    let weak = Rc::downgrade(&gene);
    let mut copy = crate::organism::GenomeCloner::default();
    let a = copy.copy(&first);
    let b = copy.copy(&peer);
    let n = copy.copy(&null);
    let again = copy.copy(&first);
    assert!(Rc::ptr_eq(&a, &again));
    assert!(!Rc::ptr_eq(&a, &b));
    assert!(
        a.borrow()
            .gene_array()
            .unwrap()
            .same_instance(b.borrow().gene_array().unwrap())
    );
    assert!(!a.borrow().gene_array().unwrap().same_instance(&array));
    assert!(n.borrow().gene_array().is_none());
    assert!(Rc::ptr_eq(
        &a.borrow().gene_ref(0).unwrap(),
        &b.borrow().gene_ref(2).unwrap()
    ));
    a.borrow()
        .gene_ref(0)
        .unwrap()
        .borrow_mut()
        .set_length(19.0);
    assert_eq!(b.borrow().gene_ref(2).unwrap().borrow().length(), 19.0);
    assert_eq!(gene.borrow().length(), 0.0);
    drop((gene, array, first, peer, null));
    assert!(weak.upgrade().is_none());
}

#[test]
fn typed_gene_stage_and_batch_export_preserve_raw_bits_nullable_topology_and_writer_boundary() {
    let payload = 0xfff8000000000042;
    let gene = Gene::new_with_color(
        f64::from_bits(payload),
        -0.0,
        ColorKind::Green.color_ref(),
        i32::MIN,
        [i32::MIN; 36],
    )
    .shared();
    let array =
        GeneArrayRef::from_slots(vec![Some(Rc::clone(&gene)), None, Some(Rc::clone(&gene))]);
    let a = raw(Some(array.clone()));
    let b = raw(Some(array));
    let n = raw(None);
    let color = gene.borrow().color_handle().unwrap().clone();
    let phenotype = ColorArrayRef::from_slots(vec![Some(color.clone()), None, Some(color.clone())]);
    let document = crate::legacy_genetics::export_genome_color_roots(
        &[Some(&a), Some(&b), None, Some(&n), Some(&a)],
        &[
            crate::legacy_color::ColorRoot::Colors(Some(&phenotype)),
            crate::legacy_color::ColorRoot::Color(Some(&color)),
        ],
        limits(),
        8 << 20,
    )
    .unwrap();
    let graph = LegacyGraph::new(&document, limits()).unwrap();
    let mut stager = GeneticsStager::new(&graph, 8 << 20).unwrap();
    let restored: Vec<_> = document.records[..5]
        .iter()
        .map(|v| {
            let Record::Value(value) = v else {
                panic!("Root")
            };
            stager.genome(*value).unwrap()
        })
        .collect();
    let Record::Value(array_value) = document.records[5] else {
        panic!("Color[] root")
    };
    let Record::Value(color_value) = document.records[6] else {
        panic!("Color root")
    };
    let cached = stager.color_array(array_value).unwrap().unwrap();
    let temporary = stager.color(color_value).unwrap().unwrap();
    assert!(cached.get(0).unwrap().unwrap().same_instance(&temporary));
    assert!(
        restored[0]
            .as_ref()
            .unwrap()
            .borrow()
            .gene_ref(0)
            .unwrap()
            .borrow()
            .color_handle()
            .unwrap()
            .same_instance(&temporary)
    );
    let first = restored[0].as_ref().unwrap();
    let second = restored[1].as_ref().unwrap();
    assert!(restored[2].is_none());
    assert!(
        restored[3]
            .as_ref()
            .unwrap()
            .borrow()
            .gene_array()
            .is_none()
    );
    assert!(Rc::ptr_eq(first, restored[4].as_ref().unwrap()));
    assert!(
        first
            .borrow()
            .gene_array()
            .unwrap()
            .same_instance(second.borrow().gene_array().unwrap())
    );
    assert_eq!(
        first
            .borrow()
            .gene_ref(0)
            .unwrap()
            .borrow()
            .length()
            .to_bits(),
        payload
    );
    assert_eq!(first.borrow().clade_id().unwrap().units(), [0, 0xd800]);
    assert!(
        first
            .borrow()
            .clade_id()
            .unwrap()
            .same_instance(second.borrow().clade_id().unwrap())
            == a.borrow()
                .clade_id()
                .unwrap()
                .same_instance(b.borrow().clade_id().unwrap())
    );
    let bytes = encode_document(&document, limits()).unwrap();
    let normalized = StreamReader::new(&bytes, limits())
        .unwrap()
        .read_document()
        .unwrap();
    let graph = LegacyGraph::new(&normalized, limits()).unwrap();
    let mut stager = GeneticsStager::new(&graph, 8 << 20).unwrap();
    let Record::Value(value) = normalized.records[0] else {
        panic!("Root")
    };
    let normalized = stager.genome(value).unwrap().unwrap();
    assert_eq!(
        normalized
            .borrow()
            .gene_ref(0)
            .unwrap()
            .borrow()
            .length()
            .to_bits(),
        0x7ff8000000000000
    );
    assert_eq!(
        normalized
            .borrow()
            .gene_ref(0)
            .unwrap()
            .borrow()
            .theta()
            .to_bits(),
        (-0.0f64).to_bits()
    );
    assert_eq!(gene_count(&normalized), 3);
    let long = raw(Some(GeneArrayRef::from_slots(Vec::new())));
    long.borrow_mut()
        .set_clade_text(Some(JavaText::from_units(vec![0x0800; 33000])));
    let document = export_genomes(&[Some(&long)], limits(), 8 << 20).unwrap();
    let bytes = encode_document(&document, limits()).unwrap();
    let read = StreamReader::new(&bytes, limits())
        .unwrap()
        .read_document()
        .unwrap();
    let graph = LegacyGraph::new(&read, limits()).unwrap();
    let mut stager = GeneticsStager::new(&graph, 8 << 20).unwrap();
    let Record::Value(root) = read.records[0] else {
        panic!("Root")
    };
    assert_eq!(
        stager
            .genome(root)
            .unwrap()
            .unwrap()
            .borrow()
            .clade_id()
            .unwrap()
            .units(),
        vec![0x0800; 33000]
    );
}
fn gene_count(code: &crate::organism::GenomeRef) -> usize {
    code.borrow().gene_count().unwrap()
}

#[test]
fn gene_staging_budget_and_malformed_slot_fail_without_mutating_live_owners() {
    let gene = Gene::default().shared();
    let active = raw(Some(GeneArrayRef::from_slots(vec![Some(Rc::clone(&gene))])));
    let document = export_genomes(&[Some(&active)], limits(), 8 << 20).unwrap();
    let before = document.clone();
    let graph = LegacyGraph::new(&document, limits()).unwrap();
    assert!(matches!(GeneticsStager::new(&graph,0),Err(e)if e.kind==GraphErrorKind::Budget));
    assert!(
        matches!(export_genomes(&[Some(&active)],limits(),0),Err(e)if e.kind==GraphErrorKind::Budget)
    );
    assert_eq!(document, before);
    let text = raw(Some(GeneArrayRef::from_slots(Vec::new())));
    text.borrow_mut()
        .set_clade_text(Some(JavaText::from_units(vec![65; 100])));
    let mut units = limits();
    units.string_units = 64;
    assert!(
        matches!(export_genomes(&[Some(&text)],units,8<<20),Err(e)if e.kind==GraphErrorKind::Budget)
    );
    let mut bytes = limits();
    bytes.string_bytes = 64;
    assert!(
        matches!(export_genomes(&[Some(&text)],bytes,8<<20),Err(e)if e.kind==GraphErrorKind::Budget)
    );
    let null_colors = ColorArrayRef::from_slots(vec![None; 4096]);
    let roots = [crate::legacy_color::ColorRoot::Colors(Some(&null_colors))];
    let mut color_work = limits();
    color_work.handles = 64;
    color_work.array_elements = 4096;
    color_work.total_array_elements = 4096;
    color_work.work = 1000;
    let rejected =
        crate::legacy_genetics::export_genome_color_roots(&[], &roots, color_work, 8 << 20);
    assert!(matches!(rejected,Err(e)if e.kind==GraphErrorKind::Budget));
    assert_eq!(document, before);
    assert_eq!(null_colors.len(), 4096);
    assert!(null_colors.slots().iter().all(Option::is_none));
    color_work.work = 32768;
    let accepted =
        crate::legacy_genetics::export_genome_color_roots(&[], &roots, color_work, 8 << 20)
            .unwrap();
    let Record::Value(crate::object_stream::Value::Handle(array_id)) = accepted.records[0] else {
        panic!("Color[] root")
    };
    let Node::Array(array) = &accepted.arena.nodes[array_id.0 as usize] else {
        panic!("Color[] array")
    };
    let crate::object_stream::ArrayData::References(values) = &array.data else {
        panic!("Color references")
    };
    assert_eq!(values.len(), 4096);
    assert!(
        values
            .iter()
            .all(|v| *v == crate::object_stream::Value::Null)
    );
    encode_document(&accepted, color_work).unwrap();
    assert_eq!(document, before);
    assert!(null_colors.slots().iter().all(Option::is_none));
    let mut bad = document.clone();
    for node in &mut bad.arena.nodes {
        if let Node::Object(object) = node {
            let descriptor = match &document.arena.nodes[object.descriptor.0 as usize] {
                Node::Descriptor(v) => v,
                _ => panic!("Descriptor"),
            };
            let schema = class_schema(descriptor.class);
            if schema.name == "biogenesis.Gene" {
                let position = schema
                    .fields
                    .iter()
                    .position(|v| v.name == "_length")
                    .unwrap();
                object.slots[0].values[position] = FieldValue::Int(7);
            }
        }
    }
    let graph = LegacyGraph::new(&bad, limits()).unwrap();
    let mut stager = GeneticsStager::new(&graph, 8 << 20).unwrap();
    let Record::Value(value) = bad.records[0] else {
        panic!("Root")
    };
    assert!(matches!(stager.genome(value),Err(e)if e.kind==GraphErrorKind::Type));
    let mut bad_slot = document.clone();
    let Record::Value(root) = bad_slot.records[0] else {
        panic!("Root")
    };
    for node in &mut bad_slot.arena.nodes {
        if let Node::Array(array) = node {
            if let crate::object_stream::ArrayData::References(values) = &mut array.data {
                if !values.is_empty() {
                    values[0] = root;
                }
            }
        }
    }
    let slot_graph = LegacyGraph::new(&bad_slot, limits()).unwrap();
    let mut slot_stager = GeneticsStager::new(&slot_graph, 8 << 20).unwrap();
    assert!(matches!(slot_stager.genome(root),Err(e)if e.kind==GraphErrorKind::Class));
    assert_eq!(gene.borrow().length(), 0.0);
    assert!(Rc::ptr_eq(&active.borrow().gene_ref(0).unwrap(), &gene));
    assert_eq!(document, before);
    let mut low = limits();
    low.work = 1;
    let graph = LegacyGraph::new(&document, low).unwrap();
    assert!(matches!(GeneticsStager::new(&graph,8<<20),Err(e)if e.kind==GraphErrorKind::Budget));
}

#[test]
fn nullable_genes_fail_editor_and_xml_before_publication() {
    let absent = Genome::from_raw_array(None, properties(), 0, None, false);
    assert!(matches!(
        encode_genome_xml(&absent),
        Err(XmlError::NullGeneArray)
    ));
    let handle = GenomeHandle::from_owned(absent);
    assert!(matches!(handle.draft(), Err(GeneticsError::NullGeneArray)));
    let code = Genome::from_nullable_parts(
        &[
            Some(Gene::new(2.0, 0.0, ColorKind::Green.argb(), 0, [0; 36]).shared()),
            None,
        ],
        properties(),
    );
    assert!(matches!(
        encode_genome_xml(&code),
        Err(XmlError::NullGene(1))
    ));
    let handle = GenomeHandle::from_owned(code);
    assert!(matches!(
        handle.draft(),
        Err(GeneticsError::NullGene { index: 1 })
    ));
    assert!(handle.detached_clone().gene_slot(1).unwrap().is_none());
}

#[test]
fn complete_actor_retention_charges_spare_gene_array_capacity_and_repeated_reference_work() {
    let small = raw(Some(GeneArrayRef::from_slots(Vec::new())));
    let mut spare = Vec::with_capacity(4096);
    spare.push(None);
    let array = GeneArrayRef::from_slots(spare);
    let large = raw(Some(array.clone()));
    let make = |code| {
        let actor = Rc::new(RefCell::new(crate::organism::Organism::pasted_candidate(
            Some(code),
            OrganismState::default(),
        )));
        actor.borrow().state.bounds.bind_actor(&actor).unwrap();
        actor
    };
    let a = make(small);
    let b = make(large);
    let mut bound = limits();
    bound.handles = 128;
    assert!(crate::legacy_rectangles::retain_actor_context(vec![a], bound, 16 << 10).is_ok());
    assert!(
        matches!(crate::legacy_rectangles::retain_actor_context(vec![Rc::clone(&b)],bound,16<<10),Err(e)if e.kind==GraphErrorKind::Budget)
    );
    assert!(
        b.borrow()
            .genome
            .as_ref()
            .unwrap()
            .borrow()
            .gene_array()
            .unwrap()
            .same_instance(&array)
    );
    assert_eq!(array.len(), 1);
    assert_eq!(array.capacity(), 4096);
    let roots = vec![Some(b.borrow().genome.as_ref().unwrap().clone()); 64];
    let refs: Vec<_> = roots.iter().map(|v| v.as_ref()).collect();
    let mut work = limits();
    work.work = 10;
    assert!(matches!(export_genomes(&refs,work,8<<20),Err(e)if e.kind==GraphErrorKind::Budget));
}

#[test]
fn effect_flags_precede_null_gene_count_and_cached_color_access() {
    for symmetry in [1, 8] {
        let mut properties = properties();
        properties.symmetry = symmetry;
        properties.modifiers.sibling_battle = true;
        properties.modifiers.generation_battle = true;
        properties.modifiers.altruist = true;
        let code = Genome::from_raw_array(None, properties, 0, None, false);
        let mut effects = EffectsState::default();
        effects.spin = 11;
        effects.healing = 17;
        let mut body = Body::uncreated();
        body.segment_count = 1;
        body.eyes.enabled = true;
        let mut reproduction = ReproductionState::default();
        reproduction.early_energy = 23;
        assert_eq!(
            effects.apply(
                Some(&code),
                &mut body,
                0,
                &mut reproduction,
                &EffectsParams::default()
            ),
            Err(GeometryError::Genetics(GeneticsError::NullGeneArray))
        );
        assert!(effects.sibling_battle && effects.generation_battle && effects.altruist);
        assert_eq!(
            (
                effects.spin,
                effects.healing,
                reproduction.early_energy,
                body.update_effects
            ),
            (0, 0, 0, 2)
        );
        assert!(!body.eyes.enabled);
        assert!(body.phenotype_colors.is_none());
    }
}

#[test]
fn initial_life_reads_null_first_gene_after_base_life_prefix() {
    let code = Genome::from_nullable_parts(&[None], properties());
    let params = EffectsParams {
        max_age: 9,
        age_divisor: 4,
        ..EffectsParams::default()
    };
    let mut reproduction = ReproductionState {
        maximum_age: 73,
        energy: 79,
        maximum_countdown: 83,
        ..ReproductionState::default()
    };
    assert_eq!(
        crate::organism::initialize_life(&code, 1, &mut reproduction, &params),
        Err(GeometryError::Genetics(GeneticsError::NullGene {
            index: 0
        }))
    );
    assert_eq!(
        (
            reproduction.maximum_age,
            reproduction.energy,
            reproduction.maximum_countdown
        ),
        (9, 43, 8)
    );
}
