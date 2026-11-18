// SPDX-License-Identifier: GPL-2.0-or-later

//! Bounded genotype reads and same-directory replacement after complete XML preflight

use crate::{
    Genome, XmlError,
    atomic_file::{FileReplaceError, ReplacePhase},
    encode_genome_xml, parse_genome_xml, replace_file_bytes,
    xml::MAX_DOCUMENT_BYTES,
};
use std::{
    fmt,
    fs::{self, OpenOptions},
    io::{self, Read},
    path::{Path, PathBuf},
};

#[cfg(test)]
use std::fs::File;
#[cfg(test)]
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(test)]
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// File failures retain their operation and distinguish a committed replacement from an uncommitted attempt
#[derive(Debug)]
pub enum GenomeFileError {
    Xml(XmlError),
    Io {
        operation: &'static str,
        path: PathBuf,
        source: io::Error,
        replacement_committed: bool,
        cleanup_error: Option<io::Error>,
    },
}

impl fmt::Display for GenomeFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Xml(error) => error.fmt(f),
            Self::Io {
                operation,
                path,
                source,
                replacement_committed,
                cleanup_error,
            } => {
                write!(f, "Cannot {operation} {}: {source}", path.display())?;
                if *replacement_committed {
                    f.write_str("; replacement completed, but disk durability is unconfirmed")?;
                }
                if let Some(error) = cleanup_error {
                    write!(f, "; cannot remove the temporary file: {error}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for GenomeFileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            Self::Xml(error) => error,
            Self::Io { source, .. } => source,
        })
    }
}

impl From<FileReplaceError> for GenomeFileError {
    fn from(error: FileReplaceError) -> Self {
        let FileReplaceError {
            phase,
            path,
            mut source,
            replacement_committed,
            cleanup_error,
            ..
        } = error;
        let operation = match phase {
            ReplacePhase::Replace => "replace genotype",
            ReplacePhase::InspectDestination => "inspect genotype destination",
            ReplacePhase::Save => "save genotype",
            ReplacePhase::CreateTemporary => "create temporary genotype",
            ReplacePhase::TemporaryNamesOccupied => {
                source = io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "Temporary genotype names are occupied",
                );
                "create temporary genotype"
            }
            ReplacePhase::WriteAndReplace => "write and replace genotype",
            ReplacePhase::SynchronizeDirectory => "synchronize genotype directory",
        };
        Self::Io {
            operation,
            path,
            source,
            replacement_committed,
            cleanup_error,
        }
    }
}

fn file_error(operation: &'static str, path: &Path, source: io::Error) -> GenomeFileError {
    GenomeFileError::Io {
        operation,
        path: path.to_owned(),
        source,
        replacement_committed: false,
        cleanup_error: None,
    }
}

/// Read at most the XML byte limit plus one sentinel byte before parsing a fresh owned genome
pub fn load_genome_xml(path: &Path) -> Result<Genome, GenomeFileError> {
    let require_regular = |metadata: fs::Metadata| {
        if metadata.is_file() {
            Ok(())
        } else {
            Err(file_error(
                "read genotype",
                path,
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Genotype input must be a regular file",
                ),
            ))
        }
    };
    require_regular(
        fs::metadata(path).map_err(|error| file_error("inspect genotype", path, error))?,
    )?;
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        // Linux O_NONBLOCK prevents a FIFO path swap from blocking before handle validation
        options.custom_flags(0o4000);
    }
    let file = options
        .open(path)
        .map_err(|error| file_error("open genotype", path, error))?;
    require_regular(
        file.metadata()
            .map_err(|error| file_error("inspect opened genotype", path, error))?,
    )?;
    let mut bytes = Vec::new();
    file.take(MAX_DOCUMENT_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| file_error("read genotype", path, error))?;
    parse_genome_xml(&bytes).map_err(GenomeFileError::Xml)
}

/// Encode fully before writing, then synchronize and replace through a private sibling file
/// Reject symbolic links and read-only destinations; preserve existing regular-file permissions
/// Unix also synchronizes the parent directory; other platforms have no additional durability promise
pub fn save_genome_xml(path: &Path, genome: &Genome) -> Result<(), GenomeFileError> {
    let bytes = encode_genome_xml(genome).map_err(GenomeFileError::Xml)?;
    replace_file_bytes(path, &bytes).map_err(GenomeFileError::from)
}

#[cfg(test)]
fn replace_genotype_file(
    path: &Path,
    bytes: &[u8],
    permissions: Option<fs::Permissions>,
) -> Result<(), GenomeFileError> {
    crate::atomic_file::replace_with_permissions(path, bytes, permissions)
        .map_err(GenomeFileError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::JavaRandom;
    use std::ffi::OsString;

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            for _ in 0..128 {
                let path = std::env::temp_dir().join(format!(
                    "bgr-genotype-file-{}-{}",
                    std::process::id(),
                    TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
                ));
                match fs::create_dir(&path) {
                    Ok(()) => return Self(path),
                    Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                    Err(error) => panic!("Cannot create isolated test directory: {error}"),
                }
            }
            panic!("Test directory names are occupied");
        }

        fn contents(&self) -> Vec<OsString> {
            let mut names: Vec<_> = fs::read_dir(&self.0)
                .unwrap()
                .map(|item| item.unwrap().file_name())
                .collect();
            names.sort();
            names
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("Cannot remove owned test directory");
        }
    }

    fn genome() -> Genome {
        Genome::random(&mut JavaRandom::new(827364))
    }

    #[test]
    fn successful_replacement_roundtrips_and_leaves_no_temporary_file() {
        let directory = TestDirectory::new();
        let path = directory.0.join("organism.bgg");
        fs::write(&path, b"previous file").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        }
        let original = genome();
        save_genome_xml(&path, &original).unwrap();
        let loaded = load_genome_xml(&path).unwrap();
        assert_eq!(
            encode_genome_xml(&loaded).unwrap(),
            encode_genome_xml(&original).unwrap()
        );
        assert_eq!(directory.contents(), [OsString::from("organism.bgg")]);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o640
            );
        }
    }

    #[test]
    fn xml_preflight_failure_preserves_existing_bytes_and_directory() {
        let directory = TestDirectory::new();
        let path = directory.0.join("organism.bgg");
        fs::write(&path, b"previous file").unwrap();
        let invalid = genome();
        invalid.gene_ref(0).unwrap().borrow_mut().set_color(None);
        assert!(matches!(
            save_genome_xml(&path, &invalid),
            Err(GenomeFileError::Xml(XmlError::UnsupportedColor { .. }))
        ));
        assert_eq!(fs::read(&path).unwrap(), b"previous file");
        assert_eq!(directory.contents(), [OsString::from("organism.bgg")]);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn valid_long_destination_name_does_not_expand_the_temporary_name() {
        let directory = TestDirectory::new();
        let name = format!("{}.bgg", "a".repeat(240));
        let path = directory.0.join(&name);
        fs::write(&path, b"previous file").unwrap();
        let original = genome();
        save_genome_xml(&path, &original).unwrap();
        assert_eq!(
            encode_genome_xml(&load_genome_xml(&path).unwrap()).unwrap(),
            encode_genome_xml(&original).unwrap()
        );
        assert_eq!(directory.contents(), [OsString::from(name)]);
    }

    #[test]
    fn filesystem_failure_preserves_existing_entries() {
        let directory = TestDirectory::new();
        let target = directory.0.join("organism.bgg");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("keep"), b"previous file").unwrap();
        assert!(matches!(
            save_genome_xml(&target, &genome()),
            Err(GenomeFileError::Io {
                replacement_committed: false,
                ..
            })
        ));
        assert_eq!(fs::read(target.join("keep")).unwrap(), b"previous file");
        assert_eq!(directory.contents(), [OsString::from("organism.bgg")]);
    }

    #[test]
    fn late_rename_failure_removes_only_the_owned_temporary_file() {
        let directory = TestDirectory::new();
        let target = directory.0.join("organism.bgg");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("keep"), b"previous file").unwrap();
        assert!(matches!(
            replace_genotype_file(&target, b"complete replacement", None),
            Err(GenomeFileError::Io {
                replacement_committed: false,
                cleanup_error: None,
                ..
            })
        ));
        assert_eq!(fs::read(target.join("keep")).unwrap(), b"previous file");
        assert_eq!(directory.contents(), [OsString::from("organism.bgg")]);
    }

    #[test]
    fn oversized_file_returns_the_parser_resource_error() {
        let directory = TestDirectory::new();
        let path = directory.0.join("large.bgg");
        File::create(&path)
            .unwrap()
            .set_len(MAX_DOCUMENT_BYTES as u64 + 10_000)
            .unwrap();
        assert!(matches!(
            load_genome_xml(&path),
            Err(GenomeFileError::Xml(XmlError::ResourceLimit {
                resource: "source bytes",
                maximum: MAX_DOCUMENT_BYTES
            }))
        ));
    }

    #[test]
    fn nonregular_input_is_rejected_before_reading() {
        let directory = TestDirectory::new();
        assert!(matches!(
            load_genome_xml(&directory.0),
            Err(GenomeFileError::Io { source, replacement_committed: false, .. })
                if source.kind() == io::ErrorKind::InvalidInput
        ));
        #[cfg(target_os = "linux")]
        {
            assert!(matches!(
                load_genome_xml(Path::new("/dev/null")),
                Err(GenomeFileError::Io { source, replacement_committed: false, .. })
                    if source.kind() == io::ErrorKind::InvalidInput
            ));
        }
    }

    #[cfg(unix)]
    #[test]
    fn symbolic_link_and_readonly_destinations_remain_untouched() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let directory = TestDirectory::new();
        let target = directory.0.join("keep.bgg");
        fs::write(&target, b"previous file").unwrap();
        let link = directory.0.join("link.bgg");
        symlink(&target, &link).unwrap();
        assert!(save_genome_xml(&link, &genome()).is_err());
        assert!(fs::symlink_metadata(&link).unwrap().is_symlink());
        fs::set_permissions(&target, fs::Permissions::from_mode(0o400)).unwrap();
        assert!(save_genome_xml(&target, &genome()).is_err());
        assert_eq!(fs::read(&target).unwrap(), b"previous file");
    }

    #[test]
    fn direct_byte_replacement_preserves_binary_payload_and_leaves_no_temporary_file() {
        let directory = TestDirectory::new();
        let path = directory.0.join("organism.png");
        let bytes = b"\x89PNG\r\n\x1a\n\x00\xff\x80";
        fs::write(&path, b"previous file").unwrap();
        crate::replace_file_bytes(&path, bytes).unwrap();
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(directory.contents(), [OsString::from("organism.png")]);
    }
}
