// SPDX-License-Identifier: GPL-2.0-or-later

use super::*;
use super::outgoing::{ConnectionRef, TransferOutcome};
use crate::{network_wire::{GeneticCodePayload, WireError}, object_stream::Limits};
use std::num::NonZeroUsize;

/// Main-thread connection identity and corridor owners bound to one exact World
pub struct NetworkPeer {
    world: WorldHandle,
    netcode: i32,
    connection: ConnectionRef,
    incoming: IncomingCorridorRef,
    outgoing: OutgoingCorridorRef,
}

/// A claimed live genotype; worker messages must use caller-owned scalar correlations
pub struct NetworkTicket {
    connection: ConnectionRef,
    ticket: u64,
}

impl Drop for NetworkTicket {
    fn drop(&mut self) {
        // Abandoned claims release pending ownership without advancing presentation or biology
        let _ = self.connection.borrow_mut().pending.finish(self.ticket, TransferOutcome::Cancelled);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkTransferOutcome {
    Acknowledged,
    Failed,
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkPeerError {
    WrongPeer,
    MissingTicket,
    IncomingFull,
    Allocation,
    Wire(WireError),
}

impl fmt::Debug for NetworkPeer {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.debug_struct("NetworkPeer").field("netcode", &self.netcode)
            .field("state", &self.state()).finish_non_exhaustive()
    }
}

impl fmt::Debug for NetworkTicket {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        output.debug_struct("NetworkTicket").field("ticket", &self.ticket).finish_non_exhaustive()
    }
}

impl fmt::Display for NetworkPeerError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongPeer => output.write_str("Network ticket belongs to another peer"),
            Self::MissingTicket => output.write_str("Network ticket has already completed"),
            Self::IncomingFull => output.write_str("Incoming ownership capacity is full"),
            Self::Allocation => output.write_str("Incoming queue allocation failed"),
            Self::Wire(error) => fmt::Display::fmt(error, output),
        }
    }
}

impl std::error::Error for NetworkPeerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self { Self::Wire(error) => Some(error), _ => None }
    }
}

impl World {
    /// Update only outgoing policy; the finite queue capacity applies to subsequently created peers
    pub fn configure_network_outgoing(&self, config: OutgoingConfig) {
        self.session.0.parameters.borrow_mut().outgoing = Some(config);
    }

    /// Draw the outbound identifier before constructing incoming and outgoing corridors
    pub fn network_peer_outbound(&self) -> Result<NetworkPeer, WorldError> {
        let netcode = self.session.0.random.borrow_mut().next_i32();
        self.network_peer_inbound(netcode)
    }

    /// Supplied inbound identifiers consume no netcode draw; construction does not add the pair
    pub fn network_peer_inbound(&self, netcode: i32) -> Result<NetworkPeer, WorldError> {
        let connection = self.new_connection_core(52)?;
        let incoming = self.new_incoming_corridor();
        let outgoing = self.new_outgoing_corridor(Rc::clone(&connection));
        Ok(NetworkPeer { world: self.handle(), netcode, connection, incoming, outgoing })
    }
}

impl NetworkPeer {
    pub fn netcode(&self) -> i32 { self.netcode }
    pub fn state(&self) -> i32 { self.connection.borrow().state }

    /// Compare root identity, including replacements which retain the same session
    pub fn belongs_to(&self, world: &World) -> bool { self.world.same_owner(&world.handle()) }

    /// Return incoming then outgoing rectangle values without advancing presentation
    pub fn bounds(&self) -> [IntRect; 2] {
        [self.incoming.borrow().bounds, self.outgoing.borrow().bounds]
    }

    /// Add once per successful state transition; the caller owns timers and status feedback
    pub fn connect(&self) -> bool {
        if self.state() == 51 { return false; }
        self.world.alias().add_corridors(Rc::clone(&self.incoming), Rc::clone(&self.outgoing));
        self.connection.borrow_mut().state = 51;
        true
    }

    /// Remove from the original bound World even after the application's active root changes
    pub fn disconnect(&self) -> Option<[IntRect; 2]> {
        if self.state() == 50 { return None; }
        let dirty = self.world.alias().remove_corridors(&self.incoming, &self.outgoing);
        self.connection.borrow_mut().state = 50;
        Some(dirty)
    }

    /// Claim ownership without freezing the genotype's contents
    pub fn claim(&self) -> Option<NetworkTicket> {
        let ticket = self.connection.borrow_mut().pending.claim_next()?;
        Some(NetworkTicket { connection: Rc::clone(&self.connection), ticket })
    }

    fn check_ticket(&self, ticket: &NetworkTicket) -> Result<(), NetworkPeerError> {
        if !Rc::ptr_eq(&self.connection, &ticket.connection) { return Err(NetworkPeerError::WrongPeer); }
        Ok(())
    }

    /// Export on the owner thread only after WAITING_CODE; no socket or biological state advances
    pub fn export(&self, ticket: &NetworkTicket, limits: Limits) -> Result<GeneticCodePayload, NetworkPeerError> {
        self.check_ticket(ticket)?;
        let genome = self.connection.borrow().pending.genome(ticket.ticket).cloned()
            .ok_or(NetworkPeerError::MissingTicket)?;
        GeneticCodePayload::from_genome(&genome, limits).map_err(NetworkPeerError::Wire)
    }

    /// Release pending ownership independently of the corridor's retained presentation traveller
    pub fn finish(&self, ticket: &NetworkTicket, outcome: NetworkTransferOutcome) -> Result<(), NetworkPeerError> {
        self.check_ticket(ticket)?;
        let outcome = match outcome {
            NetworkTransferOutcome::Acknowledged => TransferOutcome::Acknowledged,
            NetworkTransferOutcome::Failed => TransferOutcome::Failed,
            NetworkTransferOutcome::Cancelled => TransferOutcome::Cancelled,
        };
        self.connection.borrow_mut().pending.finish(ticket.ticket, outcome)
            .map(drop).ok_or(NetworkPeerError::MissingTicket)
    }

    pub fn pending_outgoing(&self) -> usize { self.connection.borrow().pending.entries().count() }

    /// Return queued genotypes and whether one placement candidate is retained
    pub fn incoming_counts(&self) -> (usize, bool) {
        let incoming = self.incoming.borrow();
        (incoming.pending.len(), incoming.candidate.is_some())
    }

    /// Hydrate within payload limits and publish FIFO; capacity includes a retained placement candidate
    /// Publication is not placement; callers reject stale World identity before invoking this operation
    pub fn publish(&self, payload: GeneticCodePayload, capacity: NonZeroUsize) -> Result<(), NetworkPeerError> {
        {
            let mut incoming = self.incoming.borrow_mut();
            if incoming.pending.len().saturating_add(usize::from(incoming.candidate.is_some())) >= capacity.get() {
                return Err(NetworkPeerError::IncomingFull);
            }
            incoming.pending.try_reserve(1).map_err(|_| NetworkPeerError::Allocation)?;
        }
        let genome = payload.into_genome().map_err(NetworkPeerError::Wire)?;
        self.incoming.borrow_mut().receive(Some(genome));
        Ok(())
    }
}

#[cfg(test)]
#[path = "network_peer_tests.rs"]
mod tests;
