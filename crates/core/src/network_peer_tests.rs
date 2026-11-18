// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use crate::{ColorKind, Gene, GenomeProperties, JavaText};

fn world(size: i32) -> World {
    let world = World::new(WorldConfig { seed: 827364, width: size, height: size, initial_attempts: 0 }).unwrap();
    world.configure_network_outgoing(OutgoingConfig {
        user_name: "Local".into(), pending_per_connection: NonZeroUsize::new(2).unwrap(),
    });
    world
}

fn fixture_payload(label: &str) -> GeneticCodePayload {
    let bytes = include_bytes!("../tests/fixtures/network_wire_source/recv-code-shared.request.bin");
    let mut reader = crate::network_wire::Reader::new(bytes, Limits::default()).unwrap();
    reader.read_request().unwrap();
    let genome = reader.read_genetic_code().unwrap().into_genome().unwrap();
    genome.borrow_mut().set_clade_id(Some(label.into()));
    GeneticCodePayload::from_genome(&genome, Limits::default()).unwrap()
}

#[test]
fn peer_construction_consumes_shared_rng_in_source_identifier_and_corridor_order() {
    for size in [99, 7000] {
        let world = world(size);
        let mut expected = world.random_snapshot();
        let netcode = expected.next_i32();
        let positions: Vec<_> = (0..2).map(|_| if size <= 100 {
            [0, 0]
        } else {
            [expected.next_bounded(size - 100).unwrap(), expected.next_bounded(size - 100).unwrap()]
        }).collect();
        let peer = world.network_peer_outbound().unwrap();
        assert_eq!(peer.netcode(), netcode);
        assert_eq!(peer.state(), 52);
        for (bounds, position) in peer.bounds().into_iter().zip(positions) {
            assert_eq!([bounds.x, bounds.y], position);
            assert_eq!([bounds.width, bounds.height], [size.min(100); 2]);
        }
        assert_eq!(world.random_snapshot(), expected);
        assert!(world.advance_presentation().unwrap().is_empty());
        for _ in 0..2 {
            if size > 100 {
                expected.next_bounded(size - 100).unwrap();
                expected.next_bounded(size - 100).unwrap();
            }
        }
        let inbound = world.network_peer_inbound(-7).unwrap();
        assert_eq!(inbound.netcode(), -7);
        assert_eq!(world.random_snapshot(), expected);
    }
}

#[test]
fn peer_transitions_keep_first_rectangle_equal_removal_and_supplied_rectangle_growth() {
    let world = world(99);
    let first = world.network_peer_inbound(1).unwrap();
    let second = world.network_peer_inbound(2).unwrap();
    assert!(first.connect());
    assert!(second.connect());
    assert!(!second.connect());
    assert_eq!(world.incoming_corridors.borrow().len(), 2);
    assert_eq!(world.outgoing_corridors.borrow().len(), 2);
    let dirty = second.disconnect().unwrap();
    assert_eq!(dirty, [IntRect { x: 0, y: 0, width: 100, height: 100 }; 2]);
    assert_eq!(second.bounds(), dirty);
    assert!(Rc::ptr_eq(&world.incoming_corridors.borrow()[0], &second.incoming));
    assert!(Rc::ptr_eq(&world.outgoing_corridors.borrow()[0], &second.outgoing));
    assert_eq!(first.bounds(), [IntRect { x: 0, y: 0, width: 99, height: 99 }; 2]);
    assert_eq!(second.state(), 50);
    assert_eq!(second.disconnect(), None);
    assert!(world.corridors_exist.get());
}

#[test]
fn peer_identity_survives_new_game_and_cleans_only_its_old_root_after_reset() {
    let mut world = world(7000);
    let peer = world.network_peer_outbound().unwrap();
    peer.connect();
    world.new_game(7000, 7000, 0, world.parameters()).unwrap();
    assert!(peer.belongs_to(&world));
    assert_eq!(world.incoming_corridors.borrow().len(), 1);
    assert_eq!(world.outgoing_corridors.borrow().len(), 1);
    let old = peer.world.alias();
    let same_session = World::with_session(
        WorldConfig { seed: 827364, width: 7000, height: 7000, initial_attempts: 0 }, world.session(),
    ).unwrap();
    assert!(old.session().same_owner(&same_session.session()));
    assert!(!peer.belongs_to(&same_session));
    world.reset().unwrap();
    assert!(!peer.belongs_to(&world));
    assert!(peer.belongs_to(&old));
    let random = world.random_snapshot();
    peer.disconnect().unwrap();
    assert!(old.incoming_corridors.borrow().is_empty());
    assert!(old.outgoing_corridors.borrow().is_empty());
    assert!(world.incoming_corridors.borrow().is_empty());
    assert!(world.outgoing_corridors.borrow().is_empty());
    assert_eq!(world.random_snapshot(), random);
}

#[test]
fn live_ticket_export_preserves_raw_username_and_separates_terminal_and_presentation_owners() {
    let mut world = world(7000);
    let mut expected_parameters = world.parameters();
    let config = OutgoingConfig {
        user_name: JavaText::from_units(vec![0xd800, 0x58]),
        pending_per_connection: NonZeroUsize::new(2).unwrap(),
    };
    expected_parameters.outgoing = Some(config.clone());
    let random = world.random_snapshot();
    world.configure_network_outgoing(config);
    assert_eq!(world.parameters(), expected_parameters);
    assert_eq!(world.random_snapshot(), random);
    let mut code = Genome::from_parts(
        &[Gene::new(8.0, 0.0, ColorKind::Green.argb(), 0, [0; 36]).shared()],
        GenomeProperties { symmetry: 1, ..Default::default() },
    );
    code.set_clade_id(Some("root".into()));
    world.paste_genome(code, [1000, 1000]).unwrap().unwrap();
    let actor = Rc::clone(world.organisms.borrow().last().unwrap());
    let genome = Rc::clone(actor.borrow().genome.as_ref().unwrap());
    let peer = world.network_peer_outbound().unwrap();
    peer.connect();
    assert!(world.send_outgoing(&peer.outgoing, &actor, &mut actor.borrow_mut()).unwrap());
    let mut units = vec![0xd800, 0x58, 58, 114, 111, 111, 116];
    assert_eq!(genome.borrow().clade_id().unwrap().units(), units.as_slice());
    assert_eq!(peer.pending_outgoing(), 1);
    let ticket = peer.claim().unwrap();
    assert!(peer.claim().is_none());
    let foreign = world.network_peer_inbound(9).unwrap();
    assert!(matches!(foreign.export(&ticket, Limits::default()), Err(NetworkPeerError::WrongPeer)));
    assert_eq!(foreign.finish(&ticket, NetworkTransferOutcome::Failed), Err(NetworkPeerError::WrongPeer));
    units.push(0xdc00);
    genome.borrow_mut().set_clade_text(Some(JavaText::from_units(units.clone())));
    let random = world.random_snapshot();
    let detached = peer.export(&ticket, Limits::default()).unwrap().into_genome().unwrap();
    assert_eq!(detached.borrow().clade_id().unwrap().units(), units.as_slice());
    assert!(!Rc::ptr_eq(&detached, &genome));
    assert_eq!(world.random_snapshot(), random);
    peer.finish(&ticket, NetworkTransferOutcome::Acknowledged).unwrap();
    assert_eq!(peer.pending_outgoing(), 0);
    assert!(matches!(peer.export(&ticket, Limits::default()), Err(NetworkPeerError::MissingTicket)));
    assert_eq!(peer.finish(&ticket, NetworkTransferOutcome::Cancelled), Err(NetworkPeerError::MissingTicket));
    assert!(peer.outgoing.borrow().traveller.is_some());
    for _ in 0..17 { world.draw_outgoing(&peer.outgoing).unwrap(); }
    assert!(peer.outgoing.borrow().can_send());
}

#[test]
fn incoming_publication_keeps_fifo_owners_and_counts_retained_candidates_against_capacity() {
    let world = world(7000);
    let peer = world.network_peer_inbound(1).unwrap();
    peer.connect();
    let capacity = NonZeroUsize::new(2).unwrap();
    let random = world.random_snapshot();
    peer.publish(fixture_payload("first"), capacity).unwrap();
    peer.publish(fixture_payload("second"), capacity).unwrap();
    assert_eq!(peer.incoming_counts(), (2, false));
    assert_eq!(world.population(), 0);
    assert_eq!(world.random_snapshot(), random);
    assert_eq!(peer.publish(fixture_payload("third"), capacity), Err(NetworkPeerError::IncomingFull));
    peer.incoming.borrow_mut().bounds.width = 0;
    assert!(matches!(world.single_step(), Err(WorldError::InvalidCorridorDimension { axis: "width", value: 0 })));
    assert_eq!(peer.incoming_counts(), (1, true));
    assert_eq!(peer.publish(fixture_payload("third"), capacity), Err(NetworkPeerError::IncomingFull));
    let incoming = peer.incoming.borrow();
    let candidate = incoming.candidate.as_ref().unwrap().borrow();
    let first = candidate.genome.as_ref().unwrap().borrow();
    let second = incoming.pending.front().unwrap().as_ref().unwrap().borrow();
    assert_eq!(first.clade_id().unwrap().units(), "first".encode_utf16().collect::<Vec<_>>().as_slice());
    assert_eq!(second.clade_id().unwrap().units(), "second".encode_utf16().collect::<Vec<_>>().as_slice());
    assert!(Rc::ptr_eq(&first.gene_ref(0).unwrap(), &first.gene_ref(1).unwrap()));
    assert_eq!(first.gene_ref(0).unwrap().borrow().theta().to_bits(), (-0.0_f64).to_bits());
    assert_eq!(first.gene_ref(0).unwrap().borrow().color(), Some(0x80402010));
    assert_eq!(world.population(), 0);
}

#[test]
fn abandoned_ticket_cancels_pending_ownership_without_releasing_the_traveller_or_rolling_back_biology() {
    let mut world = world(7000);
    let code = Genome::from_parts(
        &[Gene::new(8.0, 0.0, ColorKind::Green.argb(), 0, [0; 36]).shared()],
        GenomeProperties { symmetry: 1, ..Default::default() },
    );
    world.paste_genome(code, [1000, 1000]).unwrap().unwrap();
    let actor = Rc::clone(world.organisms.borrow().last().unwrap());
    let peer = world.network_peer_outbound().unwrap();
    peer.connect();
    assert!(world.send_outgoing(&peer.outgoing, &actor, &mut actor.borrow_mut()).unwrap());
    let after_send = (world.population(), actor.borrow().state.alive, actor.borrow().state.energy.to_bits(),
        world.oxygen.get().to_bits(), world.carbon_dioxide.get().to_bits());
    assert_eq!(peer.pending_outgoing(), 1);
    drop(peer.claim().unwrap());
    assert_eq!(peer.pending_outgoing(), 0);
    assert!(peer.claim().is_none());
    assert!(Rc::ptr_eq(peer.outgoing.borrow().traveller.as_ref().unwrap(), &actor));
    assert_eq!((world.population(), actor.borrow().state.alive, actor.borrow().state.energy.to_bits(),
        world.oxygen.get().to_bits(), world.carbon_dioxide.get().to_bits()), after_send);
    for _ in 0..17 { world.draw_outgoing(&peer.outgoing).unwrap(); }
    assert!(peer.outgoing.borrow().can_send());
}


#[test]
fn corridor_paint_seeds_null_ordinary_segments_without_changing_white_default() {
    let mut world = world(7000);
    let gene = Gene::new(8.0, 0.0, ColorKind::Green.argb(), 0, [0; 36]).shared();
    let code = Genome::from_parts(&[gene], GenomeProperties { symmetry: 1, ..Default::default() });
    world.paste_genome(code, [1000, 1000]).unwrap().unwrap();
    let actor = Rc::clone(world.organisms.borrow().last().unwrap());
    {
        let actor = actor.borrow();
        assert!(actor.state.alive && actor.state.temporary_color_frames <= 0 && !actor.body.eyes.enabled);
        assert_eq!(actor.body.segment_count, 1);
        actor.body.set_color(0, None).unwrap();
    }
    let peer = world.network_peer_inbound(1).unwrap();
    assert!(peer.connect());
    let independent = world.clone();
    let random = world.random_snapshot();
    let white = world.render_segments().unwrap();
    assert_eq!(white.len(), 1);
    assert_eq!((white[0].alpha, white[0].rgb), (255, 0x00ff_ffff));
    let (corridors, paint) = world.advance_corridor_presentation().unwrap();
    assert_eq!(corridors.len(), 4);
    assert_eq!(paint, 0xff00_0080);
    let ordinary = world.render_segments_from_color(paint).unwrap();
    assert_eq!((ordinary[0].alpha, ordinary[0].rgb), (255, 0x0000_0080));
    let expected: Vec<_> = independent.advance_presentation().unwrap().into_iter().filter_map(|command| {
        match command { DrawCommand::Line(segment) => Some(segment), _ => None }
    }).collect();
    assert_eq!(ordinary, expected);
    assert_eq!(world.random_snapshot(), random);
    assert_eq!(independent.random_snapshot(), random);
}


#[test]
fn full_outgoing_quota_defers_admission_until_terminal_capacity_returns() {
    let mut world = world(101);
    world.configure_network_outgoing(OutgoingConfig {
        user_name: "Local".into(), pending_per_connection: NonZeroUsize::new(8).unwrap(),
    });
    let mut code = Genome::from_parts(
        &[Gene::new(8.0, 0.0, ColorKind::Green.argb(), 0, [0; 36]).shared()],
        GenomeProperties { symmetry: 1, ..Default::default() },
    );
    code.set_clade_id(Some("root".into()));
    world.paste_genome(code, [50, 50]).unwrap().unwrap();
    let actor = Rc::clone(world.organisms.borrow().last().unwrap());
    actor.borrow_mut().state.velocity = [0.25, 0.0];
    let genome = Rc::clone(actor.borrow().genome.as_ref().unwrap());
    let peer = world.network_peer_inbound(1).unwrap();
    assert!(peer.connect());
    for _ in 0..8 {
        peer.connection.borrow_mut().pending.try_enqueue(Rc::clone(&genome)).unwrap();
    }
    assert_eq!(peer.pending_outgoing(), 8);
    assert!(peer.outgoing.borrow().can_send());
    assert!(peer.outgoing.borrow().bounds.contains(actor.borrow().state.integer_center));
    world.single_step().unwrap();
    assert!(actor.borrow().state.alive);
    assert_eq!(peer.pending_outgoing(), 8);
    assert!(peer.outgoing.borrow().traveller.is_none());
    assert_eq!(genome.borrow().clade_id().unwrap().units(), "root".encode_utf16().collect::<Vec<_>>().as_slice());
    let ticket = peer.claim().unwrap();
    peer.finish(&ticket, NetworkTransferOutcome::Cancelled).unwrap();
    drop(ticket);
    assert_eq!(peer.pending_outgoing(), 7);
    world.single_step().unwrap();
    assert!(!actor.borrow().state.alive);
    assert_eq!(peer.pending_outgoing(), 8);
    assert!(Rc::ptr_eq(peer.outgoing.borrow().traveller.as_ref().unwrap(), &actor));
    assert_eq!(genome.borrow().clade_id().unwrap().units(), "Local:root".encode_utf16().collect::<Vec<_>>().as_slice());
    world.single_step().unwrap();
    assert_eq!(peer.pending_outgoing(), 8);
}
