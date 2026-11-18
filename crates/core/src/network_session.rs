// SPDX-License-Identifier: GPL-2.0-or-later

//! One legacy command on an established socket; callers own connection policy and publication
//! Callbacks run synchronously and must bound waits by the session deadline and cancellation flag
//! An acknowledgment lost after publication remains an ambiguous legacy transfer, never an automatic retry

use crate::network_wire::{self, GeneticCodePayload, Request};
use crate::object_stream::{Arena, Limits};
use std::{cell::Cell, fmt, io::{self, Read, Write}, net::{Shutdown, TcpStream},
    sync::atomic::{AtomicBool, Ordering}, time::{Duration, Instant}};

#[derive(Clone, Copy, Debug)]
pub struct SessionOptions {
    pub limits: Limits,
    pub deadline: Instant,
    pub read_stall: Duration,
    pub write_stall: Duration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reply {
    pub initial: i32,
    pub final_reply: Option<i32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServerDecision {
    Reply(i32),
    ReceiveCode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    Wire(network_wire::WireError),
    Io(io::ErrorKind),
    InvalidOptions,
    MissingGeneticCode,
    UnexpectedDecision,
    Admission,
    Cancelled,
    Deadline,
    ReadStall,
    WriteStall,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionError {
    pub kind: ErrorKind,
    pub shutdown_error: Option<io::ErrorKind>,
}

impl fmt::Display for SessionError {
    fn fmt(&self, output: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(output, "Legacy socket session: {:?}; shutdown: {:?}", self.kind, self.shutdown_error)
    }
}
impl std::error::Error for SessionError {}

type Result<T> = std::result::Result<T, SessionError>;
fn failure(kind: ErrorKind) -> SessionError { SessionError { kind, shutdown_error: None } }

struct Socket<'a> {
    stream: TcpStream,
    options: SessionOptions,
    cancelled: &'a AtomicBool,
    io_failure: Cell<Option<ErrorKind>>,
}

impl Socket<'_> {
    fn check(&self) -> Result<()> {
        if self.cancelled.load(Ordering::Relaxed) { return Err(failure(ErrorKind::Cancelled)); }
        if Instant::now() >= self.options.deadline { return Err(failure(ErrorKind::Deadline)); }
        Ok(())
    }

    fn validate(&self) -> Result<()> {
        if self.options.read_stall.is_zero() || self.options.write_stall.is_zero() {
            return Err(failure(ErrorKind::InvalidOptions));
        }
        self.stream.set_nonblocking(false).map_err(|value| failure(ErrorKind::Io(value.kind())))?;
        self.check()
    }

    fn timeout(&self, progress: Instant, reading: bool) -> Result<Duration> {
        self.check()?;
        let now = Instant::now();
        let stall = if reading { self.options.read_stall } else { self.options.write_stall };
        let elapsed = now.saturating_duration_since(progress);
        if elapsed >= stall {
            return Err(failure(if reading { ErrorKind::ReadStall } else { ErrorKind::WriteStall }));
        }
        Ok(self.options.deadline.saturating_duration_since(now)
            .min(stall - elapsed).min(Duration::from_millis(10)).max(Duration::from_nanos(1)))
    }

    fn read(&self, output: &mut [u8]) -> io::Result<usize> {
        let progress = Instant::now();
        loop {
            let timeout = match self.timeout(progress, true) {
                Ok(value) => value,
                Err(value) => { self.io_failure.set(Some(value.kind)); return Err(io::ErrorKind::ConnectionAborted.into()); }
            };
            self.stream.set_read_timeout(Some(timeout))?;
            match (&self.stream).read(output) {
                Err(value) if matches!(value.kind(), io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) => continue,
                result => return result,
            }
        }
    }

    fn write_all(&self, mut input: &[u8]) -> Result<()> {
        let mut progress = Instant::now();
        while !input.is_empty() {
            let timeout = self.timeout(progress, false)?;
            self.stream.set_write_timeout(Some(timeout)).map_err(|value| failure(ErrorKind::Io(value.kind())))?;
            match (&self.stream).write(input) {
                Ok(0) => return Err(failure(ErrorKind::Io(io::ErrorKind::WriteZero))),
                Ok(count) => { input = &input[count..]; progress = Instant::now(); }
                Err(value) if matches!(value.kind(), io::ErrorKind::Interrupted | io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) => continue,
                Err(value) => return Err(failure(ErrorKind::Io(value.kind()))),
            }
        }
        self.check()
    }

    fn wire<T>(&self, result: std::result::Result<T, network_wire::WireError>) -> Result<T> {
        result.map_err(|value| failure(self.io_failure.take().unwrap_or(ErrorKind::Wire(value))))
    }

    fn finish<T>(&self, result: Result<T>) -> Result<T> {
        match self.stream.shutdown(Shutdown::Both) {
            Ok(()) => result,
            Err(value) if value.kind() == io::ErrorKind::NotConnected => result,
            Err(value) => Err(match result {
                Ok(_) => SessionError { kind: ErrorKind::Io(value.kind()), shutdown_error: Some(value.kind()) },
                Err(mut original) => { original.shutdown_error = Some(value.kind()); original }
            }),
        }
    }
}

impl Drop for Socket<'_> {
    fn drop(&mut self) {
        // Normal exits report shutdown errors; unwinding still terminates the socket
        let _ = self.stream.shutdown(Shutdown::Both);
    }
}

impl Read for &Socket<'_> {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> { Socket::read(*self, output) }
}

fn charge_reader(reader: &mut network_wire::Reader<'_>, writer: &network_wire::Writer<'_>) -> std::result::Result<(), network_wire::WireError> {
    let (allocated, work) = writer.usage();
    let (previous_allocated, previous_work) = reader.usage();
    reader.charge_external(allocated - previous_allocated, work - previous_work)
}

fn charge_writer(writer: &mut network_wire::Writer<'_>, reader: &network_wire::Reader<'_>) -> std::result::Result<(), network_wire::WireError> {
    let (allocated, work) = reader.usage();
    let (previous_allocated, previous_work) = writer.usage();
    writer.charge_external(allocated - previous_allocated, work - previous_work)
}

/// Send one command; raw replies do not change any connection state or inactivity clock
pub fn send(stream: TcpStream, request: Request, code: Option<&GeneticCodePayload>, options: SessionOptions,
    cancelled: &AtomicBool) -> Result<Reply> {
    send_inner(stream, request, code, options, cancelled, None::<fn() -> Result<GeneticCodePayload>>)
}

/// Obtain a detached payload only after SendCode receives WAITING_CODE
/// The synchronous provider must bound any wait by the supplied deadline and cancellation flag
/// No Send bound is imposed on the provider or its captured owners
pub fn send_with_provider(stream: TcpStream, request: Request, options: SessionOptions,
    cancelled: &AtomicBool, provide: impl FnOnce() -> Result<GeneticCodePayload>) -> Result<Reply> {
    send_inner(stream, request, None, options, cancelled, Some(provide))
}

fn send_inner(stream: TcpStream, request: Request, code: Option<&GeneticCodePayload>, options: SessionOptions,
    cancelled: &AtomicBool, provide: Option<impl FnOnce() -> Result<GeneticCodePayload>>) -> Result<Reply> {
    let socket = Socket { stream, options, cancelled, io_failure: Cell::new(None) };
    let result = (|| {
        socket.validate()?;
        let supplied;
        let empty = Arena::default();
        let arena = if matches!(request, Request::SendCode { .. }) {
            code.map_or(&empty, |payload| &payload.document().arena)
        } else { &empty };
        let mut writer = socket.wire(network_wire::Writer::new(arena, options.limits))?;
        socket.write_all(writer.output())?;
        let (allocated, work) = writer.usage();
        let mut source = &socket;
        let mut reader = socket.wire(network_wire::Reader::from_reader_charged(&mut source, options.limits, allocated, work))?;
        socket.wire(charge_writer(&mut writer, &reader))?;
        let start = writer.position();
        socket.wire(writer.write_request(request))?;
        socket.wire(charge_reader(&mut reader, &writer))?;
        socket.write_all(&writer.output()[start..])?;
        let initial = socket.wire(reader.read_i32())?;
        let final_reply = if matches!(request, Request::SendCode { .. }) && initial == 9 {
            let payload = if let Some(payload) = code { payload } else {
                let provide = provide.ok_or(failure(ErrorKind::MissingGeneticCode))?;
                socket.check()?;
                let result = provide();
                socket.check()?;
                supplied = result?;
                &supplied
            };
            socket.wire(charge_writer(&mut writer, &reader))?;
            if code.is_none() {
                writer = socket.wire(writer.bind_empty_arena(&payload.document().arena))?;
            }
            let start = writer.position();
            socket.wire(writer.write_genetic_code(payload.root()))?;
            socket.wire(charge_reader(&mut reader, &writer))?;
            socket.write_all(&writer.output()[start..])?;
            Some(socket.wire(reader.read_i32())?)
        } else { None };
        socket.wire(reader.finish())?;
        socket.check()?;
        Ok(Reply { initial, final_reply })
    })();
    socket.finish(result)
}

/// Receive one command, hydrate detached owners, then publish before acknowledging accepted delivery
/// Callbacks own admission policy and bound waits by deadline and cancellation; checks also surround publication
pub fn receive(
    stream: TcpStream, options: SessionOptions, cancelled: &AtomicBool,
    decide: impl FnOnce(Request) -> std::result::Result<ServerDecision, AdmissionError>,
    publish: impl FnOnce(GeneticCodePayload) -> std::result::Result<(), AdmissionError>,
) -> Result<Request> {
    let socket = Socket { stream, options, cancelled, io_failure: Cell::new(None) };
    let result = (|| {
        socket.validate()?;
        let mut source = &socket;
        let mut reader = socket.wire(network_wire::Reader::from_reader(&mut source, options.limits))?;
        let empty = Arena::default();
        let (allocated, work) = reader.usage();
        let mut writer = socket.wire(network_wire::Writer::new_charged(&empty, options.limits, allocated, work))?;
        socket.wire(charge_reader(&mut reader, &writer))?;
        socket.write_all(writer.output())?;
        let request = socket.wire(reader.read_request())?;
        socket.check()?;
        let decision = decide(request).map_err(|_| failure(ErrorKind::Admission))?;
        socket.check()?;
        socket.wire(charge_writer(&mut writer, &reader))?;
        let start = writer.position();
        match decision {
            ServerDecision::Reply(reply) => {
                if matches!(request, Request::SendCode { .. }) && matches!(reply, 9 | 10) {
                    return Err(failure(ErrorKind::UnexpectedDecision));
                }
                socket.wire(writer.write_i32(reply))?;
                socket.wire(charge_reader(&mut reader, &writer))?;
                socket.wire(reader.finish())?;
                socket.write_all(&writer.output()[start..])?;
            }
            ServerDecision::ReceiveCode => {
                if !matches!(request, Request::SendCode { .. }) { return Err(failure(ErrorKind::UnexpectedDecision)); }
                socket.wire(writer.write_i32(9))?;
                let boundary = writer.position();
                socket.wire(writer.write_i32(10))?;
                socket.wire(charge_reader(&mut reader, &writer))?;
                socket.write_all(&writer.output()[start..boundary])?;
                let payload = socket.wire(reader.read_genetic_code())?;
                socket.wire(payload.validate())?;
                socket.check()?;
                publish(payload).map_err(|_| failure(ErrorKind::Admission))?;
                socket.check()?;
                socket.write_all(&writer.output()[boundary..])?;
            }
        }
        Ok(request)
    })();
    socket.finish(result)
}

#[cfg(test)]
#[path = "network_session_tests.rs"]
mod tests;
