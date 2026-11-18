// SPDX-License-Identifier: GPL-2.0-or-later

//! Same-directory byte replacement after the caller has completed encoding

use std::{
    fmt,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug)]
pub(crate) enum ReplacePhase {
    Replace,
    InspectDestination,
    Save,
    CreateTemporary,
    TemporaryNamesOccupied,
    WriteAndReplace,
    SynchronizeDirectory,
}

impl ReplacePhase {
    fn operation(self) -> &'static str {
        match self {
            Self::Replace => "replace file",
            Self::InspectDestination => "inspect file destination",
            Self::Save => "save file",
            Self::CreateTemporary | Self::TemporaryNamesOccupied => "create temporary file",
            Self::WriteAndReplace => "write and replace file",
            Self::SynchronizeDirectory => "synchronize file directory",
        }
    }
}

/// Failures distinguish an uncommitted attempt from a replacement with unconfirmed durability
#[derive(Debug)]
pub struct FileReplaceError {
    pub operation: &'static str,
    pub path: PathBuf,
    pub source: io::Error,
    pub replacement_committed: bool,
    pub cleanup_error: Option<io::Error>,
    pub(crate) phase: ReplacePhase,
}

impl FileReplaceError {
    fn new(phase: ReplacePhase, path: &Path, source: io::Error) -> Self {
        Self {
            operation: phase.operation(),
            path: path.to_owned(),
            source,
            replacement_committed: false,
            cleanup_error: None,
            phase,
        }
    }
}

impl fmt::Display for FileReplaceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Cannot {} {}: {}",
            self.operation,
            self.path.display(),
            self.source
        )?;
        if self.replacement_committed {
            f.write_str("; replacement completed, but disk durability is unconfirmed")?;
        }
        if let Some(error) = &self.cleanup_error {
            write!(f, "; cannot remove the temporary file: {error}")?;
        }
        Ok(())
    }
}

impl std::error::Error for FileReplaceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

fn temporary_file(parent: &Path, destination: &Path) -> Result<(PathBuf, File), FileReplaceError> {
    for _ in 0..128 {
        let temporary_name = format!(
            ".biogenesis-r-{}-{}.tmp",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        );
        let path = parent.join(temporary_name);
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => {
                return Err(FileReplaceError::new(
                    ReplacePhase::CreateTemporary,
                    destination,
                    error,
                ));
            }
        }
    }
    Err(FileReplaceError::new(
        ReplacePhase::TemporaryNamesOccupied,
        destination,
        io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Temporary file names are occupied",
        ),
    ))
}

/// Replace through a private sibling after the caller has produced all output bytes
/// Reject symbolic links and read-only destinations; preserve existing regular-file permissions
/// Unix also synchronizes the parent directory; other platforms have no additional durability promise
pub fn replace_file_bytes(path: &Path, bytes: &[u8]) -> Result<(), FileReplaceError> {
    let permissions = match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if !metadata.is_file() || metadata.permissions().readonly() {
                return Err(FileReplaceError::new(
                    ReplacePhase::Replace,
                    path,
                    io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "Destination must be a writable regular file",
                    ),
                ));
            }
            Some(metadata.permissions())
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(FileReplaceError::new(
                ReplacePhase::InspectDestination,
                path,
                error,
            ));
        }
    };
    replace_with_permissions(path, bytes, permissions)
}

pub(crate) fn replace_with_permissions(
    path: &Path,
    bytes: &[u8],
    permissions: Option<fs::Permissions>,
) -> Result<(), FileReplaceError> {
    path.file_name().ok_or_else(|| {
        FileReplaceError::new(
            ReplacePhase::Save,
            path,
            io::Error::new(io::ErrorKind::InvalidInput, "Destination has no file name"),
        )
    })?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let (temporary, mut file) = temporary_file(parent, path)?;
    let prepared = (|| {
        if let Some(permissions) = permissions {
            file.set_permissions(permissions)?;
        }
        file.write_all(bytes)?;
        file.sync_all()
    })();
    drop(file);
    let replacement = prepared.and_then(|()| fs::rename(&temporary, path));
    if let Err(source) = replacement {
        let mut error = FileReplaceError::new(ReplacePhase::WriteAndReplace, path, source);
        error.cleanup_error = fs::remove_file(&temporary).err();
        return Err(error);
    }
    #[cfg(unix)]
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|source| {
            let mut error =
                FileReplaceError::new(ReplacePhase::SynchronizeDirectory, parent, source);
            error.replacement_committed = true;
            error
        })?;
    Ok(())
}
