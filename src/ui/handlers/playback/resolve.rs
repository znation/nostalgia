//! The track-id resolution handlers: turning a recorded Apple Music track id
//! into a `Song` through the service's library seam, and recording the answer.
//! Three question-and-reply pairs serve the relaunch — the stopped transport's
//! replay of the restored last-played id, the restored current track's Now
//! Playing title resolution, and the signed-out Recently Played restore — and
//! the first pair's lookup is also the transport Play/Pause fallback's seam call.
//!
//! Nothing here plays on its own except the replay's reply, which routes
//! through [`begin_play`] so the restored track and a row press sound
//! identically; the other replies record names or restore rows only.

use super::begin_play;
use crate::apple_music::AppleMusicService;
use crate::library::Song;
use crate::ui::loading::replay_failure_report;
use crate::ui::{Message, WinampPlayer, views};
use iced::Task;

/// Resolves the replay id through the service's library seam. A failed
/// lookup is reported to stdout with [`replay_failure_report`] and carried
/// as no song, so [`replay_resolved`]'s none-branch ships the same
/// `nothing to replay` report it prints when a transport-side lookup fails.
pub(in crate::ui) async fn resolve_replay_song(
    service: AppleMusicService,
    track_id: String,
) -> Option<Song> {
    match service.get_song_by_id(&track_id).await {
        Ok(song) => song,
        Err(error) => {
            eprintln!("{}", replay_failure_report(&track_id, &error));
            None
        }
    }
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
    // The restore is in flight from now: a persist before the reply composes
    // the seed with any play committed since the boot, so the save cannot
    // erase the seed's rows while the walk decides what it can answer. The
    // reply clears the flag whatever it holds.
    player.recently_played_restore_pending = true;
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
    // One resolve, one flag: whatever this reply holds — an answer, no
    // usable answer, or a skip because a session landed under it (then the
    // view fetches the service's own list) — the boot restore is over.
    player.recently_played_restore_pending = false;
    if player.apple_music_service.session().is_some() {
        return Task::none();
    }
    // A play that committed while the resolve was in flight is newer than
    // every snapshot row, even when the snapshot does not name its song (so
    // the restore walk cannot re-record it at the top the way it re-records
    // the snapshot's newest row): the walk drops it to the list's oldest
    // end, and the next save would write the run's newest play as the
    // oldest. Snap the plays recorded since the boot back on top of the
    // walk's answer, oldest first, so `record` keeps their own play order
    // with the newest pinned last.
    let mid_flight_plays = player
        .played_songs
        .newest_first()
        .into_iter()
        .rev()
        .collect::<Vec<_>>();
    player.played_songs.restore(songs);
    for song in mid_flight_plays {
        player.played_songs.record(song);
    }
    Task::none()
}
