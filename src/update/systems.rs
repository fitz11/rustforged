//! Bevy systems for update checking and downloading.

use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
use futures_lite::future;

use super::operations::check_for_updates;
use super::state::{DownloadTask, UpdateCheckTask, UpdateState};

/// System to start the update check on startup
pub fn start_update_check(mut commands: Commands, mut update_state: ResMut<UpdateState>) {
    update_state.is_checking = true;

    let task_pool = AsyncComputeTaskPool::get();
    let task = task_pool.spawn(async move { check_for_updates() });

    commands.spawn(UpdateCheckTask(task));
}

/// System to poll the update check task
pub fn poll_update_check(
    mut commands: Commands,
    mut update_state: ResMut<UpdateState>,
    mut tasks: Query<(Entity, &mut UpdateCheckTask)>,
) {
    for (entity, mut task) in tasks.iter_mut() {
        if let Some(result) = future::block_on(future::poll_once(&mut task.0)) {
            update_state.is_checking = false;
            update_state.update_available = result.update_available;
            update_state.latest_version = result.latest_version;
            update_state.release_url = result.release_url;
            update_state.release_notes = result.release_notes;
            update_state.download_url = result.download_url;
            update_state.error = result.error;

            commands.entity(entity).despawn();
        }
    }
}

/// System to poll the download task
pub fn poll_download_task(
    mut commands: Commands,
    mut update_state: ResMut<UpdateState>,
    mut tasks: Query<(Entity, &mut DownloadTask)>,
) {
    for (entity, mut task) in tasks.iter_mut() {
        if let Some(result) = future::block_on(future::poll_once(&mut task.0)) {
            update_state.is_downloading = false;

            if result.success {
                update_state.downloaded_path = result.path;
                update_state.download_error = None;
            } else {
                update_state.download_error = result.error;
            }

            commands.entity(entity).despawn();
        }
    }
}
