// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use crate::{network_wire::{Reader, Writer}, object_stream::{ErrorKind as StreamKind, LimitKind}};
use std::{net::TcpListener, sync::{Arc, mpsc}, thread};

const NET: i32 = 0x12345678;
const HEADER: [u8; 4] = [0xac, 0xed, 0, 5];

fn options() -> SessionOptions {
    SessionOptions { limits: Limits::default(), deadline: Instant::now() + Duration::from_secs(3),
        read_stall: Duration::from_millis(200), write_stall: Duration::from_millis(200) }
}

fn pair() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).unwrap();
    let client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
    let (server, _) = listener.accept().unwrap();
    for stream in [&client, &server] {
        stream.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        stream.set_write_timeout(Some(Duration::from_secs(2))).unwrap();
    }
    (client, server)
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(crate::oracle_tests::fixture_path("crates/core")
        .join("tests/fixtures/network_wire_source").join(name)).unwrap()
}

fn payload() -> GeneticCodePayload {
    let bytes = fixture("recv-code-shared.request.bin");
    let mut reader = Reader::new(&bytes, Limits::default()).unwrap();
    reader.read_request().unwrap();
    reader.read_genetic_code().unwrap()
}

fn check_payload(payload: GeneticCodePayload) {
    let genome = payload.into_genome().unwrap();
    let code = genome.borrow();
    assert!(std::rc::Rc::ptr_eq(&code.gene_ref(0).unwrap(), &code.gene_ref(1).unwrap()));
    assert_eq!(code.gene_ref(0).unwrap().borrow().length().to_bits(), 3.5_f64.to_bits());
    assert_eq!(code.gene_ref(0).unwrap().borrow().theta().to_bits(), (-0.0_f64).to_bits());
    assert_eq!(code.gene_ref(0).unwrap().borrow().color(), Some(0x80402010));
    assert_eq!(code.clade_id().unwrap().units(), &[0x41, 0, 0xd83e, 0xddec, 0xd800, 0x5a]);
}

#[test]
fn all_four_established_sessions_preserve_publication_before_ack_without_eof() {
    fn requires_send<T: Send>() {}
    requires_send::<GeneticCodePayload>();
    for (request, initial, final_reply) in [
        (Request::Connect { version: 900, port: 7777, netcode: NET }, 2, None),
        (Request::KeepAlive { netcode: NET }, 14, None),
        (Request::SendCode { netcode: NET }, 9, Some(10)),
        (Request::Disconnect { netcode: NET }, 15, None),
    ] {
        let (client, server) = pair();
        let (published, delivered) = mpsc::sync_channel(1);
        let visible = Arc::new(AtomicBool::new(false));
        let receiver_visible = Arc::clone(&visible);
        let receiver = thread::spawn(move || {
            receive(server, options(), &AtomicBool::new(false),
                |actual| {
                    assert_eq!(actual, request);
                    Ok(if matches!(actual, Request::SendCode { .. }) { ServerDecision::ReceiveCode }
                        else { ServerDecision::Reply(initial) })
                },
                |payload| {
                    published.send(payload).unwrap();
                    receiver_visible.store(true, Ordering::SeqCst);
                    Ok(())
                }).unwrap()
        });
        let code = payload();
        let response = send(client, request, Some(&code), options(), &AtomicBool::new(false)).unwrap();
        assert_eq!(response, Reply { initial, final_reply });
        if final_reply.is_some() {
            assert!(visible.load(Ordering::SeqCst), "Final ACK followed ownership publication");
            check_payload(delivered.recv_timeout(Duration::from_secs(1)).unwrap());
        } else { assert!(!visible.load(Ordering::SeqCst)); }
        assert_eq!(receiver.join().unwrap(), request);
    }
}

#[test]
fn unknown_initial_and_final_replies_remain_data_without_retransmission() {
    for request in [Request::Connect { version: -1, port: 65536, netcode: -7 }, Request::SendCode { netcode: NET }] {
        let (client, server) = pair();
        let receiver = thread::spawn(move || receive(server, options(), &AtomicBool::new(false),
            |actual| { assert_eq!(actual, request); Ok(ServerDecision::Reply(99)) },
            |_| panic!("No payload requested")).unwrap());
        assert_eq!(send(client, request, None, options(), &AtomicBool::new(false)).unwrap(),
            Reply { initial: 99, final_reply: None });
        receiver.join().unwrap();
    }
    let (client, server) = pair();
    let peer = thread::spawn(move || {
        let mut source = &server;
        let mut reader = Reader::from_reader(&mut source, Limits::default()).unwrap();
        let empty = Arena::default();
        let mut writer = Writer::new(&empty, Limits::default()).unwrap();
        (&server).write_all(writer.output()).unwrap();
        assert_eq!(reader.read_request().unwrap(), Request::SendCode { netcode: NET });
        // A coalesced source response preserves its unread final primitive
        writer.write_i32s(&[9, 99]).unwrap();
        (&server).write_all(&writer.output()[4..]).unwrap();
        reader.read_genetic_code().unwrap().validate().unwrap();
    });
    let code = payload();
    assert_eq!(send(client, Request::SendCode { netcode: NET }, Some(&code), options(), &AtomicBool::new(false)).unwrap(),
        Reply { initial: 9, final_reply: Some(99) });
    peer.join().unwrap();
}

#[test]
fn caller_cannot_bypass_code_validation_with_waiting_or_success_reply() {
    for forbidden in [9, 10] {
        let (mut client, server) = pair();
        let receiver = thread::spawn(move || receive(server, options(), &AtomicBool::new(false),
            |_| Ok(ServerDecision::Reply(forbidden)), |_| panic!("Publication bypass")));
        let empty = Arena::default();
        let mut writer = Writer::new(&empty, Limits::default()).unwrap();
        client.write_all(writer.output()).unwrap();
        let mut header = [0; 4];
        client.read_exact(&mut header).unwrap();
        assert_eq!(header, HEADER);
        writer.write_request(Request::SendCode { netcode: NET }).unwrap();
        client.write_all(&writer.output()[4..]).unwrap();
        let mut replies = Vec::new();
        client.read_to_end(&mut replies).unwrap();
        assert!(replies.is_empty(), "No WAITING_CODE or CODE_RECEIVED bypass for reply {forbidden}");
        assert_eq!(receiver.join().unwrap().unwrap_err().kind, ErrorKind::UnexpectedDecision);
    }
}

#[test]
fn malformed_roots_and_rejected_publication_never_acknowledge_success() {
    for name in ["recv-code-null.request.bin", "recv-code-wrong-class.request.bin"] {
        let bytes = fixture(name);
        let (mut client, server) = pair();
        let receiver = thread::spawn(move || receive(server, options(), &AtomicBool::new(false),
            |_| Ok(ServerDecision::ReceiveCode), |_| panic!("Invalid genotype published")));
        client.write_all(&bytes[..4]).unwrap();
        let mut header = [0; 4];
        client.read_exact(&mut header).unwrap();
        assert_eq!(header, HEADER);
        client.write_all(&bytes[4..14]).unwrap();
        let mut waiting = [0; 6];
        client.read_exact(&mut waiting).unwrap();
        assert_eq!(waiting, [0x77, 4, 0, 0, 0, 9]);
        client.write_all(&bytes[14..]).unwrap();
        let mut final_reply = Vec::new();
        client.read_to_end(&mut final_reply).unwrap();
        assert!(final_reply.is_empty());
        assert!(matches!(receiver.join().unwrap().unwrap_err().kind, ErrorKind::Wire(_)));
    }
    let (client, server) = pair();
    let receiver = thread::spawn(move || receive(server, options(), &AtomicBool::new(false),
        |_| Ok(ServerDecision::ReceiveCode), |value| { value.validate().unwrap(); Err(AdmissionError) }));
    let code = payload();
    assert!(send(client, Request::SendCode { netcode: NET }, Some(&code), options(), &AtomicBool::new(false)).is_err());
    assert_eq!(receiver.join().unwrap().unwrap_err().kind, ErrorKind::Admission);
    // A later independent session succeeds after the rejected publication
    let (client, server) = pair();
    let receiver = thread::spawn(move || receive(server, options(), &AtomicBool::new(false),
        |_| Ok(ServerDecision::Reply(14)), |_| panic!("No code")));
    assert_eq!(send(client, Request::KeepAlive { netcode: NET }, None, options(), &AtomicBool::new(false)).unwrap().initial, 14);
    receiver.join().unwrap().unwrap();
}

#[test]
fn cumulative_session_budget_rejects_before_publication() {
    use crate::{legacy_graph::LegacyGraph, legacy_genetics::GeneticsStager, object_stream::{Record, StreamReader, Value}};
    let bytes = fixture("recv-code-shared.request.bin");
    let full = Limits::default();
    let mut input = std::io::Cursor::new(&bytes);
    let mut parser = StreamReader::from_reader(&mut input, full).unwrap();
    parser.read_record().unwrap();
    let (document, parser_allocation, parser_work) = parser.read_one_with_usage().unwrap();
    let root = match document.records[0] { Record::Value(Value::Handle(root)) => root, _ => panic!("GeneticCode root") };
    let (root_allocation, root_work) = {
        let graph = LegacyGraph::new(&document, full).unwrap();
        graph.expect_class(root, "biogenesis.GeneticCode").unwrap();
        (graph.allocated_bytes(), full.work - graph.remaining_work())
    };
    let (typed_allocation, typed_work) = {
        let graph = LegacyGraph::new(&document, full).unwrap();
        let mut stager = GeneticsStager::new(&graph, full.allocated_bytes).unwrap();
        stager.genome(Value::Handle(root)).unwrap().unwrap();
        (graph.allocated_bytes(), full.work - graph.remaining_work())
    };
    let empty = Arena::default();
    let mut reply = Writer::new(&empty, full).unwrap();
    reply.write_i32(9).unwrap();
    reply.write_i32(10).unwrap();
    let (reply_allocation, reply_work) = reply.usage();
    for limits in [
        Limits { allocated_bytes: parser_allocation + root_allocation + typed_allocation + reply_allocation - 1, ..full },
        Limits { work: parser_work + root_work + typed_work + reply_work - 1, ..full },
    ] {
        // Each isolated phase fits; only their cumulative socket-session use exceeds the allowance
        let mut input = std::io::Cursor::new(&bytes);
        let mut parser = StreamReader::from_reader(&mut input, limits).unwrap();
        parser.read_record().unwrap();
        parser.read_one_with_usage().unwrap();
        GeneticCodePayload::from_document(document.clone(), limits).unwrap().validate().unwrap();
        let mut reply = Writer::new(&empty, limits).unwrap();
        reply.write_i32(9).unwrap(); reply.write_i32(10).unwrap();
        let (client, server) = pair();
        let published = Arc::new(AtomicBool::new(false));
        let observed = Arc::clone(&published);
        let receiver = thread::spawn(move || {
            let mut configuration = options(); configuration.limits = limits;
            receive(server, configuration, &AtomicBool::new(false), |_| Ok(ServerDecision::ReceiveCode),
                |_| { observed.store(true, Ordering::SeqCst); Ok(()) })
        });
        let code = payload();
        assert!(send(client, Request::SendCode { netcode: NET }, Some(&code), options(), &AtomicBool::new(false)).is_err(),
            "No final success acknowledgment after cumulative exhaustion");
        assert_eq!(receiver.join().unwrap().unwrap_err().kind,
            ErrorKind::Wire(network_wire::WireError { offset: 0, kind: network_wire::ErrorKind::Budget }));
        assert!(!published.load(Ordering::SeqCst));
    }
}

#[test]
fn cancellation_deadline_partial_header_and_read_stall_close_the_socket() {
    let (client, mut peer) = pair();
    let cancelled = AtomicBool::new(true);
    assert_eq!(send(client, Request::KeepAlive { netcode: NET }, None, options(), &cancelled).unwrap_err().kind,
        ErrorKind::Cancelled);
    assert_eq!(peer.read(&mut [0]).unwrap(), 0);
    for partial in [false, true] {
        let (client, mut peer) = pair();
        let hold = thread::spawn(move || {
            let mut header = [0; 4];
            peer.read_exact(&mut header).unwrap();
            if partial { peer.write_all(&HEADER[..2]).unwrap(); }
            thread::sleep(Duration::from_millis(100));
            assert_eq!(peer.read(&mut [0]).unwrap(), 0);
        });
        let mut configuration = options();
        configuration.read_stall = if partial { Duration::from_millis(25) } else { Duration::from_secs(1) };
        configuration.deadline = Instant::now() + if partial { Duration::from_secs(1) } else { Duration::from_millis(25) };
        assert_eq!(send(client, Request::KeepAlive { netcode: NET }, None, configuration, &AtomicBool::new(false)).unwrap_err().kind,
            if partial { ErrorKind::ReadStall } else { ErrorKind::Deadline });
        hold.join().unwrap();
    }
    let (client, mut peer) = pair();
    let cancelled = Arc::new(AtomicBool::new(false));
    let trigger = Arc::clone(&cancelled);
    let hold = thread::spawn(move || {
        let mut header = [0; 4]; peer.read_exact(&mut header).unwrap();
        trigger.store(true, Ordering::SeqCst);
        assert_eq!(peer.read(&mut [0]).unwrap(), 0);
    });
    assert_eq!(send(client, Request::KeepAlive { netcode: NET }, None, options(), &cancelled).unwrap_err().kind,
        ErrorKind::Cancelled);
    hold.join().unwrap();
}

#[test]
fn partial_eof_and_stalled_writes_fail_and_terminate() {
    let (client, mut peer) = pair();
    let hold = thread::spawn(move || {
        let mut header = [0; 4]; peer.read_exact(&mut header).unwrap();
        peer.write_all(&HEADER[..2]).unwrap();
    });
    assert!(matches!(send(client, Request::KeepAlive { netcode: NET }, None, options(), &AtomicBool::new(false)).unwrap_err().kind,
        ErrorKind::Wire(network_wire::WireError { kind: network_wire::ErrorKind::Stream(StreamKind::Truncated { needed: 4 }), .. })));
    hold.join().unwrap();
    let (stream, _peer) = pair();
    let cancelled = AtomicBool::new(false);
    let mut configuration = options();
    configuration.write_stall = Duration::from_millis(25);
    let socket = Socket { stream, options: configuration, cancelled: &cancelled, io_failure: Cell::new(None) };
    let bytes = vec![0; 8 * 1024 * 1024];
    let result = socket.write_all(&bytes);
    assert_eq!(socket.finish(result).unwrap_err().kind, ErrorKind::WriteStall);
}


#[test]
fn session_meters_reject_combined_allocation_and_poison_output() {
    let bytes = fixture("send-connect-2.response.bin");
    let empty = Arena::default();
    let request = Request::Connect { version: 900, port: 7777, netcode: NET };
    let mut reader = Reader::new(&bytes, Limits::default()).unwrap();
    reader.read_i32().unwrap();
    let (allocated, work) = reader.usage();
    let mut writer = Writer::new_charged(&empty, Limits::default(), allocated, work).unwrap();
    writer.write_request(request).unwrap();
    let total = writer.usage().0;
    let limits = Limits { allocated_bytes: total - 1, ..Limits::default() };
    let mut alone = Writer::new(&empty, limits).unwrap();
    alone.write_request(request).unwrap();
    let mut reader = Reader::new(&bytes, limits).unwrap();
    reader.read_i32().unwrap();
    let (allocated, work) = reader.usage();
    let mut combined = Writer::new_charged(&empty, limits, allocated, work).unwrap();
    assert_eq!(combined.write_request(request).unwrap_err().kind,
        network_wire::ErrorKind::Stream(StreamKind::Limit(LimitKind::Allocation)));
    assert_eq!(combined.write_i32(10).unwrap_err().kind, network_wire::ErrorKind::Poisoned);
    assert_eq!(combined.finish().unwrap_err().kind, network_wire::ErrorKind::Poisoned);
}

#[test]
fn slow_partial_progress_cannot_restart_the_total_deadline() {
    let (client, mut peer) = pair();
    let hold = thread::spawn(move || {
        let mut header = [0; 4]; peer.read_exact(&mut header).unwrap();
        for byte in HEADER {
            if peer.write_all(&[byte]).is_err() { break; }
            thread::sleep(Duration::from_millis(25));
        }
    });
    let mut configuration = options();
    configuration.deadline = Instant::now() + Duration::from_millis(50);
    configuration.read_stall = Duration::from_secs(1);
    assert_eq!(send(client, Request::KeepAlive { netcode: NET }, None, configuration, &AtomicBool::new(false)).unwrap_err().kind,
        ErrorKind::Deadline);
    hold.join().unwrap();
}


#[test]
fn missing_outgoing_code_terminates_both_session_ends() {
    let (client, server) = pair();
    let receiver = thread::spawn(move || receive(server, options(), &AtomicBool::new(false),
        |_| Ok(ServerDecision::ReceiveCode), |_| panic!("Missing payload published")));
    assert_eq!(send(client, Request::SendCode { netcode: NET }, None, options(), &AtomicBool::new(false)).unwrap_err().kind,
        ErrorKind::MissingGeneticCode);
    assert!(receiver.join().unwrap().is_err());
}

#[test]
fn delayed_provider_waits_for_code_request_and_preserves_one_stream() {
    let (client, server) = pair();
    let (provided, calls) = mpsc::sync_channel(1);
    let mut configuration = options();
    configuration.read_stall = Duration::from_secs(1);
    let sender = thread::spawn(move || send_with_provider(client, Request::SendCode { netcode: NET },
        configuration, &AtomicBool::new(false), || {
            provided.send(()).unwrap();
            Ok(payload())
        }));
    let empty = Arena::default();
    let mut writer = Writer::new(&empty, Limits::default()).unwrap();
    let mut source = &server;
    let mut reader = Reader::from_reader(&mut source, Limits::default()).unwrap();
    (&server).write_all(writer.output()).unwrap();
    assert_eq!(reader.read_request().unwrap(), Request::SendCode { netcode: NET });
    assert!(matches!(calls.recv_timeout(Duration::from_millis(25)), Err(mpsc::RecvTimeoutError::Timeout)));
    writer.write_i32(9).unwrap();
    (&server).write_all(&writer.output()[4..]).unwrap();
    calls.recv_timeout(Duration::from_secs(1)).unwrap();
    check_payload(reader.read_genetic_code().unwrap());
    let start = writer.position();
    writer.write_i32(10).unwrap();
    (&server).write_all(&writer.output()[start..]).unwrap();
    assert_eq!(sender.join().unwrap().unwrap(), Reply { initial: 9, final_reply: Some(10) });
    assert!(matches!(calls.try_recv(), Err(mpsc::TryRecvError::Disconnected)));
}

#[test]
fn refused_and_non_code_commands_never_invoke_the_provider() {
    for (request, reply) in [
        (Request::SendCode { netcode: NET }, 99),
        (Request::Connect { version: 900, port: 7777, netcode: NET }, 2),
        (Request::KeepAlive { netcode: NET }, 9),
        (Request::Disconnect { netcode: NET }, 15),
    ] {
        let (client, server) = pair();
        let receiver = thread::spawn(move || receive(server, options(), &AtomicBool::new(false),
            |actual| { assert_eq!(actual, request); Ok(ServerDecision::Reply(reply)) },
            |_| panic!("No code requested")).unwrap());
        let called = std::rc::Rc::new(Cell::new(false));
        assert_eq!(send_with_provider(client, request, options(), &AtomicBool::new(false), || {
            called.set(true);
            Ok(payload())
        }).unwrap(), Reply { initial: reply, final_reply: None });
        assert!(!called.get());
        receiver.join().unwrap();
    }
}

#[test]
fn provider_failure_and_cancellation_send_no_object_and_close_the_session() {
    for mode in 0..3 {
        let (client, server) = pair();
        let cancelled = Arc::new(AtomicBool::new(false));
        let trigger = Arc::clone(&cancelled);
        let peer = thread::spawn(move || {
            let mut source = &server;
            let mut reader = Reader::from_reader(&mut source, Limits::default()).unwrap();
            let empty = Arena::default();
            let mut writer = Writer::new(&empty, Limits::default()).unwrap();
            (&server).write_all(writer.output()).unwrap();
            assert_eq!(reader.read_request().unwrap(), Request::SendCode { netcode: NET });
            if mode == 1 { trigger.store(true, Ordering::SeqCst); }
            writer.write_i32(9).unwrap();
            if let Err(error) = (&server).write_all(&writer.output()[4..]) {
                assert!(mode == 1 && matches!(error.kind(), io::ErrorKind::BrokenPipe
                    | io::ErrorKind::ConnectionReset | io::ErrorKind::ConnectionAborted | io::ErrorKind::NotConnected));
            }
            assert!(reader.read_genetic_code().is_err());
        });
        let called = Cell::new(0);
        let error = send_with_provider(client, Request::SendCode { netcode: NET }, options(), &cancelled, || {
            called.set(called.get() + 1);
            if mode == 0 { return Err(failure(ErrorKind::Io(io::ErrorKind::BrokenPipe))); }
            cancelled.store(true, Ordering::SeqCst);
            Ok(payload())
        }).unwrap_err();
        assert_eq!(error.kind, if mode == 0 { ErrorKind::Io(io::ErrorKind::BrokenPipe) } else { ErrorKind::Cancelled });
        assert_eq!(called.get(), if mode == 1 { 0 } else { 1 });
        peer.join().unwrap();
    }
}

#[test]
fn delayed_payload_keeps_the_request_bytes_in_its_output_limit() {
    let code = payload();
    let mut isolated = Writer::new(&code.document().arena, Limits::default()).unwrap();
    isolated.write_genetic_code(code.root()).unwrap();
    let limit = isolated.position();
    drop(isolated);
    let (client, server) = pair();
    let receiver = thread::spawn(move || receive(server, options(), &AtomicBool::new(false),
        |_| Ok(ServerDecision::ReceiveCode), |_| panic!("Exhausted output published")));
    let mut configuration = options();
    configuration.limits.output_bytes = limit;
    let error = send_with_provider(client, Request::SendCode { netcode: NET }, configuration,
        &AtomicBool::new(false), || Ok(code)).unwrap_err();
    assert!(matches!(error.kind, ErrorKind::Wire(network_wire::WireError {
        kind: network_wire::ErrorKind::Stream(StreamKind::Limit(LimitKind::Output)), ..
    })));
    assert!(receiver.join().unwrap().is_err());
}
