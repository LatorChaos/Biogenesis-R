// SPDX-License-Identifier: GPL-2.0-or-later

//! Locked recovery storage; quota counts file lengths, including staging and unknown entries
//! The marker reserves strict recovery and atomic-writer temporary names for this store

use biogenesis_core::{World, WorldError, WorldFileError, FileReplaceError};
use std::{
    fmt,
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

const MARKER: &str = ".biogenesis-recovery-v1";
const MAGIC: &[u8] = b"Biogenesis-R managed recovery v1\n";
const LOCK: &str = ".biogenesis-recovery.lock";
const MAX_ENTRIES: usize = 128;
const MAX_REJECTIONS: usize = 16;

#[derive(Clone, Copy, Debug)]
pub struct RecoveryPolicy {
    pub maximum_bytes: u64,
    pub maximum_snapshots: usize,
}
impl Default for RecoveryPolicy {
    fn default() -> Self {
        Self { maximum_bytes: 1024 * 1024 * 1024, maximum_snapshots: 8 }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotInfo {
    pub generation: u64,
    pub time: i64,
    pub path: PathBuf,
    pub bytes: u64,
}

#[derive(Clone, Debug)]
pub struct RecoveryRejection {
    pub path: PathBuf,
    pub details: String,
}

#[derive(Debug)]
pub enum RecoveryError {
    Policy,
    Ownership(String),
    Quota,
    NoValidSnapshot,
    Io { operation: &'static str, path: PathBuf, source: io::Error, committed: bool },
    World { source: WorldFileError, committed: bool },
    Drawing { source: WorldError, committed: bool },
    Replace(FileReplaceError),
    Finalization(Box<RecoveryError>),
}
impl RecoveryError {
    pub fn replacement_committed(&self) -> bool {
        match self {
            Self::Io { committed, .. } | Self::World { committed, .. } | Self::Drawing { committed, .. } => *committed,
            Self::Replace(error) => error.replacement_committed,
            Self::Finalization(_) => true,
            _ => false,
        }
    }
}
impl fmt::Display for RecoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Policy => f.write_str("Recovery requires 1..=8 snapshots and at most 1 GiB of file lengths"),
            Self::Ownership(details) => write!(f, "Cannot own recovery directory: {details}"),
            Self::Quota => f.write_str("Recovery quota cannot admit staging while preserving the last valid and protected files"),
            Self::NoValidSnapshot => f.write_str("No valid recovery snapshot; rejected diagnostics are retained"),
            Self::Io { operation, path, source, committed } => {
                write!(f, "Cannot {operation} {}: {source}", path.display())?;
                if *committed { f.write_str("; a recovery file was committed")?; }
                Ok(())
            }
            Self::World { source, committed } => {
                source.fmt(f)?;
                if *committed { f.write_str("; recovery staging was committed but could not be validated")?; }
                Ok(())
            }
            Self::Drawing { source, committed } => {
                write!(f, "Cannot draw recovery World: {source}")?;
                if *committed { f.write_str("; recovery staging was committed but is not usable")?; }
                Ok(())
            }
            Self::Replace(error) => error.fmt(f),
            Self::Finalization(error) => write!(f, "{error}; a valid recovery staging file was committed"),
        }
    }
}
impl std::error::Error for RecoveryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::World { source, .. } => Some(source),
            Self::Drawing { source, .. } => Some(source),
            Self::Replace(error) => Some(error),
            Self::Finalization(error) => Some(error),
            _ => None,
        }
    }
}
fn io_error(operation: &'static str, path: &Path, source: io::Error, committed: bool) -> RecoveryError {
    RecoveryError::Io { operation, path: path.to_owned(), source, committed }
}

struct Entry {
    info: SnapshotInfo,
    pending: bool,
    metadata: fs::Metadata,
}
struct Inventory {
    snapshots: Vec<Entry>,
    remnants: Vec<(PathBuf, fs::Metadata)>,
    total: u64,
    entries: usize,
}

fn snapshot_name(generation: u64, time: i64, pending: bool) -> String {
    let prefix = if pending { ".recovery" } else { "recovery" };
    format!("{prefix}-{generation:020}-{:016x}.bgw", time as u64)
}
fn parse_snapshot(name: &str) -> Option<(u64, i64, bool)> {
    let pending = name.starts_with(".recovery-");
    let text = name.strip_prefix(if pending { ".recovery-" } else { "recovery-" })?.strip_suffix(".bgw")?;
    let (generation, time) = text.split_once('-')?;
    let generation = generation.parse::<u64>().ok()?;
    let time = u64::from_str_radix(time, 16).ok()? as i64;
    (name == snapshot_name(generation, time, pending)).then_some((generation, time, pending))
}
fn writer_temporary(name: &str) -> bool {
    let Some(text) = name.strip_prefix(".biogenesis-r-").and_then(|s| s.strip_suffix(".tmp")) else { return false; };
    let Some((pid, sequence)) = text.split_once('-') else { return false; };
    let (Ok(pid), Ok(sequence)) = (pid.parse::<u32>(), sequence.parse::<u64>()) else { return false; };
    pid != 0 && name == format!(".biogenesis-r-{pid}-{sequence}.tmp")
}
fn same_file(a: &fs::Metadata, b: &fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        a.dev() == b.dev() && a.ino() == b.ino()
    }
    #[cfg(not(unix))]
    { a.len() == b.len() && a.created().ok() == b.created().ok() && a.modified().ok() == b.modified().ok() }
}
fn no_links(path: &Path) -> Result<(), RecoveryError> {
    for ancestor in path.ancestors() {
        if ancestor.is_symlink() {
            return Err(RecoveryError::Ownership(format!("Symbolic link: {}", ancestor.display())));
        }
    }
    Ok(())
}
fn sync_directory(directory: &Path, committed: bool) -> Result<(), RecoveryError> {
    #[cfg(unix)]
    File::open(directory).and_then(|file| file.sync_all())
        .map_err(|error| io_error("synchronize recovery directory", directory, error, committed))?;
    #[cfg(not(unix))]
    let _ = (directory, committed);
    Ok(())
}

#[cfg(target_os = "linux")]
fn prepare_marker(directory: &Path) -> Result<File, RecoveryError> {
    use std::{io::Write, os::unix::fs::OpenOptionsExt};
    // An unnamed inode cannot strand a reserved writer path after process death
    let mut file = OpenOptions::new().write(true).mode(0o600).custom_flags(libc::O_TMPFILE)
        .open(directory).map_err(|error| io_error("create unnamed recovery marker", directory, error, false))?;
    file.write_all(MAGIC).and_then(|()| file.sync_all())
        .map_err(|error| io_error("write recovery marker", directory, error, false))?;
    Ok(file)
}

#[cfg(target_os = "linux")]
fn initialize_marker_with_sync(directory: &Path, synchronize: fn(&Path, bool) -> Result<(), RecoveryError>) -> Result<(), RecoveryError> {
    use std::{ffi::CString, os::fd::AsRawFd};
    let file = prepare_marker(directory)?;
    let parent = File::open(directory)
        .map_err(|error| io_error("open recovery directory", directory, error, false))?;
    let source = CString::new(format!("/proc/self/fd/{}", file.as_raw_fd())).unwrap();
    // Following this owned descriptor avoids requiring CAP_DAC_READ_SEARCH
    if unsafe { libc::linkat(libc::AT_FDCWD, source.as_ptr(), parent.as_raw_fd(), c".biogenesis-recovery-v1".as_ptr(), libc::AT_SYMLINK_FOLLOW) } != 0 {
        return Err(io_error("publish recovery marker", &directory.join(MARKER), io::Error::last_os_error(), false));
    }
    synchronize(directory, true)
}

fn initialize_marker(directory: &Path) -> Result<(), RecoveryError> {
    #[cfg(target_os = "linux")]
    { initialize_marker_with_sync(directory, sync_directory) }
    #[cfg(not(target_os = "linux"))]
    biogenesis_core::replace_file_bytes(&directory.join(MARKER), MAGIC).map_err(RecoveryError::Replace)
}

struct Protection {
    path: PathBuf,
    metadata: Option<fs::Metadata>,
}
impl Protection {
    fn new(path: &Path) -> Result<Self, RecoveryError> {
        let metadata = match fs::metadata(path) {
            Ok(metadata) => Some(metadata),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(io_error("inspect protected path", path, error, false)),
        };
        let resolved = if metadata.is_some() {
            fs::canonicalize(path)
        } else {
            let parent = path.parent().filter(|p| !p.as_os_str().is_empty()).unwrap_or(Path::new("."));
            fs::canonicalize(parent).map(|parent| parent.join(path.file_name().unwrap_or_default()))
        }.map_err(|error| io_error("resolve protected path", path, error, false))?;
        Ok(Self { path: resolved, metadata })
    }
    fn contains(&self, path: &Path, metadata: &fs::Metadata) -> bool {
        self.path == path || self.metadata.as_ref().is_some_and(|owner| same_file(owner, metadata))
    }
}

pub(super) struct PreparedCheckpoint {
    protection: Option<Protection>,
    bytes: Vec<u8>,
    time: i64,
}

pub struct RecoveryStore {
    directory: PathBuf,
    lock: File,
    policy: RecoveryPolicy,
    recovered: Option<SnapshotInfo>,
    rejections: Vec<RecoveryRejection>,
}
impl RecoveryStore {
    /// Hold a nonblocking exclusive file lock until this store is dropped
    /// Only exact reserved names under a valid marker are owned; unknown names are preserved
    pub fn open(directory: &Path, policy: RecoveryPolicy) -> Result<Self, RecoveryError> {
        if policy.maximum_snapshots == 0 || policy.maximum_snapshots > 8
            || policy.maximum_bytes < MAGIC.len() as u64 || policy.maximum_bytes > 1024 * 1024 * 1024 {
            return Err(RecoveryError::Policy);
        }
        no_links(directory)?;
        if !directory.exists() {
            let mut builder = fs::DirBuilder::new();
            #[cfg(unix)]
            { use std::os::unix::fs::DirBuilderExt; builder.mode(0o700); }
            builder.create(directory).map_err(|error| io_error("create recovery directory", directory, error, false))?;
        }
        no_links(directory)?;
        let directory = fs::canonicalize(directory).map_err(|error| io_error("resolve recovery directory", directory, error, false))?;
        let path = directory.join(LOCK);
        if path.is_symlink() { return Err(RecoveryError::Ownership("Lock path is a symbolic link".into())); }
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        { use std::os::unix::fs::OpenOptionsExt; options.mode(0o600); }
        #[cfg(target_os = "linux")]
        { use std::os::unix::fs::OpenOptionsExt; options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK); }
        let lock = options.open(&path).map_err(|error| io_error("open recovery lock", &path, error, false))?;
        let metadata = lock.metadata().map_err(|error| io_error("inspect recovery lock", &path, error, false))?;
        if !metadata.is_file() || metadata.len() != 0 {
            return Err(RecoveryError::Ownership("Lock must be an empty regular file".into()));
        }
        lock.try_lock().map_err(|error| RecoveryError::Ownership(error.to_string()))?;
        let store = Self { directory, lock, policy, recovered: None, rejections: Vec::new() };
        let marker = store.directory.join(MARKER);
        if !marker.exists() && !marker.is_symlink() {
            let inventory = store.inventory()?;
            if !inventory.snapshots.is_empty() || !inventory.remnants.is_empty() {
                return Err(RecoveryError::Ownership("Reserved names exist without an ownership marker".into()));
            }
            if inventory.total.checked_add(MAGIC.len() as u64).is_none_or(|total| total > policy.maximum_bytes) {
                return Err(RecoveryError::Quota);
            }
            initialize_marker(&store.directory)?;
        }
        store.verify_owner()?;
        Ok(store)
    }

    pub fn last_recovered(&self) -> Option<&SnapshotInfo> { self.recovered.as_ref() }
    pub fn rejections(&self) -> &[RecoveryRejection] { &self.rejections }

    fn reject(&mut self, path: &Path, error: &impl fmt::Display) {
        let details: String = error.to_string().chars().take(2048).collect();
        if self.rejections.iter().any(|item| item.path == path && item.details == details) { return; }
        if self.rejections.len() == MAX_REJECTIONS { self.rejections.remove(0); }
        self.rejections.push(RecoveryRejection { path: path.to_owned(), details });
    }
    fn verify_owner(&self) -> Result<(), RecoveryError> {
        no_links(&self.directory)?;
        let path = self.directory.join(LOCK);
        let metadata = fs::symlink_metadata(&path).map_err(|error| io_error("inspect recovery lock", &path, error, false))?;
        let opened = self.lock.metadata().map_err(|error| io_error("inspect owned recovery lock", &path, error, false))?;
        if !metadata.is_file() || !same_file(&metadata, &opened) || metadata.len() != 0 {
            return Err(RecoveryError::Ownership("Lock path changed".into()));
        }
        self.marker_file(false).map(|_| ())
    }

    fn marker_file(&self, write: bool) -> Result<(File, bool), RecoveryError> {
        let path = self.directory.join(MARKER);
        let before = fs::symlink_metadata(&path)
            .map_err(|error| io_error("inspect recovery marker", &path, error, false))?;
        let mut options = OpenOptions::new();
        options.read(true).write(write);
        #[cfg(target_os = "linux")]
        { use std::os::unix::fs::OpenOptionsExt; options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK); }
        let mut file = options.open(&path)
            .map_err(|error| io_error("open recovery marker", &path, error, false))?;
        let opened = file.metadata().map_err(|error| io_error("inspect admitted recovery marker", &path, error, false))?;
        self.verify_marker_identity(&before, &opened)?;
        let mut bytes = Vec::new();
        (&mut file).take(MAGIC.len() as u64 + 1).read_to_end(&mut bytes)
            .map_err(|error| io_error("read recovery marker", &path, error, false))?;
        if bytes.len() != MAGIC.len() || bytes[..MAGIC.len() - 1] != MAGIC[..MAGIC.len() - 1]
            || !matches!(bytes[MAGIC.len() - 1], b'\n' | b'!') {
            return Err(RecoveryError::Ownership("Invalid recovery marker".into()));
        }
        let after = fs::symlink_metadata(&path)
            .map_err(|error| io_error("reinspect recovery marker", &path, error, false))?;
        self.verify_marker_identity(&after, &opened)?;
        Ok((file, bytes[MAGIC.len() - 1] == b'!'))
    }

    fn verify_marker_identity(&self, path: &fs::Metadata, opened: &fs::Metadata) -> Result<(), RecoveryError> {
        if !path.is_file() || !same_file(path, opened) || opened.len() != MAGIC.len() as u64 {
            return Err(RecoveryError::Ownership("Recovery marker identity or size changed".into()));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if opened.nlink() != 1 {
                return Err(RecoveryError::Ownership("Recovery marker must have one link and the current owner".into()));
            }
        }
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::fs::MetadataExt;
            if opened.uid() != unsafe { libc::geteuid() } {
                return Err(RecoveryError::Ownership("Recovery marker has a different owner".into()));
            }
        }
        Ok(())
    }

    pub fn manual_pending(&self) -> Result<bool, RecoveryError> {
        self.verify_owner()?;
        self.marker_file(false).map(|(_, pending)| pending)
    }

    pub fn set_manual_pending(&self, pending: bool) -> Result<(), RecoveryError> {
        self.set_manual_pending_with_sync(pending, File::sync_all)
    }

    fn set_manual_pending_with_sync(&self, pending: bool, synchronize: fn(&File) -> io::Result<()>) -> Result<(), RecoveryError> {
        self.verify_owner()?;
        let (mut file, _) = self.marker_file(true)?;
        let path = self.directory.join(MARKER);
        // One state byte needs no extra quota; torn or unknown states refuse automatic startup
        file.seek(SeekFrom::Start((MAGIC.len() - 1) as u64))
            .and_then(|_| file.write_all(if pending { b"!" } else { b"\n" }))
            .map_err(|error| io_error("write manual recovery state", &path, error, true))?;
        if let Err(error) = synchronize(&file) {
            if !pending {
                file.seek(SeekFrom::Start((MAGIC.len() - 1) as u64))
                    .and_then(|_| file.write_all(b"!"))
                    .and_then(|_| file.sync_all())
                    .map_err(|restore| io_error("restore pending manual recovery state", &path, restore, true))?;
            }
            return Err(io_error("synchronize manual recovery state", &path, error, true));
        }
        let after = fs::symlink_metadata(&path)
            .map_err(|error| io_error("reinspect manual recovery state", &path, error, true))?;
        let opened = file.metadata().map_err(|error| io_error("inspect written manual recovery state", &path, error, true))?;
        self.verify_marker_identity(&after, &opened)
    }

    fn inventory(&self) -> Result<Inventory, RecoveryError> {
        let mut inventory = Inventory { snapshots: Vec::new(), remnants: Vec::new(), total: 0, entries: 0 };
        for item in fs::read_dir(&self.directory).map_err(|error| io_error("list recovery directory", &self.directory, error, false))? {
            inventory.entries += 1;
            if inventory.entries > MAX_ENTRIES { return Err(RecoveryError::Ownership("Recovery directory exceeds 128 entries".into())); }
            let item = item.map_err(|error| io_error("read recovery entry", &self.directory, error, false))?;
            let path = item.path();
            let metadata = fs::symlink_metadata(&path).map_err(|error| io_error("inspect recovery entry", &path, error, false))?;
            if !metadata.is_file() && !metadata.is_symlink() {
                return Err(RecoveryError::Ownership(format!("Unknown non-file entry: {}", path.display())));
            }
            inventory.total = inventory.total.checked_add(metadata.len()).ok_or(RecoveryError::Quota)?;
            if metadata.is_file() {
                if let Some(name) = item.file_name().to_str() {
                    if let Some((generation, time, pending)) = parse_snapshot(name) {
                        inventory.snapshots.push(Entry { info: SnapshotInfo { generation, time, path, bytes: metadata.len() }, pending, metadata });
                    } else if writer_temporary(name) { inventory.remnants.push((path, metadata)); }
                }
            }
        }
        if inventory.snapshots.iter().filter(|entry| !entry.pending).count() > 8 {
            return Err(RecoveryError::Ownership("More than eight completed recovery snapshots".into()));
        }
        inventory.snapshots.sort_by_key(|entry| entry.info.generation);
        if inventory.snapshots.windows(2).any(|pair| pair[0].info.generation == pair[1].info.generation) {
            return Err(RecoveryError::Ownership("Duplicate recovery generation".into()));
        }
        Ok(inventory)
    }
    fn remove_owned(&self, path: &Path, expected: &fs::Metadata, committed: bool) -> Result<(), RecoveryError> {
        self.verify_owner()?;
        let current = fs::symlink_metadata(path).map_err(|error| io_error("inspect owned recovery entry", path, error, committed))?;
        if !current.is_file() || !same_file(expected, &current) || current.len() != expected.len() {
            return Err(RecoveryError::Ownership("Owned recovery entry changed before deletion".into()));
        }
        fs::remove_file(path).map_err(|error| io_error("remove owned recovery entry", path, error, committed))?;
        sync_directory(&self.directory, committed)
    }
    fn newest_valid(&mut self, caller: &World, entries: &[Entry]) -> Option<(World, SnapshotInfo)> {
        let limits = super::world_file::world_file_limits();
        for entry in entries.iter().rev() {
            match caller.load_legacy_file(&entry.info.path, limits, limits.allocated_bytes) {
                Ok(world) => match world.render_segments() {
                    Ok(_) => return Some((world, entry.info.clone())),
                    Err(error) => self.reject(&entry.info.path, &error),
                },
                Err(error) => self.reject(&entry.info.path, &error),
            }
        }
        None
    }

    /// Return a drawable new root; publication, pause, and filename adoption belong to the caller
    pub fn recover_latest(&mut self, caller: &World) -> Result<Option<World>, RecoveryError> {
        self.verify_owner()?;
        self.rejections.clear();
        self.recovered = None;
        let inventory = self.inventory()?;
        if inventory.total > self.policy.maximum_bytes { return Err(RecoveryError::Quota); }
        match self.newest_valid(caller, &inventory.snapshots) {
            Some((world, info)) => { self.recovered = Some(info); Ok(Some(world)) }
            None if inventory.snapshots.is_empty() => Ok(None),
            None => Err(RecoveryError::NoValidSnapshot),
        }
    }

    /// Encode once, admit exact file lengths, then preserve the last valid point through durable staging
    pub fn checkpoint(&mut self, world: &World, protected_path: Option<&Path>) -> Result<SnapshotInfo, RecoveryError> {
        let prepared = self.prepare_checkpoint(world, protected_path)?;
        self.finish_checkpoint(world, prepared)
    }

    pub(super) fn prepare_checkpoint(&mut self, world: &World, protected_path: Option<&Path>) -> Result<PreparedCheckpoint, RecoveryError> {
        self.verify_owner()?;
        self.recovered = None;
        let protection = protected_path.map(Protection::new).transpose()?;
        let limits = super::world_file::world_file_limits();
        let bytes = world.encode_legacy_file_bytes(limits, limits.allocated_bytes)
            .map_err(|source| RecoveryError::World { source, committed: false })?;
        Ok(PreparedCheckpoint { protection, bytes, time: world.time() })
    }

    pub(super) fn finish_checkpoint(&mut self, world: &World, prepared: PreparedCheckpoint) -> Result<SnapshotInfo, RecoveryError> {
        self.verify_owner()?;
        let PreparedCheckpoint { protection, bytes, time } = prepared;
        let protected = |path: &Path, metadata: &fs::Metadata| protection.as_ref().is_some_and(|owner| owner.contains(path, metadata));
        let limits = super::world_file::world_file_limits();
        let mut inventory = self.inventory()?;
        // No PID probing: the marker and held lock own this exact temporary namespace
        for (path, metadata) in &inventory.remnants {
            if !protected(path, metadata) { self.remove_owned(path, metadata, false)?; }
        }
        inventory = self.inventory()?;
        let last_good = self.newest_valid(world, &inventory.snapshots).map(|(_, info)| info.path);
        if let Some(path) = &last_good {
            File::open(path).and_then(|file| file.sync_all())
                .map_err(|error| io_error("synchronize previous recovery point", path, error, false))?;
            sync_directory(&self.directory, false)?;
        }
        let generation = inventory.snapshots.last().map_or(Some(1), |entry| entry.info.generation.checked_add(1))
            .ok_or_else(|| RecoveryError::Ownership("Recovery generation exhausted".into()))?;
        let pending = self.directory.join(snapshot_name(generation, time, true));
        let destination = self.directory.join(snapshot_name(generation, time, false));
        if pending.exists() || pending.is_symlink() || destination.exists() || destination.is_symlink()
            || protection.as_ref().is_some_and(|owner| owner.path == pending || owner.path == destination) {
            return Err(RecoveryError::Ownership("Recovery destination is occupied or protected".into()));
        }
        if inventory.entries + 2 > MAX_ENTRIES { return Err(RecoveryError::Quota); }
        let length = bytes.len() as u64;
        let mut remaining = inventory.total.checked_add(length).ok_or(RecoveryError::Quota)?;
        let mut before = Vec::new();
        for (index, entry) in inventory.snapshots.iter().enumerate() {
            if remaining <= self.policy.maximum_bytes { break; }
            if last_good.as_ref() != Some(&entry.info.path) && !protected(&entry.info.path, &entry.metadata) {
                before.push(index);
                remaining -= entry.info.bytes;
            }
        }
        if remaining > self.policy.maximum_bytes { return Err(RecoveryError::Quota); }
        let mut after = Vec::new();
        let mut completed = inventory.snapshots.iter().enumerate()
            .filter(|(index, entry)| !entry.pending && !before.contains(index)).count() + 1;
        for (index, entry) in inventory.snapshots.iter().enumerate() {
            if before.contains(&index) || protected(&entry.info.path, &entry.metadata) { continue; }
            if entry.pending || completed > self.policy.maximum_snapshots {
                after.push(index);
                if !entry.pending { completed -= 1; }
            }
        }
        if completed > self.policy.maximum_snapshots { return Err(RecoveryError::Quota); }
        for index in before {
            let entry = &inventory.snapshots[index];
            self.remove_owned(&entry.info.path, &entry.metadata, false)?;
        }
        self.verify_owner()?;
        biogenesis_core::replace_file_bytes(&pending, &bytes).map_err(RecoveryError::Replace)?;
        let staged = match world.load_legacy_file(&pending, limits, limits.allocated_bytes) {
            Ok(staged) => staged,
            Err(source) => {
                self.reject(&pending, &source);
                return Err(RecoveryError::World { source, committed: true });
            }
        };
        if let Err(source) = staged.render_segments() {
            self.reject(&pending, &source);
            return Err(RecoveryError::Drawing { source, committed: true });
        }
        drop(staged);
        // Pending is now a valid, durable recovery point before the previous point can be removed
        for index in after {
            let entry = &inventory.snapshots[index];
            self.remove_owned(&entry.info.path, &entry.metadata, true)
                .map_err(|error| RecoveryError::Finalization(Box::new(error)))?;
        }
        fs::rename(&pending, &destination).map_err(|error| io_error("complete recovery snapshot", &pending, error, true))?;
        sync_directory(&self.directory, true)?;
        Ok(SnapshotInfo { generation, time, path: destination, bytes: length })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use biogenesis_core::{WorldConfig, object_stream::Limits};
    use std::sync::atomic::{AtomicU64, Ordering};

    const WORLD_BYTES: &[u8] = include_bytes!("../../core/tests/fixtures/object_stream_codec/world.ser");
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn changed_owner_after_capture_rejects_background_write() {
        let directory = Directory::new();
        let world = directory.world();
        let mut store = directory.store(RecoveryPolicy::default());
        let prepared = store.prepare_checkpoint(&world, None).unwrap();
        std::fs::write(store.directory.join(MARKER), b"invalid marker").unwrap();
        assert!(matches!(store.finish_checkpoint(&world, prepared), Err(RecoveryError::Ownership(_))));
        assert!(std::fs::read_dir(&store.directory).unwrap().all(|entry| {
            !entry.unwrap().file_name().to_string_lossy().contains(".bgw")
        }));
    }

    #[test]
    fn detached_validation_accepts_live_session_values_without_reinitializing() {
        let directory = Directory::new();
        let world = directory.world();
        let mut parameters = world.parameters();
        parameters.initial_energy = f64::NAN;
        let random = world.random_snapshot();
        let context = World::detached_validation_context(
            WorldConfig { seed: world.seed(), width: -1, height: 0, initial_attempts: world.initial_attempts() },
            parameters, random.clone(),
        );
        assert!(context.parameters().initial_energy.is_nan());
        assert_eq!(context.random_snapshot(), random);
        let mut store = directory.store(RecoveryPolicy::default());
        let prepared = store.prepare_checkpoint(&world, None).unwrap();
        let result = store.finish_checkpoint(&context, prepared).unwrap();
        assert!(result.path.exists());
        assert_eq!(context.random_snapshot(), random);
    }
    struct Directory(PathBuf);
    impl Directory {
        fn new() -> Self {
            for _ in 0..128 {
                let path = std::env::temp_dir().join(format!(
                    "bgr-recovery-{}-{}", std::process::id(), SEQUENCE.fetch_add(1, Ordering::Relaxed),
                ));
                match fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                    Err(error) => panic!("Cannot create test directory: {error}"),
                }
            }
            panic!("Test directory names occupied");
        }
        fn store(&self, policy: RecoveryPolicy) -> RecoveryStore {
            RecoveryStore::open(&self.0.join("recovery"), policy).unwrap()
        }
        fn world(&self) -> World {
            let path = self.0.join("source.bgw");
            fs::write(&path, WORLD_BYTES).unwrap();
            let caller = World::new(WorldConfig { initial_attempts: 0, ..WorldConfig::default() }).unwrap();
            let limits = Limits::default();
            caller.load_legacy_file(&path, limits, limits.allocated_bytes).unwrap()
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) { fs::remove_dir_all(&self.0).expect("Remove owned test directory"); }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn initial_marker_publication_survives_process_death_and_preserves_impostors() {
        use std::{io::Write, os::unix::fs::{MetadataExt, OpenOptionsExt}, process::Command};
        const CHILD_DIRECTORY: &str = "BGR_TEST_MARKER_DIRECTORY";
        const CHILD_PHASE: &str = "BGR_TEST_MARKER_PHASE";
        if let Some(directory) = std::env::var_os(CHILD_DIRECTORY) {
            let directory = PathBuf::from(directory);
            let phase = std::env::var(CHILD_PHASE).unwrap();
            if phase == "before" {
                let lock = OpenOptions::new().read(true).write(true).create_new(true).mode(0o600)
                    .open(directory.join(LOCK)).unwrap();
                lock.try_lock().unwrap();
                let file = prepare_marker(&directory).unwrap();
                assert_eq!(file.metadata().unwrap().nlink(), 0);
                assert_eq!(fs::read_dir(&directory).unwrap().count(), 1);
                println!("MARKER_CRASH_READY before");
                io::stdout().flush().unwrap();
                std::process::exit(73);
            }
            assert_eq!(phase, "after");
            let _store = RecoveryStore::open(&directory, RecoveryPolicy::default()).unwrap();
            assert_eq!(fs::read(directory.join(MARKER)).unwrap(), MAGIC);
            println!("MARKER_CRASH_READY after");
            io::stdout().flush().unwrap();
            std::process::exit(73);
        }
        let test_name = std::thread::current().name().unwrap().to_owned();
        for phase in ["before", "after"] {
            let fixture = Directory::new();
            let directory = fixture.0.join("recovery");
            fs::create_dir(&directory).unwrap();
            let output = Command::new(std::env::current_exe().unwrap())
                .args(["--exact", &test_name, "--nocapture"])
                .env(CHILD_DIRECTORY, &directory).env(CHILD_PHASE, phase).output().unwrap();
            assert_eq!(output.status.code(), Some(73), "{output:?}");
            assert!(String::from_utf8(output.stdout).unwrap().contains(&format!("MARKER_CRASH_READY {phase}")));
            assert_eq!(directory.join(MARKER).exists(), phase == "after");
            assert_eq!(fs::read_dir(&directory).unwrap().count(), if phase == "before" { 1 } else { 2 });
            let store = RecoveryStore::open(&directory, RecoveryPolicy::default()).unwrap();
            store.verify_owner().unwrap();
            let marker = directory.join(MARKER);
            let before = fs::metadata(&marker).unwrap();
            assert!(initialize_marker(&directory).is_err());
            assert!(same_file(&before, &fs::metadata(&marker).unwrap()));
            assert_eq!(fs::read(&marker).unwrap(), MAGIC);
            assert_eq!(fs::read_dir(&directory).unwrap().count(), 2);
        }
        let fixture = Directory::new();
        let directory = fixture.0.join("recovery");
        fs::create_dir(&directory).unwrap();
        let impostor = directory.join(".biogenesis-r-1234-8.tmp");
        fs::write(&impostor, MAGIC).unwrap();
        let before = fs::metadata(&impostor).unwrap();
        assert!(matches!(RecoveryStore::open(&directory, RecoveryPolicy::default()), Err(RecoveryError::Ownership(_))));
        assert!(!directory.join(MARKER).exists());
        assert!(same_file(&before, &fs::metadata(&impostor).unwrap()));
        assert_eq!(fs::read(&impostor).unwrap(), MAGIC);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn published_marker_survives_directory_sync_failure() {
        let fixture = Directory::new();
        let directory = fixture.0.join("recovery");
        fs::create_dir(&directory).unwrap();
        let error = initialize_marker_with_sync(&directory, |directory, committed| {
            assert_eq!(fs::read(directory.join(MARKER)).unwrap(), MAGIC);
            Err(io_error("synchronize recovery directory", directory, io::Error::other("injected directory sync failure"), committed))
        }).unwrap_err();
        assert!(error.replacement_committed());
        assert!(error.to_string().contains("a recovery file was committed"));
        assert_eq!(fs::read(directory.join(MARKER)).unwrap(), MAGIC);
        let store = RecoveryStore::open(&directory, RecoveryPolicy::default()).unwrap();
        store.verify_owner().unwrap();
        assert_eq!(fs::read_dir(&directory).unwrap().count(), 2);
    }

    #[test]
    fn manual_marker_quota_failure_preserves_primary_bytes_and_pending_on_reopen() {
        let fixture = Directory::new();
        let world = fixture.world();
        let mut store = fixture.store(RecoveryPolicy { maximum_bytes: MAGIC.len() as u64, maximum_snapshots: 8 });
        let before = store.inventory().unwrap();
        store.set_manual_pending(true).unwrap();
        assert!(store.manual_pending().unwrap());
        assert_eq!(store.inventory().unwrap().total, before.total);
        assert_eq!(store.inventory().unwrap().entries, before.entries);
        let path = fixture.0.join("manual.bgw");
        let limits = super::super::world_file::world_file_limits();
        world.save_legacy_file(&path, limits, limits.allocated_bytes).unwrap();
        assert!(matches!(store.checkpoint(&world, Some(&path)), Err(RecoveryError::Quota)));
        assert_eq!(fs::read(&path).unwrap(), WORLD_BYTES);
        drop(store);
        let store = fixture.store(RecoveryPolicy::default());
        assert!(store.manual_pending().unwrap());
        assert_eq!(fs::metadata(store.directory.join(MARKER)).unwrap().len(), 33);
    }

    #[test]
    fn manual_marker_clears_only_after_exact_durable_checkpoint_without_model_changes() {
        let fixture = Directory::new();
        let world = fixture.world();
        let before = world.clone();
        let mut store = fixture.store(RecoveryPolicy::default());
        store.set_manual_pending(true).unwrap();
        let first = store.checkpoint(&world, None).unwrap();
        assert!(store.manual_pending().unwrap());
        assert_eq!(fs::read(&first.path).unwrap(), WORLD_BYTES);
        store.set_manual_pending(false).unwrap();
        assert!(!store.manual_pending().unwrap());
        assert_eq!(fs::read(store.directory.join(MARKER)).unwrap(), MAGIC);
        store.set_manual_pending(true).unwrap();
        let second = store.checkpoint(&world, None).unwrap();
        assert!(second.generation > first.generation);
        assert_eq!(second.time, first.time);
        store.set_manual_pending(false).unwrap();
        assert_eq!(world, before);
        assert_eq!(fs::read(&second.path).unwrap(), WORLD_BYTES);
    }

    #[test]
    fn manual_marker_sync_errors_keep_pending_and_reject_corrupt_or_linked_metadata() {
        let fixture = Directory::new();
        let store = fixture.store(RecoveryPolicy::default());
        fn fail(_: &File) -> io::Result<()> { Err(io::Error::other("injected marker synchronization failure")) }
        assert!(store.set_manual_pending_with_sync(true, fail).is_err());
        assert!(store.manual_pending().unwrap());
        assert!(store.set_manual_pending_with_sync(false, fail).is_err());
        assert!(store.manual_pending().unwrap());
        let path = store.directory.join(MARKER);
        fs::hard_link(&path, fixture.0.join("alias")).unwrap();
        assert!(store.manual_pending().is_err());
        assert!(store.set_manual_pending(false).is_err());
        fs::remove_file(fixture.0.join("alias")).unwrap();
        let mut corrupt = MAGIC.to_vec();
        corrupt[0] = b'?';
        fs::write(&path, &corrupt).unwrap();
        assert!(store.manual_pending().is_err());
        assert!(store.set_manual_pending(false).is_err());
        assert_eq!(fs::read(&path).unwrap(), corrupt);
    }

    #[test]
    fn manual_marker_survives_process_exit_before_manual_or_after_checkpoint() {
        use std::process::Command;
        const DIRECTORY: &str = "BGR_MANUAL_CRASH_DIRECTORY";
        const PHASE: &str = "BGR_MANUAL_CRASH_PHASE";
        if let Some(directory) = std::env::var_os(DIRECTORY) {
            let store = RecoveryStore::open(Path::new(&directory), RecoveryPolicy::default()).unwrap();
            store.set_manual_pending(true).unwrap();
            if std::env::var(PHASE).unwrap() == "checkpoint" {
                let fixture = Directory(PathBuf::from(directory).parent().unwrap().to_owned());
                let world = fixture.world();
                let mut store = store;
                store.checkpoint(&world, None).unwrap();
                std::mem::forget(fixture);
            }
            std::process::exit(73);
        }
        let name = std::thread::current().name().unwrap().to_owned();
        for phase in ["pending", "checkpoint"] {
            let fixture = Directory::new();
            let directory = fixture.0.join("recovery");
            let output = Command::new(std::env::current_exe().unwrap())
                .args(["--exact", &name, "--nocapture"])
                .env(DIRECTORY, &directory).env(PHASE, phase).output().unwrap();
            assert_eq!(output.status.code(), Some(73), "{output:?}");
            let store = fixture.store(RecoveryPolicy::default());
            assert!(store.manual_pending().unwrap());
            assert_eq!(store.inventory().unwrap().snapshots.len(), if phase == "pending" { 0 } else { 1 });
        }
    }

    #[test]
    fn checkpoint_and_recovery_use_ordinary_bytes_without_publishing_or_advancing_rng() {
        let directory = Directory::new();
        let world = directory.world();
        let before = world.clone();
        let random = world.random_snapshot();
        let statistics = world.open_statistics();
        let owner = world.handle();
        let mut store = directory.store(RecoveryPolicy::default());
        let info = store.checkpoint(&world, None).unwrap();
        assert_eq!(info.generation, 1);
        assert_eq!(info.time, world.time());
        assert_eq!(fs::read(&info.path).unwrap(), WORLD_BYTES);
        assert_eq!(info.bytes, WORLD_BYTES.len() as u64);
        let recovered = store.recover_latest(&world).unwrap().unwrap();
        assert_eq!(store.last_recovered(), Some(&info));
        assert!(recovered.session().same_owner(&world.session()));
        assert!(!recovered.handle().same_owner(&owner));
        assert_eq!(world, before);
        assert_eq!(world.random_snapshot(), random);
        assert_eq!(world.open_statistics(), statistics);
    }

    #[test]
    fn quota_counts_old_point_staging_metadata_and_unknown_lengths_before_deletion() {
        let directory = Directory::new();
        let world = directory.world();
        let policy = RecoveryPolicy {
            maximum_bytes: MAGIC.len() as u64 + 2 * WORLD_BYTES.len() as u64 - 1,
            maximum_snapshots: 8,
        };
        let mut store = directory.store(policy);
        let first = store.checkpoint(&world, None).unwrap();
        assert!(matches!(store.checkpoint(&world, None), Err(RecoveryError::Quota)));
        assert_eq!(fs::read(&first.path).unwrap(), WORLD_BYTES);
        let unrelated = store.directory.join("notes.txt");
        File::create(&unrelated).unwrap().set_len(policy.maximum_bytes + 1).unwrap();
        assert!(matches!(store.checkpoint(&world, None), Err(RecoveryError::Quota)));
        assert_eq!(fs::metadata(&unrelated).unwrap().len(), policy.maximum_bytes + 1);
        assert_eq!(fs::read(&first.path).unwrap(), WORLD_BYTES);
    }

    #[test]
    fn single_slot_retention_waits_for_durable_staging_and_survives_reopen() {
        let directory = Directory::new();
        let world = directory.world();
        let policy = RecoveryPolicy {
            maximum_bytes: MAGIC.len() as u64 + 2 * WORLD_BYTES.len() as u64,
            maximum_snapshots: 1,
        };
        let mut store = directory.store(policy);
        let first = store.checkpoint(&world, None).unwrap();
        let second = store.checkpoint(&world, None).unwrap();
        assert_eq!(second.generation, first.generation + 1);
        assert!(!first.path.exists());
        assert_eq!(fs::read(&second.path).unwrap(), WORLD_BYTES);
        assert_eq!(store.inventory().unwrap().snapshots.len(), 1);
        drop(store);
        let mut reopened = directory.store(policy);
        assert!(reopened.recover_latest(&world).unwrap().is_some());
        assert_eq!(reopened.last_recovered(), Some(&second));
    }

    #[test]
    fn recovery_falls_back_from_corrupt_newest_and_retains_rejection_details() {
        let directory = Directory::new();
        let world = directory.world();
        let before = world.clone();
        let random = world.random_snapshot();
        let mut store = directory.store(RecoveryPolicy::default());
        let first = store.checkpoint(&world, None).unwrap();
        let second = store.checkpoint(&world, None).unwrap();
        fs::write(&second.path, b"interrupted or corrupt input").unwrap();
        assert!(store.recover_latest(&world).unwrap().is_some());
        assert_eq!(store.last_recovered(), Some(&first));
        assert_eq!(store.rejections().len(), 1);
        assert_eq!(store.rejections()[0].path, second.path);
        assert!(!store.rejections()[0].details.is_empty());
        assert_eq!(fs::read(&second.path).unwrap(), b"interrupted or corrupt input");
        assert_eq!(world, before);
        assert_eq!(world.random_snapshot(), random);
        fs::write(&second.path, WORLD_BYTES).unwrap();
        assert!(store.recover_latest(&world).unwrap().is_some());
        assert_eq!(store.last_recovered(), Some(&second));
        assert!(store.rejections().is_empty());
        fs::write(&first.path, b"bad").unwrap();
        fs::write(&second.path, b"bad").unwrap();
        assert!(matches!(store.recover_latest(&world), Err(RecoveryError::NoValidSnapshot)));
        assert!(store.last_recovered().is_none());
        assert_eq!(store.rejections().len(), 2);
    }

    #[test]
    fn interrupted_pending_point_is_recoverable_and_cleaned_only_after_a_new_point() {
        let directory = Directory::new();
        let world = directory.world();
        let mut store = directory.store(RecoveryPolicy::default());
        let first = store.checkpoint(&world, None).unwrap();
        let pending = store.directory.join(snapshot_name(2, world.time(), true));
        biogenesis_core::replace_file_bytes(&pending, WORLD_BYTES).unwrap();
        assert!(store.recover_latest(&world).unwrap().is_some());
        assert_eq!(store.last_recovered().unwrap().path, pending);
        assert!(first.path.exists());
        let third = store.checkpoint(&world, None).unwrap();
        assert_eq!(third.generation, 3);
        assert!(!pending.exists());
        assert!(first.path.exists());
        assert!(store.last_recovered().is_none());
        assert_eq!(fs::read(third.path).unwrap(), WORLD_BYTES);
    }

    #[test]
    fn protected_points_block_retention_and_reserved_remnants_are_reconciled_under_lock() {
        let directory = Directory::new();
        let world = directory.world();
        let mut store = directory.store(RecoveryPolicy { maximum_snapshots: 1, ..RecoveryPolicy::default() });
        let first = store.checkpoint(&world, None).unwrap();
        assert!(matches!(store.checkpoint(&world, Some(&first.path)), Err(RecoveryError::Quota)));
        assert_eq!(fs::read(&first.path).unwrap(), WORLD_BYTES);
        let remnant = store.directory.join(".biogenesis-r-1234-8.tmp");
        let unknown = store.directory.join(".biogenesis-r-unrelated.tmp");
        fs::write(&remnant, b"partial").unwrap();
        fs::write(&unknown, b"personal data").unwrap();
        store.checkpoint(&world, Some(&remnant)).unwrap();
        assert_eq!(fs::read(&remnant).unwrap(), b"partial");
        store.checkpoint(&world, None).unwrap();
        assert!(!remnant.exists());
        assert_eq!(fs::read(unknown).unwrap(), b"personal data");
    }

    #[test]
    fn ownership_is_nonblocking_and_invalid_names_directories_and_markers_are_preserved() {
        let directory = Directory::new();
        let world = directory.world();
        let mut store = directory.store(RecoveryPolicy::default());
        #[cfg(target_os = "linux")]
        {
            use std::os::fd::AsRawFd;
            let flags = unsafe { libc::fcntl(store.lock.as_raw_fd(), libc::F_GETFL) };
            assert!(flags >= 0);
            assert_eq!(flags & (libc::O_NOFOLLOW | libc::O_NONBLOCK), libc::O_NOFOLLOW | libc::O_NONBLOCK);
        }
        assert!(matches!(RecoveryStore::open(&store.directory, store.policy), Err(RecoveryError::Ownership(_))));
        let unknown = store.directory.join("recovery-not-owned.bgw");
        fs::write(&unknown, b"personal data").unwrap();
        let subdirectory = store.directory.join("unrelated");
        fs::create_dir(&subdirectory).unwrap();
        assert!(matches!(store.checkpoint(&world, None), Err(RecoveryError::Ownership(_))));
        assert_eq!(fs::read(&unknown).unwrap(), b"personal data");
        assert!(subdirectory.is_dir());
        fs::remove_dir(&subdirectory).unwrap();
        fs::write(store.directory.join(MARKER), b"different owner").unwrap();
        assert!(matches!(store.recover_latest(&world), Err(RecoveryError::Ownership(_))));
        assert_eq!(fs::read(store.directory.join(MARKER)).unwrap(), b"different owner");
    }

    #[test]
    fn inventory_bound_fails_closed_and_missing_parent_does_not_create_a_tree() {
        let directory = Directory::new();
        assert!(RecoveryStore::open(&directory.0.join("missing/recovery"), RecoveryPolicy::default()).is_err());
        assert!(!directory.0.join("missing").exists());
        let world = directory.world();
        let mut store = directory.store(RecoveryPolicy::default());
        for index in 0..MAX_ENTRIES { fs::write(store.directory.join(format!("unknown-{index}")), b"").unwrap(); }
        assert!(matches!(store.checkpoint(&world, None), Err(RecoveryError::Ownership(_))));
        assert!(store.directory.join("unknown-0").is_file());
    }

    #[test]
    fn decoded_but_undrawable_newest_falls_back_and_cannot_displace_last_usable_point() {
        use biogenesis_core::object_stream::{StreamReader, StreamWriter, Node, FieldValue, Value, class_schema};
        let directory = Directory::new();
        let world = directory.world();
        let before = world.clone();
        let random = world.random_snapshot();
        let mut store = directory.store(RecoveryPolicy::default());
        let first = store.checkpoint(&world, None).unwrap();
        let limits = Limits::default();
        let mut document = StreamReader::new(
            include_bytes!("../../core/tests/fixtures/world_hydration/actor_closure-call_0.ser"), limits,
        ).unwrap().read_document().unwrap();
        let mut targets = Vec::new();
        for (index, node) in document.arena.nodes.iter().enumerate() {
            let Node::Object(object) = node else { continue; };
            for (slot_index, slot) in object.slots.iter().enumerate() {
                let Node::Descriptor(descriptor) = &document.arena.nodes[slot.descriptor.0 as usize] else { panic!("Descriptor"); };
                let schema = class_schema(descriptor.class);
                if schema.name == "biogenesis.Organism" {
                    let position = schema.fields.iter().position(|field| field.name == "x1").unwrap();
                    targets.push((index, slot_index, position));
                }
            }
        }
        assert!(!targets.is_empty());
        for (index, slot, position) in targets {
            let Node::Object(object) = &mut document.arena.nodes[index] else { unreachable!() };
            object.slots[slot].values[position] = FieldValue::Reference(Value::Null);
        }
        let mut writer = StreamWriter::new(&document.arena, limits).unwrap();
        for record in &document.records { writer.write_record(record).unwrap(); }
        let bytes = writer.finish().unwrap();
        let bad = store.directory.join(snapshot_name(2, world.time(), false));
        biogenesis_core::replace_file_bytes(&bad, &bytes).unwrap();
        let decoded = world.load_legacy_file(&bad, limits, limits.allocated_bytes).unwrap();
        assert!(decoded.render_segments().is_err());
        assert!(store.recover_latest(&world).unwrap().is_some());
        assert_eq!(store.last_recovered(), Some(&first));
        assert_eq!(store.rejections().last().unwrap().path, bad);
        // Admission can discard corrupt generations, but must retain the only drawable point
        let encoded = decoded.encode_legacy_file_bytes(limits, limits.allocated_bytes).unwrap();
        store.policy.maximum_bytes = store.inventory().unwrap().total + encoded.len() as u64 - first.bytes;
        let error = store.checkpoint(&decoded, None).unwrap_err();
        assert!(matches!(error, RecoveryError::Drawing { .. }));
        assert!(error.replacement_committed());
        assert_eq!(fs::read(&first.path).unwrap(), WORLD_BYTES);
        assert_eq!(world, before);
        assert_eq!(world.random_snapshot(), random);
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_and_protected_hardlink_aliases_never_become_cleanup_targets() {
        use std::os::unix::fs::symlink;
        let directory = Directory::new();
        let world = directory.world();
        let mut store = directory.store(RecoveryPolicy { maximum_snapshots: 1, ..RecoveryPolicy::default() });
        let first = store.checkpoint(&world, None).unwrap();
        let alias = directory.0.join("active-world.bgw");
        fs::hard_link(&first.path, &alias).unwrap();
        assert!(matches!(store.checkpoint(&world, Some(&alias)), Err(RecoveryError::Quota)));
        assert_eq!(fs::read(&alias).unwrap(), WORLD_BYTES);
        let linked = store.directory.join(".biogenesis-r-1234-9.tmp");
        symlink(&alias, &linked).unwrap();
        store.checkpoint(&world, None).unwrap();
        assert!(linked.is_symlink());
        assert_eq!(fs::read(&alias).unwrap(), WORLD_BYTES);
        let linked_directory = directory.0.join("linked-directory");
        symlink(&store.directory, &linked_directory).unwrap();
        assert!(matches!(RecoveryStore::open(&linked_directory, store.policy), Err(RecoveryError::Ownership(_))));
    }

    #[test]
    #[ignore = "Run guarded dense managed recovery after the reviewed World-file profile is wired"]
    fn required_dense_post_genesis_managed_recovery_roundtrip() {
        fn same_bytes(left: &Path, right: &Path) -> bool {
            let mut remaining = fs::metadata(left).unwrap().len();
            if fs::metadata(right).unwrap().len() != remaining { return false; }
            let (mut left, mut right) = (File::open(left).unwrap(), File::open(right).unwrap());
            let (mut a, mut b) = ([0_u8; 8192], [0_u8; 8192]);
            while remaining != 0 {
                let count = remaining.min(a.len() as u64) as usize;
                left.read_exact(&mut a[..count]).unwrap();
                right.read_exact(&mut b[..count]).unwrap();
                if a[..count] != b[..count] { return false; }
                remaining -= count as u64;
            }
            true
        }
        fn peak_rss(stage: &str) {
            let vm_hwm_kib = fs::read_to_string("/proc/self/status").ok().and_then(|status| {
                let line = status.lines().find(|line| line.starts_with("VmHWM:"))?;
                let mut fields = line.split_whitespace();
                if fields.next() != Some("VmHWM:") { return None; }
                let value = fields.next()?.parse::<u64>().ok()?;
                (fields.next() == Some("kB") && fields.next().is_none()).then_some(value)
            });
            match vm_hwm_kib {
                Some(value) => println!("DENSE_MANAGED_RECOVERY_RSS stage={stage} status=available vm_hwm_kib={value} scope=test_process_peak"),
                None => println!("DENSE_MANAGED_RECOVERY_RSS stage={stage} status=unavailable scope=test_process_peak"),
            }
        }
        let state = |world: &World| (
            world.dimensions(), world.population(), world.frame(), world.time(), world.tick(), world.paused(),
            world.oxygen().to_bits(), world.carbon_dioxide().to_bits(), world.methane().to_bits(),
        );
        let path = PathBuf::from(std::env::var_os("BGR_DENSE_GENESIS_INPUT")
            .expect("Pinned post-genesis Source World fixture path"));
        assert_eq!(fs::metadata(&path).unwrap().len(), 69_278_502);
        let directory = Directory::new();
        let caller = World::new(WorldConfig { initial_attempts: 0, ..WorldConfig::default() }).unwrap();
        let caller_state = state(&caller);
        let caller_owner = caller.handle();
        let caller_statistics = caller.open_statistics();
        let session = caller.session();
        let random = caller.random_snapshot();
        let parameters = caller.parameters();
        let limits = super::super::world_file::world_file_limits();
        println!("DENSE_MANAGED_RECOVERY_STAGE stage=load status=begin input_bytes=69278502 allocation_limit={} work_limit={}", limits.allocated_bytes, limits.work);
        let world = caller.load_legacy_file(&path, limits, limits.allocated_bytes)
            .expect("Dense Source World must load through the configured public policy");
        assert!(!world.handle().same_owner(&caller_owner));
        assert!(world.session().same_owner(&session));
        assert_eq!(world.dimensions(), (7000, 7000));
        assert_eq!(world.population(), 12500);
        assert_eq!(world.frame(), 0);
        world.resume();
        let before = state(&world);
        let owner = world.handle();
        let statistics = world.open_statistics();
        let unchanged = || {
            assert!(caller.handle().same_owner(&caller_owner));
            assert!(caller.session().same_owner(&session));
            assert!(caller.open_statistics() == caller_statistics);
            assert_eq!(state(&caller), caller_state);
            assert!(world.handle().same_owner(&owner));
            assert!(world.session().same_owner(&session));
            assert!(world.open_statistics() == statistics);
            assert_eq!(state(&world), before);
            assert_eq!(caller.random_snapshot(), random);
            assert_eq!(world.random_snapshot(), random);
            assert!(caller.parameters() == parameters && world.parameters() == parameters);
        };
        unchanged();
        peak_rss("loaded");
        let policy = RecoveryPolicy::default();
        assert_eq!(policy.maximum_bytes, 1_u64 << 30);
        assert_eq!(policy.maximum_snapshots, 8);
        let mut store = directory.store(policy);
        println!("DENSE_MANAGED_RECOVERY_STAGE stage=checkpoint-first status=begin");
        let first = store.checkpoint(&world, Some(&path)).expect("First dense managed checkpoint");
        unchanged();
        peak_rss("checkpoint-first");
        assert_eq!(first.generation, 1);
        assert_eq!(first.time, world.time());
        assert_eq!(first.bytes, 69_278_502);
        assert!(same_bytes(&path, &first.path), "First checkpoint changed Source bytes");
        println!("DENSE_MANAGED_RECOVERY_STAGE stage=checkpoint-second status=begin");
        let second = store.checkpoint(&world, Some(&path)).expect("Second dense managed checkpoint");
        unchanged();
        peak_rss("checkpoint-second");
        assert_eq!(second.generation, 2);
        assert_eq!(second.time, world.time());
        assert_eq!(second.bytes, 69_278_502);
        assert_ne!(first.path, second.path);
        assert!(first.path.is_file());
        assert!(same_bytes(&path, &second.path), "Second checkpoint changed Source bytes");
        println!("DENSE_MANAGED_RECOVERY_STAGE stage=recover-latest status=begin");
        let recovered = store.recover_latest(&world).expect("Recover dense managed checkpoint")
            .expect("A completed dense checkpoint must be available");
        unchanged();
        peak_rss("managed-path-complete");
        assert_eq!(store.last_recovered(), Some(&second));
        assert!(store.rejections().is_empty());
        assert!(!recovered.handle().same_owner(&owner));
        assert!(recovered.session().same_owner(&session));
        assert_eq!(recovered.random_snapshot(), random);
        assert!(recovered.parameters() == parameters);
        assert_eq!(state(&recovered), before);
        let inventory = store.inventory().unwrap();
        assert_eq!(inventory.snapshots.len(), 2);
        assert!(inventory.snapshots.iter().all(|entry| !entry.pending));
        assert!(inventory.remnants.is_empty());
        assert!(inventory.total <= policy.maximum_bytes);
        assert_eq!(store.policy.maximum_bytes, policy.maximum_bytes);
        assert_eq!(store.policy.maximum_snapshots, policy.maximum_snapshots);
        println!("DENSE_MANAGED_RECOVERY_STAGE stage=compatibility-readback status=begin");
        let readback = directory.0.join("recovered-readback.bgw");
        recovered.save_legacy_file(&readback, limits, limits.allocated_bytes)
            .expect("Recovered dense World must save through the same public policy");
        assert!(same_bytes(&path, &readback), "Recovered World changed Source bytes");
        unchanged();
        peak_rss("compatibility-readback-complete");
        println!("DENSE_MANAGED_RECOVERY_PASS bytes=69278502 snapshots=2 caller_rng_unchanged=true profile_acceptance=false");
    }
}
