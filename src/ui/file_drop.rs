//! Files dragged onto the editor window from the desktop.
//!
//! An image is copied into the library (reusing an identical file already there) and placed
//! where it was dropped: a battlemap-sized image on the Background layer, fitted to the
//! grid; anything smaller on the Token layer, fitted to one cell. A map file is opened, the
//! same as Open.

use bevy::prelude::*;
use bevy::tasks::{IoTaskPool, Task};
use bevy::window::{FileDragAndDrop, PrimaryWindow};
use std::path::Path;

use crate::assets::{AssetLibrary, RefreshAssetLibrary};
use crate::common::poll_task;
use crate::editor::guess_battlemap_cells;
use crate::editor::history::{EditorCommand, MapObject, RecordEditorCommand};
use crate::editor::params::CameraParams;
use crate::editor::{CurrentTool, EditorTool};
use crate::map::{Layer, MapData, PlacedItem, Selected, StackTops};

use super::map_actions::MapFileAction;

/// An image counts as a battlemap if it covers at least this many cells across
const BATTLEMAP_MIN_CELLS: u32 = 6;

/// Image files being copied into the library, with where they were dropped
#[derive(Resource, Default)]
pub struct DroppedImages {
    tasks: Vec<(Vec2, Task<Result<ImportedImage, String>>)>,
}

/// An image copied into the library
#[derive(Debug)]
struct ImportedImage {
    /// Library-relative path
    path: String,
    /// Size in pixels
    size: UVec2,
}

const IMAGE_EXTENSIONS: [&str; 8] = ["png", "jpg", "jpeg", "webp", "gif", "bmp", "tiff", "tif"];

fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

/// Handles files dropped on the editor window.
pub fn handle_file_drops(
    mut drops: MessageReader<FileDragAndDrop>,
    primary: Query<Entity, With<PrimaryWindow>>,
    camera: CameraParams,
    library: Res<AssetLibrary>,
    mut dropped: ResMut<DroppedImages>,
    mut file_actions: MessageWriter<MapFileAction>,
) {
    let primary = primary.single().ok();
    for drop in drops.read() {
        let FileDragAndDrop::DroppedFile { window, path_buf } = drop else {
            continue;
        };
        if Some(*window) != primary {
            continue;
        }
        let ext = extension(path_buf);
        if ext == "json" {
            file_actions.write(MapFileAction::Open(path_buf.clone()));
        } else if IMAGE_EXTENSIONS.contains(&ext.as_str()) {
            // Where it was dropped, or the middle of the view if the cursor isn't known
            let at = camera
                .cursor_world_pos()
                .or_else(|| camera.camera_world_pos())
                .unwrap_or(Vec2::ZERO);
            let source = path_buf.clone();
            let root = library.library_path.clone();
            dropped.tasks.push((
                at,
                IoTaskPool::get().spawn(async move { copy_into_library(&source, &root) }),
            ));
        } else {
            info!("Ignoring dropped file {:?}: not an image or map", path_buf);
        }
    }
}

/// Copies `source` to the library root, keeping its name unless a different file already
/// has it (then "name (2).png" and so on). An identical file already there is reused.
fn copy_into_library(source: &Path, root: &Path) -> Result<ImportedImage, String> {
    let (width, height) = image::image_dimensions(source)
        .map_err(|e| format!("Can't read {}: {}", source.display(), e))?;
    let name = source
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("The file has no usable name")?;
    let stem = Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(name);
    let ext = Path::new(name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
    let contents =
        std::fs::read(source).map_err(|e| format!("Can't read {}: {}", source.display(), e))?;

    for n in 1..1000 {
        let file_name = if n == 1 {
            name.to_string()
        } else {
            format!("{stem} ({n}).{ext}")
        };
        let dest = root.join(&file_name);
        match std::fs::File::create_new(&dest) {
            Ok(mut file) => {
                use std::io::Write;
                file.write_all(&contents)
                    .map_err(|e| format!("Can't write {}: {}", dest.display(), e))?;
                return Ok(ImportedImage {
                    path: file_name,
                    size: UVec2::new(width, height),
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                if std::fs::read(&dest).is_ok_and(|existing| existing == contents) {
                    return Ok(ImportedImage {
                        path: file_name,
                        size: UVec2::new(width, height),
                    });
                }
            }
            Err(e) => return Err(format!("Can't write {}: {}", dest.display(), e)),
        }
    }
    Err(format!("Too many files named like {}", name))
}

/// The item for an image dropped at `at`: a battlemap on the Background layer, fitted to
/// the grid with its lower-left corner on a grid line; a smaller image on the Token layer,
/// fitted to the cell it was dropped in.
fn dropped_item(image: &ImportedImage, at: Vec2, grid_size: f32) -> (PlacedItem, Transform) {
    let cells = guess_battlemap_cells(image.size);
    let image_size = image.size.as_vec2().max(Vec2::ONE);
    if cells.max_element() >= BATTLEMAP_MIN_CELLS {
        let size = cells.as_vec2() * grid_size;
        let corner = ((at - size / 2.0) / grid_size).round() * grid_size;
        let item = PlacedItem {
            asset_path: image.path.clone(),
            layer: Layer::Background,
            z_index: 0,
        };
        let transform = Transform::from_translation((corner + size / 2.0).extend(0.0))
            .with_scale((size / image_size).extend(1.0));
        (item, transform)
    } else {
        let scale = (grid_size / image_size.x).min(grid_size / image_size.y);
        let center = crate::editor::snap_to_grid(at, grid_size, true);
        let item = PlacedItem {
            asset_path: image.path.clone(),
            layer: Layer::Token,
            z_index: 0,
        };
        let transform = Transform::from_translation(center.extend(0.0))
            .with_scale(Vec3::new(scale, scale, 1.0));
        (item, transform)
    }
}

/// Places dropped images once they're in the library, each as one undo step, and selects
/// them.
#[allow(clippy::too_many_arguments)]
pub fn place_dropped_images(
    mut commands: Commands,
    mut dropped: ResMut<DroppedImages>,
    asset_server: Res<AssetServer>,
    map_data: Res<MapData>,
    stack: StackTops,
    selected: Query<Entity, With<Selected>>,
    mut current_tool: ResMut<CurrentTool>,
    mut history: MessageWriter<RecordEditorCommand>,
    mut refresh: MessageWriter<RefreshAssetLibrary>,
) {
    let mut finished = Vec::new();
    dropped
        .tasks
        .retain_mut(|(at, task)| match poll_task(task) {
            Some(result) => {
                finished.push((*at, result));
                false
            }
            None => true,
        });
    if finished.is_empty() {
        return;
    }
    refresh.write(RefreshAssetLibrary);

    let mut stacker = stack.stacker();
    let mut placed = Vec::new();
    for (at, result) in finished {
        let image = match result {
            Ok(image) => image,
            Err(e) => {
                warn!("Couldn't import a dropped image: {}", e);
                continue;
            }
        };
        let (item, mut transform) = dropped_item(&image, at, map_data.grid_size);
        // A locked or hidden layer takes no new items; the image is still in the library
        if !map_data.is_layer_editable(item.layer) {
            continue;
        }
        transform.translation.z = stacker.next_z(&item);
        placed.push(MapObject::new_item(item, transform));
    }
    if placed.is_empty() {
        return;
    }

    for entity in &selected {
        commands.entity(entity).remove::<Selected>();
    }
    for object in &placed {
        let entity = object.spawn(&mut commands, &asset_server);
        commands.entity(entity).insert(Selected);
    }
    current_tool.tool = EditorTool::Select;
    history.write(RecordEditorCommand {
        command: EditorCommand::Add(placed),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(path: &str, w: u32, h: u32) -> ImportedImage {
        ImportedImage {
            path: path.to_string(),
            size: UVec2::new(w, h),
        }
    }

    #[test]
    fn test_battlemap_goes_on_the_background_fitted_to_the_grid() {
        let (item, transform) =
            dropped_item(&image("cave.png", 2100, 1400), Vec2::new(10.0, 10.0), 70.0);
        assert_eq!(item.layer, Layer::Background);
        // 30x20 cells at 70px: 2100x1400 world units, scale 1
        assert_eq!(transform.scale, Vec3::ONE);
        let corner = transform.translation.truncate() - Vec2::new(1050.0, 700.0);
        assert_eq!(corner % 70.0, Vec2::ZERO);
    }

    #[test]
    fn test_small_image_becomes_a_one_cell_token() {
        let (item, transform) =
            dropped_item(&image("orc.png", 280, 140), Vec2::new(100.0, 30.0), 70.0);
        assert_eq!(item.layer, Layer::Token);
        assert_eq!(transform.scale.x, 0.25);
        assert_eq!(transform.translation.truncate(), Vec2::new(105.0, 35.0));
    }

    #[test]
    fn test_copy_reuses_identical_files_and_renames_different_ones() {
        let source_dir = tempfile::tempdir().unwrap();
        let library = tempfile::tempdir().unwrap();
        let source = source_dir.path().join("orc.png");
        image::RgbaImage::new(2, 2).save(&source).unwrap();

        let first = copy_into_library(&source, library.path()).unwrap();
        let again = copy_into_library(&source, library.path()).unwrap();
        assert_eq!(first.path, "orc.png");
        assert_eq!(again.path, "orc.png");

        image::RgbaImage::new(3, 3).save(&source).unwrap();
        let different = copy_into_library(&source, library.path()).unwrap();
        assert_eq!(different.path, "orc (2).png");
        assert_eq!(different.size, UVec2::new(3, 3));
    }
}
