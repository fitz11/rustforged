//! File writes that behave well in synced folders.

use std::io::Write;
use std::path::Path;

/// Writes `contents` to `path` atomically: the data goes to a hidden temp file in the same
/// folder, is flushed to disk, then renamed over the target. Readers and sync clients see
/// either the old file or the new one, never a partly written file.
pub fn write_atomic(path: &Path, contents: &[u8]) -> std::io::Result<()> {
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let existing_permissions = std::fs::metadata(path).ok().map(|m| m.permissions());

    let mut builder = tempfile::Builder::new();
    builder.prefix(".rustforged-").suffix(".tmp");
    // Temp files default to owner-only; create like a normal file instead (umask applies)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(std::fs::Permissions::from_mode(0o666));
    }
    let mut temp = builder.tempfile_in(dir)?;

    temp.write_all(contents)?;
    if let Some(permissions) = existing_permissions {
        temp.as_file().set_permissions(permissions)?;
    }
    temp.as_file().sync_all()?;
    temp.persist(path).map_err(|e| e.error)?;
    Ok(())
}

/// Like [`write_atomic`], but leaves the file alone when it already has these contents: no
/// new modification time, so nothing for a sync client to upload. Returns whether it wrote.
pub fn write_if_changed(path: &Path, contents: &[u8]) -> std::io::Result<bool> {
    if std::fs::read(path).is_ok_and(|existing| existing == contents) {
        return Ok(false);
    }
    write_atomic(path, contents)?;
    Ok(true)
}

/// This machine's host name, used to say which machine last saved a map.
pub fn machine_name() -> Option<String> {
    let name = gethostname::gethostname()
        .to_string_lossy()
        .trim()
        .to_string();
    (!name.is_empty()).then_some(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_atomic_replaces_contents_without_leaving_temp_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("map.json");
        std::fs::write(&path, "old").unwrap();

        write_atomic(&path, b"new").unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
        let names: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, ["map.json"]);
    }

    #[cfg(unix)]
    #[test]
    fn test_write_atomic_keeps_existing_permissions() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("map.json");
        std::fs::write(&path, "old").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640)).unwrap();

        write_atomic(&path, b"new").unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o640);
    }

    #[test]
    fn test_write_if_changed_skips_identical_contents() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("map.json");

        assert!(write_if_changed(&path, b"one").unwrap());
        let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
        assert!(!write_if_changed(&path, b"one").unwrap());
        assert_eq!(
            std::fs::metadata(&path).unwrap().modified().unwrap(),
            modified
        );
        assert!(write_if_changed(&path, b"two").unwrap());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "two");
    }
}
