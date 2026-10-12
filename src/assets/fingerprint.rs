//! Content fingerprints for library assets, and the resolver that uses them to find an
//! asset again after its file was renamed or moved outside the app.
//!
//! Map files store a fingerprint per referenced asset. When a reference no longer matches a
//! file (renamed on another machine, moved in a file manager, or not synced yet), loading
//! tries, in order: the exact path, the same path ignoring case and Unicode normalization,
//! a file with identical content, and a uniquely named file. Hashing is lazy and filtered
//! by file size first, so online-only files from sync clients aren't downloaded just to be
//! ruled out.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;
use unicode_normalization::UnicodeNormalization;

/// Hex characters kept from the BLAKE3 hash (128 bits).
const HASH_HEX_LEN: usize = 32;

/// Identifies an asset's content independently of its path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetFingerprint {
    pub size: u64,
    /// Leading 128 bits of the file's BLAKE3 hash, hex-encoded
    pub hash: String,
}

impl AssetFingerprint {
    pub fn of_file(path: &Path) -> std::io::Result<Self> {
        let mut hasher = blake3::Hasher::new();
        let size = std::io::copy(&mut std::fs::File::open(path)?, &mut hasher)?;
        let hash = hasher.finalize().to_hex()[..HASH_HEX_LEN].to_string();
        Ok(Self { size, hash })
    }
}

/// Size and modification time of a file, from metadata alone (no content read).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FileStamp {
    pub size: u64,
    /// Nanoseconds since the Unix epoch, if the platform reports it
    pub modified_ns: Option<u64>,
}

impl FileStamp {
    pub fn of_metadata(metadata: &std::fs::Metadata) -> Self {
        let modified_ns = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .and_then(|d| u64::try_from(d.as_nanos()).ok());
        Self {
            size: metadata.len(),
            modified_ns,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedFingerprint {
    stamp: FileStamp,
    fingerprint: AssetFingerprint,
}

/// Fingerprints of library files, keyed by library-relative path. An entry is only trusted
/// while the file's size and modification time are unchanged.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FingerprintCache {
    entries: HashMap<String, CachedFingerprint>,
}

impl FingerprintCache {
    pub fn get(&self, path: &str, stamp: FileStamp) -> Option<&AssetFingerprint> {
        self.entries
            .get(path)
            .filter(|e| stamp.modified_ns.is_some() && e.stamp == stamp)
            .map(|e| &e.fingerprint)
    }

    pub fn insert(&mut self, path: String, stamp: FileStamp, fingerprint: AssetFingerprint) {
        self.entries
            .insert(path, CachedFingerprint { stamp, fingerprint });
    }

    /// Adds or replaces entries from `other`.
    pub fn merge(&mut self, other: FingerprintCache) {
        self.entries.extend(other.entries);
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Drops entries for paths that aren't in the library any more.
    pub fn retain_paths(&mut self, mut keep: impl FnMut(&str) -> bool) {
        self.entries.retain(|path, _| keep(path));
    }
}

/// A snapshot of the library that background tasks can fingerprint and resolve against
/// without touching ECS state.
#[derive(Clone)]
pub struct LibraryIndex {
    root: PathBuf,
    /// Library-relative paths and their stamps, sorted by path
    assets: Vec<(String, FileStamp)>,
    cache: FingerprintCache,
    /// Fingerprints computed through this index, to merge back into the library's cache
    computed: FingerprintCache,
}

impl LibraryIndex {
    pub fn new(
        root: PathBuf,
        assets: impl IntoIterator<Item = (String, FileStamp)>,
        cache: FingerprintCache,
    ) -> Self {
        let mut assets: Vec<_> = assets.into_iter().collect();
        assets.sort_by(|a, b| a.0.cmp(&b.0));
        Self {
            root,
            assets,
            cache,
            computed: FingerprintCache::default(),
        }
    }

    fn stamp(&self, path: &str) -> Option<FileStamp> {
        self.assets
            .binary_search_by(|(p, _)| p.as_str().cmp(path))
            .ok()
            .map(|i| self.assets[i].1)
    }

    pub fn contains(&self, path: &str) -> bool {
        self.stamp(path).is_some()
    }

    /// Fingerprint of a library asset, from the cache when its stamp still matches.
    pub fn fingerprint(&mut self, path: &str) -> Option<AssetFingerprint> {
        let stamp = self.stamp(path)?;
        if let Some(fp) = self
            .computed
            .get(path, stamp)
            .or_else(|| self.cache.get(path, stamp))
        {
            return Some(fp.clone());
        }
        let fingerprint = AssetFingerprint::of_file(&self.root.join(path)).ok()?;
        self.computed
            .insert(path.to_string(), stamp, fingerprint.clone());
        Some(fingerprint)
    }

    /// Fingerprints computed so far, for merging into the library's cache.
    pub fn into_computed(self) -> FingerprintCache {
        self.computed
    }

    /// Finds the library asset a map reference points at, if any.
    ///
    /// `known` is the fingerprint the map recorded for `path` when it was last saved.
    pub fn resolve(&mut self, path: &str, known: Option<&AssetFingerprint>) -> Option<String> {
        let path = path.replace('\\', "/");
        if self.contains(&path) {
            return Some(path);
        }

        // Same path apart from letter case or Unicode normalization (Windows/macOS vs Linux)
        let folded = fold(&path);
        if let Some(found) = self.unique(|p| fold(p) == folded) {
            return Some(found);
        }

        let name = fold(file_name(&path));

        // Identical content elsewhere in the library (renamed or moved)
        if let Some(known) = known {
            let candidates: Vec<String> = self
                .assets
                .iter()
                .filter(|(_, stamp)| stamp.size == known.size)
                .map(|(p, _)| p.clone())
                .collect();
            let matches: Vec<String> = candidates
                .into_iter()
                .filter(|p| self.fingerprint(p).as_ref() == Some(known))
                .collect();
            // Identical copies look the same; prefer one that kept the file name
            if let Some(found) = matches
                .iter()
                .find(|p| fold(file_name(p)) == name)
                .or(matches.first())
            {
                return Some(found.clone());
            }
        }

        // The only file in the library with that name, when the map recorded no fingerprint
        // (saved before fingerprints). With a fingerprint that matched nothing, a same-named
        // file is a different picture, or the right one hasn't synced yet: relinking to it
        // would lose the reference for good on the next save, so it's only a suggestion
        // (see same_name_candidate).
        if known.is_none() {
            return self.unique(|p| fold(file_name(p)) == name);
        }
        None
    }

    /// The only library file with the same name as `path`, offered for a reference that
    /// couldn't be resolved (its content didn't match).
    pub fn same_name_candidate(&self, path: &str) -> Option<String> {
        let name = fold(file_name(&path.replace('\\', "/")));
        self.unique(|p| fold(file_name(p)) == name)
    }

    fn unique(&self, mut matches: impl FnMut(&str) -> bool) -> Option<String> {
        let mut found = self.assets.iter().filter(|(p, _)| matches(p));
        let first = found.next()?;
        found.next().is_none().then(|| first.0.clone())
    }
}

/// Case- and normalization-insensitive form of a path for comparisons.
fn fold(s: &str) -> String {
    s.nfc().flat_map(char::to_lowercase).collect()
}

fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Helpers for tests elsewhere in the crate that build libraries on disk.
#[cfg(test)]
pub mod test_support {
    use super::FileStamp;
    use std::path::Path;

    pub fn stamp_of(path: &Path) -> FileStamp {
        FileStamp::of_metadata(&std::fs::metadata(path).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Library on disk for resolver tests: each entry is (relative path, file contents).
    fn library(files: &[(&str, &str)]) -> (tempfile::TempDir, LibraryIndex) {
        let dir = tempfile::tempdir().unwrap();
        let mut assets = Vec::new();
        for (path, contents) in files {
            let full = dir.path().join(path);
            std::fs::create_dir_all(full.parent().unwrap()).unwrap();
            std::fs::write(&full, contents).unwrap();
            let stamp = FileStamp::of_metadata(&std::fs::metadata(&full).unwrap());
            assets.push((path.to_string(), stamp));
        }
        let index = LibraryIndex::new(
            dir.path().to_path_buf(),
            assets,
            FingerprintCache::default(),
        );
        (dir, index)
    }

    fn fingerprint_of(contents: &str) -> AssetFingerprint {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f");
        std::fs::write(&path, contents).unwrap();
        AssetFingerprint::of_file(&path).unwrap()
    }

    #[test]
    fn test_fingerprint_is_stable_and_content_based() {
        let a = fingerprint_of("orc pixels");
        assert_eq!(a, fingerprint_of("orc pixels"));
        assert_ne!(a.hash, fingerprint_of("goblin pixels").hash);
        assert_eq!(a.size, 10);
        assert_eq!(a.hash.len(), HASH_HEX_LEN);
    }

    #[test]
    fn test_resolve_exact_path() {
        let (_dir, mut index) = library(&[("tokens/orc.png", "orc")]);
        assert_eq!(
            index.resolve("tokens/orc.png", None),
            Some("tokens/orc.png".into())
        );
    }

    #[test]
    fn test_resolve_windows_separators() {
        let (_dir, mut index) = library(&[("tokens/orc.png", "orc")]);
        assert_eq!(
            index.resolve("tokens\\orc.png", None),
            Some("tokens/orc.png".into())
        );
    }

    #[test]
    fn test_resolve_ignores_case_and_unicode_normalization() {
        // NFC "é" in the library, decomposed "e" + combining accent in the map
        let (_dir, mut index) = library(&[("Tokens/Caf\u{e9}.png", "cafe")]);
        assert_eq!(
            index.resolve("tokens/cafe\u{301}.png", None),
            Some("Tokens/Caf\u{e9}.png".into())
        );
    }

    #[test]
    fn test_resolve_renamed_file_by_content() {
        let (_dir, mut index) = library(&[
            ("tokens/orc-chief.png", "orc"),
            ("tokens/goblin.png", "gob"),
        ]);
        let known = fingerprint_of("orc");
        assert_eq!(
            index.resolve("tokens/orc.png", Some(&known)),
            Some("tokens/orc-chief.png".into())
        );
    }

    #[test]
    fn test_resolve_prefers_identical_copy_with_same_name() {
        let (_dir, mut index) = library(&[("a/copy.png", "orc"), ("b/orc.png", "orc")]);
        let known = fingerprint_of("orc");
        assert_eq!(
            index.resolve("tokens/orc.png", Some(&known)),
            Some("b/orc.png".into())
        );
    }

    #[test]
    fn test_resolve_same_name_different_content_is_only_a_candidate() {
        // Moved and edited, or an unrelated file while the right one is still syncing:
        // either way not relinked silently
        let (_dir, mut index) = library(&[("monsters/orc.png", "orc, retouched")]);
        let known = fingerprint_of("orc");
        assert_eq!(index.resolve("tokens/orc.png", Some(&known)), None);
        assert_eq!(
            index.same_name_candidate("tokens/orc.png"),
            Some("monsters/orc.png".into())
        );
    }

    #[test]
    fn test_resolve_unique_name_without_fingerprint() {
        // Maps saved before fingerprints still find a moved file by name
        let (_dir, mut index) = library(&[("monsters/orc.png", "orc")]);
        assert_eq!(
            index.resolve("tokens/orc.png", None),
            Some("monsters/orc.png".into())
        );
    }

    #[test]
    fn test_resolve_ambiguous_name_is_missing() {
        let (_dir, mut index) = library(&[("a/orc.png", "one"), ("b/orc.png", "two")]);
        assert_eq!(index.resolve("tokens/orc.png", None), None);
    }

    #[test]
    fn test_resolve_missing_file() {
        let (_dir, mut index) = library(&[("tokens/goblin.png", "gob")]);
        assert_eq!(
            index.resolve("tokens/orc.png", Some(&fingerprint_of("orc"))),
            None
        );
    }

    #[test]
    fn test_resolve_only_hashes_same_size_candidates() {
        let (dir, mut index) = library(&[("big.png", "much longer contents"), ("a.png", "orc")]);
        // Make the large file unreadable as content; a size mismatch must skip it entirely
        std::fs::remove_file(dir.path().join("big.png")).unwrap();
        let known = fingerprint_of("orc");
        assert_eq!(
            index.resolve("gone.png", Some(&known)),
            Some("a.png".into())
        );
        assert!(
            index
                .computed
                .get("a.png", index.stamp("a.png").unwrap())
                .is_some()
        );
        assert!(!index.computed.entries.contains_key("big.png"));
    }

    #[test]
    fn test_cache_entry_invalidated_by_stamp_change() {
        let mut cache = FingerprintCache::default();
        let stamp = FileStamp {
            size: 3,
            modified_ns: Some(10),
        };
        cache.insert("a.png".into(), stamp, fingerprint_of("orc"));
        assert!(cache.get("a.png", stamp).is_some());
        let touched = FileStamp {
            modified_ns: Some(11),
            ..stamp
        };
        assert!(cache.get("a.png", touched).is_none());
        // Without a modification time, a size match alone isn't trusted
        let unknown = FileStamp {
            size: 3,
            modified_ns: None,
        };
        cache.insert("b.png".into(), unknown, fingerprint_of("orc"));
        assert!(cache.get("b.png", unknown).is_none());
    }
}
