//! The Now Playing preview's backend telemetry: the one-second poll for the
//! loaded preview's position, sample rate and channel mode, length, and
//! finished state; the live spectrum poll; and the seek drag and release.
//!
//! Every query runs in a task and every reply carries the epoch it was issued
//! under, and it is dropped while a transport transition is still in flight.
//! The transport handlers in the parent module begin and end those
//! transitions, and the preview-end advance reuses their stepping and Stop
//! paths.

use std::fmt::Display;
use std::time::Duration;

use iced::Task;

use crate::state::TransportMode;
use crate::ui::{Message, WinampPlayer, views};

use super::stepping::{forward_track_id, showing_songs};
use super::stop;

/// The position poll's tick: schedule a query of the loaded preview's
/// position and of whether it has reached its end; while its sample rate and
/// channel mode or its own length is still unknown, also query that value. It
/// also advances the paused readout's blink phase, reusing this one-second
/// poll rather than adding a timer. Every query runs in a task, so the UI
/// thread never blocks on the audio backend, and each reply carries the epoch
/// the query was issued under, so a transition that lands first invalidates
/// it. While a transport transition is still reaching the backend, none is
/// issued: the backend would still report the previous preview, and the epoch
/// no longer identifies it.
pub(in crate::ui) fn position_tick(player: &mut WinampPlayer) -> Task<Message> {
    if player.pending_position_transitions > 0 {
        return Task::none();
    }
    let mode = player.state.blocking_lock().transport_mode();
    player.blink_visible = if mode == TransportMode::Paused {
        !player.blink_visible
    } else {
        true
    };
    let epoch = player.position_epoch;
    let position_service = player.apple_music_service.clone();
    let position = Task::perform(
        async move { position_service.playback_position() },
        move |position| Message::PlaybackPosition { epoch, position },
    );
    let mut queries = vec![position];
    // The format and preview length are read from the decoded source when the
    // preview is appended and do not change while that player lives; a Stop or
    // a new selection clears both through `reset_position_readout`. Query each
    // only while its value is missing, so a steady preview polls the backend
    // for the position and finished state alone: every reply rebuilds the
    // whole screen, and those replies would rebuild it for a value that cannot
    // have changed. A slow preview load, or a source that cannot supply one of
    // them, leaves that value missing, so the next tick retries until the
    // backend can answer.
    if player.playback_format.is_none() {
        let format_service = player.apple_music_service.clone();
        queries.push(Task::perform(
            async move { format_service.playback_format() },
            move |format| Message::PlaybackFormat { epoch, format },
        ));
    }
    if player.playback_duration.is_none() {
        let duration_service = player.apple_music_service.clone();
        queries.push(Task::perform(
            async move { duration_service.playback_duration() },
            move |duration| Message::PlaybackDuration { epoch, duration },
        ));
    }
    let finished_service = player.apple_music_service.clone();
    queries.push(Task::perform(
        async move { finished_service.preview_finished() },
        move |finished| Message::PlaybackFinished { epoch, finished },
    ));
    Task::batch(queries)
}

/// Whether a poll reply stamped `epoch` still answers the current preview and
/// may be stored: its epoch is the current one, so a reply in flight across a
/// Stop or a new selection cannot restore the previous preview's data, and no
/// transport transition is in flight, since the backend may still be holding
/// the previous player. Shared by every preview reply handler so the rule is
/// named once rather than repeated per field.
fn reply_is_current(player: &WinampPlayer, epoch: u64) -> bool {
    player.pending_position_transitions == 0 && epoch == player.position_epoch
}

/// The position query's reply: store the reported position, or clear the
/// readout when the backend reports no position. A reply that
/// [`reply_is_current`] rejects is dropped, so a poll in flight across a Stop
/// or a new selection cannot restore the previous track's elapsed value, and
/// neither can one that lands while a transport transition is still in
/// flight.
pub(in crate::ui) fn playback_position(
    player: &mut WinampPlayer,
    epoch: u64,
    position: Option<Duration>,
) -> Task<Message> {
    if reply_is_current(player, epoch) {
        player.playback_position = position;
    }
    Task::none()
}

/// The format query's reply: store the reported sample rate and channel mode,
/// or clear them so the status row drops its prefix and dims both channel
/// labels when the backend reports no format. A reply that
/// [`reply_is_current`] rejects is dropped, so a poll in flight across a Stop
/// or a new selection cannot restore the previous preview's format, and
/// neither can one that lands while a transport transition is still in flight.
pub(in crate::ui) fn playback_format(
    player: &mut WinampPlayer,
    epoch: u64,
    format: Option<crate::audio::PlaybackFormat>,
) -> Task<Message> {
    if reply_is_current(player, epoch) {
        player.playback_format = format;
    }
    Task::none()
}

/// The duration query's reply: store the loaded preview's own length, or
/// clear the seek bar's scale (and close the Jump to time panel) when the
/// backend reports none. A reply that
/// [`reply_is_current`] rejects is dropped, so a poll in flight across a Stop
/// or a new selection cannot scale the bar to the previous preview, and
/// neither can one that lands while a transport transition is still in flight.
pub(in crate::ui) fn playback_duration(
    player: &mut WinampPlayer,
    epoch: u64,
    duration: Option<Duration>,
) -> Task<Message> {
    if reply_is_current(player, epoch) {
        player.playback_duration = duration;
        // The Jump to time panel needs a known length (the view hides it
        // without one), so it closes instead of lingering invisibly.
        if duration.is_none() {
            player.jump_time_visible = false;
        }
    }
    Task::none()
}

/// The finished-state query's reply: a loaded preview that reached its end
/// while playing advances exactly as the Next button would. The showing list
/// and the Shuffle/Repeat flags are read under one lock, [`forward_track_id`]
/// picks the song, and its id resolves back to an index in that same list, so
/// the scheduled `TrackSelected` names the row the view drew. When there is no
/// next song — Repeat off at the last one, where `transport::next_track_id`
/// re-lands on the current song, or a Shuffle list with no other song than the
/// current one — or the picked id is no longer in the list, it takes the
/// existing [`stop`] path, so playback becomes stopped and the elapsed
/// readout, status row's reported format, visualizer, and seek bar clear
/// together while `current_track` keeps naming the song for Play. With Repeat
/// on and Shuffle
/// off, a wrap back to the same song is a step: even a one-song list repeats.
/// A reply that is not finished, is stale, arrives while a transition is in
/// flight, or lands while paused or stopped is dropped: Pause and Stop never
/// advance. A natural end is a forward step, not a Previous, so it records
/// nothing in `PlayHistory`. When `stop_after_current` is armed, a reply that
/// passes those checks takes the [`stop`] path instead of picking a song, and
/// disarms the flag; a dropped reply leaves it armed. With Shuffle on and
/// Repeat off, a reply on the song that completed the Shuffle round also
/// takes the [`stop`] path; Next and Repeat on start a new round.
pub(in crate::ui) fn playback_finished(
    player: &mut WinampPlayer,
    epoch: u64,
    finished: bool,
) -> Task<Message> {
    if !finished || !reply_is_current(player, epoch) {
        return Task::none();
    }
    let (stepped, list_epoch) = {
        let state = player.state.blocking_lock();
        if !state.is_playing {
            return Task::none();
        }
        if player.stop_after_current {
            drop(state);
            player.stop_after_current = false;
            return stop(player);
        }
        if state.shuffle
            && !state.repeat
            && player
                .shuffle_round
                .ends_round(state.current_track.as_deref())
        {
            drop(state);
            return stop(player);
        }
        let (items, list_epoch) = showing_songs(player);
        let current = state.current_track.as_deref();
        let picked = forward_track_id(
            items,
            current,
            state.shuffle,
            state.repeat,
            &player.shuffle_round,
        );
        // A pick that is still the current song is a step only when Repeat
        // wraps a one-song list back to itself. With Repeat off, Next
        // re-lands on the last song at the end; with Shuffle on, a list with
        // no other song can only pick itself. Both stop rather than replay.
        let same_song_steps = state.repeat && !state.shuffle;
        let index = picked
            .filter(|track_id| same_song_steps || Some(track_id.as_str()) != current)
            .and_then(|track_id| items.iter().position(|song| song.id == track_id));
        (index, list_epoch)
    };
    match stepped {
        Some(index) => Task::batch([
            Task::done(Message::TrackSelected {
                epoch: list_epoch,
                index,
            }),
            // A natural preview end follows the advanced row exactly as Next
            // does, so keep that row in view too.
            super::super::browse::scroll::scroll_browse_to_playing_row(player, index),
        ]),
        None => stop(player),
    }
}

/// The spectrum poll's tick: schedule a query of the loaded preview's live
/// ten-band spectrum. The query runs in a task, so the UI thread never blocks
/// on the audio backend, and the reply carries the epoch the query was issued
/// under, so a transition that lands first invalidates it. While a transport
/// transition is still reaching the backend, none is issued: the backend would
/// still report the previous preview.
pub(in crate::ui) fn spectrum_tick(player: &mut WinampPlayer) -> Task<Message> {
    if player.pending_position_transitions > 0 {
        return Task::none();
    }
    let epoch = player.position_epoch;
    let service = player.apple_music_service.clone();
    Task::perform(
        async move { service.playback_spectrum() },
        move |spectrum| Message::Spectrum { epoch, spectrum },
    )
}

/// The spectrum query's reply: advance the bars' falloff and the peak
/// markers' hold-and-fall levels, or clear both when the backend reports no spectrum. A reply that
/// [`reply_is_current`] rejects is dropped, so a poll in flight across a Stop
/// or a new selection cannot restore the previous preview's bars, and neither
/// can one that lands while a transport transition is still in flight.
pub(in crate::ui) fn spectrum(
    player: &mut WinampPlayer,
    epoch: u64,
    spectrum: Option<crate::audio::Spectrum>,
) -> Task<Message> {
    if reply_is_current(player, epoch) {
        player.spectrum_peaks = views::next_spectrum_peaks(player.spectrum_peaks, spectrum);
        player.spectrum = views::next_spectrum_bars(player.spectrum, spectrum);
    }
    Task::none()
}

/// The seek bar's drag: store the slider value as the in-progress thumb
/// position without touching the backend. Dragging must not seek on every
/// step, so only [`seek_preview_released`] reaches the audio seam.
pub(in crate::ui) fn seek_preview_changed(player: &mut WinampPlayer, value: f32) -> Task<Message> {
    player.seek_preview = Some(Duration::from_secs_f32(value.max(0.0)));
    Task::none()
}

/// The seek bar's release: seek once to the stored drag, record that position
/// optimistically, and invalidate any position or duration reply already in
/// flight. A backend failure is logged and the drag cleared, so the next poll
/// restores the real position and length. A release with no drag stored only
/// clears the field.
pub(in crate::ui) fn seek_preview_released(player: &mut WinampPlayer) -> Task<Message> {
    let Some(position) = player.seek_preview.take() else {
        return Task::none();
    };
    seek_preview_to(player, position);
    Task::none()
}

/// Seeks the preview to `position`, records it optimistically, and invalidates
/// any position or duration reply already in flight. A backend failure is
/// logged and leaves the recorded position alone. Shared by the seek bar's
/// release and the Jump to time panel.
pub(in crate::ui) fn seek_preview_to(player: &mut WinampPlayer, position: Duration) {
    match player.apple_music_service.seek_output_to(position) {
        Ok(()) => {
            player.position_epoch = player.position_epoch.wrapping_add(1);
            player.playback_position = Some(position);
        }
        Err(error) => eprintln!("{}", seek_to_failure_report(position, &error)),
    }
}

/// The report for a rejected absolute seek: names the requested target and the
/// backend's cause, so a failed seek-bar release or Jump to time says where it
/// was trying to go instead of only that a seek failed. `error` is a `Display`
/// so the absolute and relative UI seek paths share one wording.
pub(in crate::ui) fn seek_to_failure_report(position: Duration, error: &impl Display) -> String {
    format!("seeking the preview to {position:?} failed: {error}")
}

/// The report for a rejected relative seek: names the requested offset and the
/// backend's cause. The UI knows only the distance and direction, not the
/// absolute target the backend would compute, so the offset is what the log
/// can name for a failed arrow-key or wheel nudge.
pub(in crate::ui) fn seek_by_failure_report(offset_secs: i64, error: &impl Display) -> String {
    format!("seeking the preview by {offset_secs}s failed: {error}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seek_to_failure_report_names_the_target_and_the_cause() {
        let report = seek_to_failure_report(Duration::from_secs(12), &"seek refused");
        assert_eq!(report, "seeking the preview to 12s failed: seek refused");
    }

    #[test]
    fn seek_by_failure_report_names_the_offset_and_the_cause() {
        let report = seek_by_failure_report(-5, &"seek refused");
        assert_eq!(report, "seeking the preview by -5s failed: seek refused");
    }
}
