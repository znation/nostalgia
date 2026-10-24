//! The mixer message handlers: the volume, the balance, the seek nudges, the
//! equalizer, and both panel-visibility toggles (the equalizer panel's and the
//! library section's).
//!
//! The volume, balance, and equalizer handlers write a clamped value into the
//! shared state and forward the result to the audio backend, so a slider drag
//! reaches the playing preview without restarting it. The seek nudge forwards
//! a relative seek without touching shared state, and each panel toggle flips
//! only its player layout flag; neither schedules work.

use iced::Task;

use crate::apple_music::{AppleMusicError, AppleMusicService};
use crate::equalizer::Preset;
use crate::state::AppState;
use crate::ui::views;
use crate::ui::{Message, WinampPlayer};

/// The volume slider's drag: sets the playback level.
pub(in crate::ui) fn volume_change(player: &mut WinampPlayer, volume: f32) -> Task<Message> {
    mutate_volume(player, |state| state.set_volume(volume))
}

/// The up arrow: nudges the slider's value by its own step, so the keyboard
/// and the drag share one granularity; `nudge_volume` clamps.
pub(in crate::ui) fn volume_up(player: &mut WinampPlayer) -> Task<Message> {
    mutate_volume(player, |state| state.nudge_volume(views::VOLUME_STEP))
}

/// The down arrow: nudges the slider's value down by its own step.
pub(in crate::ui) fn volume_down(player: &mut WinampPlayer) -> Task<Message> {
    mutate_volume(player, |state| state.nudge_volume(-views::VOLUME_STEP))
}

/// A wheel roll over the player panels: nudges the slider by one step in the
/// roll's vertical direction through the same up/down handlers as the arrow
/// keys. A zero or non-finite direction is a no-op, so a horizontal-only roll
/// leaves both the volume and the backend untouched; the `>`/`<` comparison
/// sends NaN down the no-op path.
pub(in crate::ui) fn volume_wheel(player: &mut WinampPlayer, direction: f32) -> Task<Message> {
    if direction > 0.0 {
        volume_up(player)
    } else if direction < 0.0 {
        volume_down(player)
    } else {
        Task::none()
    }
}

/// The balance slider's drag: sets the stereo balance.
pub(in crate::ui) fn balance_change(player: &mut WinampPlayer, balance: f32) -> Task<Message> {
    mutate_balance(player, |state| state.set_balance(balance))
}

/// The left arrow: moves the playing preview back by Winamp's five-second
/// step; a seek while stopped is a no-op in the backend.
pub(in crate::ui) fn seek_backward(player: &mut WinampPlayer) -> Task<Message> {
    seek_preview(player, -views::SEEK_STEP_SECS)
}

/// The right arrow: moves the playing preview forward by Winamp's
/// five-second step.
pub(in crate::ui) fn seek_forward(player: &mut WinampPlayer) -> Task<Message> {
    seek_preview(player, views::SEEK_STEP_SECS)
}

/// The player row's EQ button and the E key: shows or hides the normal-mode
/// equalizer section. This is a layout-only toggle, distinct from
/// [`toggle_equalizer`], which flips the audio equalizer's on/off state; it
/// changes no shared EQ value and schedules no work. Shade mode builds no
/// equalizer panel, so there the toggle leaves the flag alone instead of
/// flipping a hidden setting.
pub(in crate::ui) fn toggle_equalizer_panel(player: &mut WinampPlayer) -> Task<Message> {
    if player.shaded {
        return Task::none();
    }
    player.eq_panel_visible = !player.eq_panel_visible;
    Task::none()
}

/// The player row's PL button: shows or hides the normal-mode library section.
/// This is a layout-only toggle that changes no playback, equalizer, browse,
/// search, or selection state and schedules no work.
pub(in crate::ui) fn toggle_library_panel(player: &mut WinampPlayer) -> Task<Message> {
    player.library_panel_visible = !player.library_panel_visible;
    Task::none()
}

/// The equalizer's on/off button.
pub(in crate::ui) fn toggle_equalizer(player: &mut WinampPlayer) -> Task<Message> {
    mutate_equalizer(player, AppState::toggle_equalizer)
}

/// The preamp slider's drag.
pub(in crate::ui) fn eq_preamp_change(player: &mut WinampPlayer, gain: f32) -> Task<Message> {
    mutate_equalizer(player, |state| state.set_eq_preamp(gain))
}

/// A band slider's drag.
pub(in crate::ui) fn eq_band_change(
    player: &mut WinampPlayer,
    band: usize,
    gain: f32,
) -> Task<Message> {
    mutate_equalizer(player, |state| state.set_eq_band(band, gain))
}

/// A preset pick from the equalizer's preset list.
pub(in crate::ui) fn eq_preset_selected(
    player: &mut WinampPlayer,
    preset: Preset,
) -> Task<Message> {
    mutate_equalizer(player, |state| state.apply_eq_preset(preset))
}

/// Locks the shared playback state, applies `mutation`, forwards the resulting
/// clamped value to the audio backend, and returns no task.
///
/// The volume, balance, and equalizer helpers below all repeat the same
/// sequence — `blocking_lock`, one mutation, read the clamped value back
/// under the lock, then one non-blocking channel send to the audio backend —
/// so the shape lives here once and each helper names only its value and
/// backend call. The state write and the forward happen under the one lock,
/// so a slider drag stays responsive and its messages stay ordered. A backend
/// error is logged; the state write has already committed.
fn mutate_and_forward(
    player: &WinampPlayer,
    mutation: impl FnOnce(&mut AppState),
    forward: impl FnOnce(&AppleMusicService, &AppState) -> Result<(), AppleMusicError>,
    failure: &'static str,
) -> Task<Message> {
    let mut state = player.state.blocking_lock();
    mutation(&mut state);
    if let Err(error) = forward(&player.apple_music_service, &state) {
        eprintln!("{failure}: {error}");
    }
    Task::none()
}

/// Forwards the mutated state's clamped volume to the audio backend. The
/// volume handlers — the slider's drag and the two arrow keys — all use this
/// so a slider drag reaches the playing preview without restarting it; the
/// lock-mutate-forward shape is [`mutate_and_forward`]'s.
fn mutate_volume(player: &WinampPlayer, mutation: impl FnOnce(&mut AppState)) -> Task<Message> {
    mutate_and_forward(
        player,
        mutation,
        |service, state| service.set_output_volume(state.volume()),
        "forwarding volume to the audio backend failed",
    )
}

/// Forwards the mutated state's clamped balance to the audio backend. The
/// balance slider's drag uses this so a drag reaches the playing preview
/// without restarting it; the lock-mutate-forward shape is
/// [`mutate_and_forward`]'s.
fn mutate_balance(player: &WinampPlayer, mutation: impl FnOnce(&mut AppState)) -> Task<Message> {
    mutate_and_forward(
        player,
        mutation,
        |service, state| service.set_output_balance(state.balance()),
        "forwarding balance to the audio backend failed",
    )
}

/// Forwards a relative seek to the audio backend and returns no task. The
/// left and right arrow handlers use this so a key press moves the playing
/// preview without touching shared state: the backend owns the position, and
/// `AppleMusicService::seek_output` is a non-blocking channel send. A
/// backend error is logged.
fn seek_preview(player: &WinampPlayer, offset_secs: i64) -> Task<Message> {
    if let Err(error) = player.apple_music_service.seek_output(offset_secs) {
        eprintln!("seeking the preview failed: {error}");
    }
    Task::none()
}

/// Forwards the mutated state's clamped curve to the audio backend. The
/// equalizer handlers — the on/off button, the preamp and band sliders, and
/// the preset pick — use this so a toggle, slider drag, or preset pick
/// reaches the playing preview without restarting it; the running `EqSource`
/// re-reads its shared curve between samples. The lock-mutate-forward shape is
/// [`mutate_and_forward`]'s.
fn mutate_equalizer(player: &WinampPlayer, mutation: impl FnOnce(&mut AppState)) -> Task<Message> {
    mutate_and_forward(
        player,
        mutation,
        |service, state| service.set_output_equalizer(state.eq_snapshot()),
        "forwarding equalizer to the audio backend failed",
    )
}
