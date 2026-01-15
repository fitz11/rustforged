//! Library export and import operations.

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

/// Name of the library metadata file.
pub(crate) const LIBRARY_METADATA_FILE: &str = ".library.json";

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

/// Check if a zip entry name is safe (no path traversal or absolute paths).
fn is_safe_path(name: &str) -> bool {
    // Reject absolute paths
    if Path::new(name).is_absolute() {
        return false;
    }
    // Reject path traversal attempts
    for component in Path::new(name).components() {
        if matches!(component, std::path::Component::ParentDir) {
            return false;
        }
    }
    true
}

/// Import a library from a zip file to a destination directory.
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
        let name = entry.name();

        // Security: check for path traversal attacks
        if !is_safe_path(name) {
            return Err(format!(
                "Invalid archive: contains unsafe path '{}'. \
                 Archive may be malicious.",
                name
            ));
        }

        // Check if .library.json is at the root level
        if name == LIBRARY_METADATA_FILE || name == format!("{}/", LIBRARY_METADATA_FILE) {
            has_library_metadata = true;
        }

        // Accumulate total decompressed size
        total_size += entry.size();
    }

    if !has_library_metadata {
        return Err(
            "Invalid library archive: missing .library.json file.\n\n\
             This zip file does not contain a valid asset library."
                .to_string(),
        );
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

    // Extract all files
    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| format!("Failed to read zip entry: {}", e))?;
        let outpath = dest_path.join(entry.name());

        if entry.is_dir() {
            std::fs::create_dir_all(&outpath)
                .map_err(|e| format!("Failed to create directory: {}", e))?;
        } else {
            // Ensure parent directory exists
            if let Some(parent) = outpath.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create parent directory: {}", e))?;
            }
            let mut outfile = File::create(&outpath)
                .map_err(|e| format!("Failed to create file: {}", e))?;
            std::io::copy(&mut entry, &mut outfile)
                .map_err(|e| format!("Failed to extract file: {}", e))?;
        }
    }

    Ok(())
}
