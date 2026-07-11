#![allow(dead_code)]

use std::path::Path;

pub fn replace_file(source: &Path, destination: &Path) -> Result<(), String> {
    let source_parent = source
        .parent()
        .ok_or_else(|| "source file has no parent directory".to_string())?;
    let destination_parent = destination
        .parent()
        .ok_or_else(|| "destination file has no parent directory".to_string())?;
    let source_parent = source_parent
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let destination_parent = destination_parent
        .canonicalize()
        .map_err(|error| error.to_string())?;
    if source_parent != destination_parent {
        return Err("atomic replacement requires the same parent directory".into());
    }
    replace_file_platform(source, destination)
}

#[cfg(windows)]
fn replace_file_platform(source: &Path, destination: &Path) -> Result<(), String> {
    use std::{ffi::OsStr, os::windows::ffi::OsStrExt};
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    fn wide(value: &OsStr) -> Vec<u16> {
        value.encode_wide().chain(std::iter::once(0)).collect()
    }

    let source = wide(source.as_os_str());
    let destination = wide(destination.as_os_str());
    let ok = unsafe {
        MoveFileExW(
            source.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if ok == 0 {
        Err(std::io::Error::last_os_error().to_string())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_file_platform(source: &Path, destination: &Path) -> Result<(), String> {
    std::fs::rename(source, destination).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::replace_file;
    use std::fs;

    #[test]
    fn replaces_a_nonexistent_destination() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source.txt");
        let destination = dir.path().join("destination.txt");
        fs::write(&source, "first version").unwrap();

        replace_file(&source, &destination).unwrap();

        assert!(!source.exists());
        assert_eq!(fs::read_to_string(destination).unwrap(), "first version");
    }

    #[test]
    fn replaces_an_existing_destination_twice() {
        let dir = tempfile::tempdir().unwrap();
        let destination = dir.path().join("destination.txt");
        fs::write(&destination, "old version").unwrap();

        let first_source = dir.path().join("source-one.txt");
        fs::write(&first_source, "first replacement").unwrap();
        replace_file(&first_source, &destination).unwrap();
        assert_eq!(
            fs::read_to_string(&destination).unwrap(),
            "first replacement"
        );

        let second_source = dir.path().join("source-two.txt");
        fs::write(&second_source, "second replacement").unwrap();
        replace_file(&second_source, &destination).unwrap();
        assert_eq!(
            fs::read_to_string(destination).unwrap(),
            "second replacement"
        );
    }

    #[test]
    fn preserves_an_existing_destination_when_the_source_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("missing.txt");
        let destination = dir.path().join("destination.txt");
        fs::write(&destination, "old version").unwrap();

        assert!(replace_file(&source, &destination).is_err());

        assert_eq!(fs::read_to_string(destination).unwrap(), "old version");
    }

    #[test]
    fn rejects_source_and_destination_in_different_parents() {
        let dir = tempfile::tempdir().unwrap();
        let source_parent = dir.path().join("source-parent");
        let destination_parent = dir.path().join("destination-parent");
        fs::create_dir(&source_parent).unwrap();
        fs::create_dir(&destination_parent).unwrap();
        let source = source_parent.join("source.txt");
        let destination = destination_parent.join("destination.txt");
        fs::write(&source, "source value").unwrap();

        assert!(replace_file(&source, &destination).is_err());

        assert!(source.exists());
        assert!(!destination.exists());
    }
}
