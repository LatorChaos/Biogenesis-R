// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use crate::legacy_graph::{GraphErrorKind, LegacyGraph};
use crate::legacy_spatial::{DecodedSpatial, decode_world_spatial};
use crate::object_stream::{
    Document, FieldValue, Limits, Node, NodeId, Record, StreamReader, Value, class_schema,
};
use std::collections::{BTreeMap, HashMap, HashSet};

#[path = "legacy_jts_streams.rs"]
mod inputs;

const NATIVE: &str = include_str!("../tests/fixtures/legacy_jts/jts.tsv");

fn document(name: &str) -> Document {
    let bytes = inputs::STREAMS
        .iter()
        .find(|(label, _)| *label == name)
        .unwrap()
        .1;
    StreamReader::new(bytes, Limits::default())
        .unwrap()
        .read_document()
        .unwrap()
}

struct Case {
    name: &'static str,
    rows: Vec<Vec<&'static str>>,
}

fn cases() -> Vec<Case> {
    let mut result: Vec<Case> = Vec::new();
    let mut counts = BTreeMap::new();
    for line in NATIVE.lines() {
        let row: Vec<_> = line.split('\t').collect();
        *counts.entry(row[0]).or_insert(0_usize) += 1;
        let length = match row[0] {
            "C" | "U" | "A" | "W" | "G" | "N" | "F" => 5,
            "I" | "K" | "R" | "B" => 4,
            "P" | "E" => 2,
            "L" => 6,
            "H" => 12,
            "Q" => 3,
            "T" => match row[2] {
                "tree" | "envelope" => 8,
                "node" => 7,
                "item" | "list" => 6,
                _ => panic!("Unknown tree kind"),
            },
            other => panic!("Unknown source row {other}"),
        };
        assert_eq!(row.len(), length);
        if row[0] == "C" {
            result.push(Case {
                name: row[1],
                rows: Vec::new(),
            });
        }
        result.last_mut().unwrap().rows.push(row);
    }
    assert_eq!(
        counts,
        BTreeMap::from([
            ("C", 5),
            ("U", 1490),
            ("I", 98),
            ("A", 35),
            ("K", 205),
            ("P", 23),
            ("F", 23),
            ("R", 56),
            ("B", 23),
            ("W", 276),
            ("G", 1242),
            ("N", 1840),
            ("H", 176),
            ("L", 69),
            ("Q", 11),
            ("E", 5),
            ("T", 551),
        ])
    );
    assert_eq!(result.len(), 5);
    assert_eq!(inputs::STREAMS.len(), 23);
    result
}

#[derive(Clone, Debug)]
enum Reference {
    Tree(crate::shared_world_owners::TreeRef),
    Boundable(crate::shared_world_owners::BoundableRef),
    Envelope(crate::shared_world_owners::EnvelopeRef),
    List(crate::shared_world_owners::SharedListRef),
}
impl Reference {
    fn key(&self) -> (u8,usize) {
        match self {
            Self::Tree(value)=>(b'T',value.identity()),
            Self::Boundable(value)=>(b'B',value.identity()),
            Self::Envelope(value)=>(b'E',value.identity()),
            Self::List(value)=>(b'L',value.identity()),
        }
    }
}
impl PartialEq for Reference { fn eq(&self, other: &Self) -> bool { self.key()==other.key() } }
impl Eq for Reference {}
impl std::hash::Hash for Reference {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) { std::hash::Hash::hash(&self.key(),state); }
}

#[derive(Default)]
struct References {
    // Observed owners stay alive for the case, as in the Source IdentityHashMap
    source: HashMap<String, Reference>,
    actual: HashMap<Reference, String>,
    roles: HashMap<String, usize>,
    tokens: HashMap<usize, String>,
}

impl References {
    fn bind(&mut self, source: &str, actual: Reference) {
        if let Some(previous) = self.source.insert(source.to_owned(), actual.clone()) {
            assert_eq!(previous, actual, "Shared {source}");
        }
        if let Some(previous) = self.actual.insert(actual, source.to_owned()) {
            assert_eq!(previous, source, "Distinct {source}");
        }
    }

    fn actor(&mut self, source: &str, token: usize) {
        if let Some(previous) = self.roles.insert(source.to_owned(), token) {
            assert_eq!(previous, token, "Actor {source}");
        }
        if let Some(previous) = self.tokens.insert(token, source.to_owned()) {
            assert_eq!(previous, source, "Distinct actor {source}");
        }
    }

    fn tree(
        &mut self,
        spatial: &SpatialTree,
        rows: &HashMap<&str, &Vec<&str>>,
        label: &str,
        actual: Reference,
        visited: &mut HashSet<String>,
    ) {
        self.bind(label, actual.clone());
        if !visited.insert(label.to_owned()) { return; }
        let row = rows[label];
        match (row[2], actual) {
            ("tree", Reference::Tree(owner)) => {
                let state=owner.value();
                assert_eq!(row[4], state.built.to_string(), "Stored built flag");
                assert_eq!(row[5], state.node_capacity.to_string());
                if let Some(pending) = &state.pending {
                    self.list(spatial, rows, row[6], pending, visited);
                } else { assert_eq!(row[6], "~"); }
                if let Some(root) = &state.root {
                    self.tree(spatial, rows, row[7], Reference::Boundable(root.clone()), visited);
                } else { assert_eq!(row[7], "~"); }
            }
            ("node", Reference::Boundable(owner)) => {
                let state=owner.value();
                let crate::shared_world_owners::BoundableValue::Node { level,bounds,children }=&*state else { panic!("Expected node"); };
                assert_eq!(row[4], level.to_string());
                if let Some(envelope)=bounds {
                    self.tree(spatial,rows,row[5],Reference::Envelope(envelope.clone()),visited);
                } else { assert_eq!(row[5],"~"); }
                self.list(spatial,rows,row[6],children.as_ref().unwrap(),visited);
            }
            ("item", Reference::Boundable(owner)) => {
                let state=owner.value();
                let crate::shared_world_owners::BoundableValue::Item { bounds,item }=&*state else { panic!("Expected item"); };
                self.actor(row[5],spatial.snapshot_token(item));
                self.tree(spatial,rows,row[4],Reference::Envelope(bounds.as_ref().unwrap().clone()),visited);
            }
            ("envelope", Reference::Envelope(owner)) => {
                let value=owner.get();
                for (actual, expected) in [value.min_x, value.max_x, value.min_y, value.max_y]
                    .into_iter().zip(&row[4..])
                {
                    assert_eq!(actual.to_bits(),u64::from_str_radix(expected,16).unwrap(),"Envelope {label}");
                }
            }
            _ => panic!("Tree reference kind differs for {label}"),
        }
    }

    fn list(
        &mut self,
        spatial: &SpatialTree,
        rows: &HashMap<&str, &Vec<&str>>,
        label: &str,
        actual: &crate::shared_world_owners::SharedListRef,
        visited: &mut HashSet<String>,
    ) {
        self.bind(label,Reference::List(actual.clone()));
        if !visited.insert(label.to_owned()) { return; }
        let row=rows[label];
        assert_eq!(row[2],"list");
        let values=actual.slots();
        assert_eq!(row[4].parse::<usize>().unwrap(),values.len());
        let labels:Vec<_>=if row[5]=="-" { Vec::new() } else { row[5].split(',').collect() };
        assert_eq!(labels.len(),values.len());
        for (label,value) in labels.into_iter().zip(values.iter()) {
            let crate::shared_world_owners::SharedSlot::Boundable(value)=value else { panic!("Expected boundable list element"); };
            self.tree(spatial,rows,label,Reference::Boundable(value.clone()),visited);
        }
    }

    fn snapshot(&mut self, loaded: &DecodedSpatial, case: &Case, stage: &str) -> usize {
        let members = case
            .rows
            .iter()
            .find(|row| row[0] == "L" && row[1] == stage && row[2] == "members")
            .unwrap();
        let labels: Vec<_> = if members[5] == "-" {
            Vec::new()
        } else {
            members[5].split(',').collect()
        };
        assert_eq!(loaded.members.len(), members[4].parse::<usize>().unwrap());
        assert_eq!(labels.len(), loaded.members.len());
        for (role, token) in labels.into_iter().zip(&loaded.members) {
            self.actor(role, *token);
        }
        let rows: HashMap<_, _> = case
            .rows
            .iter()
            .filter(|row| row[0] == "T" && row[1] == stage)
            .map(|row| (row[3], row))
            .collect();
        let root = rows.values().find(|row| row[2] == "tree").unwrap()[3];
        let mut visited = HashSet::new();
        self.tree(
            &loaded.tree,
            &rows,
            root,
            Reference::Tree(loaded.tree.shared_ref().clone()),
            &mut visited,
        );
        assert_eq!(visited.len(), rows.len());
        for actor in case
            .rows
            .iter()
            .filter(|row| row[0] == "H" && row[1] == stage)
        {
            let token = self.roles[actor[2]];
            assert_eq!(
                loaded.members.contains(&token),
                actor[7] == "true",
                "Membership {}",
                actor[2]
            );
        }
        assert_eq!(self.roles.len(), loaded.actors.nodes.len());
        rows.len()
    }
}

#[test]
fn all23_source_stored_trees_load_without_building_or_recalculating_caches() {
    let mut streams = 0;
    let mut observations = 0;
    for case in cases() {
        for file in case.rows.iter().filter(|row| row[0] == "F") {
            let doc = document(file[2]);
            let graph = LegacyGraph::new(&doc, Limits::default()).unwrap();
            let loaded = decode_world_spatial(&graph).unwrap();
            observations += References::default().snapshot(&loaded, &case, file[1]);
            streams += 1;
        }
    }
    assert_eq!((streams, observations), (23, 551));
}

fn rectangle(graph: &LegacyGraph<'_>, actor: NodeId) -> IntRect {
    IntRect {
        x: graph.int(actor, "java.awt.Rectangle", "x").unwrap(),
        y: graph.int(actor, "java.awt.Rectangle", "y").unwrap(),
        width: graph.int(actor, "java.awt.Rectangle", "width").unwrap(),
        height: graph.int(actor, "java.awt.Rectangle", "height").unwrap(),
    }
}

fn query(argument: &str) -> IntRect {
    let values: Vec<i32> = argument
        .split(',')
        .map(|value| value.parse().unwrap())
        .collect();
    assert_eq!(values.len(), 4);
    IntRect {
        x: values[0],
        y: values[2],
        width: values[1].checked_sub(values[0]).unwrap(),
        height: values[3].checked_sub(values[2]).unwrap(),
    }
}

#[test]
fn source_query_and_build_continuations_preserve_order_aliases_and_lazy_state() {
    let mut operations = 0;
    let mut snapshots = 0;
    let mut queries = 0;
    let mut observations = 0;
    for case in cases() {
        let first = case.rows.iter().find(|row| row[0] == "F").unwrap();
        let doc = document(first[2]);
        let graph = LegacyGraph::new(&doc, Limits::default()).unwrap();
        let mut loaded = decode_world_spatial(&graph).unwrap();
        let mut references = References::default();
        let mut bounds: Vec<_> = loaded
            .actors
            .nodes
            .iter()
            .map(|id| rectangle(&graph, *id))
            .collect();
        for (supplied, token) in case
            .rows
            .iter()
            .filter(|row| row[0] == "A")
            .zip(loaded.members.clone())
        {
            references.actor(supplied[1], token);
        }
        let mut started = false;
        let mut last_query = Vec::new();
        for row in &case.rows {
            if row[0] == "P" {
                started = true;
                observations += references.snapshot(&loaded, &case, row[1]);
                snapshots += 1;
                continue;
            }
            if !started {
                continue;
            }
            if row[0] == "K" {
                match row[2] {
                    "STRtree.insert" => {
                        let token = references.roles[row[3]];
                        let tree=loaded.tree.shared_ref().value();
                        assert!(!tree.built);
                        let pending=tree.pending.as_ref().unwrap().clone();
                        drop(tree);
                        pending.push_boundable("itemBoundables",crate::shared_world_owners::BoundableRef::new(crate::shared_world_owners::BoundableValue::Item {
                            bounds:Some(crate::shared_world_owners::EnvelopeRef::new(Envelope::from_rect(bounds[token]))),
                            item:crate::shared_world_owners::SharedSlot::Double(crate::history::JavaDoubleRef::from_bits(token as u64)),
                        })).unwrap();
                        operations += 1;
                    }
                    "STRtree.build" => {
                        loaded.tree.build_index().unwrap();
                        operations += 1;
                    }
                    "STRtree.query" => {
                        last_query = loaded.tree.query(query(row[3]));
                        operations += 1;
                    }
                    "ObjectOutputStream.writeObject" => {}
                    other => panic!("Unexpected continuation {other}"),
                }
            } else if row[0] == "Q" {
                let roles: Vec<_> = last_query
                    .iter()
                    .map(|token| references.tokens[token].as_str())
                    .collect();
                assert_eq!(
                    if roles.is_empty() {
                        "-".to_owned()
                    } else {
                        roles.join(",")
                    },
                    row[2],
                    "{} {}",
                    case.name,
                    row[1]
                );
                queries += 1;
            } else if row[0] == "I" {
                match row[2] {
                    "membership.remove_exact_identity" => {
                        let token = references.roles[row[3]];
                        loaded.members.retain(|value| *value != token);
                    }
                    "actor.Rectangle.setBounds" => {
                        let values: Vec<_> = row[3].split(',').collect();
                        let token = references.roles[values[0]];
                        bounds[token] = IntRect {
                            x: values[1].parse().unwrap(),
                            y: values[2].parse().unwrap(),
                            width: values[3].parse().unwrap(),
                            height: values[4].parse().unwrap(),
                        };
                    }
                    "g0._cladeID" | "j0._length" => {}
                    other => panic!("Unexpected source input {other}"),
                }
            }
        }
    }
    assert_eq!(
        (operations, snapshots, queries, observations),
        (18, 23, 11, 551)
    );
}

#[test]
fn clone_query_keeps_original_cold_and_tree_only_actor_tokens_remain_distinct() {
    let doc = document("multilevel-m1.bgw");
    let graph = LegacyGraph::new(&doc, Limits::default()).unwrap();
    let loaded = decode_world_spatial(&graph).unwrap();
    let cloned = loaded.tree.clone();
    let original = loaded.tree.stored_state().clone();
    assert!(cloned.query(query("-100,-90,-100,-90")).is_empty());
    assert_eq!(loaded.tree.stored_state(), original);
    assert_ne!(cloned.stored_state(), original);
    let doc = document("retained-r3.bgw");
    let graph = LegacyGraph::new(&doc, Limits::default()).unwrap();
    let loaded = decode_world_spatial(&graph).unwrap();
    assert_eq!((loaded.members.len(), loaded.actors.nodes.len()), (3, 4));
    let hits = loaded.tree.query(query("3990,4010,990,1010"));
    assert_eq!(hits.len(), 2);
    assert_eq!(hits[0], hits[1]);
    assert!(!loaded.members.contains(&hits[0]));
}

fn replace_field(document: &mut Document, id: NodeId, owner: &str, name: &str, value: FieldValue) {
    let Node::Object(object) = &document.arena.nodes[id.0 as usize] else {
        panic!("Object expected")
    };
    let mut position = None;
    for (index, slot) in object.slots.iter().enumerate() {
        let Node::Descriptor(descriptor) = &document.arena.nodes[slot.descriptor.0 as usize] else {
            panic!("Descriptor expected")
        };
        let schema = class_schema(descriptor.class);
        if schema.name == owner {
            position = Some((
                index,
                schema
                    .fields
                    .iter()
                    .position(|field| field.name == name)
                    .unwrap(),
            ));
        }
    }
    let (slot, field) = position.unwrap();
    let Node::Object(object) = &mut document.arena.nodes[id.0 as usize] else {
        unreachable!()
    };
    object.slots[slot].values[field] = value;
}

#[test]
fn duplicate_legacy_ids_do_not_merge_actual_object_nodes_and_foreign_world_is_rejected() {
    let mut doc = document("pending-p1.bgw");
    let actors = {
        let graph = LegacyGraph::new(&doc, Limits::default()).unwrap();
        decode_world_spatial(&graph).unwrap().actors.nodes
    };
    for actor in &actors {
        replace_field(
            &mut doc,
            *actor,
            "biogenesis.Organism",
            "_ID",
            FieldValue::Int(-1),
        );
    }
    {
        let graph = LegacyGraph::new(&doc, Limits::default()).unwrap();
        let loaded = decode_world_spatial(&graph).unwrap();
        assert_eq!(loaded.actors.nodes, actors);
        assert_eq!(loaded.actors.nodes.len(), 3);
        let hits = loaded.tree.query(query("990,1010,990,1010"));
        assert_eq!(hits, [0, 0]);
    }
    replace_field(
        &mut doc,
        actors[0],
        "biogenesis.Organism",
        "_world",
        FieldValue::Reference(Value::Handle(actors[1])),
    );
    let graph = LegacyGraph::new(&doc, Limits::default()).unwrap();
    let error = match decode_world_spatial(&graph) {
        Ok(_) => panic!("Foreign World accepted"),
        Err(error) => error,
    };
    assert_eq!(error.kind, GraphErrorKind::Reference);
    assert_eq!(error.field, Some("_world"));
}

#[test]
fn cyclic_child_lists_and_shared_mutable_child_lists_fail_without_publishing_a_tree() {
    let mut doc = document("default-packed-d1.bgw");
    let (root, children, list) = {
        let graph = LegacyGraph::new(&doc, Limits::default()).unwrap();
        let world = graph.world_root().unwrap();
        let tree = graph
            .required_reference(world, "biogenesis.World", "colDetTree")
            .unwrap();
        let root = graph
            .required_reference(
                tree,
                "org.locationtech.jts.index.strtree.AbstractSTRtree",
                "root",
            )
            .unwrap();
        let list = graph
            .required_reference(
                root,
                "org.locationtech.jts.index.strtree.AbstractNode",
                "childBoundables",
            )
            .unwrap();
        (root, graph.array_list(list).unwrap(), list)
    };
    let first = match children[0] {
        Value::Handle(value) => value,
        _ => panic!("Child node required"),
    };
    let original = doc.clone();
    replace_field(
        &mut doc,
        first,
        "org.locationtech.jts.index.strtree.AbstractNode",
        "childBoundables",
        FieldValue::Reference(Value::Handle(list)),
    );
    {
        let graph = LegacyGraph::new(&doc, Limits::default()).unwrap();
        let error = match decode_world_spatial(&graph) {
            Ok(_) => panic!("Shared mutable tree list accepted"),
            Err(error) => error,
        };
        assert_eq!(error.kind, GraphErrorKind::AliasedTreeList);
    }
    doc = original;
    let node = &mut doc.arena.nodes[list.0 as usize];
    let Node::Object(node) = node else {
        panic!("List required")
    };
    node.slots[0].custom[1] = Record::Value(Value::Handle(root));
    let graph = LegacyGraph::new(&doc, Limits::default()).unwrap();
    let error = match decode_world_spatial(&graph) {
        Ok(_) => panic!("Cyclic tree accepted"),
        Err(error) => error,
    };
    assert_eq!(error.kind, GraphErrorKind::TreeShape);
    assert_eq!(error.node, Some(root));
}

#[test]
#[ignore = "Run the frozen external dense codec/spatial budget gate"]
fn required_dense_default_codec_and_spatial_budget() {
    let path = std::env::var_os("BGR_DENSE_WORLD_INPUT").expect("Required frozen dense input path");
    let bytes = std::fs::read(path).expect("Owned frozen dense input");
    assert_eq!(bytes.len(), 35_915_346);
    let document = StreamReader::new(&bytes, Limits::default())
        .unwrap()
        .read_document()
        .unwrap();
    assert_eq!(document.consumed, bytes.len());
    assert_eq!(document.arena.nodes.len(), 773_700);
    let graph = LegacyGraph::new(&document, Limits::default()).unwrap();
    let loaded = decode_world_spatial(&graph).unwrap();
    assert_eq!(loaded.members.len(), 13_190);
    assert_eq!(loaded.actors.nodes.len(), 13_192);
    {
        let state = loaded.tree.stored_state();
        assert!(state.built);
        assert_eq!(state.node_capacity, 10);
        assert_eq!(
            state
                .boundables
                .iter()
                .filter(|value| matches!(value, StoredBoundable::Item { .. }))
                .count(),
            13_191
        );
        assert_eq!(
            state
                .boundables
                .iter()
                .filter(|value| matches!(value, StoredBoundable::Node { .. }))
                .count(),
            1_492
        );
        assert_eq!(state.envelopes.len(), 14_683);
    }
    let output = crate::object_stream::encode_document(&document, Limits::default()).unwrap();
    assert!(
        output == bytes,
        "Dense source stream byte round trip differs"
    );
}
