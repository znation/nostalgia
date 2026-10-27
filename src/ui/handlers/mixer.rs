//! The mixer message handlers: the volume, the balance, the seek nudges, the
//! equalizer.
//!
//! The volume, balance, and equalizer handlers write a clamped value into the
//! shared state and forward the result to the audio backend, so a slider drag
//! reaches the playing preview without restarting it. The seek nudge forwards
//! a relative seek without touching shared state.

use iced::Task;

use crate::apple_music::{AppleMusicError, AppleMusicService};
use crate::equalizer::{BAND_COUNT, Preset, preset_for_genre};
use crate::state::AppState;
use crate::ui::views;
use crate::ui::{Message, WinampPlayer};

use super::playback::preview::seek_by_failure_report;

/// The volume slider's drag: sets the playback level.
pub(in crate::ui) fn volume_change(player: &mut WinampPlayer, volume: f32) -> Task<Message> {
    player.mixer_drag = Some(views::MixerDrag::Volume);
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

/// The balance slider's wheel roll: nudges the balance by one
/// [`views::BALANCE_STEP`] in the roll's vertical direction, through the same
/// clamped setter and backend forward as a drag. Unlike a drag it leaves
/// `mixer_drag` alone, so the LCD keeps its title. A zero or `NaN` direction
/// is a no-op.
pub(in crate::ui) fn balance_wheel(player: &mut WinampPlayer, direction: f32) -> Task<Message> {
    if direction == 0.0 || direction.is_nan() {
        return Task::none();
    }
    mutate_balance(player, |state| {
        state.set_balance(state.balance() + direction.signum() * views::BALANCE_STEP)
    })
}

/// The balance slider's drag: sets the stereo balance. A move toward center
/// that lands near it snaps to exactly center (see
/// [`crate::state::snap_balance_to_center`]); "toward" is judged against the
/// drag's previous requested value, so a drag continuing inward through the
/// zone stays snapped.
pub(in crate::ui) fn balance_change(player: &mut WinampPlayer, balance: f32) -> Task<Message> {
    player.mixer_drag = Some(views::MixerDrag::Balance);
    let previous = player.balance_drag_raw.replace(balance);
    mutate_balance(player, |state| {
        state.set_balance(crate::state::snap_balance_to_center(
            previous.unwrap_or_else(|| state.balance()),
            balance,
        ))
    })
}

/// The release of a mixer or equalizer slider drag: the LCD shows the title
/// again. Harmless when no drag is shown.
pub(in crate::ui) fn mixer_released(player: &mut WinampPlayer) -> Task<Message> {
    player.mixer_drag = None;
    player.balance_drag_raw = None;
    Task::none()
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

/// The equalizer panel's AUTO button: flips the genre-matched preset flag.
/// The toggle changes no gain, so it writes shared state directly and
/// schedules nothing instead of forwarding a curve to the audio backend.
pub(in crate::ui) fn toggle_eq_auto(player: &mut WinampPlayer) -> Task<Message> {
    player.state.blocking_lock().toggle_eq_auto();
    Task::none()
}

/// Applies the preset named after `genre` when AUTO is on, through the same
/// mutate-and-forward path as a pick-list selection so the stored curve, the
/// pick list, and a running preview's filters update together. A blank or
/// unmatched genre, or AUTO off, changes nothing.
pub(in crate::ui) fn apply_auto_preset(player: &WinampPlayer, genre: &str) -> Task<Message> {
    let Some(preset) = preset_for_genre(genre) else {
        return Task::none();
    };
    if !player.state.blocking_lock().eq_auto {
        return Task::none();
    }
    mutate_equalizer(player, |state| state.apply_eq_preset(preset))
}

/// The preamp slider's drag.
pub(in crate::ui) fn eq_preamp_change(player: &mut WinampPlayer, gain: f32) -> Task<Message> {
    player.mixer_drag = Some(views::MixerDrag::EqPreamp);
    mutate_equalizer(player, |state| state.set_eq_preamp(gain))
}

/// The preamp slider's wheel roll: nudges the preamp by one
/// [`views::EQ_STEP`] in the roll's vertical direction, through the same
/// clamped setter and backend forward as a drag, but without setting
/// `mixer_drag`. A zero or `NaN` direction is a no-op.
pub(in crate::ui) fn eq_preamp_wheel(player: &mut WinampPlayer, direction: f32) -> Task<Message> {
    if direction == 0.0 || direction.is_nan() {
        return Task::none();
    }
    mutate_equalizer(player, |state| {
        state.set_eq_preamp(state.eq_preamp() + direction.signum() * views::EQ_STEP)
    })
}

/// A band slider's wheel roll: nudges only band `band` by one
/// [`views::EQ_STEP`] in the roll's vertical direction, through the same
/// clamped setter and backend forward as a drag, but without setting
/// `mixer_drag`. A zero or `NaN` direction, or an out-of-range band, is a no-op
/// before the state lock.
pub(in crate::ui) fn eq_band_wheel(
    player: &mut WinampPlayer,
    band: usize,
    direction: f32,
) -> Task<Message> {
    if band >= BAND_COUNT || direction == 0.0 || direction.is_nan() {
        return Task::none();
    }
    mutate_equalizer(player, |state| {
        let gain = state.eq_bands()[band] + direction.signum() * views::EQ_STEP;
        state.set_eq_band(band, gain)
    })
}

/// A band slider's drag.
pub(in crate::ui) fn eq_band_change(
    player: &mut WinampPlayer,
    band: usize,
    gain: f32,
) -> Task<Message> {
    if band < BAND_COUNT {
        player.mixer_drag = Some(views::MixerDrag::EqBand(band));
    }
    mutate_equalizer(player, |state| state.set_eq_band(band, gain))
}

/// A press on an equalizer dB scale label. It sets no `mixer_drag`: a click
/// has no release to clear an LCD readout.
pub(in crate::ui) fn eq_bands_set_all(player: &mut WinampPlayer, gain: f32) -> Task<Message> {
    mutate_equalizer(player, |state| state.set_all_eq_bands(gain))
}

/// A preset pick from the equalizer's preset list.
pub(in crate::ui) fn eq_preset_selected(
    player: &mut WinampPlayer,
    preset: Preset,
) -> Task<Message> {
    mutate_equalizer(player, |state| state.apply_eq_preset(preset))
}

/// The shared-state setting a mixer handler writes and then forwards to the
/// audio backend.
///
/// Naming the three as a closed enum makes [`mutate_and_forward`] own both the
/// forward call and its failure report, so a helper cannot pair one setting's
/// label with another setting's backend method, and a misspelled label cannot
/// compile. A failure names the setting and the clamped value it tried to send.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MixerSetting {
    Volume,
    Balance,
    Equalizer,
}

impl MixerSetting {
    /// Sends this setting's current clamped value from `state` to `service`.
    fn forward(self, service: &AppleMusicService, state: &AppState) -> Result<(), AppleMusicError> {
        match self {
            Self::Volume => service.set_output_volume(state.volume()),
            Self::Balance => service.set_output_balance(state.balance()),
            Self::Equalizer => service.set_output_equalizer(state.eq_snapshot()),
        }
    }

    /// The report phrase naming this setting and the value `state` holds, so a
    /// failed forward says what the preview was being set to, not only that the
    /// forward failed.
    fn description(self, state: &AppState) -> String {
        match self {
            Self::Volume => {
                format!("forwarding volume {} to the audio backend", state.volume())
            }
            Self::Balance => {
                format!(
                    "forwarding balance {} to the audio backend",
                    state.balance()
                )
            }
            Self::Equalizer => format!(
                "forwarding equalizer {:?} to the audio backend",
                state.eq_snapshot()
            ),
        }
    }
}

/// Formats the report for a rejected mixer forward: names the setting and the
/// clamped value that was sent, plus the backend's cause. Pure, like the seek
/// and transport report helpers, so the contract is testable without capturing
/// stderr.
fn forward_failure_report(
    setting: MixerSetting,
    state: &AppState,
    error: &impl std::fmt::Display,
) -> String {
    format!("{} failed: {error}", setting.description(state))
}

/// Locks the shared playback state, applies `mutation`, forwards the resulting
/// clamped value to the audio backend, and returns no task.
///
/// The volume, balance, and equalizer helpers below all repeat the same
/// sequence — `blocking_lock`, one mutation, read the clamped value back
/// under the lock, then one non-blocking channel send to the audio backend —
/// so the shape lives here once and each helper names only its setting. The
/// state write and the forward happen under the one lock, so a slider drag
/// stays responsive and its messages stay ordered. A backend error is logged;
/// the state write has already committed.
fn mutate_and_forward(
    player: &WinampPlayer,
    setting: MixerSetting,
    mutation: impl FnOnce(&mut AppState),
) -> Task<Message> {
    let mut state = player.state.blocking_lock();
    mutation(&mut state);
    if let Err(error) = setting.forward(&player.apple_music_service, &state) {
        eprintln!("{}", forward_failure_report(setting, &state, &error));
    }
    Task::none()
}

/// Forwards the mutated state's clamped volume to the audio backend. The
/// volume handlers — the slider's drag and the two arrow keys — all use this
/// so a slider drag reaches the playing preview without restarting it; the
/// lock-mutate-forward shape is [`mutate_and_forward`]'s.
fn mutate_volume(player: &WinampPlayer, mutation: impl FnOnce(&mut AppState)) -> Task<Message> {
    mutate_and_forward(player, MixerSetting::Volume, mutation)
}

/// Forwards the mutated state's clamped balance to the audio backend. The
/// balance slider's drag uses this so a drag reaches the playing preview
/// without restarting it; the lock-mutate-forward shape is
/// [`mutate_and_forward`]'s.
fn mutate_balance(player: &WinampPlayer, mutation: impl FnOnce(&mut AppState)) -> Task<Message> {
    mutate_and_forward(player, MixerSetting::Balance, mutation)
}

/// Forwards a relative seek to the audio backend and returns no task. The
/// left and right arrow handlers use this so a key press moves the playing
/// preview without touching shared state: the backend owns the position, and
/// `AppleMusicService::seek_output` is a non-blocking channel send. A
/// backend error is logged.
fn seek_preview(player: &WinampPlayer, offset_secs: i64) -> Task<Message> {
    if let Err(error) = player.apple_music_service.seek_output(offset_secs) {
        eprintln!("{}", seek_by_failure_report(offset_secs, &error));
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
    mutate_and_forward(player, MixerSetting::Equalizer, mutation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forward_failure_report_names_the_setting_and_the_value_it_sent() {
        let mut state = AppState::default();
        state.set_volume(0.25);
        assert_eq!(
            forward_failure_report(MixerSetting::Volume, &state, &"the audio device is gone"),
            "forwarding volume 0.25 to the audio backend failed: the audio device is gone"
        );

        state.set_balance(-0.5);
        assert_eq!(
            forward_failure_report(MixerSetting::Balance, &state, &"the audio device is gone"),
            "forwarding balance -0.5 to the audio backend failed: the audio device is gone"
        );

        state.set_eq_preamp(3.0);
        let equalizer =
            forward_failure_report(MixerSetting::Equalizer, &state, &"the audio device is gone");
        assert!(
            equalizer.contains("forwarding equalizer EqSnapshot {"),
            "{equalizer}"
        );
        assert!(equalizer.contains("preamp_db: 3.0"), "{equalizer}");
        assert!(
            equalizer.ends_with(" failed: the audio device is gone"),
            "{equalizer}"
        );
    }
}
