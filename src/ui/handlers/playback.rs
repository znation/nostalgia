//! The playback message handlers: the transport buttons and keys, the play
//! request and its completion, the elapsed/remaining readout toggle, and the
//! Now Playing marquee tick. The Next/Previous and jump stepping live in the
//! [`stepping`] submodule, and the loaded preview's backend polls and seek
//! drag live in [`preview`].
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
use crate::ui::views;
use crate::ui::{CurrentView, Message, WinampPlayer};

pub(in crate::ui) mod preview;
pub(in crate::ui) mod stepping;

/// The system media Play/Pause key: toggles the preview between playing and
/// paused. A track with no loaded preview stops instead, since there is
/// nothing to pause. The transport row's Play and Pause buttons use the
/// explicit [`play`] and [`pause`] instead.
pub(in crate::ui) fn play_pause(player: &mut WinampPlayer) -> Task<Message> {
    begin_position_transition(player);
    transport_into(
        &player.apple_music_service,
        "toggle play/pause",
        |service| async move { service.toggle_play_pause().await },
    )
}

/// The transport row's Play button and the keyboard's X key: an explicit play
/// rather than the media key's toggle. Play resumes (or restarts) the preview.
pub(in crate::ui) fn play(player: &mut WinampPlayer) -> Task<Message> {
    begin_position_transition(player);
    transport_into(&player.apple_music_service, "play", |service| async move {
        service.resume().await
    })
}

/// The transport row's Pause button and the keyboard's C key: an explicit
/// pause rather than the media key's toggle. Pause pauses a loaded preview;
/// with none loaded it stops the transport, since there is nothing to pause.
pub(in crate::ui) fn pause(player: &mut WinampPlayer) -> Task<Message> {
    begin_position_transition(player);
    transport_into(&player.apple_music_service, "pause", |service| async move {
        service.pause().await
    })
}

/// Stop discards the preview and clears the playing and paused flags; the
/// service remembers the last preview URL so Play can restart it. The elapsed
/// readout clears immediately, and any poll in flight is invalidated so it
/// cannot bring the old position back.
pub(in crate::ui) fn stop(player: &mut WinampPlayer) -> Task<Message> {
    reset_position_readout(player);
    begin_position_transition(player);
    transport_into(&player.apple_music_service, "stop", |service| async move {
        service.stop().await
    })
}

/// Ctrl+V: flips the "Stop after current" one-shot flag. It starts no
/// transport and changes no playback or window state; the next natural preview
/// end reads the flag.
pub(in crate::ui) fn toggle_stop_after_current(player: &mut WinampPlayer) -> Task<Message> {
    player.stop_after_current = !player.stop_after_current;
    Task::none()
}

/// The Repeat toggle: flips the shared repeat flag.
pub(in crate::ui) fn toggle_repeat(player: &mut WinampPlayer) -> Task<Message> {
    mutate_state(player, AppState::toggle_repeat)
}

/// The Shuffle toggle: flips the shared shuffle flag.
pub(in crate::ui) fn toggle_shuffle(player: &mut WinampPlayer) -> Task<Message> {
    mutate_state(player, AppState::toggle_shuffle)
}

/// The LCD readout's click: flips the large left-hand time between elapsed and
/// remaining. It touches no playback state and clears no stored position, so
/// the chosen mode survives transports and a new selection.
pub(in crate::ui) fn toggle_time_display(player: &mut WinampPlayer) -> Task<Message> {
    player.time_display = player.time_display.toggled();
    Task::none()
}

/// The indicator strip's V control: flips whether the LCD draws the stored
/// spectrum. It touches no playback state, clears no stored spectrum, and
/// schedules no work, so the poll keeps updating the bars while they are
/// hidden and the next show draws the current ones.
pub(in crate::ui) fn toggle_visualizer(player: &mut WinampPlayer) -> Task<Message> {
    player.visualizer_visible = !player.visualizer_visible;
    Task::none()
}

/// The play request: the message carries the pressed row's index plus the
/// epoch of the list that rendered it. The Songs view draws from `songs`
/// and the flat All Songs view draws from `all_songs`, so resolve the press
/// against the list the current view shows; resolving it against the other
/// list would no-op or play the wrong song. A mismatched epoch means the
/// list was replaced after the row was rendered, so the index names a
/// different song now — ignore the stale press. An out-of-range index
/// likewise names no track and is a no-op rather than a bogus play. `select`
/// applies both checks. Once the song resolves, an enabled AUTO applies the
/// preset named after its genre before the play request is issued.
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
    // Cloned before the AUTO call, which borrows the whole player while
    // `song` still borrows its list.
    let genre = song.genre.clone();
    // A newly selected song starts its title at character zero; the
    // previous song's scroll position must not carry over.
    player.marquee_offset = 0;
    // Record the played track's playlist number, title, artist, album, and
    // formatted length before handing the id to the async play: the Now
    // Playing bar resolves its number, label, artist, and time from
    // `known_tracks`, and
    // the entry must survive a later browse to a different list (which
    // replaces the visible song buffer). Recording once per play — rather
    // than folding every song of every loaded list into the map — keeps the
    // index proportional to songs actually played and takes the fold off the
    // browse path. The number is the 1-based position of the row that
    // selected the track, so re-playing the same id from another row or list
    // refreshes it alongside the other cached fields rather than keeping the
    // old number.
    player.known_tracks.insert(
        track_id.clone(),
        views::KnownTrack {
            title: song.title.clone(),
            artist: song.artist.clone(),
            album: song.album.clone(),
            year: song.year,
            genre: song.genre.clone(),
            track_number: song.track_number,
            number: format!("{}. ", index + 1),
            time: views::format_track_time(song.duration_ms),
            duration_ms: song.duration_ms,
        },
    );
    // A new selection starts with no elapsed position. Bumping the epoch
    // drops a poll reply issued for the previous track.
    reset_position_readout(player);
    // The play below replaces the loaded preview, so the poll waits for its
    // completion before it queries the backend.
    begin_position_transition(player);
    // A newer selection must win even when an older play's backend
    // call finishes later: issue a generation and hand its guard to
    // `play_track`, which evaluates it under the state lock so the
    // older play cannot overwrite the newer track. The browse path
    // uses the same `RequestGeneration` for the same reason.
    let generation = RequestGeneration::issue(&player.plays_generation);
    // The genre-matched AUTO preset reaches the same mutate-and-forward path
    // as a pick-list selection, so a running preview's filters update with
    // the stored curve before the new track replaces it.
    let _ = super::mixer::apply_auto_preset(player, &genre);
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

/// The play completion: it ends the transition the selection began, so the
/// position poll may query the backend again. Only the current track's entry
/// is ever read (see
/// `WinampPlayer::known_tracks`), so the latest play's completion drops
/// every other entry the selections above accumulated. A completion whose
/// play a newer selection superseded is skipped: its `current_track` names
/// the older committed track, but the newer selection's entry is still
/// pending, and pruning to the older track would delete it. When this
/// completion is the latest, no newer selection is pending, so reducing to
/// `current_track` is safe — and when nothing committed, clearing the map
/// keeps it bounded rather than leaving the failed selection's entry.
///
/// `committed` is the play future's own outcome, not read back from shared
/// state: a Pause or Stop processed after the play committed but before this
/// completion reaches `update` clears `is_playing` (and Pause's `paused` flag
/// is gone again after a Stop), so the flags cannot tell a committed-then-
/// paused play from a failed one. Only a committed play enters the history;
/// a failed one keeps a pending history jump available for retry.
pub(in crate::ui) fn track_played(
    player: &mut WinampPlayer,
    generation: u64,
    committed: bool,
) -> Task<Message> {
    end_position_transition(player);
    if generation == player.plays_generation.load(Ordering::SeqCst) {
        let state = player.state.blocking_lock();
        if committed && let Some(current) = state.current_track.as_deref() {
            player.play_history.record(current);
        } else if !committed {
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

/// The marquee timer only advances the title window, and it runs only while
/// the line is too long to fit the LCD's character budget (see
/// `crate::ui::subscriptions::subscriptions`), so every tick scrolls the bar.
pub(in crate::ui) fn marquee_tick(player: &mut WinampPlayer) -> Task<Message> {
    player.marquee_offset = player.marquee_offset.wrapping_add(1);
    Task::none()
}

/// A Play/Pause/Stop transition finished (or failed; the failure was
/// logged). It carries no payload — the service already wrote shared state —
/// and exists so iced re-renders the Now Playing bar after the asynchronous
/// transition, exactly as `TrackPlayed` re-renders after a play. It also ends
/// the transition the transport arm began, so the position poll may query the
/// backend again.
pub(in crate::ui) fn transport_settled(player: &mut WinampPlayer) -> Task<Message> {
    end_position_transition(player);
    Task::none()
}

/// Clears the Now Playing elapsed readout, the LCD status row's reported
/// format, the visualizer's bars, and the seek bar's scale and drag, and
/// invalidates any position, format, duration, or spectrum query already in
/// flight, so a reply issued before the track changed cannot restore the
/// previous values. Stop and a new selection call this; a Pause deliberately
/// does not, so the paused
/// player's frozen position, format, bars, and seek bar stay on screen. The
/// caller pairs it with [`begin_position_transition`], so the polls also wait
/// until the backend has stopped or replaced the player.
fn reset_position_readout(player: &mut WinampPlayer) {
    player.position_epoch = player.position_epoch.wrapping_add(1);
    player.playback_position = None;
    player.playback_format = None;
    player.playback_duration = None;
    // The Jump to time panel needs a known length (the view hides it without
    // one), so it closes with the length instead of lingering invisibly.
    player.jump_time_visible = false;
    player.spectrum = None;
    player.seek_preview = None;
}

/// Marks a backend transport transition as in flight, so the position poll
/// waits for it instead of querying a backend that still holds the previous
/// player. Every handler that schedules a transport call begins one, and the
/// completion that reports that transition ends it: the pairing keeps the
/// count above zero for exactly as long as some transition is outstanding.
fn begin_position_transition(player: &mut WinampPlayer) {
    player.pending_position_transitions = player.pending_position_transitions.saturating_add(1);
}

/// Ends the transition [`begin_position_transition`] began.
fn end_position_transition(player: &mut WinampPlayer) {
    player.pending_position_transitions = player.pending_position_transitions.saturating_sub(1);
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
