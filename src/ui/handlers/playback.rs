//! The playback message handlers: the transport buttons and keys, the track
//! stepping, the play request and its completion, and the Now Playing
//! marquee tick.
//!
//! The transport calls go through the service's async seam (`transport_into`)
//! and the play request through `play_into`, so the shared playback state is
//! written by the service rather than by the UI thread.

use std::sync::atomic::Ordering;

use iced::Task;

use crate::state::AppState;
use crate::ui::loading::{
    PLAY_TIMEOUT, RequestGeneration, play_failure_report, play_into, transport_into,
};
use crate::ui::transport;
use crate::ui::views;
use crate::ui::{CurrentView, Message, WinampPlayer};

/// The Play/Pause button: toggles the preview between playing and paused.
pub(in crate::ui) fn play_pause(player: &mut WinampPlayer) -> Task<Message> {
    transport_into(
        &player.apple_music_service,
        "toggle play/pause",
        |service| async move { service.toggle_play_pause().await },
    )
}

/// The keyboard's X key: an explicit play rather than the button's toggle.
/// X resumes (or restarts) the preview.
pub(in crate::ui) fn play(player: &mut WinampPlayer) -> Task<Message> {
    transport_into(&player.apple_music_service, "play", |service| async move {
        service.resume().await
    })
}

/// The keyboard's C key: an explicit pause rather than the button's toggle.
/// C pauses the preview.
pub(in crate::ui) fn pause(player: &mut WinampPlayer) -> Task<Message> {
    transport_into(&player.apple_music_service, "pause", |service| async move {
        service.pause().await
    })
}

/// Stop discards the preview and clears the playing flag; the service
/// remembers the last preview URL so Play can restart it.
pub(in crate::ui) fn stop(player: &mut WinampPlayer) -> Task<Message> {
    transport_into(&player.apple_music_service, "stop", |service| async move {
        service.stop().await
    })
}

/// The Repeat toggle: flips the shared repeat flag.
pub(in crate::ui) fn toggle_repeat(player: &mut WinampPlayer) -> Task<Message> {
    mutate_state(player, AppState::toggle_repeat)
}

/// The Shuffle toggle: flips the shared shuffle flag.
pub(in crate::ui) fn toggle_shuffle(player: &mut WinampPlayer) -> Task<Message> {
    mutate_state(player, AppState::toggle_shuffle)
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
pub(in crate::ui) fn step_track(player: &mut WinampPlayer, forward: bool) -> Task<Message> {
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
    let stepped = if forward && state.shuffle {
        transport::shuffled_track_id(
            items,
            state.current_track.as_deref(),
            transport::shuffle_roll(),
        )
    } else if forward {
        transport::next_track_id(items, state.current_track.as_deref(), state.repeat)
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
            Some(index) => Task::done(Message::TrackSelected { epoch, index }),
            None => Task::none(),
        },
        None => Task::none(),
    }
}

/// The play request: the message carries the pressed row's index plus the
/// epoch of the list that rendered it. The Songs view draws from `songs`
/// and the flat All Songs view draws from `all_songs`, so resolve the press
/// against the list the current view shows; resolving it against the other
/// list would no-op or play the wrong song. A mismatched epoch means the
/// list was replaced after the row was rendered, so the index names a
/// different song now — ignore the stale press. An out-of-range index
/// likewise names no track and is a no-op rather than a bogus play. `select`
/// applies both checks.
pub(in crate::ui) fn track_selected(
    player: &mut WinampPlayer,
    epoch: u64,
    index: usize,
) -> Task<Message> {
    let selected = if player.current_view == CurrentView::AllSongs {
        player.all_songs.select(epoch, index)
    } else {
        player.songs.select(epoch, index)
    };
    let Some(song) = selected else {
        return Task::none();
    };
    let track_id = song.id.clone();
    let preview_url = song.preview_url.clone();
    // A newly selected song starts its title at character zero; the
    // previous song's scroll position must not carry over.
    player.marquee_offset = 0;
    // Record the played track's title, artist, and formatted length
    // before handing the id to the async play: the Now Playing bar
    // resolves its label, artist, and time from `known_tracks`, and the
    // entry must survive a later browse to a different list (which
    // replaces the visible song buffer). Recording once per play —
    // rather than folding every song of every loaded list into the map —
    // keeps the index proportional to songs actually played and takes the
    // fold off the browse path.
    player
        .known_tracks
        .entry(track_id.clone())
        .or_insert_with(|| views::KnownTrack {
            title: song.title.clone(),
            artist: song.artist.clone(),
            time: views::format_track_time(song.duration_ms),
        });
    // A newer selection must win even when an older play's backend
    // call finishes later: issue a generation and hand its guard to
    // `play_track`, which evaluates it under the state lock so the
    // older play cannot overwrite the newer track. The browse path
    // uses the same `RequestGeneration` for the same reason.
    let generation = RequestGeneration::issue(&player.plays_generation);
    let id_for_report = track_id.clone();
    play_into(
        &player.apple_music_service,
        track_id,
        generation,
        PLAY_TIMEOUT,
        move |service, id, generation| async move {
            service
                .play_track(&id, preview_url.as_deref(), || generation.is_current())
                .await
        },
        move |err| eprintln!("{}", play_failure_report(&id_for_report, err)),
    )
}

/// The play completion: only the current track's entry is ever read (see
/// `WinampPlayer::known_tracks`), so the latest play's completion drops
/// every other entry the selections above accumulated. A completion whose
/// play a newer selection superseded is skipped: its `current_track` names
/// the older committed track, but the newer selection's entry is still
/// pending, and pruning to the older track would delete it. When this
/// completion is the latest, no newer selection is pending, so reducing to
/// `current_track` is safe — and when nothing committed, clearing the map
/// keeps it bounded rather than leaving the failed selection's entry.
pub(in crate::ui) fn track_played(player: &mut WinampPlayer, generation: u64) -> Task<Message> {
    if generation == player.plays_generation.load(Ordering::SeqCst) {
        let state = player.state.blocking_lock();
        if state.is_playing
            && let Some(current) = state.current_track.as_deref()
        {
            player.play_history.record(current);
        } else {
            // A failed play leaves shared state naming the jump's
            // target (the backend leaves `current_track` set), so keep
            // the pending jump and mark it for retry: the next Previous
            // offers that same target rather than walking past it.
            player.play_history.retry_pending();
        }
        match state.current_track.as_deref() {
            Some(current) => player.known_tracks.retain(|id, _| id == current),
            None => player.known_tracks.clear(),
        }
    }
    Task::none()
}

/// The marquee timer only advances the title window; the view uses the
/// offset solely for an overlong line, so a fitting line stays still.
pub(in crate::ui) fn marquee_tick(player: &mut WinampPlayer) -> Task<Message> {
    player.marquee_offset = player.marquee_offset.wrapping_add(1);
    Task::none()
}

/// A Play/Pause/Stop transition finished (or failed; the failure was
/// logged). It carries no payload — the service already wrote shared state —
/// and exists so iced re-renders the Now Playing bar after the asynchronous
/// transition, exactly as `TrackPlayed` re-renders after a play. The handler
/// is a no-op.
pub(in crate::ui) fn transport_settled(_player: &mut WinampPlayer) -> Task<Message> {
    Task::none()
}

/// Locks the shared playback state, applies `mutation` to it, and returns no
/// task. The synchronous handlers — the Repeat and Shuffle toggles — repeat
/// the same shared-state update — `blocking_lock`, one mutation, then
/// `Task::none()` — so the lock-and-noop shape lives here once and each
/// handler only names its mutation. The transport arms and asynchronous work
/// (fetches, plays) go through the service instead (see `transport_into` and
/// `fetch_level`), and the volume, balance, and equalizer arms use the
/// mixer's own helpers because they also forward their value to the backend.
fn mutate_state(player: &WinampPlayer, mutation: impl FnOnce(&mut AppState)) -> Task<Message> {
    let mut state = player.state.blocking_lock();
    mutation(&mut state);
    Task::none()
}
