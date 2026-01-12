use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::tasks::{IoTaskPool, Task};
use futures_lite::future;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use super::{
    AssetManifest, FogOfWarData, Layer, MapData, PlacedItem, SavedAnnotations, SavedFogOfWar,
    SavedLine, SavedMap, SavedPath, SavedPlacedItem, SavedTextBox,
};
use crate::assets::AssetLibrary;
use crate::config::UpdateLastMapPathRequest;
use crate::editor::{AnnotationMarker, DrawnLine, DrawnPath, TextAnnotation};

#[derive(Message)]
pub struct SaveMapRequest {
    pub path: PathBuf,
}

#[derive(Message)]
pub struct LoadMapRequest {
    pub path: PathBuf,
}

#[derive(Message)]
pub struct NewMapRequest;

/// Message to request switching to a different open map
#[derive(Message)]
#[allow(dead_code)] // Reserved for future map switching feature
pub struct SwitchMapRequest {
    pub map_id: u64,
}

#[derive(Resource, Default)]
pub struct MapLoadError {
    pub message: Option<String>,
}

/// Resource tracking save operation errors for display to user.
#[derive(Resource, Default)]
pub struct MapSaveError {
    pub message: Option<String>,
}

/// Resource for pre-save validation warnings about missing assets.
#[derive(Resource, Default)]
pub struct SaveValidationWarning {
    /// Whether to show the warning dialog
    pub show: bool,
    /// List of asset paths that are missing
    pub missing_assets: Vec<String>,
    /// The path we want to save to after user confirmation
    pub pending_save_path: Option<PathBuf>,
}

/// Resource for load-time validation warnings about missing assets.
#[derive(Resource, Default)]
pub struct LoadValidationWarning {
    /// Whether to show the warning dialog
    pub show: bool,
    /// List of asset paths that are missing from the library
    pub missing_assets: Vec<String>,
    /// The map file that failed to load
    pub map_path: Option<PathBuf>,
}

/// Resource tracking async map I/O operations for modal dialog
#[derive(Resource, Default)]
pub struct AsyncMapOperation {
    /// Whether a save operation is in progress
    pub is_saving: bool,
    /// Whether a load operation is in progress
    pub is_loading: bool,
    /// Description of the current operation
    pub operation_description: Option<String>,
}

impl AsyncMapOperation {
    pub fn is_busy(&self) -> bool {
        self.is_saving || self.is_loading
    }
}

/// Result of an async save operation
pub struct SaveResult {
    pub path: PathBuf,
    pub success: bool,
    pub error: Option<String>,
}

/// Component for save task
#[derive(Component)]
pub struct SaveMapTask(pub Task<SaveResult>);

/// Result of an async load operation
pub struct LoadResult {
    pub path: PathBuf,
    pub saved_map: Option<SavedMap>,
    pub error: Option<String>,
}

/// Component for load task
#[derive(Component)]
pub struct LoadMapTask(pub Task<LoadResult>);

/// Resource tracking the currently loaded map file path
#[derive(Resource, Default)]
pub struct CurrentMapFile {
    pub path: Option<PathBuf>,
}

/// Resource tracking if the current map has unsaved changes
#[derive(Resource, Default)]
pub struct MapDirtyState {
    pub is_dirty: bool,
    /// Count of entities when map was last saved/loaded (for change detection)
    pub last_known_item_count: usize,
    pub last_known_annotation_count: usize,
}

/// Represents a map that's open in memory
#[derive(Clone)]
pub struct OpenMap {
    pub id: u64,
    pub name: String,
    pub path: Option<PathBuf>,
    pub is_dirty: bool,
    pub saved_state: Option<SavedMap>,
}

/// Resource tracking all open maps
#[derive(Resource)]
pub struct OpenMaps {
    pub maps: HashMap<u64, OpenMap>,
    pub active_map_id: Option<u64>,
    pub next_id: u64,
}

impl Default for OpenMaps {
    fn default() -> Self {
        // Start with one untitled map
        let mut maps = HashMap::new();
        maps.insert(
            0,
            OpenMap {
                id: 0,
                name: "Untitled Map".to_string(),
                path: None,
                is_dirty: false,
                saved_state: None,
            },
        );
        Self {
            maps,
            active_map_id: Some(0),
            next_id: 1,
        }
    }
}

impl OpenMaps {
    /// Get the currently active map
    #[allow(dead_code)]
    pub fn active_map(&self) -> Option<&OpenMap> {
        self.active_map_id.and_then(|id| self.maps.get(&id))
    }

    /// Get the currently active map mutably
    pub fn active_map_mut(&mut self) -> Option<&mut OpenMap> {
        self.active_map_id.and_then(|id| self.maps.get_mut(&id))
    }

    /// Check if any open map has unsaved changes
    #[allow(dead_code)]
    pub fn has_any_unsaved(&self) -> bool {
        self.maps.values().any(|m| m.is_dirty)
    }

    /// Get list of maps with unsaved changes
    #[allow(dead_code)]
    pub fn unsaved_maps(&self) -> Vec<&OpenMap> {
        self.maps.values().filter(|m| m.is_dirty).collect()
    }
}

/// UI state for unsaved changes confirmation dialogs
#[derive(Resource, Default)]
pub struct UnsavedChangesDialog {
    /// Show dialog for switching maps
    #[allow(dead_code)]
    pub show_switch_confirmation: bool,
    /// The map ID we want to switch to
    #[allow(dead_code)]
    pub pending_switch_id: Option<u64>,
    /// Show dialog for closing app
    pub show_close_confirmation: bool,
    /// Show dialog for loading a new map
    #[allow(dead_code)]
    pub show_load_confirmation: bool,
    /// Path to load after confirmation
    #[allow(dead_code)]
    pub pending_load_path: Option<PathBuf>,
}

/// Starts an async save operation
#[allow(clippy::too_many_arguments)]
pub fn save_map_system(
    mut commands: Commands,
    mut events: MessageReader<SaveMapRequest>,
    map_data: Res<MapData>,
    fog_data: Res<FogOfWarData>,
    placed_items: Query<(&PlacedItem, &Transform)>,
    paths: Query<&DrawnPath>,
    lines: Query<&DrawnLine>,
    texts: Query<(&Transform, &TextAnnotation)>,
    mut async_op: ResMut<AsyncMapOperation>,
) {
    for event in events.read() {
        // Don't start a new save if one is already in progress
        if async_op.is_busy() {
            warn!("Save operation already in progress");
            continue;
        }

        let items: Vec<SavedPlacedItem> = placed_items
            .iter()
            .map(|(item, transform)| SavedPlacedItem::from_entity(item, transform))
            .collect();

        // Collect annotations
        let saved_paths: Vec<SavedPath> = paths
            .iter()
            .map(|p| SavedPath {
                points: p.points.clone(),
                color: color_to_array(p.color),
                stroke_width: p.stroke_width,
            })
            .collect();

        let saved_lines: Vec<SavedLine> = lines
            .iter()
            .map(|l| SavedLine {
                start: l.start,
                end: l.end,
                color: color_to_array(l.color),
                stroke_width: l.stroke_width,
            })
            .collect();

        let saved_texts: Vec<SavedTextBox> = texts
            .iter()
            .map(|(transform, t)| SavedTextBox {
                position: transform.translation.truncate(),
                content: t.content.clone(),
                font_size: t.font_size,
                color: color_to_array(t.color),
            })
            .collect();

        // Build asset manifest from placed items
        let asset_manifest = AssetManifest::from_items(items.iter());

        let saved_map = SavedMap {
            asset_manifest,
            map_data: map_data.clone(),
            placed_items: items,
            annotations: SavedAnnotations {
                paths: saved_paths,
                lines: saved_lines,
                text_boxes: saved_texts,
            },
            fog_of_war: SavedFogOfWar::from(&*fog_data),
        };

        let path = event.path.clone();
        let map_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("map")
            .to_string();

        // Mark as saving
        async_op.is_saving = true;
        async_op.operation_description = Some(format!("Saving {}...", map_name));

        // Spawn async task for file I/O
        let task_pool = IoTaskPool::get();
        let task = task_pool.spawn(async move {
            // Serialize (could be slow for large maps)
            match serde_json::to_string_pretty(&saved_map) {
                Ok(json) => {
                    // Write to file
                    if let Err(e) = std::fs::write(&path, json) {
                        SaveResult {
                            path,
                            success: false,
                            error: Some(format!("Failed to write file: {}", e)),
                        }
                    } else {
                        SaveResult {
                            path,
                            success: true,
                            error: None,
                        }
                    }
                }
                Err(e) => SaveResult {
                    path,
                    success: false,
                    error: Some(format!("Failed to serialize map: {}", e)),
                },
            }
        });

        commands.spawn(SaveMapTask(task));
    }
}

/// Polls save tasks and handles completion
#[allow(clippy::too_many_arguments)]
pub fn poll_save_tasks(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut SaveMapTask)>,
    mut async_op: ResMut<AsyncMapOperation>,
    mut current_map_file: ResMut<CurrentMapFile>,
    mut config_events: MessageWriter<UpdateLastMapPathRequest>,
    mut dirty_state: ResMut<MapDirtyState>,
    mut open_maps: ResMut<OpenMaps>,
    mut save_error: ResMut<MapSaveError>,
    placed_items: Query<Entity, With<PlacedItem>>,
    annotations: Query<Entity, With<AnnotationMarker>>,
) {
    for (entity, mut task) in tasks.iter_mut() {
        if let Some(result) = future::block_on(future::poll_once(&mut task.0)) {
            // Clear async state
            async_op.is_saving = false;
            async_op.operation_description = None;

            if result.success {
                info!("Map saved to {:?}", result.path);

                // Clear any previous save error
                save_error.message = None;

                // Update current map file and config
                current_map_file.path = Some(result.path.clone());
                config_events.write(UpdateLastMapPathRequest {
                    path: result.path.clone(),
                });

                // Clear dirty state
                dirty_state.is_dirty = false;
                dirty_state.last_known_item_count = placed_items.iter().count();
                dirty_state.last_known_annotation_count = annotations.iter().count();

                // Update open maps
                if let Some(active_map) = open_maps.active_map_mut() {
                    active_map.is_dirty = false;
                    active_map.path = Some(result.path.clone());
                    active_map.name = result
                        .path
                        .file_stem()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Unknown")
                        .to_string();
                }
            } else if let Some(error) = result.error {
                error!("{}", error);
                // Store error for display to user
                save_error.message = Some(error);
            }

            commands.entity(entity).despawn();
        }
    }
}

fn color_to_array(color: Color) -> [f32; 4] {
    let srgba = color.to_srgba();
    [srgba.red, srgba.green, srgba.blue, srgba.alpha]
}

fn array_to_color(arr: [f32; 4]) -> Color {
    Color::srgba(arr[0], arr[1], arr[2], arr[3])
}

/// Starts an async load operation (file I/O only)
pub fn load_map_system(
    mut commands: Commands,
    mut events: MessageReader<LoadMapRequest>,
    mut async_op: ResMut<AsyncMapOperation>,
) {
    for event in events.read() {
        // Don't start a new load if one is already in progress
        if async_op.is_busy() {
            warn!("Load operation already in progress");
            continue;
        }

        let path = event.path.clone();
        let map_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("map")
            .to_string();

        // Mark as loading
        async_op.is_loading = true;
        async_op.operation_description = Some(format!("Loading {}...", map_name));

        // Spawn async task for file I/O and parsing
        let task_pool = IoTaskPool::get();
        let task = task_pool.spawn(async move {
            // Read file
            let json = match std::fs::read_to_string(&path) {
                Ok(content) => content,
                Err(e) => {
                    return LoadResult {
                        path,
                        saved_map: None,
                        error: Some(format!("Failed to read file: {}", e)),
                    };
                }
            };

            // Parse JSON
            match serde_json::from_str::<SavedMap>(&json) {
                Ok(saved_map) => LoadResult {
                    path,
                    saved_map: Some(saved_map),
                    error: None,
                },
                Err(e) => LoadResult {
                    path,
                    saved_map: None,
                    error: Some(format!("Failed to parse map file: {}", e)),
                },
            }
        });

        commands.spawn(LoadMapTask(task));
    }
}

/// Polls load tasks and handles completion (spawns entities synchronously)
#[allow(clippy::too_many_arguments)]
pub fn poll_load_tasks(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut LoadMapTask)>,
    mut async_op: ResMut<AsyncMapOperation>,
    mut map_data: ResMut<MapData>,
    mut fog_data: ResMut<FogOfWarData>,
    mut load_error: ResMut<MapLoadError>,
    mut load_warning: ResMut<LoadValidationWarning>,
    asset_library: Res<AssetLibrary>,
    asset_server: Res<AssetServer>,
    existing_items: Query<Entity, With<PlacedItem>>,
    existing_annotations: Query<Entity, With<AnnotationMarker>>,
    mut current_map_file: ResMut<CurrentMapFile>,
    mut config_events: MessageWriter<UpdateLastMapPathRequest>,
    mut dirty_state: ResMut<MapDirtyState>,
    mut open_maps: ResMut<OpenMaps>,
) {
    for (entity, mut task) in tasks.iter_mut() {
        if let Some(result) = future::block_on(future::poll_once(&mut task.0)) {
            // Clear async state
            async_op.is_loading = false;
            async_op.operation_description = None;
            load_error.message = None;

            // Handle error
            if let Some(error) = result.error {
                load_error.message = Some(error.clone());
                error!("{}", error);
                commands.entity(entity).despawn();
                continue;
            }

            let Some(saved_map) = result.saved_map else {
                commands.entity(entity).despawn();
                continue;
            };

            // Validate all assets exist using the manifest for efficient lookup
            let available_assets: HashSet<&str> = asset_library
                .assets
                .iter()
                .map(|a| a.relative_path.as_str())
                .collect();

            let missing_assets: Vec<String> = saved_map
                .asset_manifest
                .assets
                .iter()
                .filter(|path| !available_assets.contains(path.as_str()))
                .cloned()
                .collect();

            if !missing_assets.is_empty() {
                // Show warning dialog with missing assets
                load_warning.show = true;
                load_warning.missing_assets = missing_assets.clone();
                load_warning.map_path = Some(result.path.clone());

                warn!(
                    "Cannot load map {:?}: {} missing assets",
                    result.path,
                    missing_assets.len()
                );
                commands.entity(entity).despawn();
                continue;
            }

            // Clear existing items
            for existing in existing_items.iter() {
                commands.entity(existing).despawn();
            }

            // Clear existing annotations
            for existing in existing_annotations.iter() {
                commands.entity(existing).despawn();
            }

            // Load map data
            *map_data = saved_map.map_data;

            // Load fog of war data
            *fog_data = FogOfWarData::from(saved_map.fog_of_war);

            // Spawn placed items
            for item in saved_map.placed_items {
                let texture: Handle<Image> = asset_server.load(&item.asset_path);
                // Clamp z_index to valid range (for migration from old maps with larger ranges)
                let z_index = item.z_index.clamp(0, Layer::max_z_index());
                let z = item.layer.z_base() + z_index as f32;

                // Items on non-player-visible layers (GM, FogOfWar) go to render layer 1 (editor-only)
                let render_layer = if item.layer.is_player_visible() {
                    RenderLayers::layer(0)
                } else {
                    RenderLayers::layer(1)
                };

                commands.spawn((
                    Sprite::from_image(texture),
                    Transform {
                        translation: item.position.extend(z),
                        rotation: Quat::from_rotation_z(item.rotation),
                        scale: item.scale.extend(1.0),
                    },
                    PlacedItem {
                        asset_path: item.asset_path,
                        layer: item.layer,
                        z_index,
                    },
                    render_layer,
                ));
            }

            // Spawn annotations
            let z = Layer::Annotation.z_base();

            for path in saved_map.annotations.paths {
                commands.spawn((
                    Transform::from_translation(Vec3::new(0.0, 0.0, z)),
                    DrawnPath {
                        points: path.points,
                        color: array_to_color(path.color),
                        stroke_width: path.stroke_width,
                    },
                    AnnotationMarker,
                ));
            }

            for line in saved_map.annotations.lines {
                commands.spawn((
                    Transform::from_translation(Vec3::new(0.0, 0.0, z)),
                    DrawnLine {
                        start: line.start,
                        end: line.end,
                        color: array_to_color(line.color),
                        stroke_width: line.stroke_width,
                    },
                    AnnotationMarker,
                ));
            }

            for text in saved_map.annotations.text_boxes {
                commands.spawn((
                    Transform::from_translation(text.position.extend(z)),
                    TextAnnotation {
                        content: text.content,
                        font_size: text.font_size,
                        color: array_to_color(text.color),
                    },
                    AnnotationMarker,
                ));
            }

            info!("Map loaded from {:?}", result.path);

            // Update current map file and config
            current_map_file.path = Some(result.path.clone());
            config_events.write(UpdateLastMapPathRequest {
                path: result.path.clone(),
            });

            // Clear dirty state (freshly loaded map is clean)
            dirty_state.is_dirty = false;
            dirty_state.last_known_item_count = 0; // Will be updated by detection system
            dirty_state.last_known_annotation_count = 0;

            // Update open maps - create new entry for this map
            let map_name = result
                .path
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string();

            let new_id = open_maps.next_id;
            open_maps.next_id += 1;

            open_maps.maps.insert(
                new_id,
                OpenMap {
                    id: new_id,
                    name: map_name,
                    path: Some(result.path.clone()),
                    is_dirty: false,
                    saved_state: None,
                },
            );
            open_maps.active_map_id = Some(new_id);

            commands.entity(entity).despawn();
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn new_map_system(
    mut commands: Commands,
    mut events: MessageReader<NewMapRequest>,
    mut map_data: ResMut<MapData>,
    mut fog_data: ResMut<FogOfWarData>,
    existing_items: Query<Entity, With<PlacedItem>>,
    existing_annotations: Query<Entity, With<AnnotationMarker>>,
    mut current_map_file: ResMut<CurrentMapFile>,
    mut dirty_state: ResMut<MapDirtyState>,
    mut open_maps: ResMut<OpenMaps>,
) {
    for _ in events.read() {
        // Clear existing items
        for entity in existing_items.iter() {
            commands.entity(entity).despawn();
        }

        // Clear existing annotations
        for entity in existing_annotations.iter() {
            commands.entity(entity).despawn();
        }

        // Reset map data to default
        *map_data = MapData::default();

        // Reset fog of war to default (empty = fully fogged)
        *fog_data = FogOfWarData::default();

        // Clear current map file (new map has no file yet)
        current_map_file.path = None;

        // Clear dirty state
        dirty_state.is_dirty = false;
        dirty_state.last_known_item_count = 0;
        dirty_state.last_known_annotation_count = 0;

        // Create new entry in open maps
        let new_id = open_maps.next_id;
        open_maps.next_id += 1;

        open_maps.maps.insert(
            new_id,
            OpenMap {
                id: new_id,
                name: "Untitled Map".to_string(),
                path: None,
                is_dirty: false,
                saved_state: None,
            },
        );
        open_maps.active_map_id = Some(new_id);

        info!("Created new map");
    }
}

pub fn ensure_maps_directory() {
    let maps_dir = PathBuf::from("assets/maps");
    if !maps_dir.exists()
        && let Err(e) = std::fs::create_dir_all(&maps_dir)
    {
        warn!("Failed to create maps directory: {}", e);
    }
}

/// System that detects when items are added to the map
pub fn detect_item_additions(
    mut dirty_state: ResMut<MapDirtyState>,
    mut open_maps: ResMut<OpenMaps>,
    added_items: Query<Entity, Added<PlacedItem>>,
    added_annotations: Query<Entity, Added<AnnotationMarker>>,
) {
    // Only run if something was added
    if added_items.is_empty() && added_annotations.is_empty() {
        return;
    }

    dirty_state.is_dirty = true;
    if let Some(active_map) = open_maps.active_map_mut() {
        active_map.is_dirty = true;
    }
}

/// System that detects when items are removed from the map
pub fn detect_item_removals(
    mut dirty_state: ResMut<MapDirtyState>,
    mut open_maps: ResMut<OpenMaps>,
    mut removed_items: RemovedComponents<PlacedItem>,
    mut removed_annotations: RemovedComponents<AnnotationMarker>,
) {
    // Only run if something was removed
    if removed_items.read().next().is_none() && removed_annotations.read().next().is_none() {
        return;
    }

    dirty_state.is_dirty = true;
    if let Some(active_map) = open_maps.active_map_mut() {
        active_map.is_dirty = true;
    }
}

/// System that detects when items are transformed (moved, rotated, scaled)
pub fn detect_item_transforms(
    mut dirty_state: ResMut<MapDirtyState>,
    mut open_maps: ResMut<OpenMaps>,
    changed_items: Query<Entity, (Changed<Transform>, With<PlacedItem>)>,
) {
    // Only run if transforms changed
    if changed_items.is_empty() {
        return;
    }

    dirty_state.is_dirty = true;
    if let Some(active_map) = open_maps.active_map_mut() {
        active_map.is_dirty = true;
    }
}

/// Helper to capture current map state as a SavedMap
fn capture_current_map_state(
    map_data: &MapData,
    fog_data: &FogOfWarData,
    placed_items: &Query<(&PlacedItem, &Transform)>,
    paths: &Query<&DrawnPath>,
    lines: &Query<&DrawnLine>,
    texts: &Query<(&Transform, &TextAnnotation)>,
) -> SavedMap {
    let items: Vec<SavedPlacedItem> = placed_items
        .iter()
        .map(|(item, transform)| SavedPlacedItem::from_entity(item, transform))
        .collect();

    let saved_paths: Vec<SavedPath> = paths
        .iter()
        .map(|p| SavedPath {
            points: p.points.clone(),
            color: color_to_array(p.color),
            stroke_width: p.stroke_width,
        })
        .collect();

    let saved_lines: Vec<SavedLine> = lines
        .iter()
        .map(|l| SavedLine {
            start: l.start,
            end: l.end,
            color: color_to_array(l.color),
            stroke_width: l.stroke_width,
        })
        .collect();

    let saved_texts: Vec<SavedTextBox> = texts
        .iter()
        .map(|(transform, t)| SavedTextBox {
            position: transform.translation.truncate(),
            content: t.content.clone(),
            font_size: t.font_size,
            color: color_to_array(t.color),
        })
        .collect();

    let asset_manifest = AssetManifest::from_items(items.iter());

    SavedMap {
        asset_manifest,
        map_data: map_data.clone(),
        placed_items: items,
        annotations: SavedAnnotations {
            paths: saved_paths,
            lines: saved_lines,
            text_boxes: saved_texts,
        },
        fog_of_war: SavedFogOfWar::from(fog_data),
    }
}

/// System to handle switching between open maps
#[allow(clippy::too_many_arguments)]
pub fn switch_map_system(
    mut commands: Commands,
    mut events: MessageReader<SwitchMapRequest>,
    mut map_data: ResMut<MapData>,
    mut fog_data: ResMut<FogOfWarData>,
    mut open_maps: ResMut<OpenMaps>,
    mut current_map_file: ResMut<CurrentMapFile>,
    mut dirty_state: ResMut<MapDirtyState>,
    asset_server: Res<AssetServer>,
    placed_items_query: Query<(&PlacedItem, &Transform)>,
    existing_items: Query<Entity, With<PlacedItem>>,
    existing_annotations: Query<Entity, With<AnnotationMarker>>,
    paths: Query<&DrawnPath>,
    lines: Query<&DrawnLine>,
    texts: Query<(&Transform, &TextAnnotation)>,
) {
    for event in events.read() {
        let target_id = event.map_id;

        // Don't switch to the already active map
        if open_maps.active_map_id == Some(target_id) {
            continue;
        }

        // First, save the current map state
        if let Some(current_id) = open_maps.active_map_id {
            let current_state = capture_current_map_state(
                &map_data,
                &fog_data,
                &placed_items_query,
                &paths,
                &lines,
                &texts,
            );
            let current_dirty = dirty_state.is_dirty;

            if let Some(current_map) = open_maps.maps.get_mut(&current_id) {
                current_map.saved_state = Some(current_state);
                current_map.is_dirty = current_dirty;
            }
        }

        // Now load the target map
        if let Some(target_map) = open_maps.maps.get(&target_id).cloned() {
            // Clear existing items
            for entity in existing_items.iter() {
                commands.entity(entity).despawn();
            }

            // Clear existing annotations
            for entity in existing_annotations.iter() {
                commands.entity(entity).despawn();
            }

            // Load target map state
            if let Some(saved_state) = &target_map.saved_state {
                // Restore map data
                *map_data = saved_state.map_data.clone();

                // Restore fog of war data
                *fog_data = FogOfWarData::from(saved_state.fog_of_war.clone());

                // Spawn placed items
                for item in &saved_state.placed_items {
                    let texture: Handle<Image> = asset_server.load(&item.asset_path);
                    let z_index = item.z_index.clamp(0, Layer::max_z_index());
                    let z = item.layer.z_base() + z_index as f32;

                    let render_layer = if item.layer.is_player_visible() {
                        RenderLayers::layer(0)
                    } else {
                        RenderLayers::layer(1)
                    };

                    commands.spawn((
                        Sprite::from_image(texture),
                        Transform {
                            translation: item.position.extend(z),
                            rotation: Quat::from_rotation_z(item.rotation),
                            scale: item.scale.extend(1.0),
                        },
                        PlacedItem {
                            asset_path: item.asset_path.clone(),
                            layer: item.layer,
                            z_index,
                        },
                        render_layer,
                    ));
                }

                // Spawn annotations
                let z = Layer::Annotation.z_base();

                for path in &saved_state.annotations.paths {
                    commands.spawn((
                        Transform::from_translation(Vec3::new(0.0, 0.0, z)),
                        DrawnPath {
                            points: path.points.clone(),
                            color: array_to_color(path.color),
                            stroke_width: path.stroke_width,
                        },
                        AnnotationMarker,
                    ));
                }

                for line in &saved_state.annotations.lines {
                    commands.spawn((
                        Transform::from_translation(Vec3::new(0.0, 0.0, z)),
                        DrawnLine {
                            start: line.start,
                            end: line.end,
                            color: array_to_color(line.color),
                            stroke_width: line.stroke_width,
                        },
                        AnnotationMarker,
                    ));
                }

                for text in &saved_state.annotations.text_boxes {
                    commands.spawn((
                        Transform::from_translation(text.position.extend(z)),
                        TextAnnotation {
                            content: text.content.clone(),
                            font_size: text.font_size,
                            color: array_to_color(text.color),
                        },
                        AnnotationMarker,
                    ));
                }
            } else {
                // No saved state, start with empty/default map
                *map_data = MapData::default();
                map_data.name = target_map.name.clone();
                // Reset fog of war to default (empty = fully fogged)
                *fog_data = FogOfWarData::default();
            }

            // Update current map file
            current_map_file.path = target_map.path.clone();

            // Update dirty state
            dirty_state.is_dirty = target_map.is_dirty;
            dirty_state.last_known_item_count = 0; // Will be updated by detect_map_changes
            dirty_state.last_known_annotation_count = 0;

            // Update active map
            open_maps.active_map_id = Some(target_id);

            info!("Switched to map: {}", target_map.name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
