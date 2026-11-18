// SPDX-License-Identifier: GPL-2.0-or-later

//! Bounded World reads and complete encoding before durable sibling replacement

use super::World;
use crate::{FileReplaceError, object_stream::{ErrorKind, LimitKind, Limits, StreamError, StreamReader}};
use std::{
    fmt,
    fs::{self, OpenOptions},
    io::{self, Read},
    path::{Path, PathBuf},
};

/// File errors keep codec failures separate from typed graph and filesystem failures
#[derive(Debug)]
pub enum WorldFileError {
    Stream(StreamError),
    Decode { details: String },
    Encode { details: String },
    Io { operation: &'static str, path: PathBuf, source: io::Error },
    Replace(FileReplaceError),
}

impl fmt::Display for WorldFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stream(error) => error.fmt(f),
            Self::Decode { details } => write!(f, "Cannot decode World: {details}"),
            Self::Encode { details } => write!(f, "Cannot encode World: {details}"),
            Self::Io { operation, path, source } => {
                write!(f, "Cannot {operation} {}: {source}", path.display())
            }
            Self::Replace(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for WorldFileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Stream(error) => Some(error),
            Self::Io { source, .. } => Some(source),
            Self::Replace(error) => Some(error),
            Self::Decode { .. } | Self::Encode { .. } => None,
        }
    }
}

fn file_error(operation: &'static str, path: &Path, source: io::Error) -> WorldFileError {
    WorldFileError::Io { operation, path: path.to_owned(), source }
}

fn stream_error(kind: ErrorKind) -> WorldFileError {
    WorldFileError::Stream(StreamError { offset: 0, kind })
}

fn regular_file(metadata: &fs::Metadata, path: &Path, maximum: usize) -> Result<(), WorldFileError> {
    if !metadata.is_file() {
        return Err(file_error("read World", path, io::Error::new(
            io::ErrorKind::InvalidInput, "World input must be a regular file",
        )));
    }
    if metadata.len() > u64::try_from(maximum).unwrap_or(u64::MAX) {
        return Err(stream_error(ErrorKind::Limit(LimitKind::Input)));
    }
    Ok(())
}

fn read_world_bytes(path: &Path, maximum: usize, allocation: usize, work: usize) -> Result<Vec<u8>, WorldFileError> {
    let metadata = fs::metadata(path).map_err(|error| file_error("inspect World", path, error))?;
    regular_file(&metadata, path, maximum)?;
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // Linux O_NONBLOCK prevents a FIFO path swap from blocking before handle validation
        options.custom_flags(0o4000);
    }
    let mut file = options.open(path).map_err(|error| file_error("open World", path, error))?;
    regular_file(&file.metadata().map_err(|error| file_error("inspect opened World", path, error))?, path, maximum)?;
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 8192];
    loop {
        // Probe one byte at the boundary without retaining an oversized input
        let remaining = maximum - bytes.len();
        let count = if remaining == 0 { 1 } else { remaining.min(chunk.len()) };
        let count = match file.read(&mut chunk[..count]) {
            Ok(count) => count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(file_error("read World", path, error)),
        };
        if count == 0 { return Ok(bytes); }
        if count > remaining { return Err(stream_error(ErrorKind::Limit(LimitKind::Input))); }
        let needed = bytes.len() + count;
        if needed > work { return Err(stream_error(ErrorKind::Limit(LimitKind::Work))); }
        if needed > allocation { return Err(stream_error(ErrorKind::Limit(LimitKind::Allocation))); }
        if needed > bytes.capacity() {
            let capacity = bytes.capacity().saturating_mul(2).max(chunk.len()).max(needed).min(maximum).min(allocation);
            bytes.try_reserve_exact(capacity - bytes.len())
                .map_err(|_| stream_error(ErrorKind::Allocation))?;
        }
        if bytes.capacity() > allocation { return Err(stream_error(ErrorKind::Limit(LimitKind::Allocation))); }
        bytes.extend_from_slice(&chunk[..count]);
    }
}

impl World {
    /// Read a bounded file into a new root sharing this session; publish only after success
    /// Neither successful reads nor failed reads change this root, session settings, or RNG
    pub fn load_legacy_file(&self, path: &Path, mut limits: Limits, maximum: usize) -> Result<Self, WorldFileError> {
        limits.allocated_bytes = limits.allocated_bytes.min(maximum);
        let bytes = read_world_bytes(path, limits.input_bytes, limits.allocated_bytes, limits.work)?;
        let input_capacity = bytes.capacity();
        limits.allocated_bytes -= input_capacity;
        limits.work -= bytes.len();
        let (document, allocated, work) = StreamReader::new(&bytes, limits).map_err(WorldFileError::Stream)?
            .read_document_with_usage().map_err(WorldFileError::Stream)?;
        limits.allocated_bytes -= allocated;
        limits.work -= work;
        // The Document owns its payload, so release the input allocation before hydration
        drop(bytes);
        limits.allocated_bytes += input_capacity;
        self.decode_legacy_document(&document, limits, limits.allocated_bytes)
            .map_err(|error| WorldFileError::Decode { details: format!("{error:?}") })
    }

    /// Encode fully before replacement; errors retain committed, cleanup, and durability details
    /// Existing replacement rules reject symlinks and read-only destinations and preserve permissions
    pub fn save_legacy_file(&self, path: &Path, limits: Limits, maximum: usize) -> Result<(), WorldFileError> {
        let bytes = self.encode_legacy_file_bytes(limits, maximum)?;
        crate::replace_file_bytes(path, &bytes).map_err(WorldFileError::Replace)
    }

    /// Produce ordinary legacy World bytes under the same bounds and errors as file saving
    pub fn encode_legacy_file_bytes(&self, limits: Limits, maximum: usize) -> Result<Vec<u8>, WorldFileError> {
        self.encode_legacy_bytes(limits, maximum).map_err(|error| match error {
            super::legacy_export::WorldExportError::Stream(error) => WorldFileError::Stream(error),
            error => WorldFileError::Encode { details: format!("{error:?}") },
        })
    }
}

#[cfg(test)]
#[path = "world_file_tests.rs"]
mod tests;
