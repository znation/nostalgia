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

use crate::library::Song;
use crate::state::AppState;
use crate::ui::loading::{
    PLAY_TIMEOUT, RequestGeneration, play_failure_report, play_into, replay_failure_report,
    transport_into,
};
use crate::ui::views;
use crate::ui::{CurrentView, Message, WinampPlayer};

pub(in crate::ui) mod fade;
pub(in crate::ui) mod preview;
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
            Task::perform(
                {
                    let track_id = track_id.clone();
                    async move {
                        match service.get_song_by_id(&track_id).await {
                            Ok(song) => song,
                            Err(error) => {
                                eprintln!("{}", replay_failure_report(&track_id, &error));
                                None
                            }
                        }
                    }
                },
                move |song| Message::ReplayResolved { track_id, song },
            )
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

/// An id replay's reply: `song` is what the restored id resolved to (`None`
/// when it names no song in the library, or the lookup failed — both were
/// already logged), or the lookup was rejected. The reply is dropped when it
/// no longer speaks for the boot restore: the resolved id must still be the
/// shared state's current track, and nothing may be playing or loaded, so a
/// track the user picked while the resolve was in flight cannot be knocked
/// out by the replay. A clean reply replays the song through [`begin_play`]
/// — the exact path a press on its row runs — so the restored track comes
/// back playing from its beginning with its title in the Now Playing bar.
pub(in crate::ui) fn replay_resolved(
    player: &mut WinampPlayer,
    track_id: &str,
    song: Option<Song>,
) -> Task<Message> {
    // The reply's currency checks are advisory: a shared-state lock the
    // service is holding means a transition is mid-write (a track is coming
    // to play), so the reply is treated as superseded — the transition's own
    // completion re-renders, and the next Play re-resolves. `try_lock` also
    // keeps the handler off the blocking path, so an async test runtime may
    // drive it.
    let superseded = match player.state.try_lock() {
        Ok(state) => state.current_track.as_deref() != Some(track_id) || state.is_playing,
        Err(_) => true,
    } || player.apple_music_service.has_loaded_preview();
    if superseded {
        return Task::none();
    }
    let Some(song) = song else {
        // Nothing remembered the URL and the library does not know the id,
        // so there is nothing to replay: the transport stays stopped.
        eprintln!("nothing to replay: restored track {track_id:?} is not in the library");
        return Task::none();
    };
    begin_play(player, song, None)
}

/// Zips the restored last-played track id into its title: what does the id
/// the boot `apply_to` restored name? The relaunch starts with an empty
/// `known_tracks` index, so the Now Playing bar and the window title degrade
/// to the raw id until a name is recorded; this issues the service's
/// read-only `resolve_song` (a sample-library id resolves immediately with no
/// session and no network; an Apple Music id resolves once a session is
/// stored) and routes the answer to [`current_track_resolved`].
///
/// Nothing here plays, loads, or changes transport state: the restored track
/// stays stopped.
///
/// The `Some`-then-`contains_key` guard keeps the re-issues free that
/// `session_ready` makes: once the track resolved, every later issue finds it
/// in `known_tracks` and schedules nothing — and after a Replay, it keeps the
/// `track_selected` entry with its playlist number. No current track at all
/// schedules nothing too.
pub(in crate::ui) fn resolve_current_track(player: &mut WinampPlayer) -> Task<Message> {
    let Some(id) = player.state.blocking_lock().current_track.clone() else {
        return Task::none();
    };
    if player.known_tracks.contains_key(&id) {
        return Task::none();
    }
    // A clone shares the session handle, like the fetch handlers' clones.
    let service = player.apple_music_service.clone();
    // The future must own its inputs, so iced can run it after this handler
    // returns: move the service clone into the async block and give the
    // reply builder its own copy of the id.
    let id_for_lookup = id.clone();
    Task::perform(
        async move { service.resolve_song(&id_for_lookup).await },
        move |song| Message::CurrentTrackResolved {
            id: id.clone(),
            song,
        },
    )
}

/// Records the restored track's resolution: the reply carries the id it was
/// issued for, and the handler answers only when that id is still the state's
/// current track (re-checked under the lock, then dropped) and the resolution
/// succeeded. Otherwise it changes nothing — a resolution can never overwrite
/// an entry the user's own play recorded, and a failed or stale one simply
/// leaves the id-fallback bar text standing.
///
/// The mapping is the play path's [`views::KnownTrack::from_song`] with a
/// blank `number`: the restored track's row position in whatever past list
/// selected it is not persisted. The blank-number entry resets the marquee
/// the same fresh-line reset `track_selected` applies — the bar moves from a
/// short id line to a long title line at the next message, and
/// `WinampPlayer::needs_marquee` re-evaluates on that rebuild. The transport,
/// the elapsed readout, `current_track`, and the play path are never touched:
/// the restored track stays stopped.
pub(in crate::ui) fn current_track_resolved(
    player: &mut WinampPlayer,
    id: String,
    song: Option<Song>,
) -> Task<Message> {
    let Some(song) = song else {
        return Task::none();
    };
    let still_current = player.state.blocking_lock().current_track.as_deref() == Some(id.as_str());
    if !still_current {
        return Task::none();
    }
    player
        .known_tracks
        .insert(id, views::KnownTrack::from_song(&song, String::new()));
    player.marquee_offset = 0;
    player.title_drag = None;
    Task::none()
}

/// Restores the launch's saved signed-out Recently Played list: what do the
/// ids the snapshot's `recently_played=` line carried name?
/// `adopt_saved_settings` seeded them into the player, but the songs they
/// name live in the service, not the file: this resolves them one by one
/// (a sample-library id answers immediately with no session and no network;
/// an id no library holds answers `None`) and routes the answer to
/// [`recently_played_resolved`].
///
/// A stored session schedules nothing: with a session the Recently Played
/// view fetches the service's own list and the seeded ids' rows are
/// dropped, so resolving them would only race the fetch. `session_ready`
/// never re-issues this either, for the same reason — with a session stored
/// the rows are dead anyway, and even a persistence sweep that fails to
/// fire over them is harmless, because the seed holds snapshot parity and
/// writes nothing. Nothing here plays, loads, or changes view state: the
/// list stays put until the user opens Recently Played.
///
/// The resolves run sequentially inside one task, in save order, so the
/// reply's rows line up with the seeded ids and a batch of sample-library
/// ids costs no interleaving.
pub(in crate::ui) fn resolve_recently_played(player: &mut WinampPlayer) -> Task<Message> {
    if player.apple_music_service.session().is_some() {
        return Task::none();
    }
    if player.recently_played_seeded.is_empty() {
        return Task::none();
    }
    // A clone shares the session handle, like the fetch handlers' clones;
    // the seed is owned by the handler, so the future copies it.
    let service = player.apple_music_service.clone();
    let ids = player.recently_played_seeded.clone();
    Task::perform(
        async move {
            let mut songs = Vec::with_capacity(ids.len());
            for id in &ids {
                songs.push(service.resolve_song(id).await);
            }
            songs
        },
        |songs| Message::RecentlyPlayedResolved { songs },
    )
}

/// Records the restored list: each answered id's song funnels through
/// `PlayedSongs::record`, so a replay and the history cap are governed the
/// same way a live play is, and an unanswerable id simply contributes no
/// row — the next save shortens the list, exactly like a saved playlist's
/// stale id disappears. A sign-in racing the restore (the reply lands after
/// a session was stored) is skipped: the signed-in view fetches the
/// service's own list and the restored rows would be dropped anyway.
///
/// Nothing else changes: no fetch, no view switch, no transport state — the
/// list stays put until the user opens Recently Played, whose signed-out
/// arm shows `newest_first`.
pub(in crate::ui) fn recently_played_resolved(
    player: &mut WinampPlayer,
    songs: Vec<Option<Song>>,
) -> Task<Message> {
    if player.apple_music_service.session().is_some() {
        return Task::none();
    }
    player.played_songs.restore(songs);
    Task::none()
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
