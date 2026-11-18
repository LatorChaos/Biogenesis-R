// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use crate::history::{HistoryListRef, JavaDoubleRef};
use crate::legacy_graph::{GraphErrorKind, LegacyGraph};
use crate::legacy_history::{HISTORY_FIELDS, HistoryStager, export_history_lists};
use crate::object_stream::{
    ClassData, ClassDescriptor, Document, FieldValue, JavaObject, JavaString, Limits, Node, NodeId,
    Record, StreamReader, Value, class_by_name, class_schema, encode_document,
};
use crate::statistics::tests::{
    Observation, assert_observation, observed_field, set_statistics_input,
};
use std::collections::{BTreeMap, HashMap};

const ORACLE: &str = include_str!("../tests/fixtures/history_ownership/history_ownership.tsv");
const BUDGET: usize = 8 * 1024 * 1024;

fn fixture(name: &str) -> &'static [u8] {
    macro_rules! bytes {
        ($name:literal) => {
            include_bytes!(concat!("../tests/fixtures/history_ownership/", $name))
        };
    }
    match name {
        "shared_99_history-initial.ser" => bytes!("shared_99_history-initial.ser"),
        "shared_99_history-after_event.ser" => bytes!("shared_99_history-after_event.ser"),
        "late_methane_null-initial.ser" => bytes!("late_methane_null-initial.ser"),
        "late_methane_null-after_getter.ser" => bytes!("late_methane_null-after_getter.ser"),
        "late_methane_null-after_event.ser" => bytes!("late_methane_null-after_event.ser"),
        "null_box_graph-initial.ser" => bytes!("null_box_graph-initial.ser"),
        "null_box_graph-after_graph.ser" => bytes!("null_box_graph-after_graph.ser"),
        "compatibility_integer-original.ser" => bytes!("compatibility_integer-original.ser"),
        "compatibility_integer-modified.ser" => bytes!("compatibility_integer-modified.ser"),
        "compatibility_integer-first.ser" => bytes!("compatibility_integer-first.ser"),
        "compatibility_integer-second.ser" => bytes!("compatibility_integer-second.ser"),
        _ => panic!("Unknown frozen history stream {name}"),
    }
}
fn decode(bytes: &[u8]) -> Document {
    let document = StreamReader::new(bytes, Limits::default())
        .unwrap()
        .read_document()
        .unwrap();
    assert_eq!(document.consumed, bytes.len());
    document
}
fn root(document: &Document) -> NodeId {
    let [Record::Value(Value::Handle(id))] = document.records.as_slice() else {
        panic!("One original statistics root required");
    };
    *id
}
fn history<'a>(statistics: &'a WorldStatistics, name: &str) -> Option<&'a HistoryListRef> {
    match name {
        "populationList" => statistics.get_population_list(),
        "deathList" => statistics.get_death_list(),
        "birthList" => statistics.get_birth_list(),
        "oxygenList" => statistics.get_oxygen_list(),
        "carbonDioxideList" => statistics.get_carbon_dioxide_list(),
        "methaneList" => statistics.get_methane_list(),
        _ => panic!("Unknown input history {name}"),
    }
}
fn set_history(statistics: &mut WorldStatistics, name: &str, value: Option<HistoryListRef>) {
    match name {
        "populationList" => statistics.population_list = value,
        "deathList" => statistics.death_list = value,
        "birthList" => statistics.birth_list = value,
        "oxygenList" => statistics.oxygen_list = value,
        "carbonDioxideList" => statistics.carbon_dioxide_list = value,
        "methaneList" => statistics.methane_list = value,
        _ => panic!("Unknown input history {name}"),
    }
}

// The finite recipes contain no genotype records; this composition is not a complete World writer
fn statistics_document(statistics: &WorldStatistics) -> Document {
    let roots = HISTORY_FIELDS.map(|name| history(statistics, name));
    let mut document = export_history_lists(&roots, Limits::default(), BUDGET).unwrap();
    let values: Vec<_> = document
        .records
        .iter()
        .map(|record| {
            let Record::Value(value) = record else {
                unreachable!()
            };
            *value
        })
        .collect();
    let class = class_by_name("biogenesis.WorldStatistics").unwrap();
    let schema = class_schema(class);
    let mut strings = BTreeMap::new();
    let signatures = schema
        .fields
        .iter()
        .map(|field| {
            field.signature.map(|text| {
                *strings.entry(text).or_insert_with(|| {
                    let id = NodeId(document.arena.nodes.len() as u32);
                    document.arena.nodes.push(Node::String(JavaString {
                        units: text.encode_utf16().collect(),
                        long: false,
                    }));
                    id
                })
            })
        })
        .collect();
    let descriptor = NodeId(document.arena.nodes.len() as u32);
    document.arena.nodes.push(Node::Descriptor(ClassDescriptor {
        class,
        signatures,
        annotations: Vec::new(),
        superclass: None,
    }));
    let fields = schema.fields.iter().map(|field| {
        if let Some(index) = HISTORY_FIELDS.iter().position(|name| *name == field.name) {
            return FieldValue::Reference(values[index]);
        }
        match observed_field(statistics, field.name) {
            Observation::Int(value) => FieldValue::Int(value), Observation::Long(value) => FieldValue::Long(value),
            Observation::Double(value) => FieldValue::DoubleBits(value.to_bits()),
            Observation::Genome(None) => FieldValue::Reference(Value::Null),
            _ => panic!("Finite statistics composition does not implement non-null genotype serialization"),
        }
    }).collect();
    let id = NodeId(document.arena.nodes.len() as u32);
    document.arena.nodes.push(Node::Object(JavaObject {
        descriptor,
        slots: vec![ClassData {
            descriptor,
            values: fields,
            custom: Vec::new(),
        }],
    }));
    document.records = vec![Record::Value(Value::Handle(id))];
    document
}
fn encoded(statistics: &WorldStatistics) -> Vec<u8> {
    encode_document(&statistics_document(statistics), Limits::default()).unwrap()
}

#[derive(Clone)]
enum Returned {
    Void,
    List(Option<HistoryListRef>),
    Statistics(Rc<RefCell<WorldStatistics>>),
    Bytes(Vec<u8>),
}
struct Replay<'a> {
    name: String,
    rows: &'a [Vec<&'a str>],
    statistics: Rc<RefCell<WorldStatistics>>,
    retained_statistics: Vec<Rc<RefCell<WorldStatistics>>>,
    input_boxes: BTreeMap<String, JavaDoubleRef>,
    input_lists: BTreeMap<String, HistoryListRef>,
    labels: BTreeMap<String, (u8, usize)>,
    identities: HashMap<(u8, usize), String>,
    retained_boxes: Vec<JavaDoubleRef>,
    retained_lists: Vec<HistoryListRef>,
    random: JavaRandom,
    modified: Vec<u8>,
    first: Vec<u8>,
    second: Vec<u8>,
    error: Option<crate::HistoryError>,
    returned: Option<Returned>,
}
impl<'a> Replay<'a> {
    fn new(rows: &'a [Vec<&'a str>]) -> Self {
        let initial = Self::construct(rows);
        Self {
            name: rows[0][1].into(),
            rows,
            statistics: Rc::new(RefCell::new(initial)),
            retained_statistics: Vec::new(),
            input_boxes: BTreeMap::new(),
            input_lists: BTreeMap::new(),
            labels: BTreeMap::new(),
            identities: HashMap::new(),
            retained_boxes: Vec::new(),
            retained_lists: Vec::new(),
            random: JavaRandom::new(rows[0][3].parse().unwrap()),
            modified: Vec::new(),
            first: Vec::new(),
            second: Vec::new(),
            error: None,
            returned: None,
        }
    }
    fn construct(rows: &[Vec<&str>]) -> WorldStatistics {
        let inputs: BTreeMap<_, _> = rows
            .iter()
            .filter(|row| row[0] == "U")
            .map(|row| (row[1], row[3]))
            .collect();
        let number = |name: &str| f64::from_bits(u64::from_str_radix(inputs[name], 16).unwrap());
        WorldStatistics::new(
            inputs["INITIAL_ORGANISMS"].parse().unwrap(),
            number("INITIAL_O2"),
            number("INITIAL_CO2"),
            number("INITIAL_CH4"),
        )
    }
    fn bind(&mut self, label: &str, kind: u8, identity: usize) {
        assert_ne!(label, "~");
        if let Some(old) = self.labels.insert(label.into(), (kind, identity)) {
            assert_eq!(old, (kind, identity), "{} identity {label}", self.name);
        }
        if let Some(old) = self.identities.insert((kind, identity), label.into()) {
            assert_eq!(old, label, "{} alias {label}", self.name);
        }
    }
    fn bind_list(&mut self, label: &str, value: Option<HistoryListRef>) {
        if label == "~" {
            assert!(value.is_none());
            return;
        }
        let value = value.unwrap();
        self.bind(label, 1, value.identity());
        self.retained_lists.push(value);
    }
    fn bind_box(&mut self, label: &str, value: Option<JavaDoubleRef>, bits: &str) {
        if label == "~" {
            assert!(value.is_none());
            assert_eq!(bits, "~");
            return;
        }
        let value = value.unwrap();
        assert_eq!(value.bits(), u64::from_str_radix(bits, 16).unwrap());
        self.bind(label, 2, value.identity());
        self.retained_boxes.push(value);
    }
    fn input(&mut self, row: &[&str]) {
        match row[0] {
            "IB" => {
                let value = JavaDoubleRef::from_bits(u64::from_str_radix(row[2], 16).unwrap());
                self.input_boxes.insert(row[1].into(), value);
            }
            "IL" => {
                let length: usize = row[3].parse().unwrap();
                let mut values = Vec::with_capacity(100);
                let elements: Vec<_> = self
                    .rows
                    .iter()
                    .filter(|item| item[0] == "IE" && item[1] == row[1])
                    .collect();
                assert_eq!(length, elements.len());
                for (index, item) in elements.into_iter().enumerate() {
                    assert_eq!(item[2].parse::<usize>().unwrap(), index);
                    values.push(if item[3] == "~" {
                        None
                    } else {
                        Some(self.input_boxes[item[3]].clone())
                    });
                }
                self.input_lists
                    .insert(row[1].into(), HistoryListRef::from_elements(values));
            }
            "I" => {
                if row[2] == "List" {
                    let value = if row[3] == "~" {
                        None
                    } else {
                        Some(self.input_lists[row[3]].clone())
                    };
                    set_history(&mut self.statistics.borrow_mut(), row[1], value);
                } else {
                    set_statistics_input(
                        &mut self.statistics.borrow_mut(),
                        row[1],
                        row[2],
                        row[3],
                        &BTreeMap::new(),
                    );
                }
            }
            "IW" => {
                assert_eq!(
                    (&row[1], &row[2], &row[3]),
                    (&"ArrayList.custom.compatibility_int", &"int", &"-7")
                );
                let mut document = statistics_document(&self.statistics.borrow());
                let mut changed = 0;
                for node in &mut document.arena.nodes {
                    if let Node::Object(object) = node {
                        let slot = &mut object.slots[0];
                        if let Some(Record::Block { bytes, .. }) = slot.custom.first_mut() {
                            assert_eq!(bytes.as_slice(), &2_i32.to_be_bytes());
                            *bytes = (-7_i32).to_be_bytes().to_vec();
                            changed += 1;
                        }
                    }
                }
                assert_eq!(changed, 1);
                self.modified = encode_document(&document, Limits::default()).unwrap();
            }
            _ => {}
        }
    }
    fn read(&mut self, bytes: &[u8]) {
        let document = decode(bytes);
        let graph = LegacyGraph::new(&document, Limits::default()).unwrap();
        let mut target = Self::construct(self.rows);
        HistoryStager::new(&graph, BUDGET)
            .unwrap()
            .restore_histories(&mut target, root(&document))
            .unwrap();
        self.retained_statistics.push(Rc::clone(&self.statistics));
        self.statistics = Rc::new(RefCell::new(target));
    }
    fn call(&mut self, row: &[&str]) {
        self.error = None;
        self.returned = None;
        match row[2] {
            "eventTime" => {
                let args = self
                    .rows
                    .iter()
                    .find(|item| item[0] == "KA" && item[1] == row[1])
                    .unwrap();
                let number =
                    |index: usize| f64::from_bits(u64::from_str_radix(args[index], 16).unwrap());
                let result = self.statistics.borrow_mut().event_time(
                    args[2].parse().unwrap(),
                    number(3),
                    number(4),
                    number(5),
                );
                match result {
                    Ok(()) => self.returned = Some(Returned::Void),
                    Err(error) => self.error = Some(error),
                }
            }
            "getMethaneList" => {
                self.returned = Some(Returned::List(
                    self.statistics.borrow().get_methane_list().cloned(),
                ))
            }
            "GraphInfo" => {
                let args = self
                    .rows
                    .iter()
                    .find(|item| item[0] == "KA" && item[1] == row[1])
                    .unwrap();
                assert_eq!(
                    (args[2], args[4], args[5], args[6], args[8]),
                    ("populationList", "0000000000000000", "100", "80", "history")
                );
                let maximum = f64::from_bits(u64::from_str_radix(args[3], 16).unwrap());
                let argb = u32::from_str_radix(args[7], 16).unwrap();
                let result = GraphSeries::from_history(
                    GraphSeriesKind::Population,
                    argb,
                    self.statistics.borrow().get_population_list(),
                    "populationList",
                    maximum,
                );
                self.error = result.err();
                assert!(
                    self.error.is_some(),
                    "Actual null-box constructor must fail"
                );
            }
            "read_first" => {
                let bytes = self.modified.clone();
                self.read(&bytes);
                self.returned = Some(Returned::Statistics(Rc::clone(&self.statistics)));
            }
            "write_first" => {
                self.first = encoded(&self.statistics.borrow());
                self.returned = Some(Returned::Bytes(self.first.clone()));
            }
            "read_second" => {
                let bytes = self.first.clone();
                self.read(&bytes);
                self.returned = Some(Returned::Statistics(Rc::clone(&self.statistics)));
            }
            "write_second" => {
                self.second = encoded(&self.statistics.borrow());
                self.returned = Some(Returned::Bytes(self.second.clone()));
            }
            _ => panic!("Unknown frozen target call {row:?}"),
        }
    }
    fn observe(&mut self, row: &[&str]) {
        let context = format!("History witness {} {}", self.name, row[1]);
        match row[0] {
            "P" => {
                let value = Rc::as_ptr(&self.statistics) as usize;
                self.bind(row[2], 0, value);
            }
            "S" => {
                if row[3] == "List" {
                    let value = history(&self.statistics.borrow(), row[2]).cloned();
                    self.bind_list(row[4], value);
                } else {
                    assert_observation(
                        observed_field(&self.statistics.borrow(), row[2]),
                        row[3],
                        row[4],
                        &BTreeMap::new(),
                        &format!("{context} {}", row[2]),
                    );
                }
            }
            "H" => {
                let value = history(&self.statistics.borrow(), row[2]).cloned();
                self.bind_list(row[3], value);
            }
            "L" => {
                assert_eq!(row[3], "java.util.ArrayList");
                let (_, identity) = self.labels[row[2]];
                let list = self
                    .retained_lists
                    .iter()
                    .find(|list| list.identity() == identity)
                    .unwrap();
                assert_eq!(list.len(), row[4].parse::<usize>().unwrap());
            }
            "D" => {
                let (_, identity) = self.labels[row[2]];
                let list = self
                    .retained_lists
                    .iter()
                    .find(|list| list.identity() == identity)
                    .unwrap();
                let value = list.elements()[row[3].parse::<usize>().unwrap()].clone();
                self.bind_box(row[4], value, row[5]);
            }
            "X" => {
                if row[2] == "none" {
                    assert_eq!(self.error, None, "{context}");
                } else {
                    assert_eq!(row[2], "java.lang.NullPointerException");
                    match (row[3], row[4]) {
                        ("biogenesis.WorldStatistics", "eventTime") => assert_eq!(
                            self.error,
                            Some(crate::HistoryError::MissingList {
                                field: "methaneList"
                            })
                        ),
                        ("biogenesis.GraphInfo", "<init>") => assert_eq!(
                            self.error,
                            Some(crate::HistoryError::MissingElement {
                                field: "populationList",
                                index: 0
                            })
                        ),
                        _ => panic!("Unexpected captured failure phase {row:?}"),
                    }
                }
            }
            "V" => match self.returned.clone().unwrap() {
                Returned::Void => assert_eq!(row[2], "void"),
                Returned::List(value) => {
                    assert_eq!(row[2], "List");
                    self.bind_list(row[3], value);
                }
                Returned::Statistics(value) => {
                    assert_eq!(row[2], "WorldStatistics");
                    self.bind(row[3], 0, Rc::as_ptr(&value) as usize);
                }
                Returned::Bytes(bytes) => {
                    assert_eq!(row[2], "bytes");
                    assert_eq!(bytes.len(), row[3].parse::<usize>().unwrap());
                    let name = format!(
                        "{}-{}.ser",
                        self.name,
                        if row[1] == "write_first" {
                            "first"
                        } else {
                            "second"
                        }
                    );
                    let saved = self
                        .rows
                        .iter()
                        .find(|item| item[0] == "Z" && item[2] == name)
                        .unwrap();
                    assert_eq!(row[4], saved[4]);
                    assert_eq!(bytes, fixture(&name));
                }
            },
            "R" => {
                // This captured envelope is an expectation, never an input to JavaRandom
                const HEADER: &str = "aced0005737200106a6176612e7574696c2e52616e646f6d363296344bf00a530300035a0014686176654e6578744e657874476175737369616e4400106e6578744e657874476175737369616e4a0004736565647870";
                let expected =
                    format!("{HEADER}000000000000000000{:016x}78", self.random.state48());
                assert_eq!(
                    row[3], expected,
                    "{context} complete constructor-scope Random state"
                );
            }
            "RP" => {
                let mut peek = self.random.clone();
                assert_eq!(peek.next_i64(), row[2].parse::<i64>().unwrap());
                assert_eq!(
                    peek.next_bounded(10000).unwrap(),
                    row[3].parse::<i32>().unwrap()
                );
                assert_eq!(
                    peek.next_f64().to_bits(),
                    u64::from_str_radix(row[4], 16).unwrap()
                );
            }
            "Z" => {
                let bytes = encoded(&self.statistics.borrow());
                assert_eq!(
                    bytes,
                    fixture(row[2]),
                    "{context} complete serialized statistics"
                );
                assert_eq!(bytes.len(), row[3].parse::<usize>().unwrap());
            }
            "F" => {
                assert_eq!(self.modified, fixture(row[2]));
                assert_eq!(self.modified.len(), row[3].parse::<usize>().unwrap());
            }
            _ => {}
        }
    }
}
fn replay(text: &str) -> (usize, usize, usize) {
    let rows: Vec<Vec<_>> = text
        .lines()
        .map(|line| line.split('\t').collect())
        .collect();
    let starts: Vec<_> = rows
        .iter()
        .enumerate()
        .filter_map(|(index, row)| (row[0] == "C").then_some(index))
        .collect();
    let (mut calls, mut snapshots) = (0, 0);
    for (case, start) in starts.iter().copied().enumerate() {
        let end = starts.get(case + 1).copied().unwrap_or(rows.len());
        let group = &rows[start..end];
        let mut model = Replay::new(group);
        for row in group {
            match row[0] {
                "IB" | "IL" | "I" | "IW" => model.input(row),
                "K" => {
                    model.call(row);
                    calls += 1;
                }
                "P" => {
                    model.observe(row);
                    snapshots += 1;
                }
                "S" | "H" | "L" | "D" | "X" | "V" | "R" | "RP" | "Z" | "F" => model.observe(row),
                _ => {}
            }
        }
    }
    (starts.len(), calls, snapshots)
}

#[test]
fn actual_java_history_calls_preserve_live_identity_prefixes_and_complete_streams() {
    assert_eq!(ORACLE.lines().count(), 1183);
    for (tag, expected) in [
        ("S", 520),
        ("H", 60),
        ("L", 32),
        ("D", 227),
        ("R", 10),
        ("RP", 10),
        ("Z", 10),
        ("F", 1),
        ("V", 6),
    ] {
        assert_eq!(
            ORACLE
                .lines()
                .filter(|line| line.starts_with(&format!("{tag}\t")))
                .count(),
            expected
        );
    }
    assert_eq!(replay(ORACLE), (4, 8, 10));
}

#[test]
fn history_replay_rejects_a_changed_observation_with_all_inputs_unchanged() {
    let old = "S\tinitial\tpopulationSum\tlong\t17";
    assert!(ORACLE.contains(old));
    let changed = ORACLE.replacen(old, "S\tinitial\tpopulationSum\tlong\t16", 1);
    let inputs = |text: &str| {
        text.lines()
            .filter(|line| {
                matches!(
                    line.split('\t').next(),
                    Some("C" | "U" | "IB" | "IL" | "IE" | "I" | "IW" | "KA")
                )
            })
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    assert_eq!(inputs(ORACLE), inputs(&changed));
    let failure =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| replay(&changed))).unwrap_err();
    let message = failure
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| failure.downcast_ref::<&str>().copied())
        .unwrap();
    assert!(
        message.contains("History witness shared_99_history initial populationSum"),
        "{message}"
    );
}

// Retain the original non-history graph while composing the actual history exporter
fn replace_raw_histories(
    mut document: Document,
    histories: &[Option<HistoryListRef>; 6],
) -> Document {
    let root_id = root(&document);
    let export = export_history_lists(
        &histories.each_ref().map(Option::as_ref),
        Limits::default(),
        BUDGET,
    )
    .unwrap();
    let offset = document.arena.nodes.len() as u32;
    let remap = |id: NodeId| NodeId(id.0.checked_add(offset).unwrap());
    let remap_value = |value| match value {
        Value::Null => Value::Null,
        Value::Handle(id) => Value::Handle(remap(id)),
    };
    let roots: Vec<_> = export
        .records
        .iter()
        .map(|record| {
            let Record::Value(value) = record else {
                unreachable!()
            };
            remap_value(*value)
        })
        .collect();
    for node in export.arena.nodes {
        let node = match node {
            Node::Descriptor(mut descriptor) => {
                assert!(
                    descriptor.signatures.iter().all(Option::is_none)
                        && descriptor.annotations.is_empty()
                );
                descriptor.superclass = descriptor.superclass.map(remap);
                Node::Descriptor(descriptor)
            }
            Node::Object(mut object) => {
                object.descriptor = remap(object.descriptor);
                for slot in &mut object.slots {
                    slot.descriptor = remap(slot.descriptor);
                    for value in &mut slot.values {
                        if let FieldValue::Reference(reference) = value {
                            *reference = remap_value(*reference);
                        }
                    }
                    for record in &mut slot.custom {
                        if let Record::Value(value) = record {
                            *value = remap_value(*value);
                        }
                    }
                }
                Node::Object(object)
            }
            _ => panic!("Unexpected history exporter node"),
        };
        document.arena.nodes.push(node);
    }
    let Node::Object(object) = &mut document.arena.nodes[root_id.0 as usize] else {
        unreachable!()
    };
    assert_eq!(object.slots.len(), 1);
    let schema = class_schema(class_by_name("biogenesis.WorldStatistics").unwrap());
    for (index, field) in schema.fields.iter().enumerate() {
        if let Some(history) = HISTORY_FIELDS.iter().position(|name| *name == field.name) {
            object.slots[0].values[index] = FieldValue::Reference(roots[history]);
        }
    }
    document
}

#[test]
fn actual_history_streams_stage_aliases_and_export_java_normalization() {
    let z: Vec<_> = ORACLE
        .lines()
        .filter(|line| line.starts_with("Z\t"))
        .map(|line| line.split('\t').nth(2).unwrap())
        .collect();
    assert_eq!(z.len(), 10);
    for name in z {
        let bytes = fixture(name);
        let document = decode(bytes);
        let graph = LegacyGraph::new(&document, Limits::default()).unwrap();
        let mut stage = HistoryStager::new(&graph, BUDGET).unwrap();
        let histories = stage.statistics(root(&document)).unwrap();
        assert!(stage.allocated_bytes() < BUDGET);
        let rebuilt = replace_raw_histories(document.clone(), &histories);
        assert_eq!(
            encode_document(&rebuilt, Limits::default()).unwrap(),
            bytes,
            "{name}"
        );
    }
    let bytes = fixture("compatibility_integer-modified.ser");
    let document = decode(bytes);
    assert_eq!(
        encode_document(&document, Limits::default()).unwrap(),
        bytes,
        "Raw codec preserves the actual -7 input"
    );
    let graph = LegacyGraph::new(&document, Limits::default()).unwrap();
    let histories = HistoryStager::new(&graph, BUDGET)
        .unwrap()
        .statistics(root(&document))
        .unwrap();
    assert!(
        histories
            .iter()
            .flatten()
            .all(|value| value.same_instance(histories[0].as_ref().unwrap()))
    );
    let rebuilt = replace_raw_histories(document.clone(), &histories);
    let normalized = encode_document(&rebuilt, Limits::default()).unwrap();
    assert_eq!(normalized, fixture("compatibility_integer-first.ser"));
    assert_eq!(normalized, fixture("compatibility_integer-second.ser"));
    assert_ne!(normalized, bytes);

    let original = include_bytes!("../tests/fixtures/object_stream_codec/statistics.ser");
    let document = decode(original);
    assert_eq!(document.arena.nodes.len(), 633);
    let graph = LegacyGraph::new(&document, Limits::default()).unwrap();
    let histories = HistoryStager::new(&graph, BUDGET)
        .unwrap()
        .statistics(root(&document))
        .unwrap();
    let mut list_ids = std::collections::BTreeSet::new();
    let mut box_ids = std::collections::BTreeSet::new();
    for list in histories.iter().flatten() {
        assert_eq!(list.len(), 100);
        list_ids.insert(list.identity());
        for value in list.elements().iter().flatten() {
            box_ids.insert(value.identity());
        }
    }
    assert_eq!((list_ids.len(), box_ids.len()), (6, 600));
    let rebuilt = replace_raw_histories(document.clone(), &histories);
    assert_eq!(
        encode_document(&rebuilt, Limits::default()).unwrap(),
        original.as_slice()
    );

    // A decoded raw payload remains noncanonical until the existing Java writer boundary
    let mut controlled = decode(fixture("shared_99_history-initial.ser"));
    let double_ids = {
        let graph = LegacyGraph::new(&controlled, Limits::default()).unwrap();
        let Value::Handle(list) = graph
            .reference(
                root(&controlled),
                "biogenesis.WorldStatistics",
                "populationList",
            )
            .unwrap()
        else {
            unreachable!()
        };
        let slots = graph.slots(list).unwrap();
        let ids: Vec<_> = slots[0]
            .custom
            .iter()
            .skip(1)
            .take(3)
            .map(|record| {
                let Record::Value(Value::Handle(id)) = record else {
                    panic!("Controlled first three Double references required");
                };
                graph.expect_class(*id, "java.lang.Double").unwrap();
                *id
            })
            .collect();
        assert_eq!(ids[0], ids[1]);
        assert_ne!(ids[0], ids[2]);
        [ids[0], ids[2]]
    };
    const PAYLOAD: u64 = 0x7ff8000000000042;
    for id in double_ids {
        let Node::Object(object) = &mut controlled.arena.nodes[id.0 as usize] else {
            unreachable!()
        };
        let value = &mut object.slots.last_mut().unwrap().values[0];
        assert!(matches!(value, FieldValue::DoubleBits(_)));
        *value = FieldValue::DoubleBits(PAYLOAD);
    }
    let check_boxes = |list: &HistoryListRef, expected: u64| {
        let values = list.elements();
        let first = values[0].as_ref().unwrap();
        let repeated = values[1].as_ref().unwrap();
        let distinct = values[2].as_ref().unwrap();
        assert!(first.same_instance(repeated));
        assert!(!first.same_instance(distinct));
        assert_eq!(
            (first.bits(), repeated.bits(), distinct.bits()),
            (expected, expected, expected)
        );
    };
    let graph = LegacyGraph::new(&controlled, Limits::default()).unwrap();
    let histories = HistoryStager::new(&graph, BUDGET)
        .unwrap()
        .statistics(root(&controlled))
        .unwrap();
    let first = histories[0].as_ref().unwrap();
    check_boxes(first, PAYLOAD);
    assert!(
        histories
            .iter()
            .flatten()
            .all(|list| list.same_instance(first))
    );
    let exported = export_history_lists(
        &histories.each_ref().map(Option::as_ref),
        Limits::default(),
        BUDGET,
    )
    .unwrap();
    let check_export = |document: &Document, expected: u64| {
        let graph = LegacyGraph::new(document, Limits::default()).unwrap();
        let mut stager = HistoryStager::new(&graph, BUDGET).unwrap();
        let lists: Vec<_> = document
            .records
            .iter()
            .map(|record| {
                let Record::Value(value) = record else {
                    unreachable!()
                };
                stager.list(*value).unwrap().unwrap()
            })
            .collect();
        assert_eq!(lists.len(), 6);
        assert!(lists.iter().all(|list| list.same_instance(&lists[0])));
        check_boxes(&lists[0], expected);
    };
    check_export(&exported, PAYLOAD);
    let written = encode_document(&exported, Limits::default()).unwrap();
    check_export(&decode(&written), 0x7ff8000000000000);
}

#[test]
fn history_world_copy_detaches_lists_and_keeps_immutable_box_aliases() {
    let boxed = JavaDoubleRef::from_bits(0x7ff8000000000042);
    let list = HistoryListRef::from_elements(vec![Some(boxed.clone()), None, Some(boxed.clone())]);
    let mut source = WorldStatistics::new(0, 0.0, 0.0, 0.0);
    for field in HISTORY_FIELDS {
        set_history(&mut source, field, Some(list.clone()));
    }
    let mut copy = source.copy_for_world(&mut crate::organism::GenomeCloner::default());
    let copied = copy.get_population_list().unwrap().clone();
    assert!(!copied.same_instance(&list));
    for field in HISTORY_FIELDS {
        assert!(history(&copy, field).unwrap().same_instance(&copied));
    }
    assert!(copied.elements()[0].as_ref().unwrap().same_instance(&boxed));
    assert!(
        copied.elements()[0]
            .as_ref()
            .unwrap()
            .same_instance(copied.elements()[2].as_ref().unwrap())
    );
    copy.event_time(7, 17.0, 19.0, 23.0).unwrap();
    assert_eq!((list.len(), copied.len()), (3, 9));
    assert!(list.elements()[1].is_none());
    assert_eq!(boxed.bits(), 0x7ff8000000000042);
}

#[test]
fn graph_prefix_ignores_null_tail_and_history_failures_precede_color_and_alive_scans() {
    let world = World::new(WorldConfig {
        initial_attempts: 0,
        ..WorldConfig::default()
    })
    .unwrap();
    let mut values = vec![Some(JavaDoubleRef::from_value(1.0)); 100];
    values.push(None);
    world.statistics_ref().borrow_mut().death_list = Some(HistoryListRef::from_elements(values));
    let view = world.refresh_statistics(&world.open_statistics()).unwrap();
    assert_eq!(view.plot_series[0].points.len(), 100);
    assert_eq!(view.histories.death_list.len(), 101);
    assert_eq!(view.histories.death_list[100], crate::HistorySlotSnapshot::Null);
    let gene = crate::Gene::new(12.0, 0.0, Some(0xff00ff00), 0, [0; 36]).shared();
    let code = crate::Genome::from_parts(
        &[gene],
        crate::GenomeProperties {
            symmetry: 1,
            ..crate::GenomeProperties::default()
        },
    );
    let mut actor =
        Organism::unplaced(Rc::new(RefCell::new(code)), crate::OrganismState::default()).unwrap();
    let id = actor.id;
    actor.genome = None;
    world.organisms.borrow_mut().push(Rc::new(RefCell::new(actor)));
    let statistics_owner = world.statistics_ref();
    let mut statistics = statistics_owner.borrow_mut();
    statistics.population_list = None;
    set_statistics_input(
        &mut statistics,
        "aliveBeingMostChildrenNumber",
        "int",
        "99",
        &BTreeMap::new(),
    );
    drop(statistics);
    assert_eq!(
        world.refresh_statistics(&world.open_statistics()),
        Err(StatisticsError::History(crate::HistoryError::MissingList {
            field: "populationList"
        }))
    );
    world.statistics_ref().borrow_mut().population_list = Some(HistoryListRef::empty());
    assert_eq!(
        world.refresh_statistics(&world.open_statistics()),
        Err(StatisticsError::MissingGenome { organism_id: id })
    );
    assert_observation(
        observed_field(&world.statistics_ref().borrow(), "aliveBeingMostChildrenNumber"),
        "int",
        "99",
        &BTreeMap::new(),
        "Alive scan was not reached",
    );
}

#[test]
fn world_frame_history_failure_keeps_frame_gas_and_statistics_prefixes() {
    let world = World::new(WorldConfig {
        initial_attempts: 0,
        ..WorldConfig::default()
    })
    .unwrap();
    world.resume();
    world.frame.set(255);
    let gases_before = (world.carbon_dioxide.get(), world.methane.get());
    {
        let statistics_owner = world.statistics_ref();
        let mut statistics = statistics_owner.borrow_mut();
        statistics.methane_list = None;
        set_statistics_input(
            &mut statistics,
            "deathLastTime",
            "int",
            "11",
            &BTreeMap::new(),
        );
        set_statistics_input(
            &mut statistics,
            "birthLastTime",
            "int",
            "13",
            &BTreeMap::new(),
        );
    }
    assert_eq!(
        world.step(),
        Err(WorldError::History(crate::HistoryError::MissingList {
            field: "methaneList"
        }))
    );
    assert_eq!((world.tick.get(), world.frame.get()), (1, 0));
    assert!(world.paused.get());
    assert_ne!((world.carbon_dioxide.get(), world.methane.get()), gases_before);
    assert_eq!(
        crate::statistics::tests::samples(world.statistics_ref().borrow().get_carbon_dioxide_list()),
        [world.carbon_dioxide.get()]
    );
    let statistics_owner = world.statistics_ref();
    let statistics = statistics_owner.borrow();
    assert_eq!(statistics.get_time(), 1);
    assert_eq!(
        crate::statistics::tests::samples(statistics.get_death_list()),
        [11.0]
    );
    assert_eq!(
        crate::statistics::tests::samples(statistics.get_birth_list()),
        [13.0]
    );
    assert_observation(
        observed_field(&statistics, "deathLastTime"),
        "int",
        "11",
        &BTreeMap::new(),
        "Late failure retains pending deaths",
    );
    assert_observation(
        observed_field(&statistics, "birthLastTime"),
        "int",
        "13",
        &BTreeMap::new(),
        "Late failure retains pending births",
    );
}

#[test]
fn history_staging_budgets_and_unsupported_elements_preserve_active_owners() {
    let bytes = fixture("late_methane_null-initial.ser");
    let mut document = decode(bytes);
    let id = root(&document);
    let graph = LegacyGraph::new(&document, Limits::default()).unwrap();
    assert!(
        matches!(HistoryStager::new(&graph, 0), Err(error) if error.kind == GraphErrorKind::Budget)
    );
    let Value::Handle(death) = graph
        .reference(id, "biogenesis.WorldStatistics", "deathList")
        .unwrap()
    else {
        unreachable!()
    };
    drop(graph);
    let Node::Object(list) = &mut document.arena.nodes[death.0 as usize] else {
        unreachable!()
    };
    list.slots[0].custom[1] = Record::Value(Value::Handle(id));
    let graph = LegacyGraph::new(&document, Limits::default()).unwrap();
    let mut active = WorldStatistics::new(0, 0.0, 0.0, 0.0);
    let before = active.get_population_list().unwrap().clone();
    let prior = HISTORY_FIELDS.map(|name| history(&active, name).cloned().unwrap());
    let error = HistoryStager::new(&graph, BUDGET)
        .unwrap()
        .restore_histories(&mut active, id)
        .unwrap_err();
    assert_eq!(error.kind, GraphErrorKind::Class);
    assert!(active.get_population_list().unwrap().same_instance(&before));
    for (name, owner) in HISTORY_FIELDS.iter().zip(&prior) {
        assert!(history(&active, name).unwrap().same_instance(owner));
    }
    assert_eq!(active.get_time(), 0);
    assert_eq!(active.get_population_list().unwrap().len(), 0);
    assert!(active.get_methane_list().is_some());
    let roots = HISTORY_FIELDS.map(|name| history(&active, name));
    assert!(
        matches!(export_history_lists(&roots, Limits::default(), 0), Err(error) if error.kind == GraphErrorKind::Budget)
    );
    let constrained = Limits {
        work: 0,
        ..Limits::default()
    };
    assert!(
        matches!(export_history_lists(&roots, constrained, BUDGET), Err(error) if error.kind == GraphErrorKind::Budget)
    );
    assert!(active.get_population_list().unwrap().same_instance(&before));
}
