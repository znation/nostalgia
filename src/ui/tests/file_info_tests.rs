use super::*;

#[test]
fn file_info_panel_draws_the_loaded_previews_format() {
    let (mut player, state) = test_player();
    seed_current_title(&mut player, &state, "Opening");
    player.file_info_visible = true;
    player.playback_format = Some(PlaybackFormat {
        sample_rate_hz: 44_100,
        channels: 2,
        bitrate_kbps: Some(256),
    });

    let loaded = captured_root_view_text_bounds(&player);
    let texts: Vec<&str> = loaded.texts().collect();
    assert!(
        texts.contains(&"Format: 256 KBPS  44 KHZ STEREO"),
        "the File info panel must draw the loaded preview's format: {texts:?}"
    );

    // Stop clears the format (see `handlers::playback::reset_position_readout`);
    // the panel then keeps its other lines but drops the Format line.
    player.playback_format = None;
    let stopped = captured_root_view_text_bounds(&player);
    let texts: Vec<&str> = stopped.texts().collect();
    assert!(
        texts.contains(&"Title: Opening"),
        "the panel must keep its title without a loaded preview: {texts:?}"
    );
    assert!(
        texts.iter().all(|text| !text.starts_with("Format:")),
        "the panel must drop the Format line once no preview is loaded: {texts:?}"
    );
}

#[test]
fn file_info_panel_draws_each_metadata_line_only_when_the_track_has_it() {
    let (mut player, state) = test_player();
    seed_current_title(&mut player, &state, "Opening");
    player.file_info_visible = true;

    let bare = captured_root_view_text_bounds(&player);
    let texts: Vec<&str> = bare.texts().collect();
    assert!(texts.contains(&"Title: Opening"), "{texts:?}");
    assert!(texts.contains(&"Length: 3:30"), "{texts:?}");
    for label in [
        "Artist:",
        "Album:",
        "Year:",
        "Genre:",
        "Track #:",
        "Disc #:",
        "Composer:",
        "Rating:",
    ] {
        assert!(
            texts.iter().all(|text| !text.starts_with(label)),
            "a track with no {label} data must not draw that line: {texts:?}"
        );
    }

    let track = player.known_tracks.get_mut("song-1").unwrap();
    track.artist = "Band".to_string();
    track.album = "Record".to_string();
    track.year = Some(1999);
    track.genre = "Rock".to_string();
    track.track_number = Some(4);
    track.disc_number = Some(2);
    track.composer = "Writer".to_string();
    track.content_rating = Some(crate::library::ContentRating::Explicit);
    let full = captured_root_view_text_bounds(&player);
    let texts: Vec<&str> = full.texts().collect();
    for line in [
        "Artist: Band",
        "Album: Record",
        "Year: 1999",
        "Genre: Rock",
        "Track #: 4",
        "Disc #: 2",
        "Composer: Writer",
        "Rating: Explicit",
    ] {
        assert!(texts.contains(&line), "missing {line:?}: {texts:?}");
    }

    let track = player.known_tracks.get_mut("song-1").unwrap();
    track.content_rating = Some(crate::library::ContentRating::Clean);
    let clean = captured_root_view_text_bounds(&player);
    assert!(
        clean.texts().any(|text| text == "Rating: Clean"),
        "a Clean track must draw Rating: Clean"
    );
}

// A song-list selection determines what the File info panel names: the
// selected row's song, with no `Format:` line because no preview is loaded
// for it, not the playing track and its format line. The no-selection case
// still names the loaded track exactly as it did before selections counted.
#[test]
fn file_info_overlay_names_the_selected_rows_song() {
    let (mut player, state) = test_player();
    seed_current_title(&mut player, &state, "Playing");
    player.playback_format = Some(PlaybackFormat {
        sample_rate_hz: 44_100,
        channels: 2,
        bitrate_kbps: Some(256),
    });
    seed_browse_lists(&mut player);
    player.current_view = CurrentView::AllSongs;
    player.file_info_visible = true;

    let fallback = captured_root_view_text_bounds(&player);
    let texts: Vec<&str> = fallback.texts().collect();
    assert!(
        texts.contains(&"Title: Playing"),
        "the panel must name the loaded track without a selection: {texts:?}"
    );
    assert!(
        texts.contains(&"Format: 256 KBPS  44 KHZ STEREO"),
        "the loaded track's panel must draw the format line: {texts:?}"
    );

    assert!(player.all_songs.select_row(0, 0).is_some());
    let selected = captured_root_view_text_bounds(&player);
    let texts: Vec<&str> = selected.texts().collect();
    assert!(
        texts.contains(&"Title: Opening"),
        "the panel must name the selected row's song: {texts:?}"
    );
    assert!(
        texts.contains(&"Artist: The Sample Band"),
        "the selected row's panel must draw its artist: {texts:?}"
    );
    assert!(
        texts.iter().all(|text| !text.starts_with("Format:")),
        "the selected row's panel must not draw a Format line: {texts:?}"
    );

    // Opening the panel for the row never plays it or touches the played
    // track's recorded entry or the current track itself.
    assert_eq!(
        player.known_tracks["song-1"].title, "Playing",
        "the recorded entry must stay untouched"
    );
    assert_eq!(
        state.blocking_lock().current_track.as_deref(),
        Some("song-1"),
        "the current track must stay untouched"
    );
}

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
