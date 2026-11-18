// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use std::fmt::Write;

const GENE: &[u8] = include_bytes!("../tests/fixtures/object_stream_codec/gene.ser");
const GENOME: &[u8] = include_bytes!("../tests/fixtures/object_stream_codec/genome.ser");
const STATISTICS: &[u8] = include_bytes!("../tests/fixtures/object_stream_codec/statistics.ser");
const WORLD: &[u8] = include_bytes!("../tests/fixtures/object_stream_codec/world.ser");

fn decode(bytes: &[u8]) -> Document {
    StreamReader::new(bytes, Limits::default()).unwrap().read_document().unwrap()
}

fn hex(bytes: &[u8]) -> String {
    if bytes.is_empty() { return "-".to_owned(); }
    bytes.iter().map(|value| format!("{value:02x}")).collect()
}

fn reference(value: Value) -> String {
    match value { Value::Null => "~".to_owned(), Value::Handle(id) => id.0.to_string() }
}

fn primitive(value: &FieldValue) -> String {
    match value {
        FieldValue::Boolean(value) | FieldValue::Byte(value) => hex(&[*value]),
        FieldValue::Char(value) => hex(&value.to_be_bytes()),
        FieldValue::Short(value) => hex(&value.to_be_bytes()),
        FieldValue::Int(value) => hex(&value.to_be_bytes()),
        FieldValue::Long(value) => hex(&value.to_be_bytes()),
        FieldValue::FloatBits(value) => hex(&value.to_be_bytes()),
        FieldValue::DoubleBits(value) => hex(&value.to_be_bytes()),
        FieldValue::Reference(value) => reference(*value),
    }
}

fn record_row(output: &mut String, prefix: &str, record: &Record) {
    match record {
        Record::Value(value) => writeln!(output, "{prefix}\tREF\t{}", reference(*value)).unwrap(),
        Record::Block { bytes, long } => writeln!(output, "{prefix}\tBLOCK\t{}\t{}", u8::from(*long), hex(bytes)).unwrap(),
    }
}

fn graph_rows(document: &Document) -> String {
    let mut output = String::new();
    for (id, node) in document.arena.nodes.iter().enumerate() {
        match node {
            Node::Descriptor(descriptor) => {
                let schema = class_schema(descriptor.class);
                let parent = descriptor.superclass.map_or(Value::Null, Value::Handle);
                writeln!(output, "DESC\t{id}\t{}\t{}\t{}\t{}", schema.name, schema.uid, schema.flags, reference(parent)).unwrap();
                for (index, (field, signature)) in schema.fields.iter().zip(&descriptor.signatures).enumerate() {
                    writeln!(output, "FIELD\t{id}\t{index}\t{}\t{}\t{}", char::from(field.kind), field.name, reference(signature.map_or(Value::Null, Value::Handle))).unwrap();
                }
                for (index, item) in descriptor.annotations.iter().enumerate() {
                    record_row(&mut output, &format!("ANNOTATION\t{id}\t{index}"), item);
                }
            }
            Node::String(value) => {
                let units: String = value.units.iter().map(|unit| format!("{unit:04x}")).collect();
                writeln!(output, "STRING\t{id}\t{}\t{}", u8::from(value.long), if units.is_empty() { "-" } else { &units }).unwrap();
            }
            Node::Object(object) => {
                writeln!(output, "OBJECT\t{id}\t{}", object.descriptor.0).unwrap();
                for (index, slot) in object.slots.iter().enumerate() {
                    writeln!(output, "SLOT\t{id}\t{index}\t{}", slot.descriptor.0).unwrap();
                    let fields = class_schema(document.arena.descriptor(slot.descriptor).unwrap().class).fields;
                    for (field_index, (field, value)) in fields.iter().zip(&slot.values).enumerate() {
                        writeln!(output, "VALUE\t{id}\t{index}\t{field_index}\t{}\t{}", char::from(field.kind), primitive(value)).unwrap();
                    }
                    for (item_index, item) in slot.custom.iter().enumerate() {
                        record_row(&mut output, &format!("CUSTOM\t{id}\t{index}\t{item_index}"), item);
                    }
                }
            }
            Node::Array(array) => {
                let (kind, count) = match &array.data {
                    ArrayData::Ints(values) => ('I', values.len()),
                    ArrayData::Doubles(values) => ('D', values.len()),
                    ArrayData::Floats(values) => ('F', values.len()),
                    ArrayData::References(values) => ('L', values.len()),
                };
                writeln!(output, "ARRAY\t{id}\t{}\t{kind}\t{count}", array.descriptor.0).unwrap();
                match &array.data {
                    ArrayData::Ints(values) => for (index, value) in values.iter().enumerate() { writeln!(output, "ELEMENT\t{id}\t{index}\tI\t{}", hex(&value.to_be_bytes())).unwrap(); },
                    ArrayData::Doubles(values) => for (index, value) in values.iter().enumerate() { writeln!(output, "ELEMENT\t{id}\t{index}\tD\t{}", hex(&value.to_be_bytes())).unwrap(); },
                    ArrayData::Floats(values) => for (index, value) in values.iter().enumerate() { writeln!(output, "ELEMENT\t{id}\t{index}\tF\t{}", hex(&value.to_be_bytes())).unwrap(); },
                    ArrayData::References(values) => for (index, value) in values.iter().enumerate() { writeln!(output, "ELEMENT\t{id}\t{index}\tL\t{}", reference(*value)).unwrap(); },
                }
            }
        }
    }
    for (index, item) in document.records.iter().enumerate() {
        record_row(&mut output, &format!("ROOT\t{index}"), item);
    }
    writeln!(output, "END\t{}\t{}\t{}", document.consumed, document.arena.nodes.len(), document.records.len()).unwrap();
    output
}

#[test]
fn trusted_source_streams_match_all_frozen_graph_rows_and_original_bytes() {
    let fixtures: &[(&str, &[u8], &str, usize, usize)] = &[
        ("gene", GENE, include_str!("../tests/fixtures/object_stream_codec/gene.ser.tsv"), 101, 7),
        ("genome", GENOME, include_str!("../tests/fixtures/object_stream_codec/genome.ser.tsv"), 214, 16),
        ("statistics", STATISTICS, include_str!("../tests/fixtures/object_stream_codec/statistics.ser.tsv"), 3468, 633),
        ("world", WORLD, include_str!("../tests/fixtures/object_stream_codec/world.ser.tsv"), 202, 27),
    ];
    let mut rows = 0;
    let mut handles = 0;
    for (name, bytes, expected, count, nodes) in fixtures {
        let document = decode(bytes);
        let actual = graph_rows(&document);
        assert_eq!(actual.lines().count(), *count, "{name} row count");
        assert_eq!(document.arena.nodes.len(), *nodes, "{name} handles");
        assert_eq!(document.consumed, bytes.len(), "{name} boundary");
        for (index, (actual, expected)) in actual.lines().zip(expected.lines()).enumerate() {
            assert_eq!(actual, expected, "{name} independent observation row {index}");
        }
        assert_eq!(expected.lines().count(), *count, "{name} expected count");
        assert_eq!(encode_document(&document, Limits::default()).unwrap(), *bytes, "{name} source bytes");
        rows += *count;
        handles += *nodes;
    }
    assert_eq!((rows, handles), (3985, 683));
}

#[test]
fn source_mutex_cycle_shared_backing_and_distinct_equal_nodes_remain_distinct() {
    let document = decode(WORLD);
    let Node::Object(wrapper) = document.arena.node(NodeId(9)).unwrap() else { panic!("source wrapper"); };
    assert_eq!(wrapper.slots[0].values, [FieldValue::Reference(Value::Handle(NodeId(11))), FieldValue::Reference(Value::Handle(NodeId(9)))]);
    assert_eq!(wrapper.slots[1].values, [FieldValue::Reference(Value::Handle(NodeId(11)))]);
    let arena = Arena { nodes: vec![
        Node::String(JavaString { units: vec![65], long: false }),
        Node::String(JavaString { units: vec![65], long: false }),
    ] };
    let document = Document { arena, records: vec![Record::Value(Value::Handle(NodeId(0))), Record::Value(Value::Handle(NodeId(1))), Record::Value(Value::Handle(NodeId(0)))], consumed: 0 };
    let bytes = encode_document(&document, Limits::default()).unwrap();
    assert_eq!(bytes, [0xac,0xed,0,5,0x74,0,1,65,0x74,0,1,65,0x71,0,0x7e,0,0]);
    let decoded = decode(&bytes);
    assert_eq!(decoded.arena.nodes.len(), 2);
    assert_eq!(decoded.records, document.records);
}

fn string_wire(body: &[u8], long: bool) -> Vec<u8> {
    let mut result = vec![0xac, 0xed, 0, 5, if long { 0x7c } else { 0x74 }];
    if long { result.extend_from_slice(&(body.len() as u64).to_be_bytes()); }
    else { result.extend_from_slice(&(body.len() as u16).to_be_bytes()); }
    result.extend_from_slice(body);
    result
}

#[test]
fn modified_utf_preserves_utf16_and_rejects_malformed_byte_sequences() {
    let canonical = [0xc0,0x80,65,0xdf,0xbf,0xe0,0xa0,0x80,0xed,0xa0,0x80,0xed,0xb0,0x80,0xed,0xbf,0xbf];
    let units = [0,65,0x7ff,0x800,0xd800,0xdc00,0xdfff];
    for long in [false, true] {
        let wire = string_wire(&canonical, long);
        let document = decode(&wire);
        assert_eq!(document.arena.nodes, [Node::String(JavaString { units: units.to_vec(), long })]);
        assert_eq!(encode_document(&document, Limits::default()).unwrap(), wire);
    }
    // ObjectInputStream accepts raw NUL and overlong forms; output uses canonical modified UTF
    let document = decode(&string_wire(&[0,0xc1,0x81,0xe0,0x81,0x81], false));
    assert_eq!(document.arena.nodes, [Node::String(JavaString { units: vec![0,65,65], long: false })]);
    assert_eq!(encode_document(&document, Limits::default()).unwrap(), string_wire(&[0xc0,0x80,65,65], false));
    for body in [&[0xc0][..], &[0xe0,0x80], &[0xc0,65], &[0x80], &[0xf0,0x90,0x80,0x80], &[0xff]] {
        assert_eq!(StreamReader::new(&string_wire(body, false), Limits::default()).unwrap().read_document().unwrap_err().kind, ErrorKind::MalformedUtf);
    }
}

#[test]
fn long_strings_and_imported_gene_arrays_have_no_editor_count_cap() {
    let text = vec![65; 65536];
    let wire = string_wire(&text, true);
    let document = decode(&wire);
    assert_eq!(encode_document(&document, Limits::default()).unwrap(), wire);
    let mut short = document.clone();
    let Node::String(value) = &mut short.arena.nodes[0] else { panic!("long String"); };
    value.long = false;
    assert_eq!(StreamWriter::new(&short.arena, Limits::default()).err().unwrap().kind, ErrorKind::InvalidLength);
    let wire = primitive_array_wire("[Lbiogenesis.Gene;",-8710523010866530344,129,&[0x70;129]);
    let document = decode(&wire);
    let Node::Array(array) = &document.arena.nodes[1] else { panic!("Gene array"); };
    assert_eq!(array.data, ArrayData::References(vec![Value::Null;129]));
    assert_eq!(encode_document(&document, Limits::default()).unwrap(), wire);
}

fn primitive_array_wire(name: &str, uid: i64, count: i32, payload: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0xac,0xed,0,5,0x75,0x72];
    bytes.extend_from_slice(&(name.len() as u16).to_be_bytes());
    bytes.extend_from_slice(name.as_bytes());
    bytes.extend_from_slice(&uid.to_be_bytes());
    bytes.extend_from_slice(&[2,0,0,0x78,0x70]);
    bytes.extend_from_slice(&count.to_be_bytes());
    bytes.extend_from_slice(payload);
    bytes
}

#[test]
fn standard_color_streams_match_actual_java_writer_bytes_and_float_bits() {
    let fixtures = crate::oracle_tests::fixture_path("crates/core/tests/fixtures/color-stream");
    let mut count = 0;
    for row in include_str!("../tests/fixtures/color-stream/cases.tsv").lines() {
        let fields: Vec<_> = row.split('\t').collect();
        assert_eq!(fields.len(), 2);
        let input = std::fs::read(fixtures.join(fields[0])).unwrap();
        let expected = std::fs::read(fixtures.join(fields[1])).unwrap();
        let document = decode(&input);
        assert_eq!(document.consumed, input.len());
        assert_eq!(encode_document(&document, Limits::default()).unwrap(), expected, "{}", fields[0]);
        count += 1;
    }
    assert_eq!(count, 40);
    let input = decode(include_bytes!("../tests/fixtures/color-stream/raw-float-array/input.ser"));
    let roundtrip = decode(include_bytes!("../tests/fixtures/color-stream/raw-float-array/roundtrip.ser"));
    let float_bits = |document: &Document| {
        let [Record::Value(Value::Handle(id))] = document.records.as_slice() else { panic!("one array root"); };
        let Node::Array(array) = &document.arena.nodes[id.0 as usize] else { panic!("float array"); };
        let ArrayData::Floats(bits) = &array.data else { panic!("raw float bits"); };
        bits.clone()
    };
    let mut before = Vec::new();
    let mut after = Vec::new();
    for (index, row) in include_str!("../tests/fixtures/color-stream/raw-float-bits.tsv").lines().enumerate() {
        let fields: Vec<_> = row.split('\t').collect();
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[0].parse::<usize>().unwrap(), index);
        before.push(u32::from_str_radix(fields[1], 16).unwrap());
        after.push(u32::from_str_radix(fields[2], 16).unwrap());
    }
    assert_eq!(before.len(), 12);
    assert_eq!(float_bits(&input), before);
    assert_eq!(float_bits(&roundtrip), after);
}

#[test]
fn standard_color_graph_retains_color_cache_and_colorspace_aliases() {
    let mut document = decode(include_bytes!("../tests/fixtures/color-stream/color-aliases/input.ser"));
    let [Record::Value(Value::Handle(root))] = document.records.as_slice() else { panic!("one color array root"); };
    let Node::Array(array) = &document.arena.nodes[root.0 as usize] else { panic!("Color array"); };
    let ArrayData::References(colors) = &array.data else { panic!("Color references"); };
    assert_eq!(colors.len(), 10);
    assert_eq!(colors[0], colors[1]);
    assert_ne!(colors[0], colors[2]);
    assert_eq!(colors[3], Value::Null);
    assert_eq!(colors[4], colors[5]);
    assert_ne!(colors[4], colors[6]);
    assert_eq!(colors[7], colors[9]);
    assert_ne!(colors[7], colors[8]);
    let color_slot = |value: Value| {
        let Value::Handle(id) = value else { panic!("Color handle"); };
        let Node::Object(color) = &document.arena.nodes[id.0 as usize] else { panic!("Color object"); };
        let [slot] = color.slots.as_slice() else { panic!("Color class data"); };
        slot.clone()
    };
    let float_color = color_slot(colors[4]);
    let FieldValue::Reference(Value::Handle(cache)) = float_color.values[3] else { panic!("RGB cache handle"); };
    assert_eq!(float_color.values[4], FieldValue::Reference(Value::Handle(cache)));
    let first_space = color_slot(colors[7]);
    let second_space = color_slot(colors[8]);
    assert!(matches!(first_space.values[2], FieldValue::Reference(Value::Handle(_))));
    assert_eq!(first_space.values[2], second_space.values[2]);
    let Value::Handle(color_id) = colors[7] else { panic!("Color handle"); };
    let Node::Object(color) = &mut document.arena.nodes[color_id.0 as usize] else { panic!("Color object"); };
    color.slots[0].values[2] = FieldValue::Reference(Value::Handle(cache));
    assert_eq!(StreamWriter::new(&document.arena, Limits::default()).err().unwrap().kind, ErrorKind::InvalidGraph);
}

#[test]
fn float_array_schema_and_resource_guards_reject_invalid_inputs() {
    const FLOAT_UID: i64 = 836686056779680834;
    assert_eq!(rejected(&primitive_array_wire("[F", 0, 0, &[])), ErrorKind::UidMismatch);
    let mut flags = primitive_array_wire("[F", FLOAT_UID, 0, &[]);
    flags[6 + 2 + "[F".len() + 8] = 3;
    assert_eq!(rejected(&flags), ErrorKind::SchemaMismatch);
    assert_eq!(rejected(&primitive_array_wire("[F", FLOAT_UID, -1, &[])), ErrorKind::InvalidLength);
    assert_eq!(rejected(&primitive_array_wire("[F", FLOAT_UID, 12, &[])), ErrorKind::Truncated { needed: 48 });
    let input = include_bytes!("../tests/fixtures/color-stream/raw-float-array/input.ser");
    limited_read(input, Limits { array_elements: 11, ..Limits::default() }, LimitKind::Array);
    limited_read(input, Limits { total_array_elements: 11, ..Limits::default() }, LimitKind::TotalArrays);
    let mut document = decode(input);
    let Node::Array(array) = document.arena.nodes.last_mut().unwrap() else { panic!("float array"); };
    array.data = ArrayData::Ints(vec![0; 12]);
    assert_eq!(StreamWriter::new(&document.arena, Limits::default()).err().unwrap().kind, ErrorKind::InvalidGraph);
    let mut document = decode(include_bytes!("../tests/fixtures/color-stream/space-srgb/input.ser"));
    let (descriptor, slot) = document.arena.nodes.iter().find_map(|node| {
        let Node::Object(object) = node else { return None; };
        object.slots.iter().find_map(|slot| {
            let Node::Descriptor(descriptor) = &document.arena.nodes[slot.descriptor.0 as usize] else { panic!("class descriptor"); };
            (class_schema(descriptor.class).name == "java.awt.color.ColorSpace").then(|| (slot.descriptor, slot.clone()))
        })
    }).unwrap();
    document.arena.nodes.push(Node::Object(JavaObject { descriptor, slots: vec![slot] }));
    assert_eq!(StreamWriter::new(&document.arena, Limits::default()).err().unwrap().kind, ErrorKind::InvalidGraph);
}

#[test]
fn primitive_bits_survive_reading_and_writer_canonicalizes_only_nan_payloads() {
    let bits: [u64; 6] = [0,0x8000000000000000,0x7ff0000000000000,0xfff0000000000000,0xfff8000000000001,0x7ff0000000000001];
    let payload: Vec<u8> = bits.iter().flat_map(|value| value.to_be_bytes()).collect();
    let wire = primitive_array_wire("[D",4514449696888150558,6,&payload);
    let mut document = decode(&wire);
    let Node::Array(array) = document.arena.nodes.last().unwrap() else { panic!("double array"); };
    assert_eq!(array.data, ArrayData::Doubles(bits.to_vec()));
    let encoded = encode_document(&document, Limits::default()).unwrap();
    let Node::Array(array) = decode(&encoded).arena.nodes.pop().unwrap() else { panic!("double array"); };
    assert_eq!(array.data, ArrayData::Doubles(vec![0,0x8000000000000000,0x7ff0000000000000,0xfff0000000000000,0x7ff8000000000000,0x7ff8000000000000]));
    document = decode(GENE);
    let Node::Object(color) = &mut document.arena.nodes[6] else { panic!("source Color"); };
    color.slots[0].values[0] = FieldValue::FloatBits(0xff800001);
    let encoded = encode_document(&document, Limits::default()).unwrap();
    let decoded = decode(&encoded);
    let Node::Object(color) = &decoded.arena.nodes[6] else { panic!("source Color"); };
    assert_eq!(color.slots[0].values[0], FieldValue::FloatBits(0x7fc00000));
}

#[test]
fn network_preamble_and_object_boundary_do_not_require_socket_eof() {
    let mut bytes = vec![0xac,0xed,0,5,0x77,8,0,0,0,8,0,0,0,51];
    bytes.extend_from_slice(&GENOME[4..]);
    let boundary = bytes.len();
    bytes.extend_from_slice(&[0x77,4,0,0,0,13]);
    let mut reader = StreamReader::new(&bytes, Limits::default()).unwrap();
    assert_eq!(reader.read_record().unwrap(), Record::Block { bytes: vec![0,0,0,8,0,0,0,51], long: false });
    let root = reader.read_record().unwrap();
    assert!(matches!(root, Record::Value(Value::Handle(_))));
    assert_eq!(reader.position(), boundary);
    assert_eq!(reader.arena().unwrap().nodes.len(), 16);
    assert_eq!(reader.read_record().unwrap(), Record::Block { bytes: vec![0,0,0,13], long: false });
    assert_eq!(reader.position(), bytes.len());
    assert_eq!(reader.into_arena().unwrap().nodes.len(), 16);
}

#[test]
fn block_data_keeps_framing_order_and_opaque_payloads() {
    let mut document = decode(WORLD);
    let Node::Object(list) = &mut document.arena.nodes[11] else { panic!("source ArrayList"); };
    list.slots[0].custom = vec![Record::Block { bytes: vec![0,0], long: false }, Record::Block { bytes: vec![0,0], long: true }];
    let bytes = encode_document(&document, Limits::default()).unwrap();
    assert_eq!(decode(&bytes).arena, document.arena);
    let custom = Record::Block { bytes: (0..300).map(|index| index as u8).collect(), long: true };
    let empty = Arena::default();
    let mut writer = StreamWriter::new(&empty, Limits::default()).unwrap();
    writer.write_record(&custom).unwrap();
    let wire = writer.finish().unwrap();
    assert_eq!(&wire[4..9], &[0x7a,0,0,1,44]);
    assert_eq!(decode(&wire).records, [custom]);
}

fn rejected(bytes: &[u8]) -> ErrorKind {
    StreamReader::new(bytes, Limits::default()).and_then(StreamReader::read_document).unwrap_err().kind
}

#[test]
fn unknown_schema_uid_flags_and_wrong_references_fail_explicitly() {
    let mut bytes = GENE.to_vec();
    let name = b"biogenesis.Gene";
    let position = bytes.windows(name.len()).position(|part| part == name).unwrap();
    bytes[position] = b'x';
    assert_eq!(rejected(&bytes), ErrorKind::UnknownClass);
    bytes = GENE.to_vec();
    bytes[position + name.len() + 7] ^= 1;
    assert_eq!(rejected(&bytes), ErrorKind::UidMismatch);
    bytes = GENE.to_vec();
    bytes[position + name.len() + 8] = 4;
    assert_eq!(rejected(&bytes), ErrorKind::SchemaMismatch);
    bytes = GENE.to_vec();
    bytes[position + name.len() + 10] = 39;
    assert_eq!(rejected(&bytes), ErrorKind::SchemaMismatch);
    bytes = GENE.to_vec();
    let color_name = b"java.awt.Color";
    let color_position = bytes.windows(color_name.len()).position(|part| part == color_name).unwrap();
    bytes.truncate(color_position - 4);
    bytes.extend_from_slice(&[0x71,0,0x7e,0,2]);
    assert_eq!(rejected(&bytes), ErrorKind::ReferenceType);
    assert_eq!(rejected(&[0xac,0xed,0,5,0x74,0,1,65,0x73,0x71,0,0x7e,0,0]), ErrorKind::ReferenceType);
    assert_eq!(rejected(&[0xac,0xed,0,5,0x71,0,0x7e,0,0]), ErrorKind::InvalidReference);
    for token in [0x76,0x79,0x7b,0x7d,0x7e] {
        assert_eq!(rejected(&[0xac,0xed,0,5,token]), ErrorKind::UnsupportedToken(token));
    }
    let unknown = primitive_array_wire("[S", 0, 0, &[]);
    assert_eq!(rejected(&unknown), ErrorKind::UnknownClass);
    let negative = primitive_array_wire("[I",5600894804908749477,-1,&[]);
    assert_eq!(rejected(&negative), ErrorKind::InvalidLength);
    assert_eq!(rejected(&[0xac,0xed,0,5,0x7a,0xff,0xff,0xff,0xff]), ErrorKind::InvalidLength);
    assert_eq!(rejected(&[0xac,0xed,0,5,0x7c,0xff,0xff,0xff,0xff,0xff,0xff,0xff,0xff]), ErrorKind::InvalidLength);
    let mut document = decode(GENE);
    let Node::Object(gene) = &mut document.arena.nodes[2] else { panic!("source Gene"); };
    gene.slots[0].values[39] = FieldValue::Reference(Value::Handle(NodeId(2)));
    assert_eq!(StreamWriter::new(&document.arena, Limits::default()).err().unwrap().kind, ErrorKind::InvalidGraph);
}

#[test]
fn every_incomplete_source_prefix_fails_without_exposing_partial_graph() {
    let mut prefixes = 0;
    for bytes in [GENE,GENOME,STATISTICS,WORLD] {
        for count in 0..bytes.len() {
            if count == 4 { continue; }
            assert!(StreamReader::new(&bytes[..count], Limits::default()).and_then(StreamReader::read_document).is_err(), "accepted prefix {count}/{}", bytes.len());
            prefixes += 1;
        }
    }
    assert_eq!(prefixes, 17207);
    let mut reader = StreamReader::new(&[0xac,0xed,0,5,0x71,0,0x7e,0,0], Limits::default()).unwrap();
    assert_eq!(reader.read_record().unwrap_err().kind, ErrorKind::InvalidReference);
    assert_eq!(reader.read_record().unwrap_err().kind, ErrorKind::Poisoned);
    assert_eq!(reader.arena().unwrap_err().kind, ErrorKind::Poisoned);
    assert_eq!(reader.into_arena().unwrap_err().kind, ErrorKind::Poisoned);
}

fn limited_read(bytes: &[u8], limits: Limits, kind: LimitKind) {
    let result = StreamReader::new(bytes, limits).and_then(StreamReader::read_document);
    assert_eq!(result.unwrap_err().kind, ErrorKind::Limit(kind));
}

#[test]
fn all_resource_budgets_reject_before_declared_unbounded_allocation_or_work() {
    limited_read(GENE, Limits { input_bytes: GENE.len()-1, ..Limits::default() }, LimitKind::Input);
    limited_read(GENE, Limits { handles: 0, ..Limits::default() }, LimitKind::Handles);
    limited_read(GENE, Limits { name_units: 0, ..Limits::default() }, LimitKind::Name);
    limited_read(GENE, Limits { depth: 0, ..Limits::default() }, LimitKind::Depth);
    limited_read(GENE, Limits { work: 0, ..Limits::default() }, LimitKind::Work);
    limited_read(GENE, Limits { allocated_bytes: 0, ..Limits::default() }, LimitKind::Allocation);
    let wire = string_wire(b"ab", false);
    limited_read(&wire, Limits { string_bytes: 1, ..Limits::default() }, LimitKind::String);
    limited_read(&wire, Limits { string_units: 1, ..Limits::default() }, LimitKind::String);
    limited_read(&wire, Limits { records: 0, ..Limits::default() }, LimitKind::Records);
    let wire = primitive_array_wire("[I",5600894804908749477,2,&[0;8]);
    limited_read(&wire, Limits { array_elements: 1, ..Limits::default() }, LimitKind::Array);
    limited_read(&wire, Limits { total_array_elements: 1, ..Limits::default() }, LimitKind::TotalArrays);
    let array_class = class_by_name("[Lbiogenesis.Gene;").unwrap();
    let arena = Arena { nodes: vec![
        Node::Descriptor(ClassDescriptor { class: array_class, signatures: vec![], annotations: vec![], superclass: None }),
        Node::Array(JavaArray { descriptor: NodeId(0), data: ArrayData::References(vec![Value::Handle(NodeId(999)); 2]) }),
    ] };
    assert_eq!(StreamWriter::new(&arena, Limits { array_elements: 1, ..Limits::default() }).err().unwrap().kind, ErrorKind::Limit(LimitKind::Array));
    let empty = Arena::default();
    assert_eq!(StreamWriter::new(&empty, Limits { output_bytes: 3, ..Limits::default() }).err().unwrap().kind, ErrorKind::Limit(LimitKind::Output));
    let mut writer = StreamWriter::new(&empty, Limits { output_bytes: 4, ..Limits::default() }).unwrap();
    assert_eq!(writer.write_record(&Record::Value(Value::Null)).unwrap_err().kind, ErrorKind::Limit(LimitKind::Output));
    assert_eq!(writer.finish().unwrap_err().kind, ErrorKind::Poisoned);
    assert_eq!(StreamReader::new(GENE, Limits { depth: 129, ..Limits::default() }).err().unwrap().kind, ErrorKind::InvalidLimits);
    let wire = primitive_array_wire("[I",5600894804908749477,i32::MAX,&[]);
    limited_read(&wire, Limits::default(), LimitKind::Array);
}

#[test]
fn writer_preflight_and_failed_records_never_return_partial_bytes() {
    let mut document = decode(GENE);
    let Node::Object(gene) = &mut document.arena.nodes[2] else { panic!("source Gene"); };
    gene.slots[0].values.pop();
    assert_eq!(StreamWriter::new(&document.arena, Limits::default()).err().unwrap().kind, ErrorKind::InvalidGraph);
    let empty = Arena::default();
    let mut writer = StreamWriter::new(&empty, Limits::default()).unwrap();
    assert_eq!(writer.write_record(&Record::Value(Value::Handle(NodeId(100)))).unwrap_err().kind, ErrorKind::InvalidReference);
    assert_eq!(writer.write_record(&Record::Value(Value::Null)).unwrap_err().kind, ErrorKind::Poisoned);
    assert_eq!(writer.finish().unwrap_err().kind, ErrorKind::Poisoned);
}

#[test]
fn descriptor_positions_reject_active_ids_but_bare_annotation_refs_remain_valid() {
    let mut failures = Vec::new();
    fn utf(bytes: &mut Vec<u8>, value: &str) {
        bytes.extend_from_slice(&(value.len() as u16).to_be_bytes());
        bytes.extend_from_slice(value.as_bytes());
    }
    fn prefix(bytes: &mut Vec<u8>, name: &str, uid: i64, flags: u8, fields: &[(u8, &str, Option<&str>)]) {
        bytes.push(0x72);
        utf(bytes, name);
        bytes.extend_from_slice(&uid.to_be_bytes());
        bytes.push(flags);
        bytes.extend_from_slice(&(fields.len() as u16).to_be_bytes());
        for (kind, name, signature) in fields {
            bytes.push(*kind);
            utf(bytes, name);
            if let Some(signature) = signature { bytes.push(0x74); utf(bytes, signature); }
        }
    }
    fn descriptor(name: &str, signatures: Vec<Option<NodeId>>, annotations: Vec<Record>, superclass: Option<NodeId>) -> Node {
        Node::Descriptor(ClassDescriptor { class: class_by_name(name).unwrap(), signatures, annotations, superclass })
    }
    fn object(id: u32) -> Node {
        Node::Object(JavaObject { descriptor: NodeId(id), slots: vec![ClassData {
            descriptor: NodeId(id), values: vec![FieldValue::Int(0);4], custom: vec![],
        }] })
    }
    fn text(value: &str) -> Node { Node::String(JavaString { units: value.encode_utf16().collect(), long: false }) }
    fn value(id: u32) -> Record { Record::Value(Value::Handle(NodeId(id))) }
    let header = [0xac,0xed,0,5];
    let rect_fields = [(b'I',"height",None),(b'I',"width",None),(b'I',"x",None),(b'I',"y",None)];
    let collection_fields = [(b'L',"c",Some("Ljava/util/Collection;")),(b'L',"mutex",Some("Ljava/lang/Object;"))];
    let list_fields = [(b'L',"list",Some("Ljava/util/List;"))];
    let mut object_wire = header.to_vec();
    prefix(&mut object_wire,"java.awt.Rectangle",-4345857070255674764,2,&rect_fields);
    object_wire.extend_from_slice(&[0x73,0x71,0,0x7e,0,0]);
    object_wire.extend_from_slice(&[0;16]);
    object_wire.extend_from_slice(&[0x78,0x70]);
    let mut array_wire = header.to_vec();
    prefix(&mut array_wire,"[I",5600894804908749477,2,&[]);
    array_wire.extend_from_slice(&[0x75,0x71,0,0x7e,0,0,0,0,0,0,0x78,0x70]);
    let mut superclass_wire = header.to_vec();
    prefix(&mut superclass_wire,"java.util.Collections$SynchronizedCollection",3053995032091335093,3,&collection_fields);
    prefix(&mut superclass_wire,"java.util.Collections$SynchronizedList",-7754090372962971524,2,&list_fields);
    superclass_wire.extend_from_slice(&[0x78,0x71,0,0x7e,0,0,0x78,0x70]);
    for (index,wire) in [object_wire,array_wire,superclass_wire].into_iter().enumerate() {
        let mut reader = StreamReader::new(&wire,Limits::default()).unwrap();
        match reader.read_record() {
            Err(error) if error.kind == ErrorKind::IncompleteDescriptor => {
                assert_eq!(reader.arena().unwrap_err().kind,ErrorKind::Poisoned);
                assert_eq!(reader.into_arena().unwrap_err().kind,ErrorKind::Poisoned);
            }
            actual => failures.push(format!("reader descriptor-position case {index}: {actual:?}")),
        }
    }
    let object_arena = Arena { nodes: vec![
        descriptor("java.awt.Rectangle",vec![None;4],vec![value(2)],None),object(0),object(0),
    ] };
    let array_arena = Arena { nodes: vec![
        descriptor("[I",vec![],vec![value(1)],None),
        Node::Array(JavaArray { descriptor:NodeId(0),data:ArrayData::Ints(vec![]) }),
    ] };
    let mut superclass_arena = Arena { nodes: vec![
        descriptor("java.util.Collections$SynchronizedCollection",vec![Some(NodeId(1)),Some(NodeId(2))],vec![value(3)],None),
        text("Ljava/util/Collection;"),text("Ljava/lang/Object;"),
        descriptor("java.util.Collections$SynchronizedList",vec![Some(NodeId(4))],vec![],Some(NodeId(0))),
        text("Ljava/util/List;"),
    ] };
    for (index,(arena,root)) in [(&object_arena,1),(&array_arena,0),(&superclass_arena,0)].into_iter().enumerate() {
        let mut writer = StreamWriter::new(arena,Limits::default()).unwrap();
        match writer.write_record(&value(root)) {
            Err(error) if error.kind == ErrorKind::IncompleteDescriptor => {
                assert_eq!(writer.write_record(&Record::Value(Value::Null)).unwrap_err().kind,ErrorKind::Poisoned);
                assert_eq!(writer.finish().unwrap_err().kind,ErrorKind::Poisoned);
            }
            actual => failures.push(format!("writer descriptor-position case {index}: {actual:?}; finish={:?}",
                writer.finish().map(|bytes| bytes.len()))),
        }
    }
    let bare_object = Arena { nodes: vec![
        descriptor("java.awt.Rectangle",vec![None;4],vec![value(0)],None),object(0),
    ] };
    let bare_array = Arena { nodes: vec![
        descriptor("[I",vec![],vec![value(0)],None),
        Node::Array(JavaArray { descriptor:NodeId(0),data:ArrayData::Ints(vec![]) }),
    ] };
    let separate_annotation = Arena { nodes: vec![
        descriptor("java.awt.Rectangle",vec![None;4],vec![value(2)],None),
        descriptor("java.awt.Rectangle",vec![None;4],vec![],None),object(1),
    ] };
    let Node::Descriptor(collection) = &mut superclass_arena.nodes[0] else { panic!("collection descriptor"); };
    collection.annotations.clear();
    for (arena,records) in [(bare_object,vec![value(0),value(1)]),(bare_array,vec![value(0),value(1)]),
                           (separate_annotation,vec![value(0)]),(superclass_arena,vec![value(0),value(3)])] {
        let document = Document { arena,records,consumed:0 };
        let bytes = encode_document(&document,Limits::default()).unwrap();
        let actual = decode(&bytes);
        assert_eq!(actual.arena,document.arena);
        assert_eq!(actual.records,document.records);
    }
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn actual_java_modified_utf_and_ieee_supplement_matches_complete_bytes() {
    let cases: &[(&str, &[u8], Option<&[u8]>)] = &[
        ("mutf-units", include_bytes!("../tests/fixtures/object_stream_codec/java_supplement/04-input.ser").as_slice(), Some(include_bytes!("../tests/fixtures/object_stream_codec/java_supplement/04-java-first.ser").as_slice())),
        ("long-ascii", include_bytes!("../tests/fixtures/object_stream_codec/java_supplement/05-input.ser").as_slice(), Some(include_bytes!("../tests/fixtures/object_stream_codec/java_supplement/05-java-first.ser").as_slice())),
        ("double-ieee", include_bytes!("../tests/fixtures/object_stream_codec/java_supplement/06-input.ser").as_slice(), Some(include_bytes!("../tests/fixtures/object_stream_codec/java_supplement/06-java-first.ser").as_slice())),
        ("color-alpha-negative-zero", include_bytes!("../tests/fixtures/object_stream_codec/java_supplement/07-input.ser").as_slice(), Some(include_bytes!("../tests/fixtures/object_stream_codec/java_supplement/07-java-first.ser").as_slice())),
        ("color-alpha-infinity", include_bytes!("../tests/fixtures/object_stream_codec/java_supplement/08-input.ser").as_slice(), Some(include_bytes!("../tests/fixtures/object_stream_codec/java_supplement/08-java-first.ser").as_slice())),
        ("color-alpha-nan", include_bytes!("../tests/fixtures/object_stream_codec/java_supplement/09-input.ser").as_slice(), Some(include_bytes!("../tests/fixtures/object_stream_codec/java_supplement/09-java-first.ser").as_slice())),
        ("mutf-raw-nul", include_bytes!("../tests/fixtures/object_stream_codec/java_supplement/10-input.ser").as_slice(), Some(include_bytes!("../tests/fixtures/object_stream_codec/java_supplement/10-java-first.ser").as_slice())),
        ("mutf-overlong-two", include_bytes!("../tests/fixtures/object_stream_codec/java_supplement/11-input.ser").as_slice(), Some(include_bytes!("../tests/fixtures/object_stream_codec/java_supplement/11-java-first.ser").as_slice())),
        ("mutf-overlong-three", include_bytes!("../tests/fixtures/object_stream_codec/java_supplement/12-input.ser").as_slice(), Some(include_bytes!("../tests/fixtures/object_stream_codec/java_supplement/12-java-first.ser").as_slice())),
        ("mutf-bad-continuation", include_bytes!("../tests/fixtures/object_stream_codec/java_supplement/13-input.ser").as_slice(), None),
        ("mutf-four-byte", include_bytes!("../tests/fixtures/object_stream_codec/java_supplement/14-input.ser").as_slice(), None),
        ("mutf-truncated-two", include_bytes!("../tests/fixtures/object_stream_codec/java_supplement/15-input.ser").as_slice(), None),
        ("mutf-truncated-three", include_bytes!("../tests/fixtures/object_stream_codec/java_supplement/16-input.ser").as_slice(), None),
    ];
    let mut successes = 0;
    let mut malformed = 0;
    for (name, input, expected) in cases {
        let result = StreamReader::new(input, Limits::default()).and_then(StreamReader::read_document);
        if let Some(expected) = expected {
            let document = result.unwrap_or_else(|error| panic!("{name}: {error}"));
            assert_eq!(document.consumed, input.len(), "{name} boundary");
            assert_eq!(encode_document(&document, Limits::default()).unwrap(), *expected, "{name} actual Java bytes");
            successes += 1;
        } else {
            assert_eq!(result.unwrap_err().kind, ErrorKind::MalformedUtf, "{name} actual Java failure");
            malformed += 1;
        }
    }
    assert_eq!((successes, malformed), (9, 4));
}

fn dense_budget_trace(phase: &str, budget: &Budget, offset: usize) {
    println!(
        "DENSE_GENESIS_BUDGET phase={phase} offset={offset} allocated={} work={} records={} elements={} allocation_limit={} work_limit={} depth_limit={}",
        budget.allocated,
        budget.work,
        budget.records,
        budget.elements,
        budget.limits.allocated_bytes,
        budget.limits.work,
        budget.limits.depth,
    );
}

#[test]
#[ignore = "Run the frozen external retained-tree genesis codec budget gate"]
fn required_dense_retained_tree_genesis_codec_budget() {
    let path = std::env::var_os("BGR_DENSE_GENESIS_INPUT")
        .expect("Required immutable source retained-tree genesis stream");
    let bytes = std::fs::read(path).expect("Owned source stream");
    assert_eq!(bytes.len(), 69_278_502);
    let limits = Limits::default();
    let mut reader = match StreamReader::new(&bytes, limits) {
        Ok(reader) => reader,
        Err(failure) => panic!(
            "DENSE_GENESIS_REJECT phase=reader-initial kind={:?} offset={}",
            failure.kind, failure.offset,
        ),
    };
    let record = match reader.read_record() {
        Ok(record) => record,
        Err(failure) => {
            dense_budget_trace("reader-rejected", &reader.budget, reader.position());
            panic!(
                "DENSE_GENESIS_REJECT phase=reader-record kind={:?} offset={}",
                failure.kind, failure.offset,
            );
        }
    };
    dense_budget_trace("reader-record-complete", &reader.budget, reader.position());
    let consumed = reader.position();
    assert_eq!(consumed, bytes.len(), "Source stream has one complete root");
    let document = Document {
        arena: reader.into_arena().expect("Successful source root"),
        records: vec![record],
        consumed,
    };
    assert_eq!(document.arena.nodes.len(), 1_479_961);
    {
        let graph = crate::legacy_graph::LegacyGraph::new(&document, limits)
            .expect("Source graph within the unchanged validation boundary");
        let loaded = crate::legacy_spatial::decode_world_spatial(&graph)
            .expect("Actual retained source tree");
        assert_eq!(loaded.members.len(), 12_500);
        assert_eq!(loaded.actors.nodes.len(), 25_691);
        let state = loaded.tree.stored_state();
        let tokens: Vec<_> = state.boundables.iter().filter_map(|boundable| match boundable {
            crate::spatial::StoredBoundable::Item { token, .. } => Some(*token),
            crate::spatial::StoredBoundable::Node { .. } => None,
        }).collect();
        assert_eq!(tokens.len(), 13_191);
        let members: std::collections::HashSet<_> = loaded.members.iter().copied().collect();
        assert!(tokens.iter().all(|token| !members.contains(token)));
    }
    let mut writer = match StreamWriter::new(&document.arena, limits) {
        Ok(writer) => writer,
        Err(failure) => panic!(
            "DENSE_GENESIS_REJECT phase=writer-preflight kind={:?} offset={}",
            failure.kind, failure.offset,
        ),
    };
    if let Err(failure) = writer.write_record(&document.records[0]) {
        dense_budget_trace("writer-rejected", &writer.budget, writer.position());
        panic!(
            "DENSE_GENESIS_REJECT phase=writer-record kind={:?} offset={}",
            failure.kind, failure.offset,
        );
    }
    dense_budget_trace("writer-complete", &writer.budget, writer.position());
    let encoded = writer.finish().expect("Complete source writer output");
    assert!(encoded == bytes, "Complete original-source bytes differ");
    println!(
        "DENSE_GENESIS_CODEC PASS bytes={} handles={} members=12500 union_actors=25691 tree_only=13191",
        bytes.len(), document.arena.nodes.len(),
    );
}


#[test]
fn incremental_documents_preserve_exact_cap_eof_and_reject_excess_input() {
    let bytes = [0xac, 0xed, 0, 5, 0x70];
    let expected = StreamReader::new(&bytes, Limits::default()).unwrap().read_document().unwrap();
    let mut input = std::io::Cursor::new(bytes);
    let limits = Limits { input_bytes: bytes.len(), ..Limits::default() };
    let actual = StreamReader::from_reader(&mut input, limits).unwrap().read_document().unwrap();
    assert_eq!(actual, expected);
    let mut input = std::io::Cursor::new([0xac, 0xed, 0, 5, 0x70, 0x70]);
    assert_eq!(StreamReader::from_reader(&mut input, limits).unwrap().read_document().unwrap_err().kind,
        ErrorKind::Limit(LimitKind::Input));
    let mut input = std::io::Cursor::new([0xac, 0xed, 0]);
    assert_eq!(StreamReader::from_reader(&mut input, Limits::default()).err().unwrap(),
        StreamError { offset: 0, kind: ErrorKind::Truncated { needed: 4 } });
}

#[test]
fn incremental_buffer_allocation_is_charged_before_reading() {
    struct Counted<'a>(&'a mut usize);
    impl std::io::Read for Counted<'_> {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> { *self.0 += 1; Ok(0) }
    }
    let mut calls = 0;
    let mut input = Counted(&mut calls);
    let limits = Limits { allocated_bytes: 8191, ..Limits::default() };
    assert_eq!(StreamReader::from_reader(&mut input, limits).err().unwrap().kind,
        ErrorKind::Limit(LimitKind::Allocation));
    assert_eq!(calls, 0);
}


#[test]
fn incremental_arrays_reject_minimum_beyond_input_cap_before_payload_allocation() {
    let bytes = primitive_array_wire("[I", 5600894804908749477, 100_000, &[]);
    let limits = Limits { input_bytes: bytes.len(), allocated_bytes: 16 * 1024, ..Limits::default() };
    let mut input = std::io::Cursor::new(&bytes);
    let mut reader = StreamReader::from_reader(&mut input, limits).unwrap();
    assert_eq!(reader.read_record().unwrap_err().kind, ErrorKind::Limit(LimitKind::Input));
    assert_eq!(reader.read_record().unwrap_err().kind, ErrorKind::Poisoned);
    assert_eq!(StreamReader::new(&bytes, limits).unwrap().read_record().unwrap_err().kind,
        ErrorKind::Truncated { needed: 400_000 });
}
