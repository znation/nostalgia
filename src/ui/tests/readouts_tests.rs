use super::*;

/// Plays `song-1` after `edit` changes it, and returns the `known_tracks`
/// entry `TrackSelected` recorded for it.
fn track_recorded_after_editing(edit: impl FnOnce(&mut Song)) -> views::KnownTrack {
    let (mut player, _state) = test_player();
    let mut songs = stepping_songs();
    edit(&mut songs[0]);
    let _ = update(&mut player, Message::SongsLoaded(songs));

    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 0 });

    player
        .known_tracks
        .get("song-1")
        .expect("song-1 recorded")
        .clone()
}

#[test]
fn track_selected_records_the_played_tracks_title_and_time() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));

    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 1 });

    let track = player.known_tracks.get("song-2").expect("song-2 recorded");
    assert_eq!(track.title, "Two");
    assert_eq!(track.number, "2. ");
    assert_eq!(track.time, "2:05");
    assert_eq!(track.album, "First Record");

    // The first row records song-1's album alongside its title and time.
    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 0 });
    let first = player.known_tracks.get("song-1").expect("song-1 recorded");
    assert_eq!(first.title, "One");
    assert_eq!(first.album, "First Record");
}

// The Now Playing bar's File info panel reads the release year from the same
// `known_tracks` entry as the title and album: `TrackSelected` must copy the
// played song's `year`, not drop it.
#[test]
fn known_tracks_record_the_played_songs_year() {
    let track = track_recorded_after_editing(|song| song.year = Some(2001));

    assert_eq!(track.year, Some(2001));
}

// The File info panel reads the genre from the same `known_tracks` entry as
// the title and year: `TrackSelected` must copy the played song's `genre`, not
// drop it.
#[test]
fn known_tracks_record_the_played_songs_genre() {
    let track = track_recorded_after_editing(|song| song.genre = "Rock".to_string());

    assert_eq!(track.genre.as_str(), "Rock");
}

// The File info panel reads the track number from the same `known_tracks`
// entry as the title, year, and genre: `TrackSelected` must copy the played
// song's `track_number`, not drop it.
#[test]
fn known_tracks_record_the_played_songs_track_number() {
    let track = track_recorded_after_editing(|song| song.track_number = Some(3));

    assert_eq!(track.track_number, Some(3));
}

// `TrackSelected` must also copy the played song's `disc_number` into the
// `known_tracks` entry the File info panel reads.
#[test]
fn known_tracks_record_the_played_songs_disc_number() {
    let track = track_recorded_after_editing(|song| song.disc_number = Some(2));

    assert_eq!(track.disc_number, Some(2));
}

// `TrackSelected` must also copy the played song's `composer` into the
// `known_tracks` entry the File info panel reads.
#[test]
fn known_tracks_record_the_played_songs_composer() {
    let track = track_recorded_after_editing(|song| song.composer = "Jane Doe".to_string());

    assert_eq!(track.composer.as_str(), "Jane Doe");
}

// The number is the 1-based position of the row that selected the track, so
// the first row records `"1. "` and the second `"2. "`.
#[test]
fn track_selected_records_the_selected_rows_number() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));

    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 0 });
    assert_eq!(
        player
            .known_tracks
            .get("song-1")
            .map(|track| track.number.as_str()),
        Some("1. ")
    );

    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 1 });
    assert_eq!(
        player
            .known_tracks
            .get("song-2")
            .map(|track| track.number.as_str()),
        Some("2. ")
    );
}

// Re-playing the same id from a row in another list refreshes its cached
// number (and the rest of its fields) instead of keeping the first play's
// number. song-1 is row 1 of `stepping_songs` and row 2 of the second list.
#[test]
fn replaying_a_track_from_another_row_updates_its_number() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 0 });
    assert_eq!(
        player
            .known_tracks
            .get("song-1")
            .map(|track| track.number.as_str()),
        Some("1. ")
    );

    let mut second_list = second_album_songs();
    second_list.push(sample_song());
    let _ = update(&mut player, Message::SongsLoaded(second_list));
    let _ = update(&mut player, Message::TrackSelected { epoch: 2, index: 1 });

    let track = player.known_tracks.get("song-1").expect("song-1 recorded");
    assert_eq!(track.number, "2. ");
    assert_eq!(track.title, "Opening");
}

// A preview can be paused before its first position reply lands: the backend
// still holds it and a seek can move it, so the poll must stay subscribed even
// though `playback_position` is still `None`.
#[tokio::test]
async fn a_paused_preview_needs_the_position_poll_before_its_first_reply() {
    let (player, state) = player_playing_unrecorded_preview().await;
    assert!(player.playback_position.is_none());

    state.lock().await.pause();
    assert!(!state.lock().await.is_playing);
    assert!(player.needs_position_poll());
}

// The songs list caches each row's display title and formatted length when it
// is stored, so a view refresh of a large library borrows them instead of
// recomposing every row. Pin that both caches are built in row order and
// replaced with the rows, and dropped when a navigation clears the list, so a
// refresh can never show a stale title or length beside a new row.
#[test]
fn songs_loaded_caches_each_rows_display_title_and_length() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    assert_eq!(
        player.songs.titles,
        [
            "The Sample Band - One",
            "The Sample Band - Two",
            "The Sample Band - Three"
        ]
    );
    assert_eq!(player.songs.labels, ["3:30", "2:05", "1:35"]);

    // Navigate into an album: the fetch starts by clearing the previous
    // level's rows, and both caches must be dropped with them.
    player.albums.items = vec![sample_album()];
    let _ = update(&mut player, Message::AlbumSelected { epoch: 0, index: 0 });
    assert!(player.songs.titles.is_empty());
    assert!(player.songs.labels.is_empty());

    // The next load caches the new list's title and length only.
    let _ = update(&mut player, Message::SongsLoaded(second_album_songs()));
    assert_eq!(player.songs.titles, ["The Sample Band - B-side"]);
    assert_eq!(player.songs.labels, ["1:35"]);
}

// The Now Playing bar must keep naming the playing track, not its raw id,
// after the user browses to a different album. `view` renders the bar's
// label through `WinampPlayer::now_playing_label`, so asserting that same
// resolution after a browse-away pins the actual bar path: if the label
// were resolved against `songs` (the currently-browsed album's list, which
// album-2's `SongsLoaded` just replaced) instead of the `known_tracks`
// entry recorded when song-1 was played, song-1 would no longer be "known"
// and the label would fall back to the raw id "song-1", failing this test.
#[test]
fn now_playing_label_keeps_the_track_name_after_browsing_to_another_album() {
    let (player, current_track) = play_song_1_then_browse_to_album_2();

    assert_eq!(player.now_playing_label(current_track.as_deref()), "One");
}

// The Now Playing bar's time readout comes from the same `known_tracks` entry
// as its label: a played track's duration must reach
// `WinampPlayer::now_playing_time`, and — like the title — survive a later
// browse to a different album that replaces `songs`. An unknown track (or no
// current track) reads the `--:--` placeholder.
#[test]
fn now_playing_time_shows_the_played_tracks_duration_after_browsing_to_another_album() {
    let (player, current_track) = play_song_1_then_browse_to_album_2();

    assert_eq!(player.now_playing_time(None), "--:--");
    assert_eq!(player.now_playing_time(current_track.as_deref()), "3:30");
    assert_eq!(player.now_playing_time(Some("no-such-song")), "--:--");
}

// The bar's large left readout follows the backend's reported position (or a
// held seek drag) while it is known and reads 00:00 before one is reported,
// while the right edge keeps showing the plain cached total.
#[test]
fn now_playing_elapsed_shows_the_position_while_it_is_known() {
    let (mut player, current_track) = play_song_1_then_browse_to_album_2();

    assert_eq!(
        player.now_playing_elapsed(current_track.as_deref()),
        "00:00"
    );
    player.playback_position = Some(Duration::from_secs(45));
    assert_eq!(
        player.now_playing_elapsed(current_track.as_deref()),
        "00:45"
    );
    assert_eq!(player.now_playing_time(current_track.as_deref()), "3:30");
}

// A held seek drag previews the time the thumb points at in the large
// readout, counting down from it in remaining mode, while the right-edge total
// stays put. Clearing the drag falls back to the backend's reported position,
// proving the drag is a preview and not a replacement.
#[test]
fn now_playing_elapsed_previews_the_seek_drag() {
    let (mut player, current_track) = play_song_1_then_browse_to_album_2();
    player.playback_position = Some(Duration::from_secs(45));

    let _ = update(&mut player, Message::SeekPreviewChanged(12.0));

    assert_eq!(player.readout_position(), Some(Duration::from_secs(12)));
    assert_eq!(
        player.now_playing_elapsed(current_track.as_deref()),
        "00:12"
    );
    assert_eq!(player.now_playing_time(current_track.as_deref()), "3:30");

    player.time_display = views::TimeDisplay::Remaining;
    assert_eq!(
        player.now_playing_elapsed(current_track.as_deref()),
        "-03:18"
    );

    player.seek_preview = None;
    assert_eq!(player.readout_position(), Some(Duration::from_secs(45)));
    assert_eq!(
        player.now_playing_elapsed(current_track.as_deref()),
        "-02:45"
    );
    player.time_display = views::TimeDisplay::Elapsed;
    assert_eq!(
        player.now_playing_elapsed(current_track.as_deref()),
        "00:45"
    );
}

// Pause stops the backend clock but keeps the loaded player, so the stored
// elapsed readout must survive the arm and continue to render.
// `a_pause_keeps_the_loaded_player_reporting_its_position` pins the backend
// side; this pins that the UI does not clear the value on Pause.
#[test]
fn pause_keeps_the_stored_elapsed_readout() {
    let (mut player, current_track) = play_song_1_then_browse_to_album_2();
    player.playback_position = Some(Duration::from_secs(45));

    let _ = update(&mut player, Message::Pause);

    assert_eq!(player.playback_position, Some(Duration::from_secs(45)));
    assert_eq!(
        player.now_playing_elapsed(current_track.as_deref()),
        "00:45"
    );
    assert_eq!(player.now_playing_time(current_track.as_deref()), "3:30");
}

// A seek moves the backend, and the next poll's reply carries the new
// position, which the bar shows. `seek_messages_drive_the_audio_backend`
// pins the backend call itself.
#[test]
fn a_position_reply_after_a_seek_updates_the_readout() {
    let (mut player, current_track) = play_song_1_then_browse_to_album_2();
    player.playback_position = Some(Duration::from_secs(45));

    let _ = update(&mut player, Message::SeekForward);
    let epoch = player.position_epoch;
    let _ = update(
        &mut player,
        Message::PlaybackPosition {
            epoch,
            position: Some(Duration::from_secs(50)),
        },
    );

    assert_eq!(
        player.now_playing_elapsed(current_track.as_deref()),
        "00:50"
    );
}

#[test]
fn track_selected_clears_the_elapsed_readout() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    player.playback_position = Some(Duration::from_secs(45));

    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 0 });

    assert_eq!(player.playback_position, None);
    assert_eq!(player.now_playing_elapsed(Some("song-1")), "00:00");
    assert_eq!(player.now_playing_time(Some("song-1")), "3:30");
}

#[test]
fn track_selected_clears_the_playback_format() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    player.playback_format = Some(PlaybackFormat {
        sample_rate_hz: 44_100,
        channels: 2,
        bitrate_kbps: None,
    });

    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 0 });

    assert_eq!(player.playback_format, None);
}

#[test]
fn stop_clears_the_elapsed_readout() {
    let (mut player, current_track) = play_song_1_then_browse_to_album_2();
    player.playback_position = Some(Duration::from_secs(45));

    let _ = update(&mut player, Message::Stop);

    assert_eq!(player.playback_position, None);
    assert_eq!(
        player.now_playing_elapsed(current_track.as_deref()),
        "00:00"
    );
    assert_eq!(player.now_playing_time(current_track.as_deref()), "3:30");
}

// A format reply stores exactly what the backend reported, and a later
// `None` reply clears the format (dropping the prefix and dimming both labels)
// instead of keeping the old one.
#[test]
fn playback_format_reply_stores_the_loaded_previews_format() {
    let (mut player, _state) = test_player();
    let epoch = player.position_epoch;
    let format = PlaybackFormat {
        sample_rate_hz: 44_100,
        channels: 2,
        bitrate_kbps: Some(256),
    };

    let _ = update(
        &mut player,
        Message::PlaybackFormat {
            epoch,
            format: Some(format),
        },
    );
    assert_eq!(player.playback_format, Some(format));

    let _ = update(
        &mut player,
        Message::PlaybackFormat {
            epoch,
            format: None,
        },
    );
    assert_eq!(player.playback_format, None);
}

// A format reply is dropped when it was issued before a Stop, or when it
// lands while a transport transition is still reaching the backend: either
// way it could describe the previous player. A reply at the current epoch
// with no transition pending is stored, so the drops above come from the
// guards and not from the message being ignored entirely.

#[test]
fn playback_format_reply_with_a_stale_epoch_or_a_pending_transition_is_dropped() {
    let format = |sample_rate_hz| PlaybackFormat {
        sample_rate_hz,
        channels: 2,
        bitrate_kbps: None,
    };
    assert_reply_dropped_while_stale_or_pending(
        |epoch, value| Message::PlaybackFormat {
            epoch,
            format: Some(value),
        },
        |player| player.playback_format,
        |player, value| player.playback_format = Some(value),
        format(48_000),
        format(44_100),
    );
}

// Pause keeps the loaded preview's format on the status row, while Stop
// clears it immediately, like the elapsed readout.
#[test]
fn stop_clears_the_playback_format_while_pause_keeps_it() {
    let (mut player, _current_track) = play_song_1_then_browse_to_album_2();
    let format = PlaybackFormat {
        sample_rate_hz: 44_100,
        channels: 2,
        bitrate_kbps: Some(256),
    };
    player.playback_format = Some(format);

    let _ = update(&mut player, Message::Pause);
    assert_eq!(player.playback_format, Some(format));

    let _ = update(&mut player, Message::Stop);
    assert_eq!(player.playback_format, None);
}

// A duration reply stores exactly what the backend reported, and a later
// `None` reply clears the seek bar's scale instead of keeping the old length.
#[test]
fn playback_duration_reply_stores_the_loaded_previews_length() {
    let (mut player, _state) = test_player();
    let epoch = player.position_epoch;

    let _ = update(
        &mut player,
        Message::PlaybackDuration {
            epoch,
            duration: Some(Duration::from_secs(30)),
        },
    );
    assert_eq!(player.playback_duration, Some(Duration::from_secs(30)));

    let _ = update(
        &mut player,
        Message::PlaybackDuration {
            epoch,
            duration: None,
        },
    );
    assert_eq!(player.playback_duration, None);
}

// A duration reply is dropped when it was issued before a Stop, or when it
// lands while a transport transition is still reaching the backend: either
// way it could describe the previous player. A reply at the current epoch
// with no transition pending is stored, so the drops above come from the
// guards and not from the message being ignored entirely.
#[test]
fn playback_duration_reply_with_a_stale_epoch_or_a_pending_transition_is_dropped() {
    assert_reply_dropped_while_stale_or_pending(
        |epoch, value| Message::PlaybackDuration {
            epoch,
            duration: Some(value),
        },
        |player| player.playback_duration,
        |player, value| player.playback_duration = Some(value),
        Duration::from_secs(5),
        Duration::from_secs(30),
    );
}

// The Now Playing bar renders the played track's artist before its title
// (`<artist> - <title>`). The artist comes from the same `known_tracks`
// entry as the title and time: `TrackSelected` must record the played song's
// artist, and `WinampPlayer::now_playing_artist` must resolve it — and keep
// resolving it after a later browse to a different album replaces `songs`.
// The view-level `now_playing_artist` tests build their maps by hand, so a
// regression that dropped `artist` from the recorded `KnownTrack` (or
// resolved the bar against the currently-browsed `songs` list) would clear
// every existing test and only fail here.
#[test]
fn now_playing_artist_shows_the_played_tracks_artist_after_browsing_to_another_album() {
    let (player, current_track) = play_song_1_then_browse_to_album_2();

    assert_eq!(player.now_playing_artist(None), "");
    assert_eq!(
        player.now_playing_artist(current_track.as_deref()),
        "The Sample Band"
    );
    assert_eq!(player.now_playing_artist(Some("no-such-song")), "");
}

// The playlist number comes from the same `known_tracks` entry as the artist,
// so it must survive the browse that replaces the visible song list, and be
// absent for an unknown id or no current track.
#[test]
fn now_playing_number_survives_browsing_to_another_album() {
    let (player, current_track) = play_song_1_then_browse_to_album_2();

    assert_eq!(player.now_playing_number(None), "");
    assert_eq!(player.now_playing_number(current_track.as_deref()), "1. ");
    assert_eq!(player.now_playing_number(Some("no-such-song")), "");
}

// `TrackSelected` must also copy the played song's `content_rating` into the
// `known_tracks` entry the File info panel reads.
#[test]
fn known_tracks_record_the_played_songs_content_rating() {
    let track = track_recorded_after_editing(|song| {
        song.content_rating = Some(crate::library::ContentRating::Clean)
    });

    assert_eq!(
        track.content_rating,
        Some(crate::library::ContentRating::Clean)
    );
}
