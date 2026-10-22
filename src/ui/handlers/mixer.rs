//! The mixer message handlers: the volume, the balance, the seek nudges, and
//! the equalizer.
//!
//! Each of these writes a clamped value into the shared state and forwards
//! the result to the audio backend, so a slider drag reaches the playing
//! preview without restarting it.

use iced::Task;

use crate::audio::EqSnapshot;
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

/// Locks the shared playback state, applies `mutation` to it, forwards the
/// resulting clamped volume to the audio backend, and returns no task. The
/// volume handlers — the slider's drag and the two arrow keys — all repeat
/// the same two-step update: `blocking_lock`, one mutation, read back the
/// clamped `state.volume()`, then hand it to
/// `AppleMusicService::set_output_volume`. The state write and the backend
/// forward both happen under the one lock, and the forward is a non-blocking
/// channel send, so a slider drag stays responsive and its messages stay
/// ordered. A backend error is logged; the state write has already committed.
fn mutate_volume(player: &WinampPlayer, mutation: impl FnOnce(&mut AppState)) -> Task<Message> {
    let mut state = player.state.blocking_lock();
    mutation(&mut state);
    let volume = state.volume();
    if let Err(error) = player.apple_music_service.set_output_volume(volume) {
        eprintln!("forwarding volume to the audio backend failed: {error}");
    }
    Task::none()
}

/// Locks the shared playback state, applies `mutation` to it, forwards the
/// resulting clamped balance to the audio backend, and returns no task. The
/// balance slider's drag uses this so a slider drag reaches the playing
/// preview without restarting it; the state write and the backend forward
/// both happen under the one lock, exactly as `mutate_volume` does. A
/// backend error is logged; the state write has already committed.
fn mutate_balance(player: &WinampPlayer, mutation: impl FnOnce(&mut AppState)) -> Task<Message> {
    let mut state = player.state.blocking_lock();
    mutation(&mut state);
    let balance = state.balance();
    if let Err(error) = player.apple_music_service.set_output_balance(balance) {
        eprintln!("forwarding balance to the audio backend failed: {error}");
    }
    Task::none()
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

/// Locks the shared playback state, applies `mutation` to it, forwards the
/// resulting clamped curve to the audio backend, and returns no task. The
/// equalizer handlers — the on/off button, the preamp and band sliders, and
/// the preset pick — use this so a toggle, slider drag, or preset pick
/// reaches the playing preview without restarting it; the running
/// `EqSource` re-reads its shared curve between samples. The
/// state write and the backend forward both happen under the one lock, exactly
/// as `mutate_volume` and `mutate_balance` do. A backend error is logged;
/// the state write has already committed.
fn mutate_equalizer(player: &WinampPlayer, mutation: impl FnOnce(&mut AppState)) -> Task<Message> {
    let mut state = player.state.blocking_lock();
    mutation(&mut state);
    let equalizer = EqSnapshot {
        enabled: state.eq_enabled,
        preamp_db: state.eq_preamp(),
        bands_db: state.eq_bands(),
    };
    if let Err(error) = player.apple_music_service.set_output_equalizer(equalizer) {
        eprintln!("forwarding equalizer to the audio backend failed: {error}");
    }
    Task::none()
}
