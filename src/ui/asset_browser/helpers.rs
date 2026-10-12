//! Helper functions for the asset browser.

use bevy_egui::egui;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::assets::{AssetLibrary, relative_to_library};

/// Scan the maps directory, including subfolders, and return a sorted list of
/// (display name, path). The display name is the path below maps/ without the extension,
/// e.g. "campaign/goblin cave". Hidden files and folders (sync client temp files) are skipped.
pub fn scan_maps_directory(maps_dir: &Path) -> Vec<(String, PathBuf)> {
    fn scan(maps_dir: &Path, dir: &Path, maps: &mut Vec<(String, PathBuf)>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if entry.file_name().to_string_lossy().starts_with('.') {
                continue;
            }
            if path.is_dir() {
                scan(maps_dir, &path, maps);
            } else if path.extension().and_then(|ext| ext.to_str()) == Some("json")
                && let Some(name) = relative_to_library(maps_dir, &path.with_extension(""))
            {
                maps.push((name, path));
            }
        }
    }

    let mut maps = Vec::new();
    scan(maps_dir, maps_dir, &mut maps);
    maps.sort_by_cached_key(|(name, _)| tree_sort_key(name));
    maps
}

/// Indentation added before a name for each folder level in the explorers.
const INDENT: &str = "  ";

/// `name` indented by `depth` folder levels.
pub fn indented(name: &str, depth: usize) -> String {
    format!("{}{}", INDENT.repeat(depth), name)
}

/// Sort key for "/"-separated paths that keeps each folder's contents together, right after
/// the folder itself. Plain string order doesn't: "town hall" sorts between "town" and
/// "town/inn" because ' ' comes before '/'. Case-insensitive, with the original spelling as
/// a tiebreak so folders differing only in case don't interleave.
pub fn tree_sort_key(path: &str) -> Vec<(String, String)> {
    path.split('/')
        .map(|segment| (segment.to_lowercase(), segment.to_string()))
        .collect()
}

/// One line of the maps explorer.
#[derive(Debug, PartialEq)]
pub enum MapTreeRow {
    /// A subfolder of maps/
    Folder { name: String, depth: usize },
    /// A map file, with any sync conflict copies of it to list beneath it
    Map {
        name: String,
        depth: usize,
        path: PathBuf,
        /// This map is itself a conflict copy whose original isn't in the list
        conflict_copy: bool,
        /// (file name, path) of conflict copies of this map
        conflict_copies: Vec<(String, PathBuf)>,
    },
}

/// Lays out scanned maps (sorted by [`scan_maps_directory`]) as a tree: a header for each
/// subfolder, then its maps one level deeper. Conflict copies are attached to the map they
/// conflict with rather than listed on their own.
pub fn map_tree(maps: &[(String, PathBuf)]) -> Vec<MapTreeRow> {
    let names: HashSet<&str> = maps.iter().map(|(name, _)| name.as_str()).collect();
    let mut rows = Vec::new();
    let mut open_folders: Vec<&str> = Vec::new();

    for (name, path) in maps {
        let original = conflict_copy_original(name);
        if original.is_some_and(|o| names.contains(o)) {
            continue;
        }

        let mut segments: Vec<&str> = name.split('/').collect();
        let file_name = segments.pop().unwrap_or(name);
        let shared = open_folders
            .iter()
            .zip(&segments)
            .take_while(|(open, folder)| open == folder)
            .count();
        open_folders.truncate(shared);
        for folder in &segments[shared..] {
            rows.push(MapTreeRow::Folder {
                name: folder.to_string(),
                depth: open_folders.len(),
            });
            open_folders.push(folder);
        }

        let conflict_copies = maps
            .iter()
            .filter(|(copy, _)| conflict_copy_original(copy) == Some(name.as_str()))
            .map(|(copy, copy_path)| {
                let copy_name = copy.rsplit('/').next().unwrap_or(copy);
                (copy_name.to_string(), copy_path.clone())
            })
            .collect();
        rows.push(MapTreeRow::Map {
            name: file_name.to_string(),
            depth: segments.len(),
            path: path.clone(),
            conflict_copy: original.is_some(),
            conflict_copies,
        });
    }
    rows
}

/// If `name` (a map's display name) is a sync client's conflict copy, the display name of
/// the map it conflicts with. Recognizes Dropbox and Nextcloud ("cave (DESKTOP's conflicted
/// copy 2026-10-12)", "cave (conflicted copy 2026-10-12 140233)"), older Nextcloud/ownCloud
/// ("cave_conflict-20261012-140233") and Syncthing ("cave.sync-conflict-20261012-140233-ABC").
pub fn conflict_copy_original(name: &str) -> Option<&str> {
    if let Some(i) = name.find(".sync-conflict-") {
        return Some(&name[..i]);
    }
    if let Some(i) = name.rfind("_conflict-") {
        return Some(&name[..i]);
    }
    let open = name.rfind(" (")?;
    (name.ends_with(')') && name[open..].to_lowercase().contains("conflict")).then(|| &name[..open])
}

/// Discover all folders in the library from asset paths.
pub fn discover_folders(library: &AssetLibrary) -> Vec<String> {
    let mut folders: HashSet<String> = HashSet::new();
    for asset in &library.assets {
        if !asset.folder_path.is_empty() {
            // Add the folder and all parent folders
            let mut path = String::new();
            for component in asset.folder_path.split(['/', '\\']) {
                if !path.is_empty() {
                    path.push('/');
                }
                path.push_str(component);
                folders.insert(path.clone());
            }
        }
    }
    let mut sorted: Vec<String> = folders.into_iter().collect();
    sorted.sort_by_cached_key(|folder| tree_sort_key(folder));
    sorted
}

/// Sanitize a map name for use as a filename.
pub fn sanitize_map_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Get a color for the preview square based on file extension.
pub fn extension_color(ext: &str) -> egui::Color32 {
    match ext {
        "png" => egui::Color32::from_rgb(80, 140, 200), // Blue
        "jpg" | "jpeg" => egui::Color32::from_rgb(200, 140, 80), // Orange
        "webp" => egui::Color32::from_rgb(140, 200, 80), // Green
        "gif" => egui::Color32::from_rgb(200, 80, 140), // Pink
        "bmp" => egui::Color32::from_rgb(140, 80, 200), // Purple
        "tiff" | "tif" => egui::Color32::from_rgb(80, 200, 140), // Teal
        _ => egui::Color32::from_rgb(128, 128, 128),    // Gray
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_maps_directory_includes_subfolders_and_skips_hidden() {
        let dir = tempfile::tempdir().unwrap();
        let maps = dir.path();
        std::fs::create_dir_all(maps.join("Campaign")).unwrap();
        std::fs::create_dir_all(maps.join(".stversions")).unwrap();
        for file in [
            "town.json",
            "Campaign/goblin cave.json",
            "notes.txt",
            ".rustforged-abc.tmp",
            ".stversions/town~20260101.json",
        ] {
            std::fs::write(maps.join(file), "{}").unwrap();
        }

        let names: Vec<String> = scan_maps_directory(maps)
            .into_iter()
            .map(|(name, _)| name)
            .collect();

        assert_eq!(names, ["Campaign/goblin cave", "town"]);
    }

    #[test]
    fn test_conflict_copy_original_recognizes_sync_clients() {
        for (copy, original) in [
            ("cave (DESKTOP's conflicted copy 2026-10-12)", "cave"),
            ("cave (conflicted copy 2026-10-12 140233)", "cave"),
            ("Campaign/cave (Case Conflict)", "Campaign/cave"),
            ("cave_conflict-20261012-140233", "cave"),
            ("cave.sync-conflict-20261012-140233-ABCDEFG", "cave"),
        ] {
            assert_eq!(conflict_copy_original(copy), Some(original), "{copy}");
        }
        // Copies saved on purpose, and ordinary names, aren't conflicts
        for name in ["cave (LAPTOP copy)", "cave (2)", "cave", "goblin (boss)"] {
            assert_eq!(conflict_copy_original(name), None, "{name}");
        }
    }

    fn scanned(names: &[&str]) -> Vec<(String, PathBuf)> {
        let mut maps: Vec<(String, PathBuf)> = names
            .iter()
            .map(|n| (n.to_string(), PathBuf::from(format!("/maps/{n}.json"))))
            .collect();
        maps.sort_by_cached_key(|(name, _)| tree_sort_key(name));
        maps
    }

    /// Rows as "  name" lines, two spaces per level, folders ending in "/"
    fn outline(rows: &[MapTreeRow]) -> Vec<String> {
        rows.iter()
            .map(|row| match row {
                MapTreeRow::Folder { name, depth } => format!("{}{}/", "  ".repeat(*depth), name),
                MapTreeRow::Map {
                    name,
                    depth,
                    conflict_copies,
                    ..
                } => format!(
                    "{}{}{}",
                    "  ".repeat(*depth),
                    name,
                    if conflict_copies.is_empty() {
                        String::new()
                    } else {
                        format!(" +{} copies", conflict_copies.len())
                    }
                ),
            })
            .collect()
    }

    #[test]
    fn test_tree_sort_key_keeps_folder_contents_together() {
        let mut folders = vec!["town/inn", "Town hall", "town", "dungeon"];
        folders.sort_by_cached_key(|f| tree_sort_key(f));
        assert_eq!(folders, ["dungeon", "town", "town/inn", "Town hall"]);
    }

    #[test]
    fn test_indented_adds_two_spaces_per_level() {
        assert_eq!(indented("tokens", 0), "tokens");
        assert_eq!(indented("stone", 2), "    stone");
    }

    #[test]
    fn test_map_tree_nests_maps_under_folder_headers() {
        let rows = map_tree(&scanned(&[
            "town",
            "Campaign/goblin cave",
            "Campaign/Act 2/dragon lair",
            "Campaign/Act 2/throne room",
            "Campaign hub",
            "Side/inn",
        ]));

        assert_eq!(
            outline(&rows),
            [
                "Campaign/",
                "  Act 2/",
                "    dragon lair",
                "    throne room",
                "  goblin cave",
                "Campaign hub",
                "Side/",
                "  inn",
                "town",
            ]
        );
    }

    #[test]
    fn test_map_tree_attaches_conflict_copies_to_their_map() {
        let rows = map_tree(&scanned(&[
            "Campaign/cave",
            "Campaign/cave (conflicted copy 2026-10-12 140233)",
            "lost (conflicted copy 2026-10-12 140233)",
        ]));

        assert_eq!(
            outline(&rows),
            [
                "Campaign/",
                "  cave +1 copies",
                "lost (conflicted copy 2026-10-12 140233)",
            ]
        );
        let MapTreeRow::Map { conflict_copy, .. } = &rows[2] else {
            panic!("expected a map row");
        };
        assert!(
            *conflict_copy,
            "a copy without its original is still marked"
        );
    }

    #[test]
    fn test_scan_maps_directory_missing_folder_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        assert!(scan_maps_directory(&dir.path().join("maps")).is_empty());
    }
}
