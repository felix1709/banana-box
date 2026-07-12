use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};
use zip::ZipArchive;

pub const MAX_ARCHIVE_BYTES: u64 = 2 * 1024 * 1024 * 1024;
pub const MAX_ARCHIVE_ENTRIES: usize = 10_000;
pub const MAX_ARCHIVE_FILE_BYTES: u64 = 512 * 1024 * 1024;
pub const MAX_ARCHIVE_EXTRACTED_BYTES: u64 = 4 * 1024 * 1024 * 1024;
pub const MAX_ARCHIVE_COMPRESSION_RATIO: u64 = 100;

#[derive(Clone, Copy)]
pub struct ArchiveLimits {
    pub max_archive_bytes: u64,
    pub max_entries: usize,
    pub max_file_bytes: u64,
    pub max_extracted_bytes: u64,
    pub max_compression_ratio: u64,
}

impl Default for ArchiveLimits {
    fn default() -> Self {
        Self {
            max_archive_bytes: MAX_ARCHIVE_BYTES,
            max_entries: MAX_ARCHIVE_ENTRIES,
            max_file_bytes: MAX_ARCHIVE_FILE_BYTES,
            max_extracted_bytes: MAX_ARCHIVE_EXTRACTED_BYTES,
            max_compression_ratio: MAX_ARCHIVE_COMPRESSION_RATIO,
        }
    }
}

#[derive(Debug)]
pub struct VerifiedArchiveEntry {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
    pub sha256: String,
}

pub fn read_zip(
    source: &Path,
    limits: ArchiveLimits,
    mut policy: impl FnMut(&Path, bool, u64) -> Result<(), String>,
) -> Result<Vec<VerifiedArchiveEntry>, String> {
    let metadata = std::fs::metadata(source).map_err(|_| "ARCHIVE_UNAVAILABLE")?;
    if metadata.len() > limits.max_archive_bytes {
        return Err("ARCHIVE_LIMIT_EXCEEDED".into());
    }

    let file = File::open(source).map_err(|_| "ARCHIVE_UNAVAILABLE")?;
    let mut archive = ZipArchive::new(file).map_err(|_| "INVALID_ARCHIVE")?;
    if archive.len() > limits.max_entries {
        return Err("ARCHIVE_LIMIT_EXCEEDED".into());
    }

    let mut paths = HashSet::new();
    let mut extracted = 0_u64;
    let mut verified = Vec::new();
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|_| "INVALID_ARCHIVE")?;
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err("UNSAFE_ARCHIVE_ENTRY".into());
        }

        let path = normalized_archive_path(entry.name())?;
        let directory = entry.is_dir();
        policy(&path, directory, entry.size())?;
        if directory {
            continue;
        }

        let collision_key = windows_collision_key(&path)?;
        if !paths.insert(collision_key) {
            return Err("ARCHIVE_PATH_COLLISION".into());
        }
        if entry.size() > limits.max_file_bytes || entry.size() > limits.max_extracted_bytes {
            return Err("ARCHIVE_LIMIT_EXCEEDED".into());
        }
        if exceeds_ratio(
            entry.size(),
            entry.compressed_size(),
            limits.max_compression_ratio,
        ) {
            return Err("ARCHIVE_COMPRESSION_RATIO_EXCEEDED".into());
        }

        let remaining = limits.max_extracted_bytes.saturating_sub(extracted);
        let bytes = read_bounded(&mut entry, limits.max_file_bytes.min(remaining))?;
        let actual = bytes.len() as u64;
        if actual > limits.max_file_bytes || actual > remaining {
            return Err("ARCHIVE_LIMIT_EXCEEDED".into());
        }
        if exceeds_ratio(
            actual,
            entry.compressed_size(),
            limits.max_compression_ratio,
        ) {
            return Err("ARCHIVE_COMPRESSION_RATIO_EXCEEDED".into());
        }
        extracted = extracted
            .checked_add(actual)
            .ok_or_else(|| "ARCHIVE_LIMIT_EXCEEDED".to_string())?;

        let sha256 = format!("{:x}", Sha256::digest(&bytes));
        verified.push(VerifiedArchiveEntry {
            path,
            bytes,
            sha256,
        });
    }
    Ok(verified)
}

fn read_bounded(reader: &mut impl Read, limit: u64) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer).map_err(|_| "INVALID_ARCHIVE")?;
        if read == 0 {
            return Ok(bytes);
        }
        let next = bytes
            .len()
            .checked_add(read)
            .ok_or_else(|| "ARCHIVE_LIMIT_EXCEEDED".to_string())?;
        if next as u64 > limit {
            return Err("ARCHIVE_LIMIT_EXCEEDED".into());
        }
        bytes.extend_from_slice(&buffer[..read]);
    }
}

fn exceeds_ratio(actual: u64, compressed: u64, maximum: u64) -> bool {
    if actual == 0 {
        return false;
    }
    compressed == 0 || actual > compressed.saturating_mul(maximum)
}

fn normalized_archive_path(raw: &str) -> Result<PathBuf, String> {
    if raw.is_empty() || raw.contains('\\') || raw.starts_with('/') || raw.starts_with("//") {
        return Err("UNSAFE_ARCHIVE_PATH".into());
    }

    let mut path = PathBuf::new();
    for segment in raw.trim_end_matches('/').split('/') {
        if segment.is_empty()
            || segment == "."
            || segment == ".."
            || segment.contains(':')
            || segment.ends_with(['.', ' '])
            || is_windows_reserved_name(segment)
        {
            return Err("UNSAFE_ARCHIVE_PATH".into());
        }
        path.push(segment);
    }
    (!path.as_os_str().is_empty())
        .then_some(path)
        .ok_or_else(|| "UNSAFE_ARCHIVE_PATH".to_string())
}

fn windows_collision_key(path: &Path) -> Result<String, String> {
    let mut key = String::new();
    for component in path.components() {
        let std::path::Component::Normal(segment) = component else {
            return Err("UNSAFE_ARCHIVE_PATH".into());
        };
        if !key.is_empty() {
            key.push('/');
        }
        key.push_str(&segment.to_string_lossy().to_ascii_lowercase());
    }
    Ok(key)
}

fn is_windows_reserved_name(segment: &str) -> bool {
    let name = segment
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    matches!(name.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || name
            .strip_prefix("COM")
            .or_else(|| name.strip_prefix("LPT"))
            .is_some_and(|number| {
                matches!(number, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
            })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};
    use tempfile::tempdir;
    use zip::{write::SimpleFileOptions, ZipWriter};

    fn archive_path(entries: &[(&str, &[u8])]) -> std::path::PathBuf {
        let directory = tempdir().unwrap().keep();
        let path = directory.join("fixture.zip");
        let cursor = Cursor::new(Vec::new());
        let mut writer = ZipWriter::new(cursor);
        for (name, contents) in entries {
            writer
                .start_file(name, SimpleFileOptions::default())
                .unwrap();
            writer.write_all(contents).unwrap();
        }
        std::fs::write(&path, writer.finish().unwrap().into_inner()).unwrap();
        path
    }

    fn accept_all(_path: &Path, _directory: bool, _size: u64) -> Result<(), String> {
        Ok(())
    }

    #[test]
    fn rejects_parent_path_entries_before_extraction() {
        let path = archive_path(&[("images/../../escape.txt", b"unsafe")]);

        assert_eq!(
            read_zip(&path, ArchiveLimits::default(), accept_all).unwrap_err(),
            "UNSAFE_ARCHIVE_PATH"
        );
    }

    #[test]
    fn rejects_absolute_paths_windows_aliases_and_case_collisions() {
        for entries in [
            vec![("/escape.txt", b"unsafe" as &[u8])],
            vec![("images/CON.txt", b"unsafe")],
            vec![("images/Foo.png", b"one"), ("images/foo.png", b"two")],
            vec![("images/a.txt:stream", b"unsafe")],
        ] {
            let path = archive_path(&entries);
            assert!(matches!(
                read_zip(&path, ArchiveLimits::default(), accept_all),
                Err(error) if error == "UNSAFE_ARCHIVE_PATH" || error == "ARCHIVE_PATH_COLLISION"
            ));
        }
    }

    #[test]
    fn enforces_entry_and_actual_byte_limits() {
        let path = archive_path(&[("images/large.png", b"12345")]);
        let limits = ArchiveLimits {
            max_file_bytes: 4,
            ..ArchiveLimits::default()
        };

        assert_eq!(
            read_zip(&path, limits, accept_all).unwrap_err(),
            "ARCHIVE_LIMIT_EXCEEDED"
        );
    }
}
