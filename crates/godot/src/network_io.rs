// SPDX-License-Identifier: GPL-2.0-or-later

//! Bounded socket workers exchange detached data with the simulation thread

use biogenesis_core::{
    network_session::{self, AdmissionError, ErrorKind, Reply, ServerDecision, SessionError, SessionOptions},
    network_wire::{GeneticCodePayload, Request},
    object_stream::Limits,
};
use std::{
    cell::Cell,
    fmt, io,
    net::{SocketAddr, TcpListener, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, RecvTimeoutError, SyncSender, TryRecvError, TrySendError},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

const POLL: Duration = Duration::from_millis(10);
const SESSION_TIME: Duration = Duration::from_secs(5);
const STALL_TIME: Duration = Duration::from_secs(2);
const CONNECT_TIME: Duration = Duration::from_millis(250);
const CLOSE_TIME: Duration = Duration::from_millis(500);
const CLOSE_CONNECT_TIME: Duration = Duration::from_millis(50);

pub(super) struct Admission {
    pub decision: ServerDecision,
    pub peer: Option<u64>,
}

pub(super) enum Call {
    Decide {
        remote: SocketAddr,
        request: Request,
        answer: SyncSender<Result<Admission, AdmissionError>>,
    },
    Publish {
        peer: u64,
        payload: GeneticCodePayload,
        answer: SyncSender<Result<(), AdmissionError>>,
    },
    Export {
        job: u64,
        answer: SyncSender<Result<GeneticCodePayload, SessionError>>,
    },
    Received {
        peer: Option<u64>,
        request: Option<Request>,
        result: Result<(), String>,
    },
}

pub(super) struct Rpc {
    pub generation: u64,
    pub deadline: Instant,
    pub cancelled: Arc<AtomicBool>,
    pub call: Call,
}

pub(super) type Listener = Worker<()>;
pub(super) type SendJob = Worker<Reply>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum WorkerError {
    Cancelled,
    Failure(String),
}

impl fmt::Display for WorkerError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => output.write_str("Network operation cancelled"),
            Self::Failure(message) => output.write_str(message),
        }
    }
}

impl std::error::Error for WorkerError {}

impl From<SessionError> for WorkerError {
    fn from(value: SessionError) -> Self {
        if value.kind == ErrorKind::Cancelled && value.shutdown_error.is_none() {
            Self::Cancelled
        } else {
            Self::Failure(value.to_string())
        }
    }
}

/// Explicit finish reports worker failures; Drop still cancels and joins during unwinding
pub(super) struct Worker<T: Clone> {
    cancelled: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    result: Receiver<Result<T, WorkerError>>,
    terminal: Option<Result<T, WorkerError>>,
    delivered: bool,
}

impl<T: Clone + Send + 'static> Worker<T> {
    fn launch(name: &str, run: impl FnOnce(Arc<AtomicBool>) -> Result<T, WorkerError> + Send + 'static)
        -> io::Result<Self>
    {
        let cancelled = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&cancelled);
        let (sender, result) = mpsc::sync_channel(1);
        let thread = thread::Builder::new().name(name.to_owned()).spawn(move || {
            // One producer writes once into its initially empty terminal slot
            if sender.try_send(run(flag)).is_err() {
                panic!("Network worker lost its owned terminal result slot");
            }
        })?;
        Ok(Self { cancelled, thread: Some(thread), result, terminal: None, delivered: false })
    }
}

impl<T: Clone> Worker<T> {
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
        if let Some(thread) = &self.thread { thread.thread().unpark(); }
    }

    pub fn is_finished(&self) -> bool {
        self.thread.as_ref().is_none_or(JoinHandle::is_finished)
    }

    fn poll_result(&mut self) {
        if self.terminal.is_some() { return; }
        match self.result.try_recv() {
            Ok(result) => self.terminal = Some(result),
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.terminal = Some(Err(WorkerError::Failure("Network worker stopped without a terminal result".to_owned())));
            }
        }
    }

    /// Deliver the terminal value once while retaining it for explicit finish
    pub fn try_result(&mut self) -> Option<Result<T, WorkerError>> {
        self.poll_result();
        if self.delivered { return None; }
        let result = self.terminal.clone()?;
        self.delivered = true;
        Some(result)
    }

    pub fn finish(&mut self) -> Result<T, WorkerError> {
        self.cancel();
        self.join()
    }

    fn join(&mut self) -> Result<T, WorkerError> {
        if let Some(thread) = self.thread.take() {
            if thread.join().is_err() {
                self.terminal = Some(Err(WorkerError::Failure("Network worker panicked".to_owned())));
            }
        }
        self.poll_result();
        self.terminal.clone().unwrap_or_else(|| Err(WorkerError::Failure("Network worker omitted its terminal result".to_owned())))
    }
}

impl<T: Clone> Drop for Worker<T> {
    fn drop(&mut self) {
        self.cancel();
        if let Some(thread) = self.thread.take() {
            if thread.join().is_err() {
                eprintln!("Biogenesis-R network worker panicked during cleanup");
            }
        }
    }
}

fn error(kind: ErrorKind) -> SessionError {
    SessionError { kind, shutdown_error: None }
}

fn check(cancelled: &AtomicBool, deadline: Instant) -> Result<(), SessionError> {
    if cancelled.load(Ordering::Relaxed) { return Err(error(ErrorKind::Cancelled)); }
    if Instant::now() >= deadline { return Err(error(ErrorKind::Deadline)); }
    Ok(())
}

fn interval(deadline: Instant) -> Duration {
    deadline.saturating_duration_since(Instant::now()).min(POLL)
}

fn post(tx: &SyncSender<Rpc>, mut rpc: Rpc, delivery_deadline: Instant) -> Result<(), SessionError> {
    loop {
        check(&rpc.cancelled, delivery_deadline)?;
        match tx.try_send(rpc) {
            Ok(()) => return Ok(()),
            Err(TrySendError::Disconnected(_)) => return Err(error(ErrorKind::Admission)),
            Err(TrySendError::Full(returned)) => rpc = returned,
        }
        thread::park_timeout(interval(delivery_deadline));
    }
}

fn wait_answer<T>(rx: &Receiver<T>, cancelled: &AtomicBool, deadline: Instant) -> Result<T, SessionError> {
    loop {
        check(cancelled, deadline)?;
        match rx.recv_timeout(interval(deadline)) {
            Ok(answer) => { check(cancelled, deadline)?; return Ok(answer); }
            Err(RecvTimeoutError::Disconnected) => return Err(error(ErrorKind::Admission)),
            Err(RecvTimeoutError::Timeout) => {}
        }
    }
}

fn call<T>(tx: &SyncSender<Rpc>, generation: u64, options: SessionOptions,
    cancelled: &Arc<AtomicBool>, make: impl FnOnce(SyncSender<T>) -> Call) -> Result<T, SessionError>
{
    let (answer, result) = mpsc::sync_channel(1);
    post(tx, Rpc { generation, deadline: options.deadline, cancelled: Arc::clone(cancelled), call: make(answer) },
        options.deadline)?;
    wait_answer(&result, cancelled, options.deadline)
}

fn admission<T>(result: Result<T, SessionError>, failure: &Cell<Option<SessionError>>)
    -> Result<T, AdmissionError>
{
    result.map_err(|value| { failure.set(Some(value)); AdmissionError })
}

fn options(limits: Limits) -> SessionOptions {
    SessionOptions { limits, deadline: Instant::now() + SESSION_TIME, read_stall: STALL_TIME, write_stall: STALL_TIME }
}

impl Worker<()> {
    pub fn spawn(listeners: Vec<TcpListener>, generation: u64, tx: SyncSender<Rpc>, limits: Limits)
        -> io::Result<Self>
    {
        if listeners.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "A network listener socket is required"));
        }
        for listener in &listeners { listener.set_nonblocking(true)?; }
        Self::launch("biogenesis-listener", move |cancelled| {
            while !cancelled.load(Ordering::Relaxed) {
                let mut accepted = false;
                for listener in &listeners {
                    if cancelled.load(Ordering::Relaxed) { return Err(WorkerError::Cancelled); }
                    match listener.accept() {
                        Ok((stream, remote)) => {
                            accepted = true;
                            let options = options(limits);
                            let request = Cell::new(None);
                            let peer = Cell::new(None);
                            let bridge_failure = Cell::new(None);
                            let outcome = network_session::receive(stream, options, &cancelled,
                                |actual| {
                                    request.set(Some(actual));
                                    let admitted = admission(call(&tx, generation, options, &cancelled,
                                        |answer| Call::Decide { remote, request: actual, answer }), &bridge_failure)??;
                                    peer.set(admitted.peer);
                                    Ok(admitted.decision)
                                },
                                |payload| {
                                    let peer = peer.get().ok_or(AdmissionError)?;
                                    admission(call(&tx, generation, options, &cancelled,
                                        |answer| Call::Publish { peer, payload, answer }), &bridge_failure)?
                                }).map_err(|mut value| {
                                    if value.kind == ErrorKind::Admission {
                                        if let Some(bridge) = bridge_failure.get() {
                                            value.kind = bridge.kind;
                                            if value.shutdown_error.is_none() { value.shutdown_error = bridge.shutdown_error; }
                                        }
                                    }
                                    value
                                });
                            let shutdown_failed = outcome.as_ref().err().is_some_and(|value| value.shutdown_error.is_some());
                            let session_failure = outcome.as_ref().err().copied().map(WorkerError::from);
                            let result = outcome.map(|_| ()).map_err(|value| value.to_string());
                            let rpc = Rpc { generation, deadline: options.deadline, cancelled: Arc::clone(&cancelled),
                                call: Call::Received { peer: peer.get(), request: request.get(), result } };
                            // Terminal cleanup remains deliverable after the socket deadline expires
                            if let Err(value) = post(&tx, rpc, Instant::now() + SESSION_TIME) {
                                if let Some(WorkerError::Failure(message)) = session_failure {
                                    return Err(WorkerError::Failure(format!("Network listener lost session completion: {value}; session: {message}")));
                                }
                                return Err(match WorkerError::from(value) {
                                    WorkerError::Cancelled => WorkerError::Cancelled,
                                    WorkerError::Failure(message) => WorkerError::Failure(format!("Network listener lost session completion: {message}")),
                                });
                            }
                            if shutdown_failed {
                                return Err(session_failure.expect("A shutdown fault retains its session failure"));
                            }
                        }
                        Err(value) if value.kind() == io::ErrorKind::WouldBlock => {}
                        Err(value) if value.kind() == io::ErrorKind::Interrupted => {}
                        Err(value) => return Err(WorkerError::Failure(format!("Network accept failed: {value}"))),
                    }
                }
                if !accepted { thread::park_timeout(POLL); }
            }
            Err(WorkerError::Cancelled)
        })
    }
}

pub(super) fn spawn_send(remote: SocketAddr, request: Request, generation: u64, job: u64,
    tx: SyncSender<Rpc>, limits: Limits) -> io::Result<SendJob>
{
    Worker::launch("biogenesis-send", move |cancelled| {
        let options = options(limits);
        check(&cancelled, options.deadline).map_err(WorkerError::from)?;
        let stream = TcpStream::connect_timeout(&remote, CONNECT_TIME)
            .map_err(|value| WorkerError::Failure(format!("Network connect failed: {value}")))?;
        network_session::send_with_provider(stream, request, options, &cancelled, || {
            call(&tx, generation, options, &cancelled, |answer| Call::Export { job, answer })?
        }).map_err(WorkerError::from)
    })
}

/// Finish waits for the bounded notification sequence; cancellation remains explicit
pub(super) struct CloseJob(Worker<()>);

impl CloseJob {
    pub fn cancel(&self) {
        self.0.cancel();
    }

    pub fn is_finished(&self) -> bool {
        self.0.is_finished()
    }

    pub fn finish(&mut self) -> Result<(), WorkerError> {
        self.0.join()
    }
}

fn notify_disconnect(remote: SocketAddr, netcode: i32, limits: Limits, cancelled: &AtomicBool,
    deadline: Instant) -> Result<(), WorkerError>
{
    check(cancelled, deadline).map_err(WorkerError::from)?;
    let timeout = deadline.saturating_duration_since(Instant::now()).min(CLOSE_CONNECT_TIME);
    if timeout.is_zero() { return Err(WorkerError::from(error(ErrorKind::Deadline))); }
    let stream = TcpStream::connect_timeout(&remote, timeout)
        .map_err(|value| WorkerError::Failure(format!("Network connect failed: {value}")))?;
    // Let the session report cancellation and shutdown faults even when the connected deadline has expired
    let remaining = deadline.saturating_duration_since(Instant::now()).max(Duration::from_nanos(1));
    let options = SessionOptions { limits, deadline, read_stall: remaining, write_stall: remaining };
    network_session::send(stream, Request::Disconnect { netcode }, None, options, cancelled)
        .map(|_| ()).map_err(WorkerError::from)
}

fn close_result(total: usize, delivered: usize, failed: usize, first_error: Option<String>, stopped: Option<ErrorKind>)
    -> Result<(), WorkerError>
{
    if stopped == Some(ErrorKind::Cancelled) && first_error.is_none() { return Err(WorkerError::Cancelled); }
    if delivered == total && stopped.is_none() { return Ok(()); }
    let first_error = first_error.unwrap_or_else(|| format!("Notification interrupted: {stopped:?}"));
    Err(WorkerError::Failure(format!("Shutdown notifications: {delivered}/{total} delivered; {failed} failed, {} undelivered; stopped: {stopped:?}; first error: {first_error}",
        total - delivered - failed)))
}

pub(super) fn disconnect_peers(peers: Vec<(SocketAddr, i32)>, limits: Limits) -> io::Result<CloseJob> {
    let deadline = Instant::now() + CLOSE_TIME;
    Worker::launch("biogenesis-disconnect", move |cancelled| {
        let total = peers.len();
        let mut delivered = 0;
        let mut failed = 0;
        let mut first_error = None;
        let mut stopped = None;
        for (remote, netcode) in peers {
            if let Err(value) = check(&cancelled, deadline) {
                stopped = Some(value.kind);
                break;
            }
            match notify_disconnect(remote, netcode, limits, &cancelled, deadline) {
                Ok(()) => delivered += 1,
                Err(WorkerError::Cancelled) => {
                    stopped = Some(ErrorKind::Cancelled);
                    break;
                }
                Err(WorkerError::Failure(value)) => {
                    failed += 1;
                    first_error.get_or_insert_with(|| format!("Peer {remote}: {value}"));
                }
            }
        }
        if stopped.is_none() {
            if let Err(value) = check(&cancelled, deadline) { stopped = Some(value.kind); }
        }
        close_result(total, delivered, failed, first_error, stopped)
    }).map(CloseJob)
}

#[cfg(test)]
mod tests {
    use super::*;
    use biogenesis_core::{network_wire::{Reader, Writer}, object_stream::Arena};
    use std::io::Write;
    use std::net::{Ipv4Addr, Shutdown};

    fn listener() -> TcpListener {
        TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap()
    }

    fn connect(address: SocketAddr) -> TcpStream {
        let stream = TcpStream::connect_timeout(&address, CONNECT_TIME).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        stream.set_write_timeout(Some(Duration::from_secs(3))).unwrap();
        stream
    }

    fn write_request(stream: &TcpStream, request: Request) {
        let arena = Arena::default();
        let mut writer = Writer::new(&arena, Limits::default()).unwrap();
        writer.write_request(request).unwrap();
        (&*stream).write_all(&writer.finish().unwrap()).unwrap();
    }

    fn rpc(rx: &Receiver<Rpc>) -> Rpc {
        rx.recv_timeout(Duration::from_secs(3)).unwrap()
    }

    #[test]
    fn cancellation_releases_withheld_decide_and_publish_without_ack() {
        for publishing in [false, true] {
            let socket = listener();
            let address = socket.local_addr().unwrap();
            let (tx, rx) = mpsc::sync_channel(4);
            let mut worker = Listener::spawn(vec![socket], 9, tx, Limits::default()).unwrap();
            let client = connect(address);
            if publishing {
                let bytes = std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../core/tests/fixtures/network_wire_source/recv-code-shared.request.bin")).unwrap();
                (&client).write_all(&bytes).unwrap();
            } else {
                write_request(&client, Request::Connect { version: 900, port: 8888, netcode: -1 });
            }
            let decide = rpc(&rx);
            let Call::Decide { answer, .. } = decide.call else { panic!("Expected admission"); };
            let mut held_decide = Some(answer);
            let mut source = &client;
            let mut reader = Reader::from_reader(&mut source, Limits::default()).unwrap();
            let held;
            if publishing {
                held_decide.take().unwrap().try_send(Ok(Admission { decision: ServerDecision::ReceiveCode, peer: Some(42) })).unwrap_or_else(|_| panic!("Admission receiver remains alive"));
                assert_eq!(reader.read_i32().unwrap(), 9);
                let publish = rpc(&rx);
                let Call::Publish { peer, answer, .. } = publish.call else { panic!("Expected publication"); };
                assert_eq!(peer, 42);
                held = Some(answer);
            } else {
                held = None;
            }
            let started = Instant::now();
            worker.cancel();
            assert_eq!(worker.finish(), Err(WorkerError::Cancelled));
            assert!(started.elapsed() < Duration::from_secs(1));
            assert!(reader.read_i32().is_err(), "No connection or receipt ACK precedes its main-thread action");
            drop(held);
            drop(held_decide);
            assert_eq!(worker.try_result(), Some(Err(WorkerError::Cancelled)));
            assert!(worker.try_result().is_none());
        }
    }

    #[test]
    fn cancellation_releases_withheld_export_and_socket_peer() {
        let socket = listener();
        let address = socket.local_addr().unwrap();
        socket.set_nonblocking(true).unwrap();
        let peer = thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(3);
            let stream = loop {
                match socket.accept() {
                    Ok((stream, _)) => break stream,
                    Err(value) if value.kind() == io::ErrorKind::WouldBlock && Instant::now() < deadline => {
                        thread::park_timeout(POLL);
                    }
                    Err(value) => panic!("Disposable peer accept failed or expired: {value}"),
                }
            };
            network_session::receive(stream, options(Limits::default()), &AtomicBool::new(false),
                |_| Ok(ServerDecision::ReceiveCode), |_| panic!("Cancelled export cannot publish a payload"))
        });
        let (tx, rx) = mpsc::sync_channel(4);
        let mut worker = spawn_send(address, Request::SendCode { netcode: -1 }, 9, 73, tx, Limits::default()).unwrap();
        let export = rpc(&rx);
        let Call::Export { job, answer } = export.call else { panic!("Expected delayed export"); };
        assert_eq!(job, 73);
        let started = Instant::now();
        worker.cancel();
        assert_eq!(worker.finish(), Err(WorkerError::Cancelled));
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(peer.join().unwrap().is_err());
        drop(answer);
    }

    #[test]
    fn malformed_and_stalled_sessions_allow_later_valid_requests() {
        let socket = listener();
        let address = socket.local_addr().unwrap();
        let (tx, rx) = mpsc::sync_channel(4);
        let mut worker = Listener::spawn(vec![socket], 9, tx, Limits::default()).unwrap();
        let malformed = connect(address);
        (&malformed).write_all(&[0, 0, 0, 0]).unwrap();
        malformed.shutdown(Shutdown::Write).unwrap();
        assert!(matches!(rpc(&rx).call, Call::Received { peer: None, request: None, result: Err(_) }));
        let stalled = connect(address);
        (&stalled).write_all(&[0xac, 0xed, 0, 5]).unwrap();
        assert!(matches!(rpc(&rx).call, Call::Received { peer: None, request: None, result: Err(_) }));
        drop(stalled);
        let client = connect(address);
        let request = Request::Connect { version: 900, port: 8888, netcode: -2 };
        write_request(&client, request);
        let Call::Decide { request: actual, answer, .. } = rpc(&rx).call else { panic!("Expected later admission"); };
        assert_eq!(actual, request);
        answer.try_send(Ok(Admission { decision: ServerDecision::Reply(2), peer: Some(43) })).unwrap_or_else(|_| panic!("Admission receiver remains alive"));
        let mut source = &client;
        let mut reader = Reader::from_reader(&mut source, Limits::default()).unwrap();
        assert_eq!(reader.read_i32().unwrap(), 2);
        assert!(matches!(rpc(&rx).call, Call::Received { peer: Some(43), request: Some(actual), result: Ok(()) } if actual == request));
        worker.cancel();
        assert_eq!(worker.finish(), Err(WorkerError::Cancelled));
    }

    #[test]
    fn deadline_and_idle_listener_cleanup_are_bounded() {
        let (_answer, result) = mpsc::sync_channel::<()>(1);
        let cancelled = AtomicBool::new(false);
        assert_eq!(wait_answer(&result, &cancelled, Instant::now() + Duration::from_millis(20)).unwrap_err().kind, ErrorKind::Deadline);
        let (tx, rx) = mpsc::sync_channel(1);
        for full in [false, true] {
            let deadline = Instant::now() + Duration::from_millis(20);
            let result = post(&tx, Rpc { generation: 9, deadline, cancelled: Arc::new(AtomicBool::new(false)),
                call: Call::Received { peer: None, request: None, result: Ok(()) } }, deadline);
            if full { assert_eq!(result.unwrap_err().kind, ErrorKind::Deadline); }
            else { result.unwrap(); }
        }
        assert!(rx.try_recv().is_ok());
        assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
        let socket = listener();
        let address = socket.local_addr().unwrap();
        let (tx, _rx) = mpsc::sync_channel(1);
        let mut worker = Listener::spawn(vec![socket], 9, tx, Limits::default()).unwrap();
        worker.cancel();
        assert_eq!(worker.finish(), Err(WorkerError::Cancelled));
        let rebound = TcpListener::bind(address).unwrap();
        assert_eq!(rebound.local_addr().unwrap(), address);
    }

    #[test]
    fn shutdown_notifications_deliver_to_two_peers_and_bound_stalled_cleanup() {
        let sockets = vec![listener(), listener()];
        let addresses: Vec<_> = sockets.iter().map(|socket| socket.local_addr().unwrap()).collect();
        let (tx, rx) = mpsc::sync_channel(4);
        let mut receiver = Listener::spawn(sockets, 9, tx, Limits::default()).unwrap();
        let mut closing = disconnect_peers(vec![(addresses[0], -1), (addresses[1], -2)], Limits::default()).unwrap();
        for netcode in [-1, -2] {
            let Call::Decide { request, answer, .. } = rpc(&rx).call else { panic!("Expected shutdown notification"); };
            assert_eq!(request, Request::Disconnect { netcode });
            answer.try_send(Ok(Admission { decision: ServerDecision::Reply(15), peer: Some(42) }))
                .unwrap_or_else(|_| panic!("Shutdown sender remains alive"));
            assert!(matches!(rpc(&rx).call, Call::Received { request: Some(Request::Disconnect { netcode: actual }), result: Ok(()), .. } if actual == netcode));
        }
        closing.finish().unwrap();
        receiver.cancel();
        assert_eq!(receiver.finish(), Err(WorkerError::Cancelled));

        let stalled = listener();
        let address = stalled.local_addr().unwrap();
        let started = Instant::now();
        let mut closing = disconnect_peers(vec![(address, 0); 17], Limits::default()).unwrap();
        let WorkerError::Failure(message) = closing.finish().unwrap_err() else { panic!("Deadline remains a failure"); };
        assert!(message.contains("Deadline"));
        assert!(message.contains("0/17 delivered"));
        assert!(started.elapsed() < Duration::from_secs(1));
        let mut closing = disconnect_peers(vec![(address, 0)], Limits::default()).unwrap();
        closing.cancel();
        assert_eq!(closing.finish(), Err(WorkerError::Cancelled));

        let delayed = listener();
        let address = delayed.local_addr().unwrap();
        let (tx, rx) = mpsc::sync_channel(2);
        let mut receiver = Listener::spawn(vec![delayed], 9, tx, Limits::default()).unwrap();
        let started = Instant::now();
        let mut closing = disconnect_peers(vec![(address, -3)], Limits::default()).unwrap();
        let Call::Decide { request, answer, .. } = rpc(&rx).call else { panic!("Expected gated shutdown notification"); };
        assert_eq!(request, Request::Disconnect { netcode: -3 });
        assert!(!closing.is_finished(), "Live peer must reach gated admission before notification expires");
        thread::sleep(Duration::from_millis(150));
        assert!(started.elapsed() < CLOSE_TIME, "Live peer fixture must answer within the global notification budget");
        answer.try_send(Ok(Admission { decision: ServerDecision::Reply(15), peer: Some(42) }))
            .unwrap_or_else(|_| panic!("Gated receiver admission remains alive"));
        let closing_result = closing.finish();
        let received = rpc(&rx).call;
        receiver.cancel();
        assert_eq!(receiver.finish(), Err(WorkerError::Cancelled));
        assert_eq!(closing_result, Ok(()), "Live delayed peer must retain remaining global notification time");
        assert!(matches!(received, Call::Received { peer: Some(42), request: Some(Request::Disconnect { netcode: -3 }), result: Ok(()) }));
    }

    #[test]
    fn cancellation_classification_preserves_shutdown_and_transport_faults() {
        assert_eq!(WorkerError::from(error(ErrorKind::Cancelled)), WorkerError::Cancelled);
        for failure in [
            SessionError { kind: ErrorKind::Cancelled, shutdown_error: Some(io::ErrorKind::BrokenPipe) },
            error(ErrorKind::Io(io::ErrorKind::ConnectionRefused)),
            error(ErrorKind::UnexpectedDecision),
        ] {
            assert!(matches!(WorkerError::from(failure), WorkerError::Failure(_)));
        }
        assert_eq!(close_result(2, 1, 0, None, Some(ErrorKind::Cancelled)), Err(WorkerError::Cancelled));
        let WorkerError::Failure(message) = close_result(3, 1, 1, Some("Original shutdown fault".into()), Some(ErrorKind::Cancelled)).unwrap_err()
            else { panic!("A prior genuine close failure survives cancellation"); };
        assert!(message.contains("1/3 delivered; 1 failed, 1 undelivered"));
        assert!(message.contains("Original shutdown fault"));
        assert!(matches!(close_result(2, 0, 0, None, Some(ErrorKind::Deadline)), Err(WorkerError::Failure(_))));
    }
}
