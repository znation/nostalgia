use super::*;

// The File info panel names the showing song list's selected row when one is
// picked, not the current track: `WinampPlayer::file_info_track` resolves the
// row through a shared `KnownTrack::from_song`, the same mapping
// `track_selected` records the played track with.
#[test]
fn file_info_track_resolves_the_selected_song_row() {
    let (mut player, state) = test_player();
    let mut songs = stepping_songs();
    songs[1].year = Some(1999);
    let _ = update(&mut player, Message::AllSongsLoaded(songs));
    player.current_view = CurrentView::AllSongs;
    assert!(player.all_songs.select_row(1, 1).is_some());

    // With no current track at all, the selection alone resolves the row's
    // song; the entry is not the played-track record, so `number` is empty and
    // no playback format comes back.
    let (track, format) = player.file_info_track().expect("selected row resolves");
    assert_eq!(track.title, "Two");
    assert_eq!(track.artist, "The Sample Band");
    assert_eq!(track.album, "First Record");
    assert_eq!(track.year, Some(1999));
    assert_eq!(track.time, "2:05");
    assert_eq!(track.number, "");
    assert_eq!(format, None);

    // With a current track also loaded, the selection still wins.
    state.blocking_lock().current_track = Some("song-1".to_string());
    player
        .known_tracks
        .insert("song-1".to_string(), known_track("Playing", "", ""));
    let (track, _) = player.file_info_track().expect("selected row resolves");
    assert_eq!(track.title, "Two");
}

// Without a selected song row the resolver falls back to the current track's
// `known_tracks` entry with its loaded `playback_format` — a group-list
// selection (Genres) or no selection at all is no song row.
#[test]
fn file_info_track_falls_back_to_the_current_track_without_a_selection() {
    let (mut player, state) = test_player();
    seed_current_title(&mut player, &state, "Playing");
    let format = PlaybackFormat {
        sample_rate_hz: 44_100,
        channels: 2,
        bitrate_kbps: Some(256),
    };
    player.playback_format = Some(format);

    // A selected row in the Genres group list is not a song row.
    let _ = update(
        &mut player,
        Message::GenresLoaded(group_by_genre(vec![sample_song()])),
    );
    player.current_view = CurrentView::Genres;
    assert!(player.genres.select_row(1, 0).is_some());
    let (track, resolved) = player.file_info_track().expect("current track resolves");
    assert_eq!(track.title, "Playing");
    assert_eq!(resolved, Some(format));

    // A song-list view with nothing selected likewise falls back to the
    // current track.
    let _ = update(&mut player, Message::AllSongsLoaded(stepping_songs()));
    player.current_view = CurrentView::AllSongs;
    let (track, resolved) = player.file_info_track().expect("current track resolves");
    assert_eq!(track.title, "Playing");
    assert_eq!(resolved, Some(format));
}

// A resolution for the current track records the restored song exactly as
// `track_selected`'s play-path mapping does — but with a blank `number`,
// since the restored track's row position in whatever past list selected it
// is not persisted — and touches no transport state: the restored track
// stays stopped.
#[test]
fn current_track_resolved_records_the_restored_track() {
    let (mut player, state) = test_player();
    state.blocking_lock().current_track = Some("song-1".to_string());
    let song = sample_song();

    let _ = update(
        &mut player,
        Message::CurrentTrackResolved {
            id: "song-1".to_string(),
            song: Some(song.clone()),
        },
    );

    let track = player
        .known_tracks
        .get("song-1")
        .expect("restored track recorded");
    assert_eq!(track.title, song.title);
    assert_eq!(track.artist, song.artist);
    assert_eq!(track.album, song.album);
    assert_eq!(track.year, song.year);
    assert_eq!(track.genre, song.genre);
    assert_eq!(track.time, views::format_track_time(song.duration_ms));
    assert_eq!(track.number, "");
    assert!(!state.blocking_lock().is_playing);
    assert_eq!(
        state.blocking_lock().current_track.as_deref(),
        Some("song-1")
    );
}

// A failed resolution (None) records nothing, and a resolution for an id the
// state no longer points at is dropped: a user who picked another song in the
// seconds before the answer must not be answered retroactively — the new
// selection's own `track_selected` entry, number prefix included, stands.
#[test]
fn current_track_resolved_skips_a_stale_or_failed_resolution() {
    let (mut player, state) = test_player();
    state.blocking_lock().current_track = Some("song-1".to_string());

    let _ = update(
        &mut player,
        Message::CurrentTrackResolved {
            id: "song-1".to_string(),
            song: None,
        },
    );
    assert!(
        player.known_tracks.is_empty(),
        "a failed resolution records nothing"
    );

    // A stale resolution after a real selection: `song-1` owns its play-path
    // entry, and a late answer for `song-9` is dropped.
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 0 });
    let _ = update(
        &mut player,
        Message::CurrentTrackResolved {
            id: "song-9".to_string(),
            song: Some(sample_song()),
        },
    );
    assert!(!player.known_tracks.contains_key("song-9"));
    assert_eq!(
        player
            .known_tracks
            .get("song-1")
            .map(|track| track.number.as_str()),
        Some("1. ")
    );
}

// The boot and sign-in issues are free once the track resolved: an id already
// in `known_tracks`, and no current track at all, schedule no follow-up work.
#[test]
fn resolve_current_track_skips_the_work_an_already_known_track_needs() {
    let (mut player, state) = test_player();
    seed_current_title(&mut player, &state, "Playing");
    let task = super::handlers::playback::resolve::resolve_current_track(&mut player);
    assert_no_task(task);

    let (mut player, _state) = test_player();
    let task = super::handlers::playback::resolve::resolve_current_track(&mut player);
    assert_no_task(task);
}
