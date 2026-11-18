// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use std::collections::BTreeSet;

fn prototype() -> GenomeRef {
    let bytes = include_bytes!("../tests/fixtures/object_stream_codec/genome.ser");
    let document = StreamReader::new(bytes, Limits::default()).unwrap().read_document().unwrap();
    let [Record::Value(root)] = document.records.as_slice() else { panic!("One recorded genotype root required"); };
    let graph = LegacyGraph::new(&document, Limits::default()).unwrap();
    GeneticsStager::new(&graph, Limits::default().allocated_bytes).unwrap().genome(*root).unwrap().unwrap()
}

#[test]
fn required12500_distinct_source_derived_genotypes_export_under_the_original_work_cap() {
    const COUNT: usize = 12_500;
    let prototype = prototype();
    let small = export_genomes(&[Some(&prototype), None, Some(&prototype)], Limits::default(), Limits::default().allocated_bytes).unwrap();
    let small_bytes = encode_document(&small, Limits::default()).unwrap();
    println!("BGR_EXPORT_SMALL_HEX {}", small_bytes.iter().map(|value| format!("{value:02x}")).collect::<String>());

    let expected_gene = prototype.borrow().gene_ref(0).unwrap().borrow().clone();
    let mut owners = Vec::with_capacity(COUNT);
    let shared_text = JavaText::from_units(Vec::new());
    for index in 0..COUNT {
        let gene = expected_gene.clone().shared();
        let array = GeneArrayRef::from_slots(vec![Some(Rc::clone(&gene)), None, Some(gene)]);
        let mut code = prototype.borrow().with_array(Some(array));
        code.set_generation(index as i32);
        code.set_clade_text(Some(if index % 3 == 0 {
            shared_text.clone()
        } else {
            JavaText::from_units(Vec::new())
        }));
        owners.push(Rc::new(RefCell::new(code)));
    }
    let mut roots: Vec<_> = owners.iter().map(Some).collect();
    roots.extend([Some(&owners[0]), None, Some(&owners[COUNT - 1])]);
    let limits = Limits::default();
    assert_eq!(limits.work, 128_000_000);
    let document = export_genomes(&roots, limits, limits.allocated_bytes).unwrap();
    let graph = LegacyGraph::new(&document, limits).unwrap();
    let mut codes = BTreeSet::new();
    let mut arrays = BTreeSet::new();
    let mut genes = BTreeSet::new();
    let mut texts = BTreeSet::new();
    let mut color = None;
    let mut shared_text_id = None;
    for (index, record) in document.records[..COUNT].iter().enumerate() {
        let Record::Value(Value::Handle(code)) = record else { panic!("Complete ordered genotype required"); };
        assert!(codes.insert(code.0));
        assert_eq!(graph.int(*code, CODE, "_generation").unwrap(), index as i32);
        let array = graph.required_reference(*code, CODE, "_genes").unwrap();
        assert!(arrays.insert(array.0));
        let Node::Array(array) = graph.node(array).unwrap() else { panic!("Gene array required"); };
        let ArrayData::References(values) = &array.data else { panic!("Reference array required"); };
        assert_eq!(values.len(), 3);
        assert_eq!(values[1], Value::Null);
        assert_eq!(values[0], values[2]);
        let Value::Handle(gene) = values[0] else { panic!("Recorded Gene required"); };
        assert!(genes.insert(gene.0));
        assert_eq!(graph.double_bits(gene, GENE, "_length").unwrap(), expected_gene.length().to_bits());
        assert_eq!(graph.double_bits(gene, GENE, "_theta").unwrap(), expected_gene.theta().to_bits());
        assert_eq!(graph.int(gene, GENE, "_branch").unwrap(), expected_gene.branch());
        let actual_color = graph.reference(gene, GENE, "_color").unwrap();
        if let Some(color) = color { assert_eq!(actual_color, color); }
        else { color = Some(actual_color); }
        let text = graph.required_reference(*code, CODE, "_cladeID").unwrap();
        if index % 3 == 0 {
            if let Some(previous) = shared_text_id { assert_eq!(text, previous); }
            else { shared_text_id = Some(text); assert!(texts.insert(text.0)); }
        } else {
            assert!(texts.insert(text.0));
        }
        assert_eq!(graph.string_units(text).unwrap(), []);
    }
    assert_eq!((codes.len(), arrays.len(), genes.len()), (COUNT, COUNT, COUNT));
    assert_eq!(texts.len(), COUNT - (COUNT + 2) / 3 + 1);
    assert_eq!(document.records[COUNT], document.records[0]);
    assert_eq!(document.records[COUNT + 1], Record::Value(Value::Null));
    assert_eq!(document.records[COUNT + 2], document.records[COUNT - 1]);
    let repeated = export_genomes(&roots, limits, limits.allocated_bytes).unwrap();
    assert_eq!(document, repeated);
    assert_eq!(encode_document(&document, limits).unwrap(), encode_document(&repeated, limits).unwrap());
}

#[test]
fn duplicate_genotype_edges_still_exhaust_the_work_budget() {
    let owner = prototype();
    let roots = vec![Some(&owner); 4096];
    let mut limits = Limits::default();
    limits.work = 512;
    let failure = export_genomes(&roots, limits, limits.allocated_bytes).unwrap_err();
    assert_eq!(failure.kind, GraphErrorKind::Budget);
}
