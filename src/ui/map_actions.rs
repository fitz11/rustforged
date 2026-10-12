//! New, Open, Save, Save As and Quit for the active map.
//!
//! Buttons, keyboard shortcuts and the window's close button all send a [`MapFileAction`],
//! so they ask about unsaved changes the same way: an action that would replace or close
//! the map shows the Unsaved Changes dialog first, and "Save" runs the action once the
//! save has actually succeeded.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::tasks::{IoTaskPool, Task};
use bevy::window::{PrimaryWindow, WindowCloseRequested};
use bevy_egui::{EguiContexts, egui};
use std::path::{Path, PathBuf};

use crate::assets::AssetLibrary;
use crate::input::command_key_just_pressed;
use crate::map::{
    LoadMapRequest, MapDirtyState, MapSaveFinished, NewMapRequest, OpenMaps, SaveMapRequest,
    SaveOutcome, SwitchMapRequest,
};

use super::map_dialogs::SaveAsDialogState;
use crate::common::poll_task;

/// Name of a map that hasn't been named yet
pub const UNTITLED_MAP_NAME: &str = "Untitled Map";

/// Something the user asked to do with the active map.
#[derive(Message, Debug, Clone, PartialEq)]
pub enum MapFileAction {
    New,
    Open(PathBuf),
    /// Open a map too, keeping the one being edited open in the background (scenes)
    OpenAlongside(PathBuf),
    /// Close one of the open maps
    Close(u64),
    Save,
    SaveAs,
    Quit,
    /// Quit so the downloaded installer at this path can replace the app (Windows)
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    QuitAndInstall(PathBuf),
}

/// An action that replaces or closes the active map, so its unsaved changes have to be
/// saved or discarded first.
#[derive(Debug, Clone, PartialEq)]
pub enum ReplacingAction {
    New,
    Open(PathBuf),
    /// Close this open map (the active one by the time it runs)
    Close(u64),
    Quit,
    QuitAndInstall(PathBuf),
}

impl ReplacingAction {
    /// Quitting needs every open map dealt with, not only the active one
    fn closes_everything(&self) -> bool {
        matches!(self, Self::Quit | Self::QuitAndInstall(_))
    }

    fn as_file_action(&self) -> MapFileAction {
        match self {
            Self::New => MapFileAction::New,
            Self::Open(path) => MapFileAction::Open(path.clone()),
            Self::Close(id) => MapFileAction::Close(*id),
            Self::Quit => MapFileAction::Quit,
            Self::QuitAndInstall(path) => MapFileAction::QuitAndInstall(path.clone()),
        }
    }
}

/// State of the Unsaved Changes dialog.
#[derive(Resource, Default)]
pub struct UnsavedChangesPrompt {
    /// Waiting for the user to choose Save, Don't Save or Cancel
    pub pending: Option<ReplacingAction>,
    /// The user chose Save: run this once the save succeeds
    pub after_save: Option<AfterSave>,
    /// Handle this action again next frame, once a switch to another open map is done
    /// (to ask about that map's unsaved changes, or to close it)
    pub retry: Option<MapFileAction>,
}

/// An action waiting for a save to finish.
pub struct AfterSave {
    pub action: ReplacingAction,
    /// The save it waits for; none yet while the Save Map dialog asks for a name
    pub ticket: Option<u64>,
}

impl UnsavedChangesPrompt {
    /// If an action waits for a save, it now waits for `request` (the save from the name
    /// dialog, or the one chosen after a conflict).
    pub fn wait_for(&mut self, request: &SaveMapRequest) {
        if let Some(after) = self.after_save.as_mut() {
            after.ticket = Some(request.ticket);
        }
    }
}

/// The file picker opened with Ctrl+O, while it's open.
#[derive(Resource, Default)]
pub struct OpenMapPicker {
    task: Option<Task<Option<PathBuf>>>,
}

impl OpenMapPicker {
    pub fn is_open(&self) -> bool {
        self.task.is_some()
    }

    /// Ask for a map file to open, starting in the library's maps folder.
    pub fn open(&mut self, maps_dir: &Path) {
        if self.task.is_some() {
            return;
        }
        let maps_dir = maps_dir.to_path_buf();
        self.task = Some(IoTaskPool::get().spawn(async move {
            let mut dialog = rfd::AsyncFileDialog::new()
                .set_title("Open Map")
                .add_filter("Rustforged map", &["json"]);
            if maps_dir.is_dir() {
                dialog = dialog.set_directory(&maps_dir);
            }
            dialog.pick_file().await.map(|h| h.path().to_path_buf())
        }));
    }
}

/// Opens the map picked in the Ctrl+O file picker.
pub fn poll_open_map_picker(
    mut picker: ResMut<OpenMapPicker>,
    mut actions: MessageWriter<MapFileAction>,
) {
    let Some(task) = picker.task.as_mut() else {
        return;
    };
    if let Some(result) = poll_task(task) {
        picker.task = None;
        if let Some(path) = result {
            actions.write(MapFileAction::Open(path));
        }
    }
}

/// Ctrl+S, Ctrl+Shift+S, Ctrl+O and Ctrl+N (Cmd on macOS).
pub fn handle_file_shortcuts(
    keyboard: Res<ButtonInput<KeyCode>>,
    library: Res<AssetLibrary>,
    mut picker: ResMut<OpenMapPicker>,
    mut actions: MessageWriter<MapFileAction>,
) {
    if command_key_just_pressed(&keyboard, KeyCode::KeyS) {
        if crate::input::shift_held(&keyboard) {
            actions.write(MapFileAction::SaveAs);
        } else {
            actions.write(MapFileAction::Save);
        }
    } else if command_key_just_pressed(&keyboard, KeyCode::KeyN) {
        actions.write(MapFileAction::New);
    } else if command_key_just_pressed(&keyboard, KeyCode::KeyO) {
        picker.open(&library.library_path.join("maps"));
    }
}

/// Runs a [`ReplacingAction`] once unsaved changes are dealt with, and knows which open
/// maps still have unsaved changes.
#[derive(SystemParam)]
pub struct ActionRunner<'w, 's> {
    commands: Commands<'w, 's>,
    new_events: MessageWriter<'w, NewMapRequest>,
    load_events: MessageWriter<'w, LoadMapRequest>,
    switch_events: MessageWriter<'w, SwitchMapRequest>,
    exit_events: MessageWriter<'w, AppExit>,
    primary_window: Query<'w, 's, Entity, With<PrimaryWindow>>,
    update_state: ResMut<'w, crate::update::UpdateState>,
    pub open_maps: ResMut<'w, OpenMaps>,
    pub dirty_state: ResMut<'w, MapDirtyState>,
}

impl ActionRunner<'_, '_> {
    fn active_unsaved(&self) -> bool {
        self.dirty_state.is_dirty
    }

    /// An open map in the background with unsaved changes
    fn unsaved_background_map(&self) -> Option<u64> {
        let mut unsaved: Vec<u64> = self
            .open_maps
            .maps
            .values()
            .filter(|m| Some(m.id) != self.open_maps.active_map_id && m.stashed_dirty)
            .map(|m| m.id)
            .collect();
        unsaved.sort_unstable();
        unsaved.first().copied()
    }

    /// Switches to open map `id`, keeping the one being left open
    fn switch_to(&mut self, id: u64) {
        self.switch_events.write(SwitchMapRequest {
            map_id: id,
            close_current: false,
        });
    }

    /// Runs `action`, or returns an action to handle again next frame (quitting with
    /// another open map's changes still to ask about).
    fn run(&mut self, action: ReplacingAction) -> Option<MapFileAction> {
        if action.closes_everything()
            && let Some(next) = self.unsaved_background_map()
        {
            self.switch_to(next);
            return Some(action.as_file_action());
        }
        match action {
            ReplacingAction::New => {
                self.new_events.write(NewMapRequest);
            }
            ReplacingAction::Open(path) => {
                self.load_events.write(LoadMapRequest::replace(path));
            }
            ReplacingAction::Close(id) => {
                let other = self
                    .open_maps
                    .maps
                    .keys()
                    .copied()
                    .find(|other| *other != id);
                match other {
                    Some(other) => {
                        self.switch_events.write(SwitchMapRequest {
                            map_id: other,
                            close_current: true,
                        });
                    }
                    // The last open map: a new blank one takes its place
                    None => {
                        self.new_events.write(NewMapRequest);
                    }
                }
            }
            ReplacingAction::Quit => self.quit(),
            ReplacingAction::QuitAndInstall(path) => match crate::update::launch_installer(&path) {
                Ok(()) => self.quit(),
                Err(e) => {
                    self.update_state.download_error = Some(e);
                    self.update_state.show_dialog = true;
                }
            },
        }
        None
    }

    fn quit(&mut self) {
        // Despawn the primary window so the OS can release it (prevents macOS hang)
        if let Ok(entity) = self.primary_window.single() {
            self.commands.entity(entity).despawn();
        }
        self.exit_events.write(AppExit::Success);
    }
}

/// Turn a map name into a file name: anything but letters, digits, spaces, `-` and `_`
/// becomes `_`.
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

/// Save the active map where it was last saved, or ask for a name if it never was.
/// Returns the request's ticket if it was sent right away.
fn request_save(
    open_maps: &OpenMaps,
    maps_dir: &Path,
    save_as: &mut SaveAsDialogState,
    save_events: &mut MessageWriter<SaveMapRequest>,
) -> Option<u64> {
    let active = open_maps.active_map()?;
    let filename = sanitize_map_name(&active.name);
    let path = if let Some(path) = &active.path {
        path.clone()
    } else if active.name != UNTITLED_MAP_NAME && filename.chars().any(char::is_alphanumeric) {
        // Named (e.g. renamed with F3) but never saved; a file with that name already
        // there stops the save and asks
        maps_dir.join(format!("{}.json", filename))
    } else {
        open_save_as(open_maps, save_as);
        return None;
    };
    let request = SaveMapRequest::new(path);
    let ticket = request.ticket;
    save_events.write(request);
    Some(ticket)
}

fn open_save_as(open_maps: &OpenMaps, save_as: &mut SaveAsDialogState) {
    save_as.save_filename = open_maps
        .active_map()
        .map(|m| m.name.clone())
        .unwrap_or_default();
    save_as.show_save_name_dialog = true;
}

/// Handles [`MapFileAction`]s and the window's close button. Runs after map switches, so
/// an action retried after switching to another open map sees that map as the active one.
#[allow(clippy::too_many_arguments)]
pub fn handle_map_file_actions(
    mut actions: MessageReader<MapFileAction>,
    mut close_events: MessageReader<WindowCloseRequested>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    mut prompt: ResMut<UnsavedChangesPrompt>,
    mut save_as: ResMut<SaveAsDialogState>,
    library: Res<AssetLibrary>,
    mut save_events: MessageWriter<SaveMapRequest>,
    mut runner: ActionRunner,
) {
    let primary = primary_window.single().ok();
    // With `close_when_requested: false`, closing the main window is up to us
    let quit_requests = close_events
        .read()
        .filter(|event| Some(event.window) == primary)
        .map(|_| MapFileAction::Quit)
        .collect::<Vec<_>>();
    let maps_dir = library.library_path.join("maps");
    let retry = prompt.retry.take();

    for action in retry
        .into_iter()
        .chain(actions.read().cloned())
        .chain(quit_requests)
    {
        let replacing = match action {
            MapFileAction::Save => {
                // A plain save: nothing waits for it
                prompt.after_save = None;
                request_save(&runner.open_maps, &maps_dir, &mut save_as, &mut save_events);
                continue;
            }
            MapFileAction::SaveAs => {
                open_save_as(&runner.open_maps, &mut save_as);
                continue;
            }
            MapFileAction::OpenAlongside(path) => {
                // Already open: switch to it
                let open = runner
                    .open_maps
                    .maps
                    .values()
                    .find(|m| m.path.as_ref() == Some(&path))
                    .map(|m| m.id);
                match open {
                    Some(id) => runner.switch_to(id),
                    None => {
                        runner.load_events.write(LoadMapRequest::alongside(path));
                    }
                }
                continue;
            }
            MapFileAction::Close(id) if runner.open_maps.active_map_id != Some(id) => {
                let unsaved = runner
                    .open_maps
                    .maps
                    .get(&id)
                    .is_some_and(|m| m.stashed_dirty);
                if unsaved {
                    // Show it, then ask about its changes
                    runner.switch_to(id);
                    prompt.retry = Some(MapFileAction::Close(id));
                } else {
                    runner.open_maps.maps.remove(&id);
                }
                continue;
            }
            MapFileAction::New => ReplacingAction::New,
            MapFileAction::Open(path) => ReplacingAction::Open(path),
            MapFileAction::Close(id) => ReplacingAction::Close(id),
            MapFileAction::Quit => ReplacingAction::Quit,
            MapFileAction::QuitAndInstall(path) => ReplacingAction::QuitAndInstall(path),
        };

        // The prompt is about the map being edited; quitting also gets to the others
        // (one at a time, see ActionRunner::run)
        if runner.active_unsaved() {
            prompt.pending = Some(replacing);
        } else if let Some(again) = runner.run(replacing) {
            prompt.retry = Some(again);
        }
    }
}

/// Runs the action the user chose "Save" for, once that save is done.
pub fn run_action_after_save(
    mut finished: MessageReader<MapSaveFinished>,
    mut prompt: ResMut<UnsavedChangesPrompt>,
    mut runner: ActionRunner,
) {
    for result in finished.read() {
        // Only the save the action waits for (an earlier one can finish first)
        let waits_for_this = prompt
            .after_save
            .as_ref()
            .is_some_and(|after| after.ticket == Some(result.ticket));
        if !waits_for_this {
            continue;
        }
        match result.outcome {
            SaveOutcome::Saved => {
                if let Some(after) = prompt.after_save.take() {
                    prompt.retry = runner.run(after.action);
                }
            }
            // The save conflict dialog decides; Overwrite or Save as Copy save again
            SaveOutcome::Conflict => {}
            SaveOutcome::Failed => prompt.after_save = None,
        }
    }
}

/// Renders the Unsaved Changes dialog for an action that would replace or close the map.
#[allow(clippy::too_many_arguments)]
pub fn unsaved_changes_dialog_ui(
    mut contexts: EguiContexts,
    mut prompt: ResMut<UnsavedChangesPrompt>,
    mut save_as: ResMut<SaveAsDialogState>,
    library: Res<AssetLibrary>,
    mut save_events: MessageWriter<SaveMapRequest>,
    mut runner: ActionRunner,
) -> Result {
    let Some(action) = prompt.pending.clone() else {
        return Ok(());
    };

    let active_name = runner
        .open_maps
        .active_map()
        .map(|m| m.name.clone())
        .unwrap_or_default();
    let others: Vec<String> = runner
        .open_maps
        .unsaved_maps(&runner.dirty_state)
        .into_iter()
        .filter(|m| Some(m.id) != runner.open_maps.active_map_id)
        .map(|m| m.name.clone())
        .collect();
    let question = match &action {
        ReplacingAction::New => "Save before creating a new map?".to_string(),
        ReplacingAction::Open(path) => format!(
            "Save before opening \"{}\"?",
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("the map")
        ),
        ReplacingAction::Close(_) => "Save before closing this map?".to_string(),
        ReplacingAction::Quit => "Save before closing?".to_string(),
        ReplacingAction::QuitAndInstall(_) => {
            "The app closes to install the update. Save first?".to_string()
        }
    };

    let mut choice = None;
    egui::Window::new("Unsaved Changes")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(contexts.ctx_mut()?, |ui| {
            ui.label("You have unsaved changes in:");
            ui.add_space(4.0);
            ui.label(egui::RichText::new(format!("  - {}", active_name)).strong());
            if action.closes_everything() && !others.is_empty() {
                ui.label(
                    egui::RichText::new(format!(
                        "(then {}, which you'll be asked about next)",
                        others.join(", ")
                    ))
                    .weak(),
                );
            }
            ui.add_space(8.0);
            ui.label(&question);
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                if ui.button("Save").clicked() {
                    choice = Some(Choice::Save);
                }
                if ui.button("Don't Save").clicked() {
                    choice = Some(Choice::Discard);
                }
                if ui.button("Cancel").clicked() {
                    choice = Some(Choice::Cancel);
                }
            });
        });

    match choice {
        Some(Choice::Save) => {
            prompt.pending = None;
            let maps_dir = library.library_path.join("maps");
            let ticket = request_save(&runner.open_maps, &maps_dir, &mut save_as, &mut save_events);
            prompt.after_save = Some(AfterSave { action, ticket });
        }
        Some(Choice::Discard) => {
            prompt.pending = None;
            // These changes are being dropped, so don't ask about them again
            runner.dirty_state.is_dirty = false;
            prompt.retry = runner.run(action);
        }
        Some(Choice::Cancel) => prompt.pending = None,
        None => {}
    }

    Ok(())
}

enum Choice {
    Save,
    Discard,
    Cancel,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_map_name_replaces_path_characters() {
        assert_eq!(sanitize_map_name("Goblin Cave"), "Goblin Cave");
        assert_eq!(sanitize_map_name("../a/b:c"), "___a_b_c");
        assert_eq!(sanitize_map_name("  spaced  "), "spaced");
    }
}
