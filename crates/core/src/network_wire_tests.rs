// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use crate::object_stream::{self, ErrorKind as StreamErrorKind, LimitKind};
use std::collections::BTreeSet;

#[test]
fn delayed_binding_retains_exhausted_allocation_and_work_budgets() {
    let bytes = fixture("recv-code-shared.request.bin");
    let mut reader = Reader::new(&bytes, Limits::default()).unwrap();
    reader.read_request().unwrap();
    let code = reader.read_genetic_code().unwrap();
    let full = Limits::default();
    Writer::new(&code.document().arena, full).unwrap();
    let empty = Arena::default();
    for allocation in [true, false] {
        let mut writer = Writer::new(&empty, full).unwrap();
        writer.write_request(Request::SendCode { netcode: NET }).unwrap();
        let (used_allocation, used_work) = writer.usage();
        writer.charge_external(if allocation { full.allocated_bytes - used_allocation } else { 0 },
            if allocation { 0 } else { full.work - used_work }).unwrap();
        let error = writer.bind_empty_arena(&code.document().arena).err().unwrap();
        assert_eq!(error.kind, ErrorKind::Stream(StreamErrorKind::Limit(
            if allocation { LimitKind::Allocation } else { LimitKind::Work })));
    }
}

#[test]
fn delayed_binding_cannot_replace_an_existing_arena_or_unpoison_a_writer() {
    let bytes = fixture("recv-code-shared.request.bin");
    let mut reader = Reader::new(&bytes, Limits::default()).unwrap();
    reader.read_request().unwrap();
    let code = reader.read_genetic_code().unwrap();
    let bound = Writer::new(&code.document().arena, Limits::default()).unwrap();
    assert_eq!(bound.bind_empty_arena(&code.document().arena).err().unwrap().kind,
        ErrorKind::Stream(StreamErrorKind::InvalidGraph));
    let empty = Arena::default();
    let mut failed = Writer::new(&empty, Limits::default()).unwrap();
    assert_eq!(failed.write_genetic_code(Value::Null).unwrap_err().kind, ErrorKind::NullGeneticCode);
    assert_eq!(failed.bind_empty_arena(&code.document().arena).err().unwrap().kind, ErrorKind::Poisoned);
}

const NET: i32 = 0x12345678;

fn fixture(name: &str) -> Vec<u8> {
    let root = crate::oracle_tests::fixture_path("crates/core")
        .join("tests/fixtures/network_wire_source");
    std::fs::read(root.join(name)).unwrap()
}

fn expected_request(name: &str) -> Request {
    if name.contains("-connect-") {
        let version = if matches!(name, "recv-connect-full-precedes-version"
            | "recv-connect-refusal-precedes-version" | "recv-connect-version"
            | "recv-connect-known-precedes-all" | "recv-connect-known-uninitialized") { 899 } else { 900 };
        let port = match name {
            "recv-connect-port-zero" => 0,
            "recv-connect-port-outside-socket-range" => 65536,
            _ => 7777,
        };
        let netcode = if name == "recv-connect-same-endpoint-new-code" { NET + 1 } else { NET };
        Request::Connect { version, port, netcode }
    } else if name.contains("-code-") {
        Request::SendCode { netcode: NET }
    } else if name.contains("-ping-") {
        Request::KeepAlive { netcode: NET }
    } else {
        assert!(name.contains("-disconnect-"));
        Request::Disconnect { netcode: NET }
    }
}

fn expected_replies(name: &str) -> Vec<i32> {
    if name.starts_with("send-code-final-") {
        vec![9, name.rsplit('-').next().unwrap().parse().unwrap()]
    } else if matches!(name, "send-code-empty" | "send-code-shared" | "send-code-null") {
        vec![9, 10]
    } else if name.starts_with("send-") {
        vec![name.rsplit('-').next().unwrap().parse().unwrap()]
    } else if name.starts_with("recv-connect-") {
        vec![match name {
            "recv-connect-refused" | "recv-connect-refusal-precedes-version" => 6,
            "recv-connect-full" | "recv-connect-full-precedes-version" => 4,
            "recv-connect-version" => 16,
            "recv-connect-known-precedes-all" | "recv-connect-known-uninitialized"
                | "recv-connect-same-endpoint-new-code" => 3,
            "recv-connect-success" | "recv-connect-port-zero"
                | "recv-connect-port-outside-socket-range" => 2,
            _ => panic!("Unexpected source case {name}"),
        }]
    } else if name.starts_with("recv-code-") {
        match name {
            "recv-code-unknown" | "recv-code-uninitialized" => vec![7],
            "recv-code-wrong-class" => vec![9],
            "recv-code-empty" | "recv-code-one" | "recv-code-shared" | "recv-code-null" => vec![9, 10],
            _ => panic!("Unexpected source case {name}"),
        }
    } else if name.starts_with("recv-ping-") {
        vec![if name == "recv-ping-51" { 14 } else { 7 }]
    } else {
        assert!(name.starts_with("recv-disconnect-"));
        vec![if name == "recv-disconnect-51" { 15 } else { 7 }]
    }
}

fn decoded_genome(bytes: &[u8]) -> Rc<RefCell<Genome>> {
    let mut reader = Reader::new(bytes, Limits::default()).unwrap();
    assert_eq!(reader.read_request().unwrap(), Request::SendCode { netcode: NET });
    reader.read_genetic_code().unwrap().into_genome().unwrap()
}

#[test]
fn all_original_handler_streams_preserve_headers_replies_and_payload_boundaries() {
    let observations = include_str!("../tests/fixtures/network_wire_source/observations.tsv");
    let empty = Arena::default();
    let mut names = BTreeSet::new();
    let (mut files, mut payloads, mut rejected_payloads) = (0, 0, 0);
    for line in observations.lines() {
        let row: Vec<_> = line.split('\t').collect();
        assert_eq!(row.len(), 11);
        assert_eq!(row[0], "WIRE");
        let name = row[1];
        assert!(names.insert(name));
        let bytes = fixture(&format!("{name}.request.bin"));
        assert_eq!(bytes.len(), row[2].parse::<usize>().unwrap(), "{name}");
        let mut reader = Reader::new(&bytes, Limits::default()).unwrap();
        let request = reader.read_request().unwrap();
        assert_eq!(request, expected_request(name), "{name}");
        let boundary = reader.position();
        assert_eq!(reader.remaining_primitive_bytes(), 0, "{name}");
        let mut header = Writer::new(&empty, Limits::default()).unwrap();
        header.write_request(request).unwrap();
        assert_eq!(header.finish().unwrap(), &bytes[..boundary], "{name}");
        if boundary == bytes.len() {
            reader.finish().unwrap();
        } else {
            assert!(matches!(request, Request::SendCode { .. }));
            assert_eq!(boundary, 14);
            payloads += 1;
            if matches!(name, "send-code-null" | "recv-code-null" | "recv-code-wrong-class") {
                let failure = reader.read_genetic_code().err().unwrap();
                assert_eq!(failure.kind, if name.ends_with("wrong-class") {
                    ErrorKind::WrongGeneticCode
                } else { ErrorKind::NullGeneticCode }, "{name}");
                assert_eq!(reader.read_i32().unwrap_err().kind, ErrorKind::Poisoned);
                rejected_payloads += 1;
            } else {
                let payload = reader.read_genetic_code().unwrap();
                let mut writer = Writer::new(&payload.document().arena, Limits::default()).unwrap();
                writer.write_request(request).unwrap();
                assert_eq!(writer.position(), boundary);
                writer.write_genetic_code(payload.root()).unwrap();
                assert_eq!(writer.finish().unwrap(), bytes, "{name}");
                assert_eq!(reader.read_i32().unwrap_err().kind, ErrorKind::Finished);
                let genome = payload.into_genome().unwrap();
                assert_eq!(genome.borrow().gene_count().unwrap(), if name.ends_with("empty") {
                    0
                } else if name.ends_with("shared") { 2 } else { 1 }, "{name}");
            }
        }
        let response = fixture(&format!("{name}.response.bin"));
        assert_eq!(response.len(), row[3].parse::<usize>().unwrap(), "{name}");
        let replies = expected_replies(name);
        let mut reader = Reader::new(&response, Limits::default()).unwrap();
        let mut writer = Writer::new(&empty, Limits::default()).unwrap();
        for reply in &replies { assert_eq!(reader.read_i32().unwrap(), *reply, "{name}"); }
        reader.finish().unwrap();
        if name.starts_with("recv-") {
            for reply in replies { writer.write_i32(reply).unwrap(); }
        } else {
            writer.write_i32s(&replies).unwrap();
        }
        assert_eq!(writer.finish().unwrap(), response, "{name}");
        for original in [&bytes, &response] {
            let document = StreamReader::new(original, Limits::default()).unwrap().read_document().unwrap();
            assert_eq!(object_stream::encode_document(&document, Limits::default()).unwrap(), *original, "{name}");
            files += 1;
        }
    }
    assert_eq!((names.len(), files, payloads, rejected_payloads), (49, 98, 15, 3));
}

#[test]
fn detached_genetic_codes_preserve_empty_arrays_shared_genes_bits_and_utf16() {
    let empty = decoded_genome(&fixture("recv-code-empty.request.bin"));
    assert_eq!(empty.borrow().gene_count().unwrap(), 0);
    let genome = decoded_genome(&fixture("recv-code-shared.request.bin"));
    {
        let code = genome.borrow();
        let first = code.gene_ref(0).unwrap();
        let second = code.gene_ref(1).unwrap();
        assert!(Rc::ptr_eq(&first, &second));
        let gene = first.borrow();
        assert_eq!(gene.length().to_bits(), 3.5_f64.to_bits());
        assert_eq!(gene.theta().to_bits(), (-0.0_f64).to_bits());
        assert_eq!(gene.color(), Some(0x80402010));
        assert_eq!(code.clade_id().unwrap().units(), &[0x41, 0, 0xd83e, 0xddec, 0xd800, 0x5a]);
    }
    for code in [&empty, &genome] {
        let payload = GeneticCodePayload::from_genome(code, Limits::default()).unwrap();
        let mut writer = Writer::new(&payload.document().arena, Limits::default()).unwrap();
        writer.write_request(Request::SendCode { netcode: NET }).unwrap();
        writer.write_genetic_code(payload.root()).unwrap();
        let encoded = writer.finish().unwrap();
        let restored = decoded_genome(&encoded);
        assert_eq!(restored.borrow().gene_count().unwrap(), code.borrow().gene_count().unwrap());
        if restored.borrow().gene_count().unwrap() == 2 {
            let value = restored.borrow();
            assert!(Rc::ptr_eq(&value.gene_ref(0).unwrap(), &value.gene_ref(1).unwrap()));
            assert_eq!(value.gene_ref(0).unwrap().borrow().theta().to_bits(), (-0.0_f64).to_bits());
            assert_eq!(value.gene_ref(0).unwrap().borrow().color(), Some(0x80402010));
            assert_eq!(value.clade_id().unwrap().units(), &[0x41, 0, 0xd83e, 0xddec, 0xd800, 0x5a]);
        }
    }
}

#[test]
fn primitive_reads_cross_short_long_empty_and_coalesced_blocks_without_losing_bytes() {
    let bytes = [0xac, 0xed, 0, 5, 0x77, 1, 0x12, 0x77, 0,
        0x7a, 0, 0, 0, 3, 0x34, 0x56, 0x78,
        0x77, 8, 0, 0, 0, 9, 0xff, 0xff, 0xff, 0xff];
    let mut reader = Reader::new(&bytes, Limits::default()).unwrap();
    assert_eq!(reader.read_i32().unwrap(), NET);
    assert_eq!(reader.read_i32().unwrap(), 9);
    assert_eq!(reader.remaining_primitive_bytes(), 4);
    assert_eq!(reader.read_i32().unwrap(), -1);
    assert_eq!(reader.position(), bytes.len());
    reader.finish().unwrap();
    let request = Request::Connect { version: i32::MIN, port: -1, netcode: i32::MAX };
    let empty = Arena::default();
    let mut writer = Writer::new(&empty, Limits::default()).unwrap();
    writer.write_request(request).unwrap();
    let encoded = writer.finish().unwrap();
    let mut reader = Reader::new(&encoded, Limits::default()).unwrap();
    assert_eq!(reader.read_request().unwrap(), request);
    reader.finish().unwrap();
    for value in [i32::MIN, -1, 0, i32::MAX] {
        let mut writer = Writer::new(&empty, Limits::default()).unwrap();
        writer.write_i32(value).unwrap();
        let encoded = writer.finish().unwrap();
        assert_eq!(Reader::new(&encoded, Limits::default()).unwrap().read_i32().unwrap(), value);
    }
}

#[test]
fn malformed_truncated_and_unknown_input_poison_the_reader() {
    let connect = fixture("send-connect-2.request.bin");
    for end in 0..connect.len() {
        match Reader::new(&connect[..end], Limits::default()) {
            Err(failure) => {
                assert!(end < 4, "Only incomplete ObjectStream headers fail construction: {end}");
                assert_eq!(failure, WireError { offset: 0,
                    kind: ErrorKind::Stream(StreamErrorKind::Truncated { needed: 4 }) });
            }
            Ok(mut reader) => {
                assert!(end >= 4);
                let (offset, needed) = match end {
                    4 => (4, 1),
                    5 => (5, 1),
                    _ => (6, 16),
                };
                assert_eq!(reader.read_request().unwrap_err(), WireError { offset,
                    kind: ErrorKind::Stream(StreamErrorKind::Truncated { needed }) }, "prefix {end}");
                assert_eq!(reader.read_i32().unwrap_err().kind, ErrorKind::Poisoned);
            }
        }
    }
    for (bytes, expected) in [
        (vec![0xac, 0xed, 0, 5, 0x77, 4, 0, 0, 0, 99], ErrorKind::UnknownRequest(99)),
        (vec![0xac, 0xed, 0, 5, 0x70], ErrorKind::ExpectedPrimitive),
        (vec![0xac, 0xed, 0, 5, 0x7b], ErrorKind::Stream(StreamErrorKind::UnsupportedToken(0x7b))),
    ] {
        let mut reader = Reader::new(&bytes, Limits::default()).unwrap();
        assert_eq!(reader.read_request().unwrap_err().kind, expected);
        assert_eq!(reader.read_request().unwrap_err().kind, ErrorKind::Poisoned);
    }
    assert_eq!(Reader::new(&[0, 0, 0, 0], Limits::default()).err().unwrap().kind,
        ErrorKind::Stream(StreamErrorKind::Header));
    let code = fixture("recv-code-shared.request.bin");
    for end in [14, code.len() / 2, code.len() - 1] {
        let mut reader = Reader::new(&code[..end], Limits::default()).unwrap();
        reader.read_request().unwrap();
        assert!(reader.read_genetic_code().is_err(), "payload prefix {end}");
        assert_eq!(reader.read_i32().unwrap_err().kind, ErrorKind::Poisoned);
    }
    let mut extra = fixture("recv-code-one.request.bin");
    extra[5] = 9;
    extra.insert(14, 0);
    let mut reader = Reader::new(&extra, Limits::default()).unwrap();
    reader.read_request().unwrap();
    assert_eq!(reader.read_genetic_code().err().unwrap().kind, ErrorKind::PendingPrimitive);
    assert_eq!(reader.read_i32().unwrap_err().kind, ErrorKind::Poisoned);
    let mut trailing = connect;
    trailing.push(0x70);
    let mut reader = Reader::new(&trailing, Limits::default()).unwrap();
    reader.read_request().unwrap();
    assert_eq!(reader.finish().unwrap_err().kind, ErrorKind::TrailingData);
}

#[test]
fn parser_writer_and_detached_hydration_keep_resource_failures_explicit() {
    let bytes = fixture("recv-code-shared.request.bin");
    let limits = Limits { input_bytes: bytes.len() - 1, ..Limits::default() };
    assert!(matches!(Reader::new(&bytes, limits).err().unwrap().kind,
        ErrorKind::Stream(StreamErrorKind::Limit(LimitKind::Input))));
    let limits = Limits { work: 0, ..Limits::default() };
    assert!(matches!(Reader::new(&bytes, limits).err().unwrap().kind,
        ErrorKind::Stream(StreamErrorKind::Limit(LimitKind::Work))));
    for limits in [Limits { handles: 0, ..Limits::default() },
        Limits { allocated_bytes: 8, ..Limits::default() }] {
        let mut reader = Reader::new(&bytes, limits).unwrap();
        reader.read_request().unwrap();
        assert!(reader.read_genetic_code().is_err());
        assert_eq!(reader.read_i32().unwrap_err().kind, ErrorKind::Poisoned);
    }
    let document = StreamReader::new(&bytes[14..], Limits::default());
    assert!(document.is_err(), "A payload continuation is not a fresh ObjectStream");
    let limits = Limits::default();
    let mut parser = StreamReader::new(&bytes, limits).unwrap();
    assert!(matches!(parser.read_record().unwrap(), Record::Block { .. }));
    let (document, allocated, work) = parser.read_document_with_usage().unwrap();
    let [Record::Value(Value::Handle(root))] = document.records.as_slice() else {
        panic!("Source GeneticCode root");
    };
    let (typed_allocation, typed_work) = {
        let graph = LegacyGraph::new(&document, limits).unwrap();
        graph.expect_class(*root, "biogenesis.GeneticCode").unwrap();
        let mut stager = GeneticsStager::new(&graph, limits.allocated_bytes).unwrap();
        assert!(stager.genome(Value::Handle(*root)).unwrap().is_some());
        (graph.allocated_bytes(), limits.work - graph.remaining_work())
    };
    assert!(allocated > 1 && work > 1 && typed_allocation > 0 && typed_work > 0);
    for limited in [
        Limits { allocated_bytes: allocated.checked_add(typed_allocation).unwrap() - 1, ..limits },
        Limits { work: work.checked_add(typed_work).unwrap() - 1, ..limits },
    ] {
        assert!(GeneticCodePayload::from_document(document.clone(), limited).unwrap().into_genome().is_ok(),
            "Detached hydration fits the fresh caller budget");
        let mut reader = Reader::new(&bytes, limited).unwrap();
        reader.read_request().unwrap();
        assert_eq!(reader.read_genetic_code().and_then(GeneticCodePayload::into_genome)
            .err().unwrap().kind, ErrorKind::Budget,
            "Parsing and hydration must share the caller budget");
    }
    let empty = Arena::default();
    let values = [i32::MIN; 1024];
    let mut writer = Writer::new(&empty, Limits { work: 5, ..Limits::default() }).unwrap();
    assert_eq!(writer.write_i32s(&values).unwrap_err().kind,
        ErrorKind::Stream(StreamErrorKind::Limit(LimitKind::Work)));
    assert_eq!(writer.position(), 4, "Insufficient work rejects before block output");
    assert_eq!(writer.write_i32(10).unwrap_err().kind, ErrorKind::Poisoned);
    let mut writer = Writer::new(&empty, Limits { allocated_bytes: 4, ..Limits::default() }).unwrap();
    assert_eq!(writer.write_i32s(&values).unwrap_err().kind,
        ErrorKind::Stream(StreamErrorKind::Limit(LimitKind::Allocation)));
    assert_eq!(writer.position(), 4, "Insufficient allocation rejects before block output");
    let mut writer = Writer::new(&empty, Limits::default()).unwrap();
    writer.write_i32s(&values).unwrap();
    let encoded = writer.finish().unwrap();
    assert_eq!(&encoded[4..9], &[0x7a, 0, 0, 16, 0]);
    let mut reader = Reader::new(&encoded, Limits::default()).unwrap();
    for _ in 0..1024 { assert_eq!(reader.read_i32().unwrap(), i32::MIN); }
    reader.finish().unwrap();
    let mut writer = Writer::new(&empty, Limits { output_bytes: 9, ..Limits::default() }).unwrap();
    assert!(writer.write_i32(9).is_err());
    assert_eq!(writer.write_i32(10).unwrap_err().kind, ErrorKind::Poisoned);
    assert_eq!(writer.finish().unwrap_err().kind, ErrorKind::Poisoned);
    let mut writer = Writer::new(&empty, Limits::default()).unwrap();
    assert_eq!(writer.write_genetic_code(Value::Null).unwrap_err().kind, ErrorKind::NullGeneticCode);
    assert_eq!(writer.write_i32(10).unwrap_err().kind, ErrorKind::Poisoned);
}


struct FragmentRead<'a> {
    bytes: &'a [u8],
    maximum: usize,
    delivered: usize,
}
impl std::io::Read for FragmentRead<'_> {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        let count = output.len().min(self.maximum).min(self.bytes.len());
        output[..count].copy_from_slice(&self.bytes[..count]);
        self.bytes = &self.bytes[count..];
        self.delivered += count;
        Ok(count)
    }
}

#[test]
fn all_original_streams_decode_once_through_fragmented_input() {
    let observations = include_str!("../tests/fixtures/network_wire_source/observations.tsv");
    let mut streams = 0;
    for line in observations.lines() {
        let fields: Vec<_> = line.split('\t').collect();
        for kind in ["request", "response"] {
            let bytes = fixture(&format!("{}.{}.bin", fields[1], kind));
            let expected = StreamReader::new(&bytes, Limits::default()).unwrap().read_document().unwrap();
            let mut usage = None;
            for maximum in [1, 3, 17, usize::MAX] {
                let mut input = FragmentRead { bytes: &bytes, maximum, delivered: 0 };
                let (actual, allocated, work) = StreamReader::from_reader(&mut input, Limits::default()).unwrap()
                    .read_document_with_usage().unwrap();
                assert_eq!(actual, expected, "{} {kind} fragment={maximum}", fields[1]);
                assert_eq!(input.delivered, bytes.len(), "No prefix replay or duplicate reads");
                if let Some(expected_usage) = usage { assert_eq!((allocated, work), expected_usage); }
                usage = Some((allocated, work));
                assert_eq!(object_stream::encode_document(&actual, Limits::default()).unwrap(), bytes);
            }
            streams += 1;
        }
    }
    assert_eq!(streams, 98);
}

#[test]
fn incremental_wire_root_preserves_identity_bits_utf16_and_terminal_errors() {
    let bytes = fixture("recv-code-shared.request.bin");
    let mut input = FragmentRead { bytes: &bytes, maximum: 1, delivered: 0 };
    let mut reader = Reader::from_reader(&mut input, Limits::default()).unwrap();
    assert_eq!(reader.read_request().unwrap(), Request::SendCode { netcode: NET });
    let genome = reader.read_genetic_code().unwrap().into_genome().unwrap();
    assert_eq!(reader.read_i32().unwrap_err().kind, ErrorKind::Finished);
    let code = genome.borrow();
    assert!(Rc::ptr_eq(&code.gene_ref(0).unwrap(), &code.gene_ref(1).unwrap()));
    assert_eq!(code.gene_ref(0).unwrap().borrow().theta().to_bits(), (-0.0_f64).to_bits());
    assert_eq!(code.gene_ref(0).unwrap().borrow().color(), Some(0x80402010));
    assert_eq!(code.clade_id().unwrap().units(), &[0x41, 0, 0xd83e, 0xddec, 0xd800, 0x5a]);
    for name in ["recv-code-null", "recv-code-wrong-class"] {
        let bytes = fixture(&format!("{name}.request.bin"));
        let mut input = FragmentRead { bytes: &bytes, maximum: 2, delivered: 0 };
        let mut reader = Reader::from_reader(&mut input, Limits::default()).unwrap();
        reader.read_request().unwrap();
        assert_eq!(reader.read_genetic_code().err().unwrap().kind,
            if name.ends_with("null") { ErrorKind::NullGeneticCode } else { ErrorKind::WrongGeneticCode });
        assert_eq!(reader.read_i32().unwrap_err().kind, ErrorKind::Poisoned);
    }
    let bytes = [0xac, 0xed, 0, 5, 0x77, 12, 0, 0, 0, 8, 0x12, 0x34, 0x56, 0x78, 0, 0, 0, 0];
    let mut input = FragmentRead { bytes: &bytes, maximum: 1, delivered: 0 };
    let mut reader = Reader::from_reader(&mut input, Limits::default()).unwrap();
    reader.read_request().unwrap();
    assert_eq!(reader.read_genetic_code().err().unwrap().kind, ErrorKind::PendingPrimitive);
    assert_eq!(reader.read_i32().unwrap_err().kind, ErrorKind::Poisoned);
}

#[test]
fn incremental_hydration_keeps_parser_work_and_allocation_in_the_same_budget() {
    let bytes = fixture("recv-code-shared.request.bin");
    let limits = Limits::default();
    let mut input = std::io::Cursor::new(&bytes);
    let mut parser = StreamReader::from_reader(&mut input, limits).unwrap();
    parser.read_record().unwrap();
    let (document, allocated, work) = parser.read_one_with_usage().unwrap();
    let root = match document.records[0] { Record::Value(value) => value, _ => panic!("GeneticCode root") };
    let (typed_allocation, typed_work) = {
        let graph = LegacyGraph::new(&document, limits).unwrap();
        let mut stager = GeneticsStager::new(&graph, limits.allocated_bytes).unwrap();
        stager.genome(root).unwrap().unwrap();
        (graph.allocated_bytes(), limits.work - graph.remaining_work())
    };
    for limited in [Limits { allocated_bytes: allocated + typed_allocation - 1, ..limits },
        Limits { work: work + typed_work - 1, ..limits }] {
        let mut input = std::io::Cursor::new(&bytes);
        let mut reader = Reader::from_reader(&mut input, limited).unwrap();
        reader.read_request().unwrap();
        assert_eq!(reader.read_genetic_code().and_then(GeneticCodePayload::into_genome).err().unwrap().kind,
            ErrorKind::Budget);
    }
}
