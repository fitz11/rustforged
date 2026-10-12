//! Library export and import operations.

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

use crate::assets::{LIBRARY_MANIFEST_FILE, is_root_manifest};

/// Export an entire library directory to a zip file.
pub fn export_library_to_zip(library_path: &Path, dest_path: &Path) -> Result<(), String> {
    let file = File::create(dest_path).map_err(|e| format!("Failed to create zip file: {}", e))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // Walk the library directory recursively
    fn add_directory_to_zip(
        zip: &mut ZipWriter<File>,
        base_path: &Path,
        current_path: &Path,
        options: SimpleFileOptions,
    ) -> Result<(), String> {
        let entries = std::fs::read_dir(current_path)
            .map_err(|e| format!("Failed to read directory: {}", e))?;

        for entry in entries.flatten() {
            let path = entry.path();
            let relative_path = path
                .strip_prefix(base_path)
                .map_err(|e| format!("Failed to get relative path: {}", e))?;
            let relative_str = relative_path.to_string_lossy();

            if path.is_dir() {
                // Add directory entry
                zip.add_directory(format!("{}/", relative_str), options)
                    .map_err(|e| format!("Failed to add directory to zip: {}", e))?;
                // Recurse into subdirectory
                add_directory_to_zip(zip, base_path, &path, options)?;
            } else {
                // Add file
                zip.start_file(relative_str.to_string(), options)
                    .map_err(|e| format!("Failed to start file in zip: {}", e))?;
                let mut file =
                    File::open(&path).map_err(|e| format!("Failed to open file: {}", e))?;
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer)
                    .map_err(|e| format!("Failed to read file: {}", e))?;
                zip.write_all(&buffer)
                    .map_err(|e| format!("Failed to write file to zip: {}", e))?;
            }
        }
        Ok(())
    }

    add_directory_to_zip(&mut zip, library_path, library_path, options)?;
    zip.finish()
        .map_err(|e| format!("Failed to finalize zip: {}", e))?;
    Ok(())
}

/// Maximum total size of extracted files (500 MB).
const MAX_EXTRACT_SIZE: u64 = 500 * 1024 * 1024;

/// Where to extract a library zip picked to go into `dest`: `dest` itself if it's empty or
/// doesn't exist yet, otherwise a new folder in it named after the zip, so importing never
/// mixes into (or overwrites) an existing folder.
pub fn import_destination(zip_path: &Path, dest: &Path) -> Result<PathBuf, String> {
    if is_empty_or_missing(dest) {
        return Ok(dest.to_path_buf());
    }
    let name = zip_path
        .file_stem()
        .and_then(|s| s.to_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("Imported Library");
    let target = dest.join(name);
    if is_empty_or_missing(&target) {
        Ok(target)
    } else {
        Err(format!(
            "\"{}\" already exists and isn't empty.\n\n\
             Choose an empty folder to import the library into.",
            target.display()
        ))
    }
}

fn is_empty_or_missing(dir: &Path) -> bool {
    match std::fs::read_dir(dir) {
        Ok(mut entries) => entries.next().is_none(),
        Err(_) => !dir.exists(),
    }
}

/// Import a library from a zip file into `dest_path`, which should be empty (see
/// [`import_destination`]). Existing files are never overwritten.
pub fn import_library_from_zip(zip_path: &Path, dest_path: &Path) -> Result<(), String> {
    let file = File::open(zip_path).map_err(|e| format!("Failed to open zip file: {}", e))?;
    let mut archive =
        ZipArchive::new(file).map_err(|e| format!("Failed to read zip archive: {}", e))?;

    // First pass: validate archive
    let mut has_library_metadata = false;
    let mut total_size: u64 = 0;

    for i in 0..archive.len() {
        let entry = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read zip entry: {}", e))?;

        // Security: every entry must stay inside the destination. enclosed_name rejects
        // ".." escapes and drops drive prefixes and leading slashes (C:x, \x, /x), which
        // Path::is_absolute doesn't catch on Windows.
        if entry.enclosed_name().is_none() {
            return Err(format!(
                "Invalid archive: contains unsafe path '{}'. \
                 Archive may be malicious.",
                entry.name()
            ));
        }

        // Check for a library manifest at the root level
        if is_root_manifest(entry.name()) {
            has_library_metadata = true;
        }

        // Accumulate total decompressed size
        total_size += entry.size();
    }

    if !has_library_metadata {
        return Err(format!(
            "Invalid library archive: missing {} file.\n\n\
             This zip file does not contain a valid asset library.",
            LIBRARY_MANIFEST_FILE
        ));
    }

    // Security: check for zip bombs
    if total_size > MAX_EXTRACT_SIZE {
        return Err(format!(
            "Archive too large: {} MB uncompressed (max {} MB).\n\n\
             This archive exceeds the maximum allowed size.",
            total_size / (1024 * 1024),
            MAX_EXTRACT_SIZE / (1024 * 1024)
        ));
    }

    // Create destination directory
    std::fs::create_dir_all(dest_path)
        .map_err(|e| format!("Failed to create destination directory: {}", e))?;

    // Extract all files. The declared sizes were checked above; also stop at the limit
    // while writing, in case an entry decompresses to more than it declared.
    let mut remaining = MAX_EXTRACT_SIZE;
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read zip entry: {}", e))?;
        let Some(relative) = entry.enclosed_name() else {
            continue;
        };
        let outpath = dest_path.join(relative);

        if entry.is_dir() {
            std::fs::create_dir_all(&outpath)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        } else {
            // Ensure parent directory exists
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create parent directory: {}", e))?;
            }
            let mut outfile = File::create_new(&outpath)
                .map_err(|e| format!("Failed to create {}: {}", outpath.display(), e))?;
            let written = std::io::copy(&mut (&mut entry).take(remaining + 1), &mut outfile)
                .map_err(|e| format!("Failed to extract file: {}", e))?;
            if written > remaining {
                return Err(format!(
                    "Archive too large: more than {} MB uncompressed.",
                    MAX_EXTRACT_SIZE / (1024 * 1024)
                ));
            }
            remaining -= written;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Writes a zip with a library manifest plus `entries` (name, contents).
    fn library_zip(dir: &Path, entries: &[(&str, &str)]) -> PathBuf {
        let path = dir.join("Dungeon Tiles.zip");
        let mut zip = ZipWriter::new(File::create(&path).unwrap());
        let options = SimpleFileOptions::default();
        zip.start_file(LIBRARY_MANIFEST_FILE, options).unwrap();
        zip.write_all(b"{}").unwrap();
        for (name, contents) in entries {
            zip.start_file(*name, options).unwrap();
            zip.write_all(contents.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
        path
    }

    #[test]
    fn test_import_rejects_entries_outside_the_destination() {
        for name in ["../evil.png", "C:evil.png", "a/../../evil.png"] {
            let dir = tempfile::tempdir().unwrap();
            let zip = library_zip(dir.path(), &[(name, "x")]);
            let dest = dir.path().join("library");
            // Either refused, or extracted inside the destination; never next to it
            let _ = import_library_from_zip(&zip, &dest);
            assert!(!dir.path().join("evil.png").exists(), "{name}");
        }
    }

    #[test]
    fn test_import_keeps_rooted_entries_inside_the_destination() {
        let dir = tempfile::tempdir().unwrap();
        let zip = library_zip(dir.path(), &[("\\tokens\\orc.png", "orc")]);
        let dest = dir.path().join("library");

        import_library_from_zip(&zip, &dest).unwrap();

        assert_eq!(
            std::fs::read_to_string(dest.join("tokens").join("orc.png")).unwrap(),
            "orc"
        );
    }

    #[test]
    fn test_import_into_non_empty_folder_uses_a_new_subfolder() {
        let dir = tempfile::tempdir().unwrap();
        let zip = library_zip(dir.path(), &[("tokens/orc.png", "orc")]);

        // The temp dir already holds the zip, so it isn't empty
        let dest = import_destination(&zip, dir.path()).unwrap();
        assert_eq!(dest, dir.path().join("Dungeon Tiles"));
        import_library_from_zip(&zip, &dest).unwrap();
        assert!(dest.join("tokens/orc.png").exists());

        // Importing the same zip again doesn't touch the first copy
        assert!(import_destination(&zip, dir.path()).is_err());
    }

    #[test]
    fn test_import_never_overwrites_files() {
        let dir = tempfile::tempdir().unwrap();
        let zip = library_zip(dir.path(), &[("notes.txt", "from zip")]);
        let dest = dir.path().join("library");
        std::fs::create_dir_all(&dest).unwrap();
        std::fs::write(dest.join("notes.txt"), "mine").unwrap();

        assert!(import_library_from_zip(&zip, &dest).is_err());
        assert_eq!(
            std::fs::read_to_string(dest.join("notes.txt")).unwrap(),
            "mine"
        );
    }
}
