//! The track stepping handlers: the Next/Previous steps through the showing
//! list (including the Previous restart rule), the Ctrl+Z/Ctrl+B list-edge
//! jumps, and the keypad ten-song jumps. The transport buttons, the play
//! request and its completion, and the readout and marquee handlers live in
//! the parent module; the loaded preview's backend polls live in `preview`.
//!
//! Every step schedules the landed row as a `TrackSelected` message and asks
//! the browse list to reveal it; the list and Shuffle/Repeat arithmetic come
//! from `crate::ui::transport`.

use std::time::Duration;

use iced::Task;

use crate::library::Song;
use crate::ui::transport;
use crate::ui::{CurrentView, Message, WinampPlayer};

/// The position at which Previous restarts the loaded preview instead of
/// stepping to the preceding song, matching classic Winamp's three-second
/// rule.
const PREVIOUS_RESTART_SECS: u64 = 3;

/// The song list the current view draws from, with the epoch that rendered it:
/// the flat All Songs view's `all_songs`, otherwise the browsed `songs`. The
/// preview-end advance resolves its chosen id against this same buffer, so the
/// scheduled `TrackSelected` index and epoch match the list the view drew.
/// `step_track` selects the same list inline, because its backward branch also
/// borrows `play_history` mutably and a slice borrowed through this helper
/// would hold the whole player.
pub(super) fn showing_songs(player: &WinampPlayer) -> (&[Song], u64) {
    if player.current_view == CurrentView::AllSongs {
        (&player.all_songs.items, player.all_songs.epoch)
    } else {
        (&player.songs.items, player.songs.epoch)
    }
}

/// The id a forward step selects from `items`: with `shuffle` on, a random
/// non-current song via `transport::shuffled_track_id`; otherwise the
/// `transport::next_track_id` successor, which wraps at the end only when
/// `repeat` is on. Next and the preview-end advance share this choice, so a
/// preview that runs out steps exactly where the Next button would. Pure, and
/// taking the list and flags rather than a player keeps it from locking state
/// while a caller already holds that lock.
pub(super) fn forward_track_id(
    items: &[Song],
    current: Option<&str>,
    shuffle: bool,
    repeat: bool,
) -> Option<String> {
    if shuffle {
        transport::shuffled_track_id(items, current, transport::shuffle_roll())
    } else {
        transport::next_track_id(items, current, repeat)
    }
}

/// The task the Next/Previous buttons schedule: step the current track
/// through the list the current view shows — `all_songs` for the flat All
/// Songs view, the browsed `songs` otherwise — in the given direction — with
/// Shuffle on a forward step picks a random non-current song via
/// `transport::shuffled_track_id`, and a backward step first retraces the
/// recorded play history via `PlayHistory::previous`, falling back to
/// `transport::previous_track_id` when no recorded track is still visible;
/// otherwise `forward` selects the
/// `transport::next_track_id`/`previous_track_id` stepping function — and
/// schedule the landed song as `TrackSelected`, or no task when there is
/// nothing to step through. `forward` selects the direction so the stepping
/// arithmetic stays in `transport`, and the lock-then-dispatch wiring lives
/// here once for both transport arms. The shared
/// Shuffle and Repeat flags are read under the same lock, so the
/// random-vs-sequential and wrap-vs-stop-at-the-edge decisions (see
/// `transport::shuffled_track_id` and `transport::next_track_id`) stay in
/// `transport` too.
///
/// A backward step first applies classic Winamp's restart rule: when the
/// backend still holds a loaded preview whose last reported position — the
/// one the readout shows, not a fresh backend query — is at least
/// [`PREVIOUS_RESTART_SECS`], it restarts that preview from zero instead of
/// stepping. The restart branch runs before the Shuffle/Repeat arithmetic, so
/// a late Previous neither steps nor consumes a recorded-play history entry.
/// Earlier positions, no loaded preview, and no reported position fall
/// through to the step unchanged.
pub(in crate::ui) fn step_track(player: &mut WinampPlayer, forward: bool) -> Task<Message> {
    if !forward
        && player.apple_music_service.has_loaded_preview()
        && let Some(position) = player
            .playback_position
            .filter(|position| *position >= Duration::from_secs(PREVIOUS_RESTART_SECS))
    {
        player.seek_preview = None;
        match player.apple_music_service.seek_output_to(Duration::ZERO) {
            Ok(()) => {
                player.position_epoch = player.position_epoch.wrapping_add(1);
                player.playback_position = Some(Duration::ZERO);
            }
            Err(error) => {
                let track = player.state.blocking_lock().current_track.clone();
                eprintln!(
                    "{}",
                    restart_failure_report(track.as_deref(), position, &error)
                );
            }
        }
        return Task::none();
    }

    // The stepped song is computed under the state lock, borrowing the
    // current track directly rather than cloning it to satisfy `as_deref`.
    // The lock covers the pure stepping scan, which is fast, and the
    // transport functions never lock anything themselves.
    let state = player.state.blocking_lock();
    // Step the list the current view draws from, so the scheduled
    // `TrackSelected` index and epoch match the list the arm resolves
    // against: the flat All Songs view's `all_songs`, otherwise `songs`.
    let (items, epoch) = if player.current_view == CurrentView::AllSongs {
        (&player.all_songs.items, player.all_songs.epoch)
    } else {
        (&player.songs.items, player.songs.epoch)
    };
    let stepped = if forward {
        forward_track_id(
            items,
            state.current_track.as_deref(),
            state.shuffle,
            state.repeat,
        )
    } else if state.shuffle {
        player
            .play_history
            .previous(items, state.current_track.as_deref())
            .or_else(|| {
                transport::previous_track_id(items, state.current_track.as_deref(), state.repeat)
            })
    } else {
        transport::previous_track_id(items, state.current_track.as_deref(), state.repeat)
    };
    match stepped {
        // `transport` answers with the stepped song's id, but the selection
        // message carries the song's index and the epoch of the list that
        // rendered the row (see `TrackSelected`), so resolve the id to its
        // position in this same buffer. This scan runs once per button press,
        // not per view refresh, and the stepped id always names a song in
        // this buffer.
        Some(track_id) => match items.iter().position(|song| song.id == track_id) {
            Some(index) => Task::batch([
                Task::done(Message::TrackSelected { epoch, index }),
                // Keep the landed row in view: Next and Previous (including
                // Shuffle's retrace) can step outside the visible list.
                super::super::browse::scroll::scroll_browse_to_playing_row(player, index),
            ]),
            None => Task::none(),
        },
        None => Task::none(),
    }
}

/// The report for a rejected Previous restart: names the track whose preview
/// was being restarted and the position that triggered the restart, so a
/// failed seek identifies the selection rather than only saying a seek failed.
/// `track_id` is `None` only when shared state names no current track; the
/// loaded preview still gets a report. The id is `Debug`-escaped, matching the
/// play log, so a hostile library id cannot inject terminal output.
pub(in crate::ui) fn restart_failure_report(
    track_id: Option<&str>,
    position: Duration,
    error: &impl std::fmt::Display,
) -> String {
    let target = track_id.map_or_else(
        || "the loaded preview".to_string(),
        |id| format!("track {id:?}"),
    );
    format!("restarting {target} after {position:?} failed: {error}")
}

/// The task Ctrl+Z and Ctrl+B schedule: play the first (`last` false) or last
/// song of the list the current view shows, ignoring Shuffle, Repeat, the
/// current track, and the Previous restart rule, and scroll the list to that
/// row like `step_track`. An empty list schedules nothing. It reads no shared
/// state and leaves `play_history` alone.
pub(in crate::ui) fn jump_to_edge_track(player: &mut WinampPlayer, last: bool) -> Task<Message> {
    let (items, epoch) = showing_songs(player);
    let Some(final_index) = items.len().checked_sub(1) else {
        return Task::none();
    };
    let index = if last { final_index } else { 0 };
    Task::batch([
        Task::done(Message::TrackSelected { epoch, index }),
        super::super::browse::scroll::scroll_browse_to_playing_row(player, index),
    ])
}

/// The task keypad 1 (`forward` false) and 3 schedule: play the song ten
/// places before or after the current track in the list the current view
/// shows, clamped to its first or last song, and scroll the list to that row.
/// Like `jump_to_edge_track` it ignores Shuffle, Repeat, and the Previous
/// restart rule and leaves `play_history` alone; an empty list schedules
/// nothing.
pub(in crate::ui) fn jump_ten_tracks(player: &mut WinampPlayer, forward: bool) -> Task<Message> {
    let (items, epoch) = showing_songs(player);
    let index = {
        let state = player.state.blocking_lock();
        transport::jumped_track_index(items, state.current_track.as_deref(), forward)
    };
    let Some(index) = index else {
        return Task::none();
    };
    Task::batch([
        Task::done(Message::TrackSelected { epoch, index }),
        super::super::browse::scroll::scroll_browse_to_playing_row(player, index),
    ])
}
