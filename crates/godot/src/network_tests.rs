// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use biogenesis_core::{ColorKind, Gene, Genome, GenomeProperties};
use std::sync::{Arc, atomic::AtomicBool};

struct Fixture {
    world: World,
    settings: NetworkSettings,
    network: Network,
    sockets: bool,
}

impl Fixture {
    fn new(seed: i64, size: i32) -> Self {
        let world = World::new(WorldConfig {
            seed,
            width: size,
            height: size,
            initial_attempts: 0,
        })
        .unwrap();
        let settings = NetworkSettings {
            accept_connections: true,
            local_port: 0,
            max_connections: 2,
            user_name: "Local".into(),
        };
        let mut network = Network::default();
        network.root = Some(world.handle());
        network.configure_world(&world, &settings);
        Self {
            world,
            settings,
            network,
            sockets: false,
        }
    }

    fn listen(&mut self) -> SocketAddr {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let address = listener.local_addr().unwrap();
        self.network.listener = Some(
            network_io::Listener::spawn(
                vec![listener],
                self.network.generation,
                self.network.sender.clone(),
                limits(),
            )
            .unwrap(),
        );
        self.network.bound_port = Some(address.port());
        self.sockets = true;
        address
    }

    fn decide(&mut self, version: i32, port: i32, netcode: i32) -> Admission {
        self.network
            .decide(
                &self.world,
                &self.settings,
                SocketAddr::from(([127, 0, 0, 1], 43210)),
                Request::Connect {
                    version,
                    port,
                    netcode,
                },
            )
            .unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if !self.sockets {
            // Policy-only peers have no owned remote endpoint to notify
            for connection in self.network.connections.drain(..) {
                connection.peer.disconnect();
            }
        }
        self.network.stop();
    }
}

#[test]
fn admission_preserves_source_precedence_and_only_admitted_peers_draw_corridor_rng() {
    let mut fixture = Fixture::new(4721, 7000);
    let before = fixture.world.random_snapshot();
    fixture.settings.accept_connections = false;
    fixture.settings.max_connections = 0;
    assert_eq!(fixture.decide(0, -1, 1).decision, ServerDecision::Reply(6));
    fixture.settings.accept_connections = true;
    assert_eq!(fixture.decide(0, -1, 1).decision, ServerDecision::Reply(4));
    fixture.settings.max_connections = 1;
    assert_eq!(fixture.decide(0, -1, 1).decision, ServerDecision::Reply(16));
    assert!(
        fixture
            .network
            .decide(
                &fixture.world,
                &fixture.settings,
                SocketAddr::from(([127, 0, 0, 1], 43210)),
                Request::Connect {
                    version: 900,
                    port: 65536,
                    netcode: 1
                }
            )
            .is_err()
    );
    assert_eq!(fixture.world.random_snapshot(), before);
    assert_eq!(
        fixture.decide(900, 34567, 1).decision,
        ServerDecision::Reply(2)
    );
    let after = fixture.world.random_snapshot();
    assert_ne!(after, before);
    fixture.settings.accept_connections = false;
    assert_eq!(fixture.decide(0, -1, 1).decision, ServerDecision::Reply(3));
    fixture.settings.accept_connections = true;
    assert_eq!(
        fixture.decide(0, 34567, 2).decision,
        ServerDecision::Reply(4)
    );
    fixture.settings.max_connections = 2;
    assert_eq!(
        fixture.decide(0, 34567, 2).decision,
        ServerDecision::Reply(16)
    );
    assert_eq!(
        fixture.decide(900, 34567, 2).decision,
        ServerDecision::Reply(3)
    );
    assert_eq!(fixture.network.connections.len(), 1);
    assert_eq!(fixture.world.random_snapshot(), after);
    assert!(fixture.world.paused());
    let admission = fixture
        .network
        .decide(
            &fixture.world,
            &fixture.settings,
            SocketAddr::from(([127, 0, 0, 1], 43210)),
            Request::SendCode { netcode: 1 },
        )
        .unwrap();
    assert_eq!(admission.decision, ServerDecision::ReceiveCode);
}

#[test]
fn configured_inbound_maximum_admits_seventeen_and_preserves_refusal_order() {
    let mut fixture = Fixture::new(8721, 256);
    fixture.settings.max_connections = 17;
    for index in 0..17 {
        assert_eq!(fixture.decide(900, 30000 + index, index + 1).decision, ServerDecision::Reply(2),
                   "Configured maximum17 must admit peer{}", index + 1);
        assert_eq!(fixture.network.connections.len(), (index + 1) as usize);
    }
    let random = fixture.world.random_snapshot();
    assert_eq!(fixture.decide(900, 30017, 18).decision, ServerDecision::Reply(4));
    assert_eq!(fixture.decide(0, -1, 18).decision, ServerDecision::Reply(4));
    fixture.settings.accept_connections = false;
    assert_eq!(fixture.decide(0, -1, 1).decision, ServerDecision::Reply(3));
    assert_eq!(fixture.decide(0, -1, 18).decision, ServerDecision::Reply(6));
    fixture.settings.accept_connections = true;
    fixture.settings.max_connections = 18;
    assert_eq!(fixture.decide(0, -1, 18).decision, ServerDecision::Reply(16));
    assert_eq!(fixture.decide(900, 30000, 18).decision, ServerDecision::Reply(3));
    assert_eq!(fixture.network.connections.len(), 17);
    assert_eq!(fixture.world.random_snapshot(), random);
    assert!(fixture.network.connections.iter().all(|connection| connection.peer.state() == 51));
}

#[test]
fn colliding_netcodes_select_first_record_and_disconnect_cleanup_accepts_an_expired_terminal_deadline()
 {
    let mut fixture = Fixture::new(5721, 7000);
    let now = Instant::now();
    for (id, connected) in [(1, false), (2, true)] {
        let peer = fixture.world.network_peer_inbound(42).unwrap();
        if connected {
            peer.connect();
        }
        fixture.network.connections.push(Connection {
            id,
            address: SocketAddr::from(([127, 0, 0, 1], 34567)),
            peer,
            last_response: now,
            next_check: now + KEEP_ALIVE,
        });
    }
    let remote = SocketAddr::from(([127, 0, 0, 1], 43210));
    let request = Request::Disconnect { netcode: 42 };
    assert_eq!(
        fixture
            .network
            .decide(&fixture.world, &fixture.settings, remote, request)
            .unwrap()
            .decision,
        ServerDecision::Reply(7)
    );
    fixture.network.connections[0].peer.connect();
    let decision = fixture
        .network
        .decide(&fixture.world, &fixture.settings, remote, request)
        .unwrap();
    assert_eq!(decision.decision, ServerDecision::Reply(15));
    assert_eq!(decision.peer, Some(1));
    assert_eq!(fixture.network.connections.len(), 2);
    let terminal = |generation| Rpc {
        generation,
        deadline: now - Duration::from_secs(1),
        cancelled: Arc::new(AtomicBool::new(false)),
        call: Call::Received {
            peer: Some(1),
            request: Some(request),
            result: Ok(()),
        },
    };
    fixture.network.rpc(
        &fixture.world,
        &fixture.settings,
        terminal(fixture.network.generation + 1),
    );
    assert_eq!(fixture.network.connections.len(), 2);
    fixture.network.rpc(
        &fixture.world,
        &fixture.settings,
        terminal(fixture.network.generation),
    );
    assert_eq!(fixture.network.connections.len(), 1);
    assert_eq!(fixture.network.connections[0].id, 2);
    fixture.network.rpc(
        &fixture.world,
        &fixture.settings,
        terminal(fixture.network.generation),
    );
    assert_eq!(fixture.network.connections.len(), 1);
}

fn pump_until(a: &mut Fixture, b: &mut Fixture, ready: impl Fn(&Fixture, &Fixture) -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        a.network.pump(&a.world, &a.settings).unwrap();
        b.network.pump(&b.world, &b.settings).unwrap();
        if ready(a, b) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "Network fixture deadline; A: {}; B: {}",
            a.network.error,
            b.network.error
        );
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn pump_many_until(
    hub: &mut Fixture,
    peers: &mut [Fixture],
    deadline: Instant,
    ready: impl Fn(&Fixture, &[Fixture]) -> bool,
) {
    loop {
        hub.network.pump(&hub.world, &hub.settings).unwrap();
        assert!(hub.network.jobs.len() <= JOB_LIMIT);
        for peer in peers.iter_mut() {
            peer.network.pump(&peer.world, &peer.settings).unwrap();
            assert!(peer.network.jobs.len() <= JOB_LIMIT);
        }
        if ready(hub, peers) {
            return;
        }
        assert!(Instant::now() < deadline, "Many-peer fixture deadline; hub: {}; peers: {:?}",
                hub.network.error, peers.iter().map(|peer| &peer.network.error).collect::<Vec<_>>());
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn owned_loopback_transport_exceeds_sixteen_and_bulk_stop_notifies_every_peer() {
    let connected = |fixture: &Fixture, count: usize| {
        fixture.network.connections.len() == count && fixture.network.jobs.is_empty()
            && fixture.network.error.is_empty()
            && fixture.network.connections.iter().all(|connection| connection.peer.state() == 51)
    };
    for outbound in [false, true] {
        let mut hub = Fixture::new(9721, 256);
        hub.settings.accept_connections = !outbound;
        hub.settings.max_connections = if outbound { 0 } else { 17 };
        let address = hub.listen();
        let mut peers = Vec::new();
        let mut addresses = Vec::new();
        let deadline = Instant::now() + Duration::from_secs(10);
        let peer_count = if outbound { 17 } else { 18 };
        for index in 0..peer_count {
            let mut peer = Fixture::new(10721 + index as i64, 256);
            peer.settings.accept_connections = outbound;
            peer.settings.max_connections = if outbound { 1 } else { 0 };
            let remote = peer.listen();
            addresses.push(remote);
            if outbound {
                if index == 4 {
                    assert_eq!(hub.network.jobs.len(), 4);
                    let random = hub.world.random_snapshot();
                    assert_eq!(hub.network.connect(&hub.world, &hub.settings, remote), Err(NetworkFeedback::Capacity));
                    assert_eq!(hub.network.connect(&hub.world, &hub.settings, addresses[0]), Err(NetworkFeedback::Duplicate));
                    assert_eq!(hub.network.connections.len(), 4);
                    assert_eq!(hub.world.random_snapshot(), random);
                    pump_many_until(&mut hub, &mut peers, deadline, |hub, peers| {
                        connected(hub, 4) && peers.iter().all(|peer| connected(peer, 1))
                    });
                }
                hub.network.connect(&hub.world, &hub.settings, remote).unwrap();
            } else {
                peer.network.connect(&peer.world, &peer.settings, address).unwrap();
            }
            let random = hub.world.random_snapshot();
            peers.push(peer);
            if outbound && index < 4 {
                continue;
            }
            if !outbound && index == 17 {
                pump_many_until(&mut hub, &mut peers, deadline, |hub, peers| {
                    connected(hub, 17) && peers[17].network.connections.is_empty()
                        && peers[17].network.jobs.is_empty()
                        && peers[17].network.error_feedback.as_ref() == Some(&NetworkFeedback::Refused(4))
                });
                assert_eq!(hub.world.random_snapshot(), random);
                assert!(peers[..17].iter().all(|peer| connected(peer, 1)));
            } else {
                pump_many_until(&mut hub, &mut peers, deadline, |hub, peers| {
                    connected(hub, index + 1) && peers.iter().all(|peer| connected(peer, 1))
                });
            }
        }
        assert!(connected(&hub, 17));
        if outbound {
            hub.settings.accept_connections = true;
            hub.settings.max_connections = 17;
            let netcode = (100000..100100).find(|candidate| {
                hub.network.connections.iter().all(|connection| connection.peer.netcode() != *candidate)
            }).unwrap();
            let random = hub.world.random_snapshot();
            assert_eq!(hub.decide(0, -1, netcode).decision, ServerDecision::Reply(4));
            assert_eq!(hub.world.random_snapshot(), random);
        }
        assert!(!hub.world.advance_corridor_presentation().unwrap().0.is_empty());
        hub.network.stop();
        assert!(hub.network.connections.is_empty() && hub.network.jobs.is_empty());
        assert!(hub.network.listener.is_none() && hub.network.root.is_none());
        assert!(hub.network.presentation.is_empty());
        assert!(hub.world.advance_corridor_presentation().unwrap().0.is_empty());
        assert!(hub.network.closing.is_some(), "Bulk Stop must notify all17 connected peers");
        loop {
            for peer in peers.iter_mut() {
                peer.network.pump(&peer.world, &peer.settings).unwrap();
                assert!(peer.network.jobs.len() <= JOB_LIMIT);
            }
            let closed = hub.network.poll_close();
            assert!(hub.network.cleanup_error.is_none(), "{:?}", hub.network.cleanup_error);
            if closed && peers.iter().all(|peer| peer.network.connections.is_empty() && peer.network.jobs.is_empty()) {
                break;
            }
            assert!(Instant::now() < deadline, "Bulk Stop fixture deadline");
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(hub.network.notice_feedback, Some(NetworkFeedback::AllClosed));
        assert!(hub.network.error.is_empty());
        for peer in &peers {
            assert!(peer.world.advance_corridor_presentation().unwrap().0.is_empty());
        }
        for fixture in peers.iter_mut().chain(std::iter::once(&mut hub)) {
            fixture.network.stop();
            assert!(fixture.network.connections.is_empty() && fixture.network.jobs.is_empty());
            assert!(fixture.network.listener.is_none() && fixture.network.closing.is_none());
            assert!(fixture.network.error.is_empty() && fixture.network.cleanup_error.is_none());
        }
    }
}

#[test]
fn two_owned_local_instances_transfer_live_genome_while_receiver_is_paused_then_disconnect_without_workers()
 {
    let mut a = Fixture::new(6721, 101);
    let mut b = Fixture::new(7721, 7000);
    let gene = Gene::new(8.0, 0.0, ColorKind::Green.argb(), 0, [0; 36]).shared();
    let mut genome = Genome::from_parts(
        &[gene.clone()],
        GenomeProperties {
            symmetry: 1,
            ..Default::default()
        },
    );
    genome.set_clade_id(Some("root".into()));
    let actor = a.world.paste_genome(genome, [50, 50]).unwrap().unwrap();
    let address_a = a.listen();
    let address_b = b.listen();
    a.network.connect(&a.world, &a.settings, address_b).unwrap();
    pump_until(&mut a, &mut b, |a, b| {
        a.network.connections.len() == 1
            && b.network.connections.len() == 1
            && a.network.connections[0].peer.state() == 51
            && b.network.connections[0].peer.state() == 51
    });
    assert_eq!(b.network.connections[0].address, address_a);
    let random = b.world.random_snapshot();
    for _ in 0..16 {
        a.world.single_step().unwrap();
        if a.network.connections[0].peer.pending_outgoing() != 0 {
            break;
        }
    }
    assert_eq!(a.network.connections[0].peer.pending_outgoing(), 1);
    assert!(a.world.inspect(actor).unwrap().is_none());
    assert_eq!(a.world.population(), 0);
    a.network.pump(&a.world, &a.settings).unwrap();
    let export_deadline = Instant::now() + Duration::from_secs(10);
    loop {
        b.network.pump(&b.world, &b.settings).unwrap();
        if let Ok(rpc) = a.network.receiver.try_recv() {
            let exporting = matches!(rpc.call, Call::Export { .. });
            if exporting {
                gene.borrow_mut().set_length(11.0);
            }
            a.network.rpc(&a.world, &a.settings, rpc);
            if exporting {
                break;
            }
        }
        assert!(
            Instant::now() < export_deadline,
            "Actual WAITING_CODE did not request live export"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    pump_until(&mut a, &mut b, |a, b| {
        a.network.connections[0].peer.pending_outgoing() == 0
            && b.network.connections[0].peer.incoming_counts() == (1, false)
    });
    assert!(b.world.paused());
    assert_eq!(b.world.population(), 0);
    assert_eq!(b.world.random_snapshot(), random);
    b.world.single_step().unwrap();
    assert_eq!(b.world.population(), 1);
    assert_eq!(b.network.connections[0].peer.incoming_counts(), (0, false));
    let born = b.world.render_segments().unwrap()[0].organism_id;
    let received = b.world.inspect(born).unwrap().unwrap();
    let received = received.genotype.unwrap();
    assert_eq!(
        received.clade_id().unwrap().to_utf8().unwrap(),
        "Local:root"
    );
    assert_eq!(received.gene_ref(0).unwrap().borrow().length(), 11.0);
    let id = a.network.connections[0].id;
    a.network.disconnect(id).unwrap();
    pump_until(&mut a, &mut b, |a, b| {
        a.network.connections.is_empty()
            && b.network.connections.is_empty()
            && a.network.jobs.is_empty()
            && b.network.jobs.is_empty()
    });
    a.network.stop();
    b.network.stop();
    assert!(a.network.listener.is_none() && b.network.listener.is_none());
    assert!(a.network.jobs.is_empty() && b.network.jobs.is_empty());
    let rebound_a = TcpListener::bind(address_a).unwrap();
    let rebound_b = TcpListener::bind(address_b).unwrap();
    drop((rebound_a, rebound_b));
}

#[test]
fn logical_presentation_keeps_thirty_advances_with_fifty_hz_pumps() {
    let mut fixture = Fixture::new(8721, 101);
    let start = fixture.network.last_presentation;
    fixture.network.had_corridors = true;
    let mut advances = 0;
    for frame in 1..=50 {
        advances += usize::from(
            fixture
                .network
                .present_at(&fixture.world, start + Duration::from_millis(frame * 20))
                .unwrap(),
        );
    }
    assert_eq!(advances, 30);
    assert_eq!(
        fixture.network.last_presentation,
        start + PRESENTATION_INTERVAL * 30
    );
}

#[test]
fn failed_worker_remains_visible_after_its_peer_record_is_gone() {
    for forced_release in [false, true] {
        let mut fixture = Fixture::new(9721, 101);
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::Builder::new()
            .name("biogenesis-aborted-peer-fixture".into())
            .spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(5);
                loop {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            stream.shutdown(std::net::Shutdown::Both).unwrap();
                            break;
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                        Err(error) => panic!("Owned fixture accept failed: {error}"),
                    }
                    assert!(
                        Instant::now() < deadline,
                        "Owned fixture did not receive a connection"
                    );
                    std::thread::sleep(Duration::from_millis(2));
                }
            })
            .unwrap();
        fixture
            .network
            .spawn(1, address, Request::KeepAlive { netcode: 42 }, None)
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !fixture.network.jobs.is_empty() {
            if forced_release {
                if fixture.network.jobs[0].io.is_finished() {
                    fixture.network.force_stop();
                }
            } else {
                fixture
                    .network
                    .pump(&fixture.world, &fixture.settings)
                    .unwrap();
            }
            assert!(
                Instant::now() < deadline,
                "Failed owned worker was not reaped"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
        server.join().unwrap();
        assert!(!fixture.network.error.is_empty());
        assert!(fixture.network.connections.is_empty());
        if forced_release {
            assert_eq!(
                fixture.network.cleanup_error.as_deref(),
                Some(fixture.network.error.as_str())
            );
            assert!(fixture.network.root.is_none());
            let cleanup = fixture.network.cleanup_error.clone();
            fixture.network.fail(NetworkFeedback::Duplicate);
            fixture.network.stop();
            assert!(fixture.network.poll_close());
            assert_eq!(fixture.network.cleanup_error, cleanup);
            assert_eq!(Some(fixture.network.error.clone()), cleanup);
            assert_eq!(fixture.network.error_feedback, cleanup.map(NetworkFeedback::Cleanup));
        }
    }
}

#[test]
fn graceful_close_allows_a_peer_on_the_same_owner_thread_to_acknowledge() {
    let mut a = Fixture::new(10721, 101);
    let mut b = Fixture::new(11721, 101);
    let address_a = a.listen();
    let address_b = b.listen();
    a.network.connect(&a.world, &a.settings, address_b).unwrap();
    pump_until(&mut a, &mut b, |a, b| {
        a.network.connections.len() == 1
            && b.network.connections.len() == 1
            && a.network.connections[0].peer.state() == 51
            && b.network.connections[0].peer.state() == 51
    });
    let duplicate = a.network.connect(&a.world, &a.settings, address_b).unwrap_err();
    assert_eq!(duplicate, NetworkFeedback::Duplicate);
    a.network.fail(duplicate);
    assert_eq!(a.network.error, "This address and port already has a connection");
    assert_eq!(a.network.error_feedback, Some(NetworkFeedback::Duplicate));
    assert_eq!(a.network.connections.len(), 1);
    assert_eq!(b.network.connections.len(), 1);
    a.network.stop();
    assert!(a.network.error.is_empty(), "{}", a.network.error);
    assert!(a.network.error_feedback.is_none());
    assert!(a.network.cleanup_error.is_none());
    assert!(a.network.connections.is_empty() && a.network.listener.is_none());
    assert!(a.network.closing.is_some());
    a.network.fail(NetworkFeedback::ClosingPrevious);
    a.network.stop();
    assert!(a.network.closing.is_some());
    assert!(a.network.error.is_empty(), "{}", a.network.error);
    assert!(a.network.error_feedback.is_none());
    assert!(a.network.cleanup_error.is_none());
    let deadline = Instant::now() + Duration::from_secs(3);
    while !a.network.poll_close() || !b.network.connections.is_empty() {
        b.network.pump(&b.world, &b.settings).unwrap();
        assert!(
            Instant::now() < deadline,
            "Shared owner thread did not complete graceful disconnect"
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(a.network.error.is_empty(), "{}", a.network.error);
    assert!(b.network.connections.is_empty());
    assert!(a.network.root.is_none());
    assert!(a.network.closing.is_none());
    assert!(a.network.error_feedback.is_none());
    assert!(a.network.cleanup_error.is_none());
    let rebound = TcpListener::bind(address_a).unwrap();
    drop(rebound);
}

#[test]
fn keep_alive_threshold_uses_source_integer_milliseconds() {
    for elapsed in [
        Duration::from_millis(29_999),
        Duration::from_millis(30_000),
        Duration::from_millis(30_000) + Duration::from_nanos(999_999),
    ] {
        assert!(!keep_alive_due(elapsed));
    }
    assert!(keep_alive_due(Duration::from_millis(30_001)));
}
