//! Shift+V "Fade out and stop": the output volume steps down to silence over
//! [`FADE_OUT_TICKS`] LCD ticks and then the transport stops.
//!
//! The fade only lowers the backend's gain. The saved volume in shared state is
//! never written, so the slider, the settings file, and the next play keep the
//! user's level; the backend gets that level back after the stop (or at once
//! when another transport message interrupts the fade).

use iced::Task;

use crate::ui::loading::transport_into;
use crate::ui::player::FADE_OUT_TICKS;
use crate::ui::{Message, WinampPlayer};

use super::{begin_position_transition, reset_position_readout, stop};

/// Shift+V: starts the fade while a preview plays. Paused or stopped there is
/// no sound to fade, so it acts as Stop; a press during a fade leaves the
/// countdown alone. The press itself makes no backend call.
pub(in crate::ui) fn fade_out_and_stop(player: &mut WinampPlayer) -> Task<Message> {
    if !player.state.blocking_lock().is_playing {
        return stop(player);
    }
    if player.fade_out_ticks == 0 {
        player.fade_out_ticks = FADE_OUT_TICKS;
    }
    Task::none()
}

/// A click on the Stop button: fades out while Shift is held, otherwise stops at once.
pub(in crate::ui) fn stop_button(player: &mut WinampPlayer) -> Task<Message> {
    if player.shift_held {
        fade_out_and_stop(player)
    } else {
        stop(player)
    }
}

/// One LCD tick of a running fade: lowers the backend gain one step, and on the
/// last step stops and restores the saved level. Does nothing without a fade.
pub(in crate::ui) fn fade_out_tick(player: &mut WinampPlayer) -> Task<Message> {
    if player.fade_out_ticks == 0 {
        return Task::none();
    }
    player.fade_out_ticks -= 1;
    if player.fade_out_ticks == 0 {
        return stop_restoring_volume(player);
    }
    let remaining = f32::from(player.fade_out_ticks) / f32::from(FADE_OUT_TICKS);
    let volume = player.state.blocking_lock().volume() * remaining;
    if let Err(error) = player.apple_music_service.set_output_volume(volume) {
        eprintln!("fade-out: forwarding volume {volume} to the audio backend failed: {error}");
    }
    Task::none()
}

/// Stops like [`stop`], then hands the backend the saved volume. The restore
/// follows the stop, so the muted backend cannot carry into a later Play
/// after Stop, whose resume does not re-send the volume.
pub(in crate::ui) fn stop_restoring_volume(player: &mut WinampPlayer) -> Task<Message> {
    let volume = player.state.blocking_lock().volume();
    reset_position_readout(player);
    begin_position_transition(player);
    transport_into(
        &player.apple_music_service,
        "stop",
        move |service| async move {
            service.stop().await?;
            service.set_output_volume(volume)
        },
    )
}

/// Ends a running fade without stopping: zeroes the countdown and gives the
/// backend the saved volume back. Does nothing without a fade.
pub(in crate::ui) fn cancel_fade_out(player: &mut WinampPlayer) {
    if player.fade_out_ticks == 0 {
        return;
    }
    player.fade_out_ticks = 0;
    let volume = player.state.blocking_lock().volume();
    if let Err(error) = player.apple_music_service.set_output_volume(volume) {
        eprintln!("fade-out: restoring volume {volume} on the audio backend failed: {error}");
    }
}
