// SPDX-License-Identifier: GPL-2.0-or-later

use super::network_io::{self, Admission, Call, Rpc, WorkerError};
use super::*;
use biogenesis_core::{
    NetworkPeer, NetworkSettings, NetworkTicket, NetworkTransferOutcome, OutgoingConfig,
    WorldHandle,
    network_session::{AdmissionError, ErrorKind, Reply, ServerDecision, SessionError},
    network_wire::Request,
    object_stream::Limits,
};
use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener},
    num::NonZeroUsize,
    sync::{
        atomic::Ordering,
        mpsc::{self, Receiver, SyncSender},
    },
};

const INCOMING_LIMIT: usize = 8;
const JOB_LIMIT: usize = 4;
const KEEP_ALIVE: Duration = Duration::from_secs(30);
const PRESENTATION_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / 30);

fn keep_alive_due(elapsed: Duration) -> bool {
    elapsed.as_millis() > KEEP_ALIVE.as_millis()
}

struct Connection {
    id: u64,
    address: SocketAddr,
    peer: NetworkPeer,
    last_response: Instant,
    next_check: Instant,
}

struct PendingCommand {
    id: u64,
    peer: u64,
    request: Request,
    ticket: Option<NetworkTicket>,
    io: network_io::SendJob,
}

// Codes and arguments are UI contracts; raw strings remain diagnostic compatibility fields
#[derive(Clone, Debug, PartialEq, Eq)]
enum NetworkFeedback {
    Listening(u16),
    Connecting(SocketAddr),
    Connected(SocketAddr),
    Closed(SocketAddr),
    Closing,
    AllClosed,
    Cancelled,
    InvalidLocalPort,
    InvalidRemotePort,
    InvalidAddress,
    InvalidIdentity,
    AlreadyClosed,
    WorldRequired,
    ClosingPrevious,
    Duplicate,
    Capacity,
    WorkerCapacity,
    Refused(i32),
    Listener(String),
    Transfer(String),
    Incoming(String),
    PreferencesSave(String),
    Cleanup(String),
    Operation(String),
}

impl From<String> for NetworkFeedback {
    fn from(raw: String) -> Self {
        Self::Operation(raw)
    }
}

impl From<&str> for NetworkFeedback {
    fn from(raw: &str) -> Self {
        Self::Operation(raw.to_owned())
    }
}

impl NetworkFeedback {
    fn code(&self) -> &'static str {
        match self {
            Self::Listening(_) => "listening",
            Self::Connecting(_) => "connecting",
            Self::Connected(_) => "connected",
            Self::Closed(_) => "closed",
            Self::Closing => "closing",
            Self::AllClosed => "all_closed",
            Self::Cancelled => "cancelled",
            Self::InvalidLocalPort => "invalid_local_port",
            Self::InvalidRemotePort => "invalid_remote_port",
            Self::InvalidAddress => "invalid_address",
            Self::InvalidIdentity => "invalid_identity",
            Self::AlreadyClosed => "already_closed",
            Self::WorldRequired => "world_required",
            Self::ClosingPrevious => "closing_previous",
            Self::Duplicate => "duplicate",
            Self::Capacity => "capacity",
            Self::WorkerCapacity => "worker_capacity",
            Self::Refused(_) => "refused",
            Self::Listener(_) => "listener_failed",
            Self::Transfer(_) => "transfer_failed",
            Self::Incoming(_) => "incoming_failed",
            Self::PreferencesSave(_) => "preferences_failed",
            Self::Cleanup(_) => "cleanup_failed",
            Self::Operation(_) => "operation_failed",
        }
    }

    fn raw(&self) -> String {
        match self {
            Self::Listening(port) => format!("Listening on port {port}; incoming ownership is limited to {INCOMING_LIMIT} genomes per peer"),
            Self::Connecting(address) => format!("Connecting to {address}"),
            Self::Connected(address) => format!("Connected with {address}"),
            Self::Closed(address) => format!("Connection closed with {address}"),
            Self::Closing => "Closing network connections".to_owned(),
            Self::AllClosed => "Network connections closed".to_owned(),
            Self::Cancelled => "Remaining network notifications were cancelled".to_owned(),
            Self::InvalidLocalPort => "Local port must be between 0 and 65535 to start a listener".to_owned(),
            Self::InvalidRemotePort => "Remote port must be a positive Java integer between 1 and 65535".to_owned(),
            Self::InvalidAddress => "A resolved IP address is required".to_owned(),
            Self::InvalidIdentity => "A valid connection identity is required".to_owned(),
            Self::AlreadyClosed => "This connection has already closed".to_owned(),
            Self::WorldRequired => "Initialize a world before connecting".to_owned(),
            Self::ClosingPrevious => "Previous network connections are still closing; retry after cleanup".to_owned(),
            Self::Duplicate => "This address and port already has a connection".to_owned(),
            Self::Capacity => "Network connection or socket worker capacity is full".to_owned(),
            Self::WorkerCapacity => "Network socket worker capacity is full".to_owned(),
            Self::Refused(reply) => format!("Connection refused with protocol reply {reply}"),
            Self::Incoming(raw) => format!("Incoming transfer was not acknowledged: {raw}"),
            Self::PreferencesSave(raw) => format!("Network settings changed; preference save failed: {raw}. Listener port was retained"),
            Self::Listener(raw) | Self::Transfer(raw) | Self::Cleanup(raw) | Self::Operation(raw) => raw.clone(),
        }
    }

    fn detail(&self) -> &str {
        match self {
            Self::Listener(raw) | Self::Transfer(raw) | Self::Incoming(raw)
            | Self::PreferencesSave(raw) | Self::Cleanup(raw) | Self::Operation(raw) => raw,
            _ => "",
        }
    }

    fn packet(&self) -> VarDictionary {
        let mut arguments = VarDictionary::new();
        match self {
            Self::Listening(port) => {
                arguments.set("port", i64::from(*port));
                arguments.set("limit", INCOMING_LIMIT as i64);
            }
            Self::Connecting(address) | Self::Connected(address) | Self::Closed(address) => {
                arguments.set("address", address.to_string());
            }
            Self::Refused(reply) => { arguments.set("reply", i64::from(*reply)); }
            _ => {}
        }
        let mut output = VarDictionary::new();
        output.set("code", self.code());
        output.set("arguments", &arguments);
        output.set("detail", self.detail());
        output
    }
}

pub(super) struct Network {
    root: Option<WorldHandle>,
    generation: u64,
    next_id: u64,
    revision: u64,
    connections: Vec<Connection>,
    jobs: Vec<PendingCommand>,
    listener: Option<network_io::Listener>,
    bound_port: Option<u16>,
    sender: SyncSender<Rpc>,
    receiver: Receiver<Rpc>,
    error: String,
    notice: String,
    error_feedback: Option<NetworkFeedback>,
    notice_feedback: Option<NetworkFeedback>,
    last_presentation: Instant,
    presentation: Vec<biogenesis_core::DrawCommand>,
    presentation_paint: u32,
    presentation_dirty: bool,
    had_corridors: bool,
    pending_port_restart: bool,
    closing: Option<network_io::CloseJob>,
    pending_listener: bool,
    cleanup_error: Option<String>,
}

impl Default for Network {
    fn default() -> Self {
        let (sender, receiver) = mpsc::sync_channel(16);
        Self {
            root: None,
            generation: 0,
            next_id: 1,
            revision: 0,
            connections: Vec::new(),
            jobs: Vec::new(),
            listener: None,
            bound_port: None,
            sender,
            receiver,
            error: String::new(),
            notice: String::new(),
            error_feedback: None,
            notice_feedback: None,
            last_presentation: Instant::now(),
            presentation: Vec::new(),
            presentation_paint: 0xffff_ffff,
            presentation_dirty: false,
            had_corridors: false,
            pending_port_restart: false,
            closing: None,
            pending_listener: false,
            cleanup_error: None,
        }
    }
}

fn limits() -> Limits {
    Limits {
        input_bytes: 4 << 20,
        output_bytes: 4 << 20,
        allocated_bytes: 8 << 20,
        handles: 32_768,
        array_elements: 65_536,
        total_array_elements: 262_144,
        string_bytes: 65_536,
        string_units: 32_768,
        records: 65_536,
        work: 4_000_000,
        ..Limits::default()
    }
}

fn socket_port(value: i32) -> Result<u16, String> {
    u16::try_from(value)
        .map_err(|_| "Local port must be between 0 and 65535 to start a listener".to_owned())
}

fn wildcard_listeners(port: u16) -> Result<(Vec<TcpListener>, u16), String> {
    match TcpListener::bind((Ipv6Addr::UNSPECIFIED, port)) {
        Ok(listener) => {
            let port = listener
                .local_addr()
                .map_err(|error| error.to_string())?
                .port();
            // The getter is needed to distinguish dual-stack and IPv6-only hosts
            #[expect(
                deprecated,
                reason = "Read the bound socket without platform-specific FFI"
            )]
            let only_v6 = listener.only_v6().map_err(|error| error.to_string())?;
            let mut listeners = vec![listener];
            if only_v6 {
                listeners.push(
                    TcpListener::bind((Ipv4Addr::UNSPECIFIED, port))
                        .map_err(|error| error.to_string())?,
                );
            }
            Ok((listeners, port))
        }
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::Unsupported | std::io::ErrorKind::AddrNotAvailable
            ) =>
        {
            let listener = TcpListener::bind((Ipv4Addr::UNSPECIFIED, port))
                .map_err(|error| error.to_string())?;
            let port = listener
                .local_addr()
                .map_err(|error| error.to_string())?
                .port();
            Ok((vec![listener], port))
        }
        Err(error) => Err(error.to_string()),
    }
}

impl Network {
    fn changed(&mut self) {
        self.revision = self
            .revision
            .checked_add(1)
            .expect("Network revision exhausted");
    }

    fn fail(&mut self, error: impl Into<NetworkFeedback>) {
        let feedback = error.into();
        self.error = feedback.raw();
        self.error_feedback = Some(feedback);
        self.changed();
    }

    fn clear_error(&mut self) {
        self.error.clear();
        self.error_feedback = None;
    }

    fn cleanup_failed(&mut self, error: String) {
        eprintln!("Biogenesis-R network cleanup failed: {error}");
        self.cleanup_error = Some(error.clone());
        self.fail(NetworkFeedback::Cleanup(error));
    }

    fn allocate_id(&mut self) -> Result<u64, String> {
        let id = self.next_id;
        self.next_id = id
            .checked_add(1)
            .ok_or("Network operation identity exhausted")?;
        Ok(id)
    }

    fn configure_world(&mut self, world: &World, settings: &NetworkSettings) {
        world.configure_network_outgoing(OutgoingConfig {
            user_name: settings.user_name.clone(),
            pending_per_connection: NonZeroUsize::new(INCOMING_LIMIT).unwrap(),
        });
    }

    fn attach(&mut self, world: &World, settings: &NetworkSettings) {
        if self
            .root
            .as_ref()
            .is_some_and(|root| root.same_owner(&world.handle()))
        {
            return;
        }
        if self.root.is_some()
            || self.listener.is_some()
            || !self.jobs.is_empty()
            || !self.connections.is_empty()
        {
            self.stop();
        }
        self.generation = self
            .generation
            .checked_add(1)
            .expect("Network generation exhausted");
        self.root = Some(world.handle());
        self.configure_world(world, settings);
        self.last_presentation = Instant::now();
        if settings.accept_connections {
            if let Err(error) = self.listen(settings.local_port) {
                self.fail(error);
            }
        }
    }

    fn listen(&mut self, port: i32) -> Result<(), NetworkFeedback> {
        if self.listener.is_some() {
            return Ok(());
        }
        if self.closing.is_some() {
            self.pending_listener = true;
            return Ok(());
        }
        let port = socket_port(port).map_err(|_| NetworkFeedback::InvalidLocalPort)?;
        let (listeners, actual_port) = wildcard_listeners(port).map_err(NetworkFeedback::Listener)?;
        let listener =
            network_io::Listener::spawn(listeners, self.generation, self.sender.clone(), limits())
                .map_err(|error| NetworkFeedback::Listener(error.to_string()))?;
        self.listener = Some(listener);
        self.bound_port = Some(actual_port);
        let feedback = NetworkFeedback::Listening(actual_port);
        self.notice = feedback.raw();
        self.notice_feedback = Some(feedback);
        self.changed();
        Ok(())
    }

    fn stop_listener(&mut self) {
        if let Some(mut listener) = self.listener.take() {
            listener.cancel();
            if let Err(WorkerError::Failure(error)) = listener.finish() {
                self.cleanup_failed(error);
            }
        }
        self.bound_port = None;
        self.changed();
    }

    fn remove(&mut self, id: u64) {
        let Some(index) = self
            .connections
            .iter()
            .position(|connection| connection.id == id)
        else {
            return;
        };
        let connection = &self.connections[index];
        for job in self.jobs.iter_mut().filter(|job| job.peer == id) {
            job.io.cancel();
            if let Some(ticket) = job.ticket.take() {
                if let Err(error) = connection
                    .peer
                    .finish(&ticket, NetworkTransferOutcome::Cancelled)
                {
                    self.error = error.to_string();
                    self.cleanup_error = Some(self.error.clone());
                    self.error_feedback = Some(NetworkFeedback::Cleanup(self.error.clone()));
                }
            }
        }
        while let Some(ticket) = connection.peer.claim() {
            if let Err(error) = connection
                .peer
                .finish(&ticket, NetworkTransferOutcome::Cancelled)
            {
                self.error = error.to_string();
                self.cleanup_error = Some(self.error.clone());
                self.error_feedback = Some(NetworkFeedback::Cleanup(self.error.clone()));
                break;
            }
        }
        connection.peer.disconnect();
        let feedback = NetworkFeedback::Closed(connection.address);
        self.notice = feedback.raw();
        self.notice_feedback = Some(feedback);
        self.connections.remove(index);
        self.presentation_dirty = true;
        self.changed();
    }

    fn stop(&mut self) {
        if let Some(error) = self.cleanup_error.clone() {
            self.error = error.clone();
            self.error_feedback = Some(NetworkFeedback::Cleanup(error));
        } else {
            self.clear_error();
        }
        if self.root.is_none()
            && self.listener.is_none()
            && self.jobs.is_empty()
            && self.connections.is_empty()
        {
            return;
        }
        self.generation = self
            .generation
            .checked_add(1)
            .expect("Network generation exhausted");
        if let Some(listener) = &self.listener {
            listener.cancel();
        }
        for job in &self.jobs {
            job.io.cancel();
        }
        let notifications: Vec<_> = self
            .connections
            .iter()
            .filter(|connection| connection.peer.state() == 51)
            .map(|connection| (connection.address, connection.peer.netcode()))
            .collect();
        while let Some(id) = self.connections.first().map(|connection| connection.id) {
            self.remove(id);
        }
        for mut job in std::mem::take(&mut self.jobs) {
            if let Err(WorkerError::Failure(error)) = job.io.finish() {
                self.cleanup_failed(error);
            }
        }
        self.stop_listener();
        while self.receiver.try_recv().is_ok() {}
        if !notifications.is_empty() {
            assert!(
                self.closing.is_none(),
                "New peers were admitted during prior-generation notification cleanup"
            );
            match network_io::disconnect_peers(notifications, limits()) {
                Ok(job) => {
                    self.closing = Some(job);
                    let feedback = NetworkFeedback::Closing;
                    self.notice = feedback.raw();
                    self.notice_feedback = Some(feedback);
                }
                Err(error) => self.cleanup_failed(error.to_string()),
            }
        }
        self.root = None;
        self.pending_listener = false;
        self.had_corridors = false;
        self.presentation.clear();
        self.presentation_paint = 0xffff_ffff;
        self.presentation_dirty = true;
        self.changed();
    }

    fn poll_close(&mut self) -> bool {
        if self.closing.as_ref().is_some_and(|job| job.is_finished()) {
            let mut job = self.closing.take().unwrap();
            match job.finish() {
                Ok(()) => {
                    let feedback = NetworkFeedback::AllClosed;
                    self.notice = feedback.raw();
                    self.notice_feedback = Some(feedback);
                }
                Err(WorkerError::Cancelled) => {
                    let feedback = NetworkFeedback::Cancelled;
                    self.notice = feedback.raw();
                    self.notice_feedback = Some(feedback);
                }
                Err(WorkerError::Failure(error)) => self.cleanup_failed(error),
            }
            self.changed();
        }
        self.closing.is_none()
    }

    fn force_stop(&mut self) {
        self.stop();
        if let Some(mut job) = self.closing.take() {
            job.cancel();
            if let Err(WorkerError::Failure(error)) = job.finish() {
                self.cleanup_failed(error);
            }
        }
    }

    fn spawn(
        &mut self,
        peer: u64,
        address: SocketAddr,
        request: Request,
        ticket: Option<NetworkTicket>,
    ) -> Result<(), NetworkFeedback> {
        if self.jobs.len() >= JOB_LIMIT {
            return Err(NetworkFeedback::WorkerCapacity);
        }
        let id = self.allocate_id()?;
        let io = network_io::spawn_send(
            address,
            request,
            self.generation,
            id,
            self.sender.clone(),
            limits(),
        )
        .map_err(|error| error.to_string())?;
        self.jobs.push(PendingCommand {
            id,
            peer,
            request,
            ticket,
            io,
        });
        Ok(())
    }

    fn connect(
        &mut self,
        world: &World,
        settings: &NetworkSettings,
        address: SocketAddr,
    ) -> Result<(), NetworkFeedback> {
        self.poll_close();
        self.attach(world, settings);
        if self.closing.is_some() {
            return Err(NetworkFeedback::ClosingPrevious);
        }
        self.listen(settings.local_port)?;
        if self
            .connections
            .iter()
            .any(|connection| connection.address == address)
        {
            return Err(NetworkFeedback::Duplicate);
        }
        if self.jobs.len() >= JOB_LIMIT {
            return Err(NetworkFeedback::Capacity);
        }
        let id = self.allocate_id()?;
        let peer = world
            .network_peer_outbound()
            .map_err(|error| error.to_string())?;
        let netcode = peer.netcode();
        let now = Instant::now();
        self.connections.push(Connection {
            id,
            address,
            peer,
            last_response: now,
            next_check: now + KEEP_ALIVE,
        });
        let request = Request::Connect {
            version: 900,
            port: i32::from(self.bound_port.expect("Listener has a port")),
            netcode,
        };
        if let Err(error) = self.spawn(id, address, request, None) {
            self.remove(id);
            return Err(error);
        }
        self.clear_error();
        let feedback = NetworkFeedback::Connecting(address);
        self.notice = feedback.raw();
        self.notice_feedback = Some(feedback);
        self.changed();
        Ok(())
    }

    fn disconnect(&mut self, id: u64) -> Result<(), NetworkFeedback> {
        let connection = self
            .connections
            .iter()
            .find(|connection| connection.id == id)
            .ok_or(NetworkFeedback::AlreadyClosed)?;
        let (address, netcode) = (connection.address, connection.peer.netcode());
        self.remove(id);
        self.spawn(id, address, Request::Disconnect { netcode }, None)
    }

    fn decide(
        &mut self,
        world: &World,
        settings: &NetworkSettings,
        remote: SocketAddr,
        request: Request,
    ) -> Result<Admission, AdmissionError> {
        let netcode = match request {
            Request::Connect { netcode, .. }
            | Request::Disconnect { netcode }
            | Request::KeepAlive { netcode }
            | Request::SendCode { netcode } => netcode,
        };
        let existing = self
            .connections
            .iter()
            .find(|connection| connection.peer.netcode() == netcode);
        let reply = |code| {
            Ok(Admission {
                decision: ServerDecision::Reply(code),
                peer: None,
            })
        };
        match request {
            Request::Connect { version, port, .. } => {
                if existing.is_some() {
                    return reply(3);
                }
                if !settings.accept_connections {
                    return reply(6);
                }
                if self.connections.len() >= settings.max_connections.max(0) as usize {
                    return reply(4);
                }
                if version != 900 {
                    return reply(16);
                }
                let port = u16::try_from(port)
                    .ok()
                    .filter(|value| *value != 0)
                    .ok_or(AdmissionError)?;
                let address = SocketAddr::new(remote.ip().to_canonical(), port);
                if self
                    .connections
                    .iter()
                    .any(|connection| connection.address == address)
                {
                    return reply(3);
                }
                let id = self.allocate_id().map_err(|_| AdmissionError)?;
                let peer = world
                    .network_peer_inbound(netcode)
                    .map_err(|_| AdmissionError)?;
                peer.connect();
                let now = Instant::now();
                self.connections.push(Connection {
                    id,
                    address,
                    peer,
                    last_response: now,
                    next_check: now + KEEP_ALIVE,
                });
                self.had_corridors = true;
                self.presentation_dirty = true;
                let feedback = NetworkFeedback::Connected(address);
                self.notice = feedback.raw();
                self.notice_feedback = Some(feedback);
                self.changed();
                Ok(Admission {
                    decision: ServerDecision::Reply(2),
                    peer: Some(id),
                })
            }
            _ => {
                let Some(connection) = existing.filter(|connection| connection.peer.state() == 51)
                else {
                    return reply(7);
                };
                let decision = match request {
                    Request::Disconnect { .. } => ServerDecision::Reply(15),
                    Request::KeepAlive { .. } => ServerDecision::Reply(14),
                    Request::SendCode { .. } => ServerDecision::ReceiveCode,
                    Request::Connect { .. } => unreachable!(),
                };
                Ok(Admission {
                    decision,
                    peer: Some(connection.id),
                })
            }
        }
    }

    fn rpc(&mut self, world: &World, settings: &NetworkSettings, rpc: Rpc) {
        if rpc.generation != self.generation {
            return;
        }
        let terminal = matches!(rpc.call, Call::Received { .. });
        if !terminal && (rpc.cancelled.load(Ordering::Relaxed) || Instant::now() >= rpc.deadline) {
            return;
        }
        match rpc.call {
            Call::Decide {
                remote,
                request,
                answer,
            } => {
                let _ = answer.try_send(self.decide(world, settings, remote, request));
            }
            Call::Publish {
                peer,
                payload,
                answer,
            } => {
                let result = self
                    .connections
                    .iter()
                    .find(|connection| {
                        connection.id == peer
                            && connection.peer.state() == 51
                            && connection.peer.belongs_to(world)
                    })
                    .ok_or(AdmissionError)
                    .and_then(|connection| {
                        connection
                            .peer
                            .publish(payload, NonZeroUsize::new(INCOMING_LIMIT).unwrap())
                            .map_err(|error| {
                                let feedback = NetworkFeedback::Incoming(error.to_string());
                                self.error = feedback.raw();
                                self.error_feedback = Some(feedback);
                                AdmissionError
                            })
                    });
                let _ = answer.try_send(result);
                self.changed();
            }
            Call::Export { job, answer } => {
                let result = self
                    .jobs
                    .iter()
                    .find(|pending| pending.id == job)
                    .and_then(|pending| {
                        let connection = self.connections.iter().find(|connection| {
                            connection.id == pending.peer && connection.peer.belongs_to(world)
                        })?;
                        Some(connection.peer.export(pending.ticket.as_ref()?, limits()))
                    })
                    .transpose()
                    .map_err(|_| SessionError {
                        kind: ErrorKind::Admission,
                        shutdown_error: None,
                    })
                    .and_then(|payload| {
                        payload.ok_or(SessionError {
                            kind: ErrorKind::Cancelled,
                            shutdown_error: None,
                        })
                    });
                if !rpc.cancelled.load(Ordering::Relaxed) && Instant::now() < rpc.deadline {
                    let _ = answer.try_send(result);
                }
            }
            Call::Received {
                peer,
                request,
                result,
            } => {
                if let Err(error) = &result {
                    self.fail(error.clone());
                }
                if let Some(id) = peer {
                    if matches!(request, Some(Request::Disconnect { .. })) && result.is_ok()
                        || matches!(request, Some(Request::SendCode { .. })) && result.is_err()
                    {
                        self.remove(id);
                    }
                }
            }
        }
    }

    fn completed(&mut self, job: PendingCommand, result: Result<Reply, WorkerError>) {
        if let Err(WorkerError::Failure(error)) = &result {
            self.fail(error.clone());
        }
        let Some(index) = self
            .connections
            .iter()
            .position(|connection| connection.id == job.peer)
        else {
            return;
        };
        let connection = &mut self.connections[index];
        if let Some(ticket) = job.ticket {
            let outcome = if result
                .as_ref()
                .is_ok_and(|reply| reply.initial == 9 && reply.final_reply == Some(10))
            {
                NetworkTransferOutcome::Acknowledged
            } else {
                NetworkTransferOutcome::Failed
            };
            if let Err(error) = connection.peer.finish(&ticket, outcome) {
                self.error = error.to_string();
                self.error_feedback = Some(NetworkFeedback::Transfer(self.error.clone()));
            }
        }
        let mut disconnect = false;
        match result {
            Err(_) => disconnect = true,
            Ok(reply) => match job.request {
                Request::Connect { .. } => {
                    if matches!(reply.initial, 2 | 3) {
                        connection.peer.connect();
                        connection.last_response = Instant::now();
                        connection.next_check = connection.last_response + KEEP_ALIVE;
                        self.had_corridors = true;
                        self.presentation_dirty = true;
                        let feedback = NetworkFeedback::Connected(connection.address);
                        self.notice = feedback.raw();
                        self.notice_feedback = Some(feedback);
                    } else {
                        let feedback = NetworkFeedback::Refused(reply.initial);
                        self.error = feedback.raw();
                        self.error_feedback = Some(feedback);
                        disconnect = true;
                    }
                }
                Request::KeepAlive { .. } => {
                    if reply.initial == 14 {
                        connection.last_response = Instant::now();
                    } else {
                        disconnect = true;
                    }
                }
                Request::SendCode { .. } => match reply.initial {
                    9 => {
                        if reply.final_reply == Some(10) {
                            connection.last_response = Instant::now();
                        }
                    }
                    11 => {}
                    _ => disconnect = true,
                },
                Request::Disconnect { .. } => disconnect = true,
            },
        }
        if disconnect {
            self.remove(job.peer);
        }
        self.changed();
    }

    fn pump(&mut self, world: &World, settings: &NetworkSettings) -> Result<bool, NetworkFeedback> {
        self.poll_close();
        self.attach(world, settings);
        if self.closing.is_none() && self.pending_listener {
            self.pending_listener = false;
            if settings.accept_connections {
                self.listen(settings.local_port)?;
            }
        }
        for _ in 0..16 {
            let Ok(rpc) = self.receiver.try_recv() else {
                break;
            };
            self.rpc(world, settings, rpc);
        }
        let mut index = 0;
        while index < self.jobs.len() {
            if !self.jobs[index].io.is_finished() {
                index += 1;
                continue;
            }
            let mut job = self.jobs.remove(index);
            let result = job.io.finish();
            self.completed(job, result);
        }
        if self
            .listener
            .as_ref()
            .is_some_and(|listener| listener.is_finished())
        {
            let mut listener = self.listener.take().unwrap();
            if let Err(error) = listener.finish() {
                self.fail(NetworkFeedback::Listener(error.to_string()));
            }
            self.bound_port = None;
            self.changed();
        }
        let now = Instant::now();
        for index in 0..self.connections.len() {
            if self.jobs.len() >= JOB_LIMIT {
                break;
            }
            let connection = &mut self.connections[index];
            if connection.peer.state() != 51
                || self.jobs.iter().any(|job| job.peer == connection.id)
            {
                continue;
            }
            let request = if now >= connection.next_check {
                connection.next_check = now + KEEP_ALIVE;
                keep_alive_due(now.saturating_duration_since(connection.last_response)).then_some((
                    Request::KeepAlive {
                        netcode: connection.peer.netcode(),
                    },
                    None,
                ))
            } else {
                None
            };
            let request = request.or_else(|| {
                connection.peer.claim().map(|ticket| {
                    (
                        Request::SendCode {
                            netcode: connection.peer.netcode(),
                        },
                        Some(ticket),
                    )
                })
            });
            if let Some((request, ticket)) = request {
                let (id, address) = (connection.id, connection.address);
                if let Err(error) = self.spawn(id, address, request, ticket) {
                    self.fail(error);
                    self.remove(id);
                    break;
                }
            }
        }
        self.present_at(world, now).map_err(NetworkFeedback::from)
    }

    fn present_at(&mut self, world: &World, now: Instant) -> Result<bool, String> {
        let elapsed = now.saturating_duration_since(self.last_presentation);
        let steps = (elapsed.as_nanos() / PRESENTATION_INTERVAL.as_nanos()).min(16) as usize;
        if self.had_corridors {
            if steps == 0 {
                return Ok(false);
            }
            for _ in 0..steps {
                (self.presentation, self.presentation_paint) = world
                    .advance_corridor_presentation()
                    .map_err(|error| error.to_string())?;
            }
            let remainder = elapsed.as_nanos() % PRESENTATION_INTERVAL.as_nanos();
            self.last_presentation = now - Duration::from_nanos(remainder as u64);
            self.presentation_dirty = false;
            return Ok(true);
        }
        if self.presentation_dirty {
            self.presentation.clear();
            self.presentation_dirty = false;
            return Ok(true);
        }
        Ok(false)
    }
}

impl Drop for Network {
    fn drop(&mut self) {
        self.force_stop();
    }
}

fn settings_fields(settings: &NetworkSettings) -> VarDictionary {
    let mut output = VarDictionary::new();
    output.set("accept_connections", settings.accept_connections);
    output.set("local_port", settings.local_port);
    output.set("max_connections", settings.max_connections);
    output.set("user_name", settings.user_name.display_utf8_lossy());
    output
}

impl BgrSimulation {
    pub(super) fn network_world_ready(&mut self, world: &World) {
        let settings = self
            .settings
            .get_or_insert_with(|| biogenesis_core::SettingsState::new("en".to_owned()))
            .network_settings();
        self.network.attach(world, &settings);
    }

    pub(super) fn release_network(&mut self) {
        self.network.force_stop();
    }

    pub(super) fn network_cleanup_error(&self) -> Option<String> {
        self.network.cleanup_error.clone()
    }

    pub(super) fn network_draw_paint(&self, world: &World) -> u32 {
        if self
            .network
            .root
            .as_ref()
            .is_some_and(|root| root.same_owner(&world.handle()))
        {
            self.network.presentation_paint
        } else {
            0xffff_ffff
        }
    }
}

impl Network {
    fn packet(&self, settings: &NetworkSettings) -> VarDictionary {
        let mut output = VarDictionary::new();
        output.set("ok", self.error.is_empty());
        output.set("error", self.error.as_str());
        output.set("notice", self.notice.as_str());
        output.set(
            "error_feedback",
            &self.error_feedback.as_ref().map_or_else(VarDictionary::new, NetworkFeedback::packet),
        );
        output.set(
            "notice_feedback",
            &self.notice_feedback.as_ref().map_or_else(VarDictionary::new, NetworkFeedback::packet),
        );
        output.set("closing", self.closing.is_some());
        output.set(
            "revision",
            i64::try_from(self.revision).expect("Network revision exceeds bridge range"),
        );
        output.set(
            "generation",
            i64::try_from(self.generation).expect("Network generation exceeds bridge range"),
        );
        output.set("settings", &settings_fields(settings));
        output.set("listening", self.listener.is_some());
        output.set("bound_port", i64::from(self.bound_port.unwrap_or(0)));
        output.set("pending_command_jobs", self.jobs.len() as i64);
        output.set("job_limit", JOB_LIMIT as i64);
        output.set("incoming_limit", INCOMING_LIMIT as i64);
        let mut connections = VarArray::new();
        for connection in &self.connections {
            let mut row = VarDictionary::new();
            let (queued, candidate) = connection.peer.incoming_counts();
            row.set(
                "id",
                i64::try_from(connection.id).expect("Network peer exceeds bridge range"),
            );
            row.set("address", connection.address.ip().to_string());
            row.set("port", i64::from(connection.address.port()));
            row.set("state", connection.peer.state());
            row.set("queued_incoming", queued as i64);
            row.set("incoming_candidate", candidate);
            row.set(
                "pending_outgoing",
                connection.peer.pending_outgoing() as i64,
            );
            connections.push(&row.to_variant());
        }
        output.set("connections", &connections);
        output
    }

    fn packed_presentation(&self) -> PackedFloat64Array {
        let mut output = Vec::with_capacity(self.presentation.len() * 6);
        for command in &self.presentation {
            match command {
                biogenesis_core::DrawCommand::Outline { bounds, argb }
                | biogenesis_core::DrawCommand::Fill { bounds, argb } => {
                    output.extend_from_slice(&[
                        if matches!(command, biogenesis_core::DrawCommand::Fill { .. }) {
                            1.0
                        } else {
                            0.0
                        },
                        f64::from(bounds.x),
                        f64::from(bounds.y),
                        f64::from(bounds.width),
                        f64::from(bounds.height),
                        f64::from(*argb),
                    ]);
                }
                biogenesis_core::DrawCommand::Line(line) => output.extend_from_slice(&[
                    2.0,
                    line.x1,
                    line.y1,
                    line.x2,
                    line.y2,
                    f64::from((u32::from(line.alpha) << 24) | line.rgb),
                ]),
            }
        }
        PackedFloat64Array::from(output.as_slice())
    }
}

#[godot_api(secondary)]
impl BgrSimulation {
    #[func]
    fn network_snapshot(&self) -> VarDictionary {
        let settings = self
            .settings
            .as_ref()
            .map(|state| state.network_settings())
            .unwrap_or_else(|| {
                biogenesis_core::SettingsState::new("en".to_owned()).network_settings()
            });
        self.network.packet(&settings)
    }

    #[func]
    fn network_pump(&mut self) -> VarDictionary {
        let settings = self
            .settings
            .get_or_insert_with(|| biogenesis_core::SettingsState::new("en".to_owned()))
            .network_settings();
        let previous_paint = self.network.presentation_paint;
        let changed = if let Some(world) = &self.world {
            match self.network.pump(world, &settings) {
                Ok(changed) => changed,
                Err(error) => {
                    self.network.fail(error);
                    false
                }
            }
        } else {
            false
        };
        let mut output = self.network.packet(&settings);
        output.set("presentation_changed", changed);
        if changed {
            output.set("presentation", &self.network.packed_presentation());
        }
        if previous_paint != self.network.presentation_paint && self.world.is_some() {
            output.set("world_snapshot", &self.response(None));
        }
        output
    }

    #[func]
    fn network_valid_remote_port(text: GString) -> bool {
        biogenesis_core::parse_network_remote_port(&text.to_string()).is_some()
    }

    #[func]
    fn network_connect(
        &mut self,
        resolved_address: GString,
        remote_port: GString,
    ) -> VarDictionary {
        let result = (|| {
            let port = biogenesis_core::parse_network_remote_port(&remote_port.to_string())
                .ok_or(NetworkFeedback::InvalidRemotePort)?;
            let ip = resolved_address
                .to_string()
                .parse::<IpAddr>()
                .map_err(|_| NetworkFeedback::InvalidAddress)?
                .to_canonical();
            let world = self
                .world
                .as_ref()
                .ok_or(NetworkFeedback::WorldRequired)?;
            let settings = self
                .settings
                .get_or_insert_with(|| biogenesis_core::SettingsState::new("en".to_owned()))
                .network_settings();
            self.network
                .connect(world, &settings, SocketAddr::new(ip, port))
        })();
        if let Err(error) = result {
            self.network.fail(error);
        }
        self.network_snapshot()
    }

    #[func]
    fn network_disconnect(&mut self, peer_id: i64) -> VarDictionary {
        self.network.clear_error();
        match u64::try_from(peer_id).ok().filter(|id| *id != 0) {
            Some(id) => {
                if let Err(error) = self.network.disconnect(id) {
                    self.network.fail(error);
                }
            }
            None => self.network.fail(NetworkFeedback::InvalidIdentity),
        }
        self.network_snapshot()
    }

    #[func]
    fn network_stop(&mut self) -> VarDictionary {
        self.network.stop();
        self.network_snapshot()
    }

    #[func]
    fn network_poll_stop(&mut self) -> VarDictionary {
        self.network.poll_close();
        self.network_snapshot()
    }

    #[func]
    fn network_apply(
        &mut self,
        accept: bool,
        local_port: GString,
        max_connections: GString,
        user_name: GString,
        user_name_edited: bool,
    ) -> VarDictionary {
        self.network.clear_error();
        let before = self
            .settings
            .get_or_insert_with(|| biogenesis_core::SettingsState::new("en".to_owned()))
            .network_settings();
        if before.accept_connections != accept {
            let interim = self
                .settings
                .as_mut()
                .unwrap()
                .apply_network_editor(
                    accept,
                    &before.local_port.to_string(),
                    &before.max_connections.to_string(),
                    before.user_name.clone(),
                )
                .after;
            if let Some(world) = &self.world {
                self.network.attach(world, &interim);
                if accept {
                    if let Err(error) = self.network.listen(before.local_port) {
                        self.network.fail(error);
                    }
                } else if self.network.connections.is_empty() {
                    self.network.stop_listener();
                }
            }
        }
        let user_name = if user_name_edited {
            biogenesis_core::JavaText::from_utf8(&user_name.to_string())
        } else {
            before.user_name.clone()
        };
        let report = self.settings.as_mut().unwrap().apply_network_editor(
            accept,
            &local_port.to_string(),
            &max_connections.to_string(),
            user_name,
        );
        if let Some(world) = &self.world {
            self.network.configure_world(world, &report.after);
        }
        self.network.pending_port_restart |= before.local_port != report.after.local_port;
        let saved = match self.save_network_preferences() {
            Ok(()) => true,
            Err(error) => {
                self.network.fail(NetworkFeedback::PreferencesSave(error.to_string()));
                false
            }
        };
        if saved && self.network.pending_port_restart {
            self.network.stop();
            self.network.pending_port_restart = false;
            if let Some(world) = &self.world {
                self.network.attach(world, &report.after);
            }
        }
        if saved
            && report.after.accept_connections
            && self.world.is_some()
            && self.network.listener.is_none()
        {
            if let Err(error) = self.network.listen(report.after.local_port) {
                self.network.fail(error);
            }
        }
        let mut output = self.network_snapshot();
        output.set("saved", saved);
        output.set(
            "local_port_applied",
            report.local_port == biogenesis_core::EditorOutcome::Applied,
        );
        output.set(
            "max_connections_applied",
            report.max_connections == biogenesis_core::EditorOutcome::Applied,
        );
        output.set("user_name_applied", report.user_name_applied);
        output
    }
}

#[cfg(test)]
#[path = "network_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "network_feedback_tests.rs"]
mod feedback_tests;
