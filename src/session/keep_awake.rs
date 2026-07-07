//! Prevents the OS from blanking or sleeping the display while a live session
//! is running, so a game left idle on the play view doesn't drop to a black
//! screen mid-session.
//!
//! An OS-level inhibitor is acquired when [`LiveSessionState::is_active`] flips
//! to `true` and released (by dropping the guard) when it flips back to
//! `false` or when the app shuts down. The inhibitor is scoped to the live
//! session only; ordinary map editing does not keep the display awake.

use bevy::prelude::*;

use super::state::LiveSessionState;

/// Holds the OS "keep awake" guard for the duration of a live session.
///
/// The guard is `Some` exactly while a session is active. Dropping it (either
/// on session end or when the resource itself is dropped at shutdown) releases
/// the inhibitor.
#[derive(Resource, Default)]
pub struct KeepAwakeGuard {
    guard: Option<keepawake::KeepAwake>,
}

/// Acquires or releases the display-sleep inhibitor to match the current
/// session state. Only runs when [`LiveSessionState`] changes.
pub fn sync_keep_awake(session: Res<LiveSessionState>, mut keep_awake: ResMut<KeepAwakeGuard>) {
    match (session.is_active, keep_awake.guard.is_some()) {
        // Session just started and we don't hold an inhibitor yet: acquire one.
        (true, false) => match keepawake::Builder::default()
            .display(true)
            .reason("Rustforged live session")
            .app_name("Rustforged")
            .app_reverse_domain("dev.squishygoose.rustforged")
            .create()
        {
            Ok(guard) => {
                keep_awake.guard = Some(guard);
                info!("Live session started: inhibiting display sleep");
            }
            Err(err) => warn!("Failed to inhibit display sleep for live session: {err}"),
        },
        // Session ended while we hold an inhibitor: drop it to release.
        (false, true) => {
            keep_awake.guard = None;
            info!("Live session ended: released display sleep inhibitor");
        }
        // Already in the desired state; nothing to do.
        (true, true) | (false, false) => {}
    }
}
