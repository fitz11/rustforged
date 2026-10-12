//! Unit tests for the history module.

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;

use crate::editor::annotations::DrawnPath;
use crate::map::{ItemId, Layer, MapDirtyState, PlacedItem};

use super::MAX_HISTORY_SIZE;
use super::command_history::CommandHistory;
use super::commands::EditorCommand;
use super::data_types::MapObject;
use super::systems::handle_undo_redo;

fn path(points: &[Vec2]) -> MapObject {
    MapObject::new_path(DrawnPath {
        points: points.to_vec(),
        color: Color::WHITE,
        stroke_width: 2.0,
    })
}

#[test]
fn test_command_history_push() {
    let mut history = CommandHistory::default();
    assert!(!history.can_undo());

    history.push(EditorCommand::Add(vec![]));
    assert!(history.can_undo());
    assert_eq!(history.undo_count(), 1);
}

#[test]
fn test_command_history_undo_clears_redo() {
    let mut history = CommandHistory::default();

    // Push some commands
    history.push(EditorCommand::Add(vec![]));
    history.push(EditorCommand::Add(vec![]));

    // Pop one for undo
    history.pop_undo();
    history.push_redo(EditorCommand::Remove(vec![]));
    assert!(history.can_redo());

    // Push a new command - should clear redo
    history.push(EditorCommand::Add(vec![]));
    assert!(!history.can_redo());
}

#[test]
fn test_command_history_max_size() {
    let mut history = CommandHistory::default();

    // Push more than max size
    for _ in 0..150 {
        history.push(EditorCommand::Add(vec![]));
    }

    // Should be trimmed to max size
    assert_eq!(history.undo_count(), MAX_HISTORY_SIZE);
}

#[test]
fn test_change_skips_unchanged_pairs() {
    let a = path(&[Vec2::ZERO, Vec2::ONE]);
    assert_eq!(EditorCommand::change([(a.clone(), a.clone())]), None);

    let MapObject::Path { id, mut path } = a.clone() else {
        unreachable!()
    };
    path.points[1] = Vec2::splat(5.0);
    let moved = MapObject::Path { id, path };
    assert_eq!(
        EditorCommand::change([(a.clone(), a.clone()), (a.clone(), moved.clone())]),
        Some(EditorCommand::Change(vec![(a, moved)]))
    );
}

#[test]
fn test_retarget_asset_updates_remembered_items() {
    let item = MapObject::new_item(
        PlacedItem {
            asset_path: "tokens/orc.png".to_string(),
            layer: Layer::Token,
            z_index: 0,
        },
        Transform::default(),
    );
    let mut history = CommandHistory::default();
    history.push(EditorCommand::Remove(vec![item]));

    history.retarget_asset("tokens/orc.png", "monsters/orc.png");

    let Some(EditorCommand::Remove(objects)) = history.pop_undo() else {
        panic!("expected the Remove back");
    };
    let MapObject::Item { item, .. } = &objects[0] else {
        panic!("expected an item");
    };
    assert_eq!(item.asset_path, "monsters/orc.png");
}

// ---------------------------------------------------------------------------
// Undo and redo against a running app
// ---------------------------------------------------------------------------

fn history_app() -> App {
    let mut app = App::new();
    crate::assets::register_library_source(&mut app);
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<CommandHistory>()
        .init_resource::<MapDirtyState>()
        .add_systems(Update, handle_undo_redo);
    app
}

/// Puts `object` on the map, as a tool would.
fn spawn(app: &mut App, object: &MapObject) {
    let object = object.clone();
    app.world_mut()
        .run_system_once(move |mut commands: Commands, assets: Res<AssetServer>| {
            object.spawn(&mut commands, &assets);
        })
        .unwrap();
}

/// Takes `object` off the map, as Delete would.
fn despawn(app: &mut App, object: &MapObject) {
    let id = object.id();
    let entity = app
        .world_mut()
        .query::<(Entity, &ItemId)>()
        .iter(app.world())
        .find(|(_, found)| **found == id)
        .map(|(entity, _)| entity)
        .unwrap();
    app.world_mut().despawn(entity);
}

fn record(app: &mut App, command: EditorCommand) {
    app.world_mut()
        .resource_mut::<CommandHistory>()
        .push(command);
}

fn press(app: &mut App, keys: &[KeyCode]) {
    let mut input = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    input.release_all();
    input.clear();
    for key in keys {
        input.press(*key);
    }
    app.update();
}

fn undo(app: &mut App) {
    press(app, &[KeyCode::ControlLeft, KeyCode::KeyZ]);
}

fn redo(app: &mut App) {
    press(app, &[KeyCode::ControlLeft, KeyCode::KeyY]);
}

fn paths(app: &mut App) -> Vec<Vec<Vec2>> {
    app.world_mut()
        .query::<&DrawnPath>()
        .iter(app.world())
        .map(|p| p.points.clone())
        .collect()
}

#[test]
fn test_undo_twice_after_undoing_delete_removes_the_object() {
    let mut app = history_app();
    let drawn = path(&[Vec2::ZERO, Vec2::ONE]);

    // Draw, then delete
    spawn(&mut app, &drawn);
    record(&mut app, EditorCommand::Add(vec![drawn.clone()]));
    despawn(&mut app, &drawn);
    record(&mut app, EditorCommand::Remove(vec![drawn.clone()]));

    undo(&mut app); // the delete: the path comes back as a new entity
    assert_eq!(paths(&mut app).len(), 1);
    undo(&mut app); // the draw: that new entity must go
    assert!(paths(&mut app).is_empty());

    redo(&mut app);
    assert_eq!(paths(&mut app).len(), 1);
    redo(&mut app);
    assert!(paths(&mut app).is_empty());
}

#[test]
fn test_undo_change_after_undoing_delete_restores_old_state() {
    let mut app = history_app();
    let before = path(&[Vec2::ZERO, Vec2::ONE]);
    let MapObject::Path { id, path: drawn } = before.clone() else {
        unreachable!()
    };
    let after = MapObject::Path {
        id,
        path: DrawnPath {
            points: vec![Vec2::splat(10.0), Vec2::splat(11.0)],
            ..drawn
        },
    };

    // Draw, move, delete
    spawn(&mut app, &before);
    record(&mut app, EditorCommand::Add(vec![before.clone()]));
    despawn(&mut app, &before);
    spawn(&mut app, &after);
    record(
        &mut app,
        EditorCommand::Change(vec![(before.clone(), after.clone())]),
    );
    despawn(&mut app, &after);
    record(&mut app, EditorCommand::Remove(vec![after]));

    undo(&mut app);
    assert_eq!(
        paths(&mut app),
        vec![vec![Vec2::splat(10.0), Vec2::splat(11.0)]]
    );
    undo(&mut app); // the move applies to the respawned path
    assert_eq!(paths(&mut app), vec![vec![Vec2::ZERO, Vec2::ONE]]);
    undo(&mut app);
    assert!(paths(&mut app).is_empty());
    assert!(app.world().resource::<MapDirtyState>().is_dirty);
}

#[test]
fn test_undo_and_redo_respawn_items_with_their_layer() {
    let mut app = history_app();
    let item = MapObject::new_item(
        PlacedItem {
            asset_path: "gm/trap.png".to_string(),
            layer: Layer::GM,
            z_index: 3,
        },
        Transform::from_xyz(35.0, 35.0, 0.0),
    );
    spawn(&mut app, &item);
    despawn(&mut app, &item);
    record(&mut app, EditorCommand::Remove(vec![item]));

    undo(&mut app);
    let (transform, layers) = app
        .world_mut()
        .query_filtered::<(&Transform, &bevy::camera::visibility::RenderLayers), With<PlacedItem>>()
        .single(app.world())
        .unwrap();
    assert_eq!(transform.translation.z, Layer::GM.z_base() + 3.0);
    assert_eq!(*layers, Layer::GM.render_layers());
}
