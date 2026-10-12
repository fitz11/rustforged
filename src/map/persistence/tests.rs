//! Unit tests for the persistence module.

use bevy::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::helpers::{array_to_color, color_to_array};
use super::load::read_map;
use super::resources::MapLoadError;
use super::resources::SaveConflictKind;
use super::save::{check_save_conflict, fingerprint_assets, write_map};
use crate::assets::fingerprint_test_support::stamp_of;
use crate::assets::{AssetFingerprint, FingerprintCache, LibraryIndex};
use crate::map::{
    Layer, MAP_FORMAT_VERSION, MapData, SavedAnnotations, SavedFogOfWar, SavedMap, SavedPlacedItem,
};

// color_to_array tests
#[test]
fn test_color_to_array_red() {
    let color = Color::srgba(1.0, 0.0, 0.0, 1.0);
    let arr = color_to_array(color);
    assert_eq!(arr, [1.0, 0.0, 0.0, 1.0]);
}

#[test]
fn test_color_to_array_green() {
    let color = Color::srgba(0.0, 1.0, 0.0, 1.0);
    let arr = color_to_array(color);
    assert_eq!(arr, [0.0, 1.0, 0.0, 1.0]);
}

#[test]
fn test_color_to_array_blue() {
    let color = Color::srgba(0.0, 0.0, 1.0, 1.0);
    let arr = color_to_array(color);
    assert_eq!(arr, [0.0, 0.0, 1.0, 1.0]);
}

#[test]
fn test_color_to_array_with_alpha() {
    let color = Color::srgba(0.5, 0.5, 0.5, 0.5);
    let arr = color_to_array(color);
    assert!((arr[0] - 0.5).abs() < 0.001);
    assert!((arr[1] - 0.5).abs() < 0.001);
    assert!((arr[2] - 0.5).abs() < 0.001);
    assert!((arr[3] - 0.5).abs() < 0.001);
}

#[test]
fn test_color_to_array_white() {
    let color = Color::srgba(1.0, 1.0, 1.0, 1.0);
    let arr = color_to_array(color);
    assert_eq!(arr, [1.0, 1.0, 1.0, 1.0]);
}

#[test]
fn test_color_to_array_black() {
    let color = Color::srgba(0.0, 0.0, 0.0, 1.0);
    let arr = color_to_array(color);
    assert_eq!(arr, [0.0, 0.0, 0.0, 1.0]);
}

// array_to_color tests
#[test]
fn test_array_to_color_red() {
    let arr = [1.0, 0.0, 0.0, 1.0];
    let color = array_to_color(arr);
    let srgba = color.to_srgba();
    assert_eq!(srgba.red, 1.0);
    assert_eq!(srgba.green, 0.0);
    assert_eq!(srgba.blue, 0.0);
    assert_eq!(srgba.alpha, 1.0);
}

#[test]
fn test_array_to_color_partial_alpha() {
    let arr = [0.25, 0.5, 0.75, 0.5];
    let color = array_to_color(arr);
    let srgba = color.to_srgba();
    assert!((srgba.red - 0.25).abs() < 0.001);
    assert!((srgba.green - 0.5).abs() < 0.001);
    assert!((srgba.blue - 0.75).abs() < 0.001);
    assert!((srgba.alpha - 0.5).abs() < 0.001);
}

// Round-trip tests
#[test]
fn test_color_roundtrip() {
    let original = Color::srgba(0.2, 0.4, 0.6, 0.8);
    let arr = color_to_array(original);
    let recovered = array_to_color(arr);
    let original_srgba = original.to_srgba();
    let recovered_srgba = recovered.to_srgba();

    assert!((original_srgba.red - recovered_srgba.red).abs() < 0.001);
    assert!((original_srgba.green - recovered_srgba.green).abs() < 0.001);
    assert!((original_srgba.blue - recovered_srgba.blue).abs() < 0.001);
    assert!((original_srgba.alpha - recovered_srgba.alpha).abs() < 0.001);
}

#[test]
fn test_color_roundtrip_multiple() {
    let colors = [
        Color::srgba(1.0, 0.0, 0.0, 1.0),
        Color::srgba(0.0, 1.0, 0.0, 1.0),
        Color::srgba(0.0, 0.0, 1.0, 1.0),
        Color::srgba(1.0, 1.0, 1.0, 1.0),
        Color::srgba(0.0, 0.0, 0.0, 1.0),
        Color::srgba(0.5, 0.5, 0.5, 0.5),
        Color::srgba(0.1, 0.2, 0.3, 0.4),
    ];

    for original in colors {
        let arr = color_to_array(original);
        let recovered = array_to_color(arr);
        let original_srgba = original.to_srgba();
        let recovered_srgba = recovered.to_srgba();

        assert!(
            (original_srgba.red - recovered_srgba.red).abs() < 0.001,
            "Red mismatch for {:?}",
            original
        );
        assert!(
            (original_srgba.green - recovered_srgba.green).abs() < 0.001,
            "Green mismatch for {:?}",
            original
        );
        assert!(
            (original_srgba.blue - recovered_srgba.blue).abs() < 0.001,
            "Blue mismatch for {:?}",
            original
        );
        assert!(
            (original_srgba.alpha - recovered_srgba.alpha).abs() < 0.001,
            "Alpha mismatch for {:?}",
            original
        );
    }
}

// MapLoadError tests
#[test]
fn test_map_load_error_default() {
    let error = MapLoadError::default();
    assert!(error.message.is_none());
}

// Asset reference resolution on load / fingerprinting on save

/// Writes `files` (relative path, contents) under `root` and indexes them as a library.
fn library_index(root: &Path, files: &[(&str, &str)]) -> LibraryIndex {
    let mut assets = Vec::new();
    for (path, contents) in files {
        let full = root.join(path);
        std::fs::create_dir_all(full.parent().unwrap()).unwrap();
        std::fs::write(&full, contents).unwrap();
        assets.push((path.to_string(), stamp_of(&full)));
    }
    LibraryIndex::new(root.to_path_buf(), assets, FingerprintCache::default())
}

fn fingerprint_of(contents: &str) -> AssetFingerprint {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("f"), contents).unwrap();
    AssetFingerprint::of_file(&dir.path().join("f")).unwrap()
}

fn item(asset_path: &str) -> SavedPlacedItem {
    SavedPlacedItem {
        asset_path: asset_path.to_string(),
        position: Vec2::ZERO,
        rotation: 0.0,
        scale: Vec2::ONE,
        layer: Layer::Token,
        z_index: 0,
    }
}

fn map_with(items: Vec<SavedPlacedItem>, fingerprints: &[(&str, &str)]) -> SavedMap {
    SavedMap {
        format_version: MAP_FORMAT_VERSION,
        map_id: None,
        saved_by: None,
        asset_fingerprints: fingerprints
            .iter()
            .map(|(path, contents)| (path.to_string(), fingerprint_of(contents)))
            .collect(),
        map_data: MapData::default(),
        placed_items: items,
        annotations: SavedAnnotations::default(),
        fog_of_war: SavedFogOfWar::default(),
    }
}

#[test]
fn test_read_map_relinks_moved_assets_and_reports_missing() {
    let library = tempfile::tempdir().unwrap();
    let index = library_index(
        library.path(),
        &[
            ("tokens/orc-chief.png", "orc"),
            ("tokens/goblin.png", "gob"),
        ],
    );
    let map = map_with(
        vec![
            item("tokens/orc.png"),
            item("tokens/orc.png"),
            item("tokens/gone.png"),
            item("tokens/goblin.png"),
        ],
        &[("tokens/orc.png", "orc"), ("tokens/gone.png", "gone")],
    );
    let map_path = library.path().join("maps/cave.json");
    std::fs::create_dir_all(map_path.parent().unwrap()).unwrap();
    std::fs::write(&map_path, serde_json::to_string(&map).unwrap()).unwrap();

    let result = read_map(map_path, index);

    assert!(result.error.is_none());
    assert_eq!(
        result.relinked,
        vec![(
            "tokens/orc.png".to_string(),
            "tokens/orc-chief.png".to_string()
        )]
    );
    assert_eq!(result.missing, vec![("tokens/gone.png".to_string(), None)]);
    let loaded = result.saved_map.unwrap();
    let paths: Vec<&str> = loaded
        .placed_items
        .iter()
        .map(|i| i.asset_path.as_str())
        .collect();
    assert_eq!(
        paths,
        [
            "tokens/orc-chief.png",
            "tokens/orc-chief.png",
            "tokens/gone.png",
            "tokens/goblin.png"
        ]
    );
    // Missing references keep their fingerprint so they can still be found later
    assert!(loaded.asset_fingerprints.contains_key("tokens/gone.png"));
    // Fingerprints computed while resolving are handed back for the library cache
    assert!(!result.computed.is_empty());
}

#[test]
fn test_read_map_reports_parse_errors() {
    let dir = tempfile::tempdir().unwrap();
    let map_path = dir.path().join("broken.json");
    std::fs::write(&map_path, "{ not json").unwrap();
    let index = library_index(dir.path(), &[]);

    let result = read_map(map_path, index);

    assert!(result.saved_map.is_none());
    assert!(result.error.unwrap().contains("Failed to parse"));
}

#[test]
fn test_fingerprint_assets_keeps_carried_fingerprint_for_missing_files() {
    let library = tempfile::tempdir().unwrap();
    let mut index = library_index(library.path(), &[("tokens/orc.png", "orc")]);
    let carried: BTreeMap<String, AssetFingerprint> = [
        ("tokens/gone.png".to_string(), fingerprint_of("gone")),
        ("tokens/orc.png".to_string(), fingerprint_of("stale orc")),
    ]
    .into_iter()
    .collect();
    let used: BTreeSet<String> = ["tokens/orc.png", "tokens/gone.png", "tokens/never.png"]
        .iter()
        .map(|s| s.to_string())
        .collect();

    let fingerprints = fingerprint_assets(used, &mut index, &carried);

    // Files on disk are fingerprinted fresh; missing ones fall back to what was loaded
    assert_eq!(fingerprints["tokens/orc.png"], fingerprint_of("orc"));
    assert_eq!(fingerprints["tokens/gone.png"], fingerprint_of("gone"));
    assert!(!fingerprints.contains_key("tokens/never.png"));
}

// Deterministic, skip-if-unchanged map writes

fn item_at(asset_path: &str, x: f32) -> SavedPlacedItem {
    SavedPlacedItem {
        position: Vec2::new(x, 0.0),
        ..item(asset_path)
    }
}

#[test]
fn test_write_map_output_does_not_depend_on_set_order() {
    let dir = tempfile::tempdir().unwrap();
    let items = || vec![item_at("a.png", 0.0), item_at("b.png", 2.0)];
    let mut forward = map_with(items(), &[]);
    let mut backward = map_with(items(), &[]);
    for cell in [(3, 1), (-2, 5), (0, 0)] {
        forward.fog_of_war.revealed_cells.insert(cell);
    }
    for cell in [(0, 0), (3, 1), (-2, 5)] {
        backward.fog_of_war.revealed_cells.insert(cell);
    }

    let first = dir.path().join("first.json");
    let second = dir.path().join("second.json");
    write_map(&first, &mut forward).unwrap();
    write_map(&second, &mut backward).unwrap();

    assert_eq!(
        std::fs::read_to_string(&first).unwrap(),
        std::fs::read_to_string(&second).unwrap()
    );
}

#[test]
fn test_write_map_keeps_stacking_order_within_a_layer() {
    let dir = tempfile::tempdir().unwrap();
    let mut background = item_at("floor.png", 0.0);
    background.layer = Layer::Background;
    // Listed bottom first: b.png was placed on top of a.png
    let mut map = map_with(
        vec![item_at("b.png", 0.0), item_at("a.png", 0.0), background],
        &[],
    );
    let path = dir.path().join("stack.json");
    write_map(&path, &mut map).unwrap();

    let order: Vec<_> = map
        .placed_items
        .iter()
        .map(|i| i.asset_path.as_str())
        .collect();
    // Layers bottom first; within a layer the stacking order stays
    assert_eq!(order, ["floor.png", "b.png", "a.png"]);
}

#[test]
fn test_write_map_creates_maps_folder_and_skips_unchanged_rewrite() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("maps/campaign/cave.json");
    let mut map = map_with(vec![item("a.png")], &[]);

    write_map(&path, &mut map).unwrap();
    let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
    write_map(&path, &mut map).unwrap();

    assert_eq!(
        std::fs::metadata(&path).unwrap().modified().unwrap(),
        modified
    );
}

#[test]
fn test_read_map_rejects_newer_format() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("future.json");
    let mut map = map_with(vec![], &[]);
    map.format_version = MAP_FORMAT_VERSION + 1;
    std::fs::write(&path, serde_json::to_string(&map).unwrap()).unwrap();

    let result = read_map(path, library_index(dir.path(), &[]));

    assert!(result.error.unwrap().contains("newer version"));
}

// Save conflict detection

#[test]
fn test_save_conflict_none_when_disk_matches_loaded_version() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cave.json");
    let hash = write_map(&path, &mut map_with(vec![item("a.png")], &[])).unwrap();

    assert!(check_save_conflict(&path, Some(&hash)).is_none());
}

#[test]
fn test_save_conflict_when_another_machine_saved() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cave.json");
    let loaded_hash = write_map(&path, &mut map_with(vec![item("a.png")], &[])).unwrap();
    let mut theirs = map_with(vec![item("b.png")], &[]);
    theirs.saved_by = Some("DESKTOP".into());
    write_map(&path, &mut theirs).unwrap();

    let conflict = check_save_conflict(&path, Some(&loaded_hash)).unwrap();

    assert_eq!(conflict.kind, SaveConflictKind::Changed);
    assert_eq!(conflict.saved_by.as_deref(), Some("DESKTOP"));
    assert!(conflict.modified.is_some());
}

#[test]
fn test_save_conflict_new_map_onto_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cave.json");
    write_map(&path, &mut map_with(vec![], &[])).unwrap();

    let conflict = check_save_conflict(&path, None).unwrap();

    assert_eq!(conflict.kind, SaveConflictKind::Exists);
}

#[test]
fn test_save_conflict_when_file_removed_elsewhere() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cave.json");
    let hash = write_map(&path, &mut map_with(vec![], &[])).unwrap();
    std::fs::remove_file(&path).unwrap();

    let conflict = check_save_conflict(&path, Some(&hash)).unwrap();

    assert_eq!(conflict.kind, SaveConflictKind::Deleted);
    // A brand-new map saving to a free name is not a conflict
    assert!(check_save_conflict(&path, None).is_none());
}

#[test]
fn test_read_map_hash_matches_written_hash() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("cave.json");
    let written = write_map(&path, &mut map_with(vec![item("a.png")], &[])).unwrap();

    let result = read_map(path, library_index(dir.path(), &[]));

    assert_eq!(result.disk_hash, Some(written));
}

#[test]
fn test_active_map_unsaved_follows_dirty_state() {
    use super::resources::{MapDirtyState, OpenMaps};

    let open_maps = OpenMaps::default();
    let mut dirty = MapDirtyState::default();
    assert!(!open_maps.has_any_unsaved(&dirty));

    // Edits like fog reveals only set MapDirtyState; closing must still see them
    dirty.is_dirty = true;
    assert!(open_maps.has_any_unsaved(&dirty));
    assert_eq!(open_maps.unsaved_maps(&dirty).len(), 1);
}

#[test]
fn test_background_map_unsaved_uses_stashed_flag() {
    use super::resources::{MapDirtyState, OpenMap, OpenMaps};

    let mut open_maps = OpenMaps::default();
    let mut background = open_maps.active_map().unwrap().clone();
    background.id = 7;
    background.stashed_dirty = true;
    open_maps.maps.insert(7, background);

    let dirty = MapDirtyState::default();
    let background: &OpenMap = &open_maps.maps[&7];
    assert!(open_maps.is_unsaved(background, &dirty));
    assert!(open_maps.has_any_unsaved(&dirty));
}

/// Runs poll_save_tasks on a save of `map` that has already finished writing.
fn finish_save(
    map: u64,
    generation: u64,
    setup: impl FnOnce(&mut World),
) -> (App, Vec<super::messages::MapSaveFinished>) {
    use super::messages::MapSaveFinished;
    use super::resources::{
        AsyncMapOperation, CurrentMapFile, MapChangedOnDisk, MapDirtyState, MapSaveError, OpenMaps,
        SaveConflictDialog, SaveMapTask,
    };
    use super::results::SaveResult;
    use bevy::tasks::{IoTaskPool, TaskPool};

    let mut app = App::new();
    app.init_resource::<AsyncMapOperation>()
        .init_resource::<CurrentMapFile>()
        .init_resource::<MapDirtyState>()
        .init_resource::<OpenMaps>()
        .init_resource::<MapSaveError>()
        .init_resource::<crate::assets::AssetLibrary>()
        .init_resource::<SaveConflictDialog>()
        .init_resource::<MapChangedOnDisk>()
        .add_message::<crate::config::UpdateLastMapPathRequest>()
        .add_message::<MapSaveFinished>()
        .add_systems(Update, super::save::poll_save_tasks);
    setup(app.world_mut());

    let task = IoTaskPool::get_or_init(TaskPool::new).spawn(async {
        SaveResult {
            path: std::path::PathBuf::from("maps/cave.json"),
            success: true,
            error: None,
            conflict: None,
            disk_hash: Some("hash".to_string()),
            map_id: "id".to_string(),
            asset_fingerprints: Default::default(),
            computed: Default::default(),
        }
    });
    app.world_mut().spawn(SaveMapTask {
        task,
        map,
        generation,
        ticket: 7,
    });
    for _ in 0..100 {
        app.update();
        if app
            .world_mut()
            .query::<&SaveMapTask>()
            .iter(app.world())
            .next()
            .is_none()
        {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let finished = app
        .world_mut()
        .resource_mut::<Messages<MapSaveFinished>>()
        .drain()
        .collect();
    (app, finished)
}

#[test]
fn test_edits_made_during_a_save_stay_unsaved() {
    use super::resources::MapDirtyState;

    // The save started at edit 1; another edit came in while it was writing
    let (app, finished) = finish_save(0, 1, |world| {
        let mut dirty = world.resource_mut::<MapDirtyState>();
        dirty.mark_dirty();
        dirty.mark_dirty();
    });
    assert!(app.world().resource::<MapDirtyState>().is_dirty);
    assert_eq!(finished.len(), 1);
    assert_eq!(finished[0].ticket, 7);
}

#[test]
fn test_save_result_goes_to_the_map_that_was_saved() {
    use super::resources::{CurrentMapFile, MapDirtyState, OpenMaps};

    // Map 0 was saved, but a new map (id 5) replaced it before the save finished
    let (app, _) = finish_save(0, 0, |world| {
        let mut open_maps = world.resource_mut::<OpenMaps>();
        let mut fresh = open_maps.maps.remove(&0).unwrap();
        fresh.id = 5;
        open_maps.maps.insert(5, fresh);
        open_maps.active_map_id = Some(5);
        world.resource_mut::<MapDirtyState>().mark_dirty();
    });

    let open_maps = app.world().resource::<OpenMaps>();
    assert_eq!(open_maps.maps[&5].path, None);
    assert!(app.world().resource::<CurrentMapFile>().path.is_none());
    assert!(app.world().resource::<MapDirtyState>().is_dirty);
}

fn switch_app() -> App {
    use super::messages::{MapReplaced, SwitchMapRequest};
    use super::resources::{CurrentMapFile, MapDirtyState, OpenMaps};

    let mut app = App::new();
    crate::assets::register_library_source(&mut app);
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_resource::<MapData>()
        .init_resource::<crate::map::FogOfWarData>()
        .init_resource::<OpenMaps>()
        .init_resource::<CurrentMapFile>()
        .init_resource::<MapDirtyState>()
        .init_resource::<crate::session::LiveSessionState>()
        .add_message::<SwitchMapRequest>()
        .add_message::<MapReplaced>()
        .add_systems(Update, super::map_state::switch_map_system);
    app
}

fn item_paths(app: &mut App) -> Vec<String> {
    let mut paths: Vec<String> = app
        .world_mut()
        .query::<&crate::map::PlacedItem>()
        .iter(app.world())
        .map(|i| i.asset_path.clone())
        .collect();
    paths.sort();
    paths
}

#[test]
fn test_switching_scenes_keeps_the_map_left_behind() {
    use super::messages::SwitchMapRequest;
    use super::resources::{MapDirtyState, OpenMaps};

    let mut app = switch_app();
    // Map 0 is being edited, with unsaved changes; map 7 is open in the background
    let assets = app.world().resource::<AssetServer>().clone();
    app.world_mut().spawn(
        crate::map::PlacedItem {
            asset_path: "tokens/orc.png".to_string(),
            layer: Layer::Token,
            z_index: 0,
        }
        .bundle(&assets, Transform::default()),
    );
    app.world_mut().resource_mut::<MapDirtyState>().mark_dirty();
    {
        let mut open_maps = app.world_mut().resource_mut::<OpenMaps>();
        let mut other = open_maps.active_map().unwrap().clone();
        other.id = 7;
        other.name = "Tavern".to_string();
        other.saved_state = Some(map_with(vec![item("props/bar.png")], &[]));
        open_maps.maps.insert(7, other);
    }

    app.world_mut().write_message(SwitchMapRequest {
        map_id: 7,
        close_current: false,
    });
    app.update();
    app.update();

    assert_eq!(item_paths(&mut app), vec!["props/bar.png".to_string()]);
    let open_maps = app.world().resource::<OpenMaps>();
    assert_eq!(open_maps.active_map_id, Some(7));
    let left = &open_maps.maps[&0];
    assert!(left.stashed_dirty);
    assert_eq!(left.saved_state.as_ref().unwrap().placed_items.len(), 1);
    assert!(!app.world().resource::<MapDirtyState>().is_dirty);

    // Back to map 0, closing the tavern
    app.world_mut().write_message(SwitchMapRequest {
        map_id: 0,
        close_current: true,
    });
    app.update();
    app.update();

    assert_eq!(item_paths(&mut app), vec!["tokens/orc.png".to_string()]);
    let open_maps = app.world().resource::<OpenMaps>();
    assert!(!open_maps.maps.contains_key(&7));
    assert!(app.world().resource::<MapDirtyState>().is_dirty);
}
