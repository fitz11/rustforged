mod assets;
mod common;
mod config;
mod constants;
mod editor;
mod map;
mod session;
pub mod theme;
mod ui;
mod update;

use bevy::asset::UnapprovedPathMode;
use bevy::prelude::*;
use bevy_egui::EguiPlugin;

use constants::{DEFAULT_WINDOW_HEIGHT, DEFAULT_WINDOW_WIDTH};

/// Set up file logging for debug builds
#[cfg(debug_assertions)]
fn setup_logging() -> Option<tracing_appender::non_blocking::WorkerGuard> {
    use std::fs::OpenOptions;
    use std::io::Write;
    use tracing_subscriber::prelude::*;

    // Create logs directory if it doesn't exist
    let logs_dir = std::path::Path::new("logs");
    if std::fs::create_dir_all(logs_dir).is_err() {
        eprintln!("Failed to create logs directory");
        return None;
    }

    let log_file_path = logs_dir.join("rustforged.log");

    // Append session separator to existing log file
    if let Ok(mut file) = OpenOptions::new().append(true).open(&log_file_path) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let separator = "=".repeat(80);
        let _ = writeln!(
            file,
            "\n\n{}\n=== New Session Started at {} ===\n{}\n",
            separator, timestamp, separator
        );
    }

    // Set up file appender
    let file_appender = tracing_appender::rolling::never(logs_dir, "rustforged.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    // Configure file layer (no ANSI colors for file output)
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true)
        .with_level(true);

    // Configure stdout layer (with ANSI colors)
    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_ansi(true)
        .with_target(true)
        .with_level(true);

    // Use env filter to control log levels (default to info for bevy, debug for rustforged)
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,rustforged=debug"));

    tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .with(stdout_layer)
        .init();

    Some(guard)
}

#[cfg(not(debug_assertions))]
fn setup_logging() -> Option<()> {
    None
}

fn main() {
    // Keep the guard alive for the duration of the program
    let _log_guard = setup_logging();
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Rustforged VTT Editor".into(),
                        resolution: (DEFAULT_WINDOW_WIDTH as u32, DEFAULT_WINDOW_HEIGHT as u32)
                            .into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    // Allow loading assets from absolute paths (external libraries)
                    unapproved_path_mode: UnapprovedPathMode::Allow,
                    ..default()
                }),
        )
        .add_plugins(EguiPlugin::default())
        .add_plugins(config::ConfigPlugin)
        .add_plugins(editor::EditorPlugin)
        .add_plugins(assets::AssetLibraryPlugin)
        .add_plugins(map::MapPlugin)
        .add_plugins(session::LiveSessionPlugin)
        .add_plugins(ui::UiPlugin)
        .add_plugins(update::UpdateCheckerPlugin)
        .run();
}
