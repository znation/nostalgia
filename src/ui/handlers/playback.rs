//! The playback message handlers: the transport buttons and keys, the play
//! request and its completion, the elapsed/remaining readout toggle, and the
//! Now Playing marquee tick. The Next/Previous and jump stepping live in the
//! [`stepping`] submodule, the loaded preview's backend polls and seek drag
//! live in [`preview`], and the track-id-to-song resolutions through the
//! service's library seam live in [`resolve`].
//!
//! The transport calls go through the service's async seam (`transport_into`)
//! and the play request through `play_into`, so the shared playback state is
//! written by the service rather than by the UI thread.

use std::sync::atomic::Ordering;

use iced::Task;

use crate::library::Song;
use crate::state::AppState;
use crate::ui::loading::{
    PLAY_TIMEOUT, RequestGeneration, play_failure_report, play_into, transport_into,
};
use crate::ui::views;
use crate::ui::{CurrentView, Message, WinampPlayer};

pub(in crate::ui) mod fade;
pub(in crate::ui) mod preview;
pub(in crate::ui) mod resolve;
pub(in crate::ui) mod stepping;
pub(in crate::ui) mod title_drag;

/// The system media Play/Pause key: toggles the preview between playing and
/// paused. From stopped, it starts the service's remembered URL when one
/// exists, resolving the restored last-played track through the service seam
/// otherwise; a track with no loaded preview and no restore candidate stops
/// instead, since there is nothing to pause. The transport row's Play and
/// Pause buttons use the explicit [`play`] and [`pause`] instead.
pub(in crate::ui) fn play_pause(player: &mut WinampPlayer) -> Task<Message> {
    fade::cancel_fade_out(player);
    // The flag snapshot is advisory (see [`replay_candidate`]): a lock
    // the service is holding means a transition is mid-write, so the plain
    // toggle — which serializes under the same locks — is the safe branch.
    let stopped = !player
        .state
        .try_lock()
        .map_or(true, |state| state.is_playing);
    if stopped {
        return replay_from_stopped(player, "toggle play/pause");
    }
    begin_position_transition(player);
    transport_into(
        &player.apple_music_service,
        "toggle play/pause",
        |service| async move { service.toggle_play_pause().await },
    )
}

/// The transport row's Play button and the keyboard's X key: an explicit play rather than the media
/// key's toggle. A loaded (or reloadable) preview resumes or restarts, and a playing preview restarts
/// from its start. With neither a loaded preview nor the service's remembered URL — a fresh launch
/// that restored the last-played track's id — the saved id is resolved through the service seam and
/// replayed through [`begin_play`], so the Now Playing bar's track starts again from the beginning
/// without needing its row on screen.
pub(in crate::ui) fn play(player: &mut WinampPlayer) -> Task<Message> {
    fade::cancel_fade_out(player);
    replay_from_stopped(player, "play")
}

/// The decision behind both explicit Play arms: with the transport holding a
/// preview loaded or remembering the URL `stop` kept, Play runs the
/// service-side resume — begin a position transition first, so the polls wait
/// out the transition. With neither, the transport has nothing to resume
/// (the service's own "nothing to resume" branch would leave the flags
/// alone), so the shared state's current track id is resolved through the
/// service's library seam instead, and the reply replays it as a row press
/// would. `context` names the caller in the transport failure report.
fn replay_from_stopped(player: &mut WinampPlayer, context: &'static str) -> Task<Message> {
    match replay_candidate(player) {
        None => {
            begin_position_transition(player);
            transport_into(&player.apple_music_service, context, |service| async move {
                service.resume().await
            })
        }
        Some(track_id) => {
            let service = player.apple_music_service.clone();
            let resolution = resolve::resolve_replay_song(service, track_id.clone());
            Task::perform(resolution, move |song| Message::ReplayResolved {
                track_id,
                song,
            })
        }
    }
}

/// The id Play would replay, when the transport cannot answer a resume: no
/// preview is loaded and none is remembered, so the service's resume would
/// find nothing to sound. Falls back to none — meaning take the plain
/// transport path — when the shared state's lock is held, because a service
/// transition mid-write may be about to load a preview or remember a URL;
/// the transport path serializes its own decision under those locks.
fn replay_candidate(player: &WinampPlayer) -> Option<String> {
    if player.apple_music_service.has_loaded_preview()
        || player.apple_music_service.has_remembered_preview()
    {
        return None;
    }
    player
        .state
        .try_lock()
        .ok()
        .and_then(|state| state.current_track.clone())
}

/// The transport row's Pause button and the keyboard's C key. Pause pauses a
/// playing preview and resumes a paused one, like Winamp; with none loaded it
/// stops the transport, since there is nothing to pause. It differs from the
/// media key's toggle only while stopped, where it does not start playback.
pub(in crate::ui) fn pause(player: &mut WinampPlayer) -> Task<Message> {
    fade::cancel_fade_out(player);
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
    if player.fade_out_ticks > 0 {
        player.fade_out_ticks = 0;
        return fade::stop_restoring_volume(player);
    }
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

/// The Manual playlist advance toggle: flips the shared manual-advance flag.
pub(in crate::ui) fn toggle_manual_advance(player: &mut WinampPlayer) -> Task<Message> {
    mutate_state(player, AppState::toggle_manual_advance)
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

/// The OPTIONS menu's Autoscroll song name row: flips whether the Now Playing
/// title scrolls on its own. Off stops the marquee advance (the stopped
/// offset is kept, so turning it back on resumes from there) and leaves the
/// hand-scroll title drag and the LCD tick's mixer-flash and fade-out duties
/// untouched; it changes no playback state and schedules no work.
pub(in crate::ui) fn toggle_autoscroll(player: &mut WinampPlayer) -> Task<Message> {
    player.marquee_autoscroll = !player.marquee_autoscroll;
    Task::none()
}

/// The play request: the message carries the pressed row's index plus the
/// epoch of the list that rendered it. The Songs view draws from `songs`
/// and the flat All Songs and Recently Played views draw from `all_songs` and
/// `recently_played`, so resolve the press
/// against the list the current view shows; resolving it against the other
/// list would no-op or play the wrong song. A mismatched epoch means the
/// list was replaced after the row was rendered, so the index names a
/// different song now — ignore the stale press. An out-of-range index
/// likewise names no track and is a no-op rather than a bogus play. `select`
/// applies both checks. Once the song resolves, an enabled AUTO applies the
/// preset named after its genre before the play request is issued. The play
/// half is [`begin_play`]'s body, shared with the relaunch replay so both
/// paths record, reset, and start identically.
pub(in crate::ui) fn track_selected(
    player: &mut WinampPlayer,
    epoch: u64,
    index: usize,
) -> Task<Message> {
    let selected = match player.current_view {
        CurrentView::AllSongs => player.all_songs.select(epoch, index),
        CurrentView::RecentlyPlayed => player.recently_played.select(epoch, index),
        _ => player.songs.select(epoch, index),
    };
    let Some(song) = selected else {
        return Task::none();
    };
    begin_play(player, song.clone(), Some(index))
}

/// The shared play machinery behind both ways a song starts: a press on its
/// row resolves it from the showing list first (its playlist number comes
/// from the row), and the relaunch replay resolves it through the service's
/// library seam instead (no row, so the record's number prefix stays empty);
/// once the [`Song`] is known the two are identical. The record, the AUTO
/// preset, the readout reset, and the guarded async play are all here so the
/// two paths cannot drift.
///
/// `row` is the 0-based index of the row that named the song, when a row did:
/// it supplies the record's `1. `-style playlist-number prefix. The Now
/// Playing bar resolves its number, label, artist, and time from
/// `known_tracks`, and the entry must survive a later browse to a different
/// list (which replaces the visible song buffer). Recording once per play —
/// rather than folding every song of every loaded list into the map — keeps
/// the index proportional to songs actually played and takes the fold off
/// the browse path. The number is the 1-based position of the row that
/// selected the track, so re-playing the same id from another row or list
/// refreshes it alongside the other cached fields rather than keeping the
/// old number.
fn begin_play(player: &mut WinampPlayer, song: Song, row: Option<usize>) -> Task<Message> {
    let track_id = song.id.clone();
    let preview_url = song.preview_url.clone();
    let genre = song.genre.clone();
    // A newly selected song starts its title at character zero; the
    // previous song's scroll position must not carry over.
    player.marquee_offset = 0;
    player.title_drag = None;
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
    // The per-field mapping lives in `views::KnownTrack::from_song`, shared
    // with the File info panel's selected-row resolver, so the two sites
    // cannot drift apart; the number is the 1-based position of the row that
    // selected the track.
    player.known_tracks.insert(
        track_id.clone(),
        views::KnownTrack::from_song(
            &song,
            row.map_or(String::new(), |index| format!("{}. ", index + 1)),
        ),
    );
    // A selection mid-fade plays at the saved volume, not the faded one.
    fade::cancel_fade_out(player);
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
            let items = match player.current_view {
                CurrentView::AllSongs => &player.all_songs.items,
                CurrentView::RecentlyPlayed => &player.recently_played.items,
                _ => &player.songs.items,
            };
            player.shuffle_round.record(current, items);
            // The committed play also feeds the signed-out Recently Played
            // list — but only when the showing list can name the track's
            // song. A committed play whose track is absent from the showing
            // list (the resolved-but-listed-nowhere restored track) feeds
            // the play history only, exactly as it does today.
            if let Some(song) = items.iter().find(|song| song.id == current) {
                player.played_songs.record(song.clone());
            }
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
    let fade = fade::fade_out_tick(player);
    if player.mixer_flash_ticks > 0 {
        player.mixer_flash_ticks -= 1;
    } else {
        player.marquee_offset = player.marquee_offset.wrapping_add(1);
    }
    fade
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
    player.spectrum_peaks = None;
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
