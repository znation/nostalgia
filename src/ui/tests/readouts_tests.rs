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

// A Stop resets the readout before the backend stop is scheduled, so a poll
// must wait for that transition to complete. Otherwise it could query the old
// player under the new epoch and restore the elapsed value the Stop cleared.
#[test]
fn position_poll_waits_for_a_stop_to_reach_the_backend() {
    let (mut player, current_track) = play_song_1_then_browse_to_album_2();
    player.playback_position = Some(Duration::from_secs(45));

    let _ = update(&mut player, Message::Stop);
    assert_message_schedules_no_work(&mut player, Message::PositionTick);

    let epoch = player.position_epoch;
    let _ = update(
        &mut player,
        Message::PlaybackPosition {
            epoch,
            position: Some(Duration::from_secs(45)),
        },
    );
    assert_eq!(player.playback_position, None);

    let _ = update(&mut player, Message::TransportSettled);
    assert_message_schedules_work(&mut player, Message::PositionTick);
    assert_eq!(
        player.now_playing_elapsed(current_track.as_deref()),
        "00:00"
    );
    assert_eq!(player.now_playing_time(current_track.as_deref()), "3:30");
}

// A new selection has the same window: the play task has not yet stopped the
// previous preview when `TrackSelected` clears the readout. The matching
// `TrackPlayed` completion reopens the poll only after the play reached the
// backend.
#[test]
fn position_poll_waits_for_a_selected_tracks_play_to_settle() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    player.playback_position = Some(Duration::from_secs(45));

    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 0 });
    assert_eq!(player.playback_position, None);
    assert_message_schedules_no_work(&mut player, Message::PositionTick);

    let epoch = player.position_epoch;
    let _ = update(
        &mut player,
        Message::PlaybackPosition {
            epoch,
            position: Some(Duration::from_secs(45)),
        },
    );
    assert_eq!(player.playback_position, None);

    let generation = player
        .plays_generation
        .load(std::sync::atomic::Ordering::SeqCst);
    let _ = update(
        &mut player,
        Message::TrackPlayed {
            generation,
            committed: true,
        },
    );
    assert_message_schedules_work(&mut player, Message::PositionTick);
}

// Every transport completion reports one transition, so the count only
// reaches zero once every transition has reached the backend. A Pause that
// settles while a Stop is still in flight must therefore keep the poll
// waiting: the backend still holds the preview the Stop has not discarded.
#[test]
fn an_unrelated_transport_completion_keeps_the_poll_waiting() {
    let (mut player, _current_track) = play_song_1_then_browse_to_album_2();

    let _ = update(&mut player, Message::Stop);
    let _ = update(&mut player, Message::Pause);
    assert_message_schedules_no_work(&mut player, Message::PositionTick);

    // The pause's completion lands first; the stop is still outstanding.
    let _ = update(&mut player, Message::TransportSettled);
    assert_message_schedules_no_work(&mut player, Message::PositionTick);

    // Now the stop's completion lands and the poll may query again.
    let _ = update(&mut player, Message::TransportSettled);
    assert_message_schedules_work(&mut player, Message::PositionTick);
}

// A reply stores its position at the current epoch, while a reply stamped with
// the epoch from before a Stop is dropped: a poll already in flight must not
// bring the previous track's elapsed value back. The Stop's completion lands
// first, so no transport transition is outstanding and the epoch check is the
// only guard that can drop the reply — a reply at the current epoch is stored,
// which shows the drop above came from the epoch and not from a pending
// transition.
#[test]
fn position_replies_store_and_stale_ones_are_dropped() {
    let (mut player, current_track) = play_song_1_then_browse_to_album_2();
    let current_epoch = player.position_epoch;

    let _ = update(
        &mut player,
        Message::PlaybackPosition {
            epoch: current_epoch,
            position: Some(Duration::from_secs(45)),
        },
    );
    assert_eq!(player.playback_position, Some(Duration::from_secs(45)));

    let _ = update(&mut player, Message::Stop);
    // The stop reaches the backend, so the poll is no longer waiting on a
    // transition.
    let _ = update(&mut player, Message::TransportSettled);
    assert_eq!(player.pending_position_transitions, 0);

    let _ = update(
        &mut player,
        Message::PlaybackPosition {
            epoch: current_epoch,
            position: Some(Duration::from_secs(20)),
        },
    );

    assert_eq!(player.playback_position, None);
    assert_eq!(
        player.now_playing_elapsed(current_track.as_deref()),
        "00:00"
    );
    assert_eq!(player.now_playing_time(current_track.as_deref()), "3:30");

    // A reply stamped with the epoch the Stop left behind is stored, so the
    // stale reply above was dropped by the epoch check alone.
    let stopped_epoch = player.position_epoch;
    let _ = update(
        &mut player,
        Message::PlaybackPosition {
            epoch: stopped_epoch,
            position: Some(Duration::from_secs(10)),
        },
    );
    assert_eq!(player.playback_position, Some(Duration::from_secs(10)));
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
/// Drives one poll-reply field through the guard every poll reply shares: a
/// reply from before a settled Stop is dropped, a reply at the current epoch
/// is dropped while a Pause is pending, and stored once the transition settles.
/// `reply` wraps a value in the field's message, `get`/`set` read and write the
/// field, `stale` is the replied value, and `kept` is the value already held.
fn assert_reply_dropped_while_stale_or_pending<T: Copy + PartialEq + std::fmt::Debug>(
    reply: impl Fn(u64, T) -> Message,
    get: impl Fn(&WinampPlayer) -> Option<T>,
    set: impl Fn(&mut WinampPlayer, T),
    stale: T,
    kept: T,
) {
    let (mut player, _current_track) = play_song_1_then_browse_to_album_2();
    let old_epoch = player.position_epoch;

    let _ = update(&mut player, Message::Stop);
    let _ = update(&mut player, Message::TransportSettled);
    let _ = update(&mut player, reply(old_epoch, stale));
    assert_eq!(get(&player), None);

    // A pending transition drops even a reply at the current epoch.
    let current_epoch = player.position_epoch;
    set(&mut player, kept);
    let _ = update(&mut player, Message::Pause);
    let _ = update(&mut player, reply(current_epoch, stale));
    assert_eq!(get(&player), Some(kept));

    // Once the transition settles, the same reply is stored.
    let _ = update(&mut player, Message::TransportSettled);
    let _ = update(&mut player, reply(current_epoch, stale));
    assert_eq!(get(&player), Some(stale));
}

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

// A spectrum reply stores exactly what the backend reported, and a later
// `None` reply clears the visualizer instead of keeping the old bars.
#[test]
fn spectrum_reply_stores_the_bars_and_lets_a_quieter_band_fall() {
    let (mut player, _state) = test_player();
    let epoch = player.position_epoch;
    let spectrum = recording_spectrum();

    let _ = update(
        &mut player,
        Message::Spectrum {
            epoch,
            spectrum: Some(spectrum),
        },
    );
    assert_eq!(player.spectrum, Some(spectrum));
    assert_eq!(player.spectrum_peaks, Some(spectrum));

    // A lower band leaves its peak to fall one step.
    let mut lower = spectrum;
    lower.bands[2] = 0.0;
    let _ = update(
        &mut player,
        Message::Spectrum {
            epoch,
            spectrum: Some(lower),
        },
    );
    assert_eq!(player.spectrum.unwrap().bands[2], 0.25);
    assert_eq!(player.spectrum_peaks.unwrap().bands[2], 0.375);

    let _ = update(
        &mut player,
        Message::Spectrum {
            epoch,
            spectrum: None,
        },
    );
    assert_eq!(player.spectrum, None);
    assert_eq!(player.spectrum_peaks, None);
}

// A reply from before a Stop leaves the stored peaks where they were.
#[test]
fn a_stale_spectrum_reply_leaves_the_peaks_alone() {
    let (mut player, _state) = test_player();
    let held = recording_spectrum();
    player.spectrum_peaks = Some(held);
    let stale = player.position_epoch.wrapping_sub(1);

    let _ = update(
        &mut player,
        Message::Spectrum {
            epoch: stale,
            spectrum: None,
        },
    );
    assert_eq!(player.spectrum_peaks, Some(held));
}

// A spectrum reply is dropped when it was issued before a Stop, or when it
// lands while a transport transition is still reaching the backend: either
// way it could describe the previous player. A reply at the current epoch
// with no transition pending is stored, so the drops above come from the
// guards and not from the message being ignored entirely.
#[test]
fn spectrum_reply_with_a_stale_epoch_or_a_pending_transition_is_dropped() {
    assert_reply_dropped_while_stale_or_pending(
        |epoch, value| Message::Spectrum {
            epoch,
            spectrum: Some(value),
        },
        |player| player.spectrum,
        |player, value| player.spectrum = Some(value),
        Spectrum {
            bands: [0.25; BAND_COUNT],
        },
        recording_spectrum(),
    );
}

// Pause keeps the loaded preview's bars frozen, while Stop clears them
// immediately, like the elapsed readout and the status row's format.
#[test]
fn stop_clears_the_spectrum_while_pause_keeps_it() {
    let (mut player, _current_track) = play_song_1_then_browse_to_album_2();
    let spectrum = recording_spectrum();
    player.spectrum = Some(spectrum);
    player.spectrum_peaks = Some(spectrum);

    let _ = update(&mut player, Message::Pause);
    assert_eq!(player.spectrum, Some(spectrum));
    assert_eq!(player.spectrum_peaks, Some(spectrum));

    let _ = update(&mut player, Message::Stop);
    assert_eq!(player.spectrum, None);
    assert_eq!(player.spectrum_peaks, None);
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

// Dragging the seek bar stores the thumb value in seconds and sends nothing
// to the backend: only a release seeks.
#[test]
fn seek_preview_changed_tracks_the_drag_without_touching_the_backend() {
    let (recording, mut player, _state) = player_with_recording();

    assert_no_task(update(&mut player, Message::SeekPreviewChanged(12.5)));

    assert_eq!(player.seek_preview, Some(Duration::from_millis(12_500)));
    assert_eq!(recording.calls(), vec![]);
}

// Releasing the seek bar sends exactly one absolute seek, records the dragged
// position optimistically, and bumps the epoch so replies already in flight
// cannot restore the old position.
#[test]
fn seek_preview_released_seeks_once_to_the_dragged_position() {
    let (recording, mut player, _state) = player_with_recording();
    player.playback_position = Some(Duration::from_secs(3));
    let epoch = player.position_epoch;
    let _ = update(&mut player, Message::SeekPreviewChanged(12.0));

    assert_message_schedules_no_work(&mut player, Message::SeekPreviewReleased);

    assert_eq!(
        recording.calls(),
        vec![AudioCall::SeekTo(Duration::from_secs(12))]
    );
    assert_eq!(player.playback_position, Some(Duration::from_secs(12)));
    assert_eq!(player.seek_preview, None);
    assert_eq!(player.position_epoch, epoch.wrapping_add(1));
}

// A release with no stored drag does nothing: no backend call, no position
// change, and no epoch bump.
#[test]
fn seek_preview_released_without_a_drag_does_nothing() {
    let (recording, mut player, _state) = player_with_recording();
    player.playback_position = Some(Duration::from_secs(3));
    let epoch = player.position_epoch;

    assert_message_schedules_no_work(&mut player, Message::SeekPreviewReleased);

    assert_eq!(recording.calls(), vec![]);
    assert_eq!(player.playback_position, Some(Duration::from_secs(3)));
    assert_eq!(player.position_epoch, epoch);
}

// A successful release bumps the epoch, so a position reply the poll issued
// before the seek is dropped instead of restoring the old thumb position.
#[test]
fn seek_preview_released_invalidates_an_in_flight_position_reply() {
    let (_recording, mut player, _state) = player_with_recording();
    player.playback_position = Some(Duration::from_secs(3));
    let old_epoch = player.position_epoch;
    let _ = update(&mut player, Message::SeekPreviewChanged(12.0));
    let _ = update(&mut player, Message::SeekPreviewReleased);

    let _ = update(
        &mut player,
        Message::PlaybackPosition {
            epoch: old_epoch,
            position: Some(Duration::from_secs(2)),
        },
    );

    assert_eq!(player.playback_position, Some(Duration::from_secs(12)));
}

// A rejected absolute seek is logged and leaves the optimistic state alone:
// the drag is cleared, the position and epoch stay as they were, and the next
// position poll's reply is still accepted, so it can put the thumb back on
// the player's real position. The fake counts its rejection, proving the
// release reached the failing backend instead of skipping the forward.
#[test]
fn seek_preview_released_backend_failure_leaves_the_position_alone() {
    let (audio, mut player, _state) = player_with_failing_audio(FailingAudio::failing_seek());
    player.playback_position = Some(Duration::from_secs(3));
    let epoch = player.position_epoch;
    let _ = update(&mut player, Message::SeekPreviewChanged(12.0));

    assert_message_schedules_no_work(&mut player, Message::SeekPreviewReleased);

    assert_eq!(audio.attempts(FailingCommand::Seek), 1);
    assert_eq!(player.seek_preview, None);
    assert_eq!(player.playback_position, Some(Duration::from_secs(3)));
    assert_eq!(player.position_epoch, epoch);

    let _ = update(
        &mut player,
        Message::PlaybackPosition {
            epoch,
            position: Some(Duration::from_secs(4)),
        },
    );
    assert_eq!(player.playback_position, Some(Duration::from_secs(4)));
}

/// Fires one `PositionTick` and returns the epoch it was issued at with every
/// reply its batched queries emit.
fn tick_replies(player: &mut WinampPlayer) -> (u64, Vec<Message>) {
    let epoch = player.position_epoch;
    let replies = task_outputs(update(player, Message::PositionTick), "position tick");
    (epoch, replies)
}

/// Asserts the tick's replies include a position query at `epoch`.
fn assert_position_queried(replies: &[Message], epoch: u64) {
    assert!(
        replies.iter().any(|message| matches!(
            message,
            Message::PlaybackPosition { epoch: reply_epoch, .. } if *reply_epoch == epoch
        )),
        "the tick must query the position: {replies:?}"
    );
}

#[test]
fn position_tick_schedules_a_query() {
    let (mut player, _state) = test_player();
    assert_message_schedules_work(&mut player, Message::PositionTick);
}

// The subscription itself must emit the message its `update` arm expects: a
// stream that never ticks (or yields a different message) would leave the
// readout frozen even though `PositionTick` schedules the query correctly.
// Reading two ticks also pins that the timer keeps running after the first.
// The tick must ask the backend for both values: one query for the elapsed
// position and one for the status row's format. Yield every output the
// batched task emits and check both replies arrive.
#[test]
fn position_tick_queries_position_and_format() {
    let (mut player, _state) = test_player();
    let (epoch, replies) = tick_replies(&mut player);

    assert_position_queried(&replies, epoch);
    assert!(
        replies.iter().any(|message| matches!(
            message,
            Message::PlaybackFormat { epoch: reply_epoch, .. } if *reply_epoch == epoch
        )),
        "the tick must query the format: {replies:?}"
    );
}

// The tick must also ask the backend for the preview's own length, so the
// seek bar has a scale. Drive the batched task's stream to its end and check
// the duration reply arrives beside the position.
#[test]
fn position_tick_queries_position_and_duration() {
    let (_recording, mut player, _state) = player_with_recording();
    let (epoch, replies) = tick_replies(&mut player);

    assert_position_queried(&replies, epoch);
    assert!(
        replies.iter().any(|message| matches!(
            message,
            Message::PlaybackDuration { epoch: reply_epoch, duration: Some(duration) }
                if *reply_epoch == epoch && *duration == Duration::from_secs(30)
        )),
        "the tick must query the duration: {replies:?}"
    );
}

// The tick must also ask the backend whether the preview has reached its end,
// so the transport can advance off the existing one-second poll instead of a
// new timer. Drive the batched task's stream to its end and check the finished
// reply carries the tick's epoch and the backend's state.
#[test]
fn position_tick_queries_preview_finished() {
    let (recording, mut player, _state) = player_with_recording();
    for finished in [true, false] {
        recording.set_finished(finished);
        let (epoch, replies) = tick_replies(&mut player);

        assert!(
            replies.iter().any(|message| matches!(
                message,
                Message::PlaybackFinished { epoch: reply_epoch, finished: reply_finished }
                    if *reply_epoch == epoch && *reply_finished == finished
            )),
            "the tick must query the finished state {finished}: {replies:?}"
        );
    }
}

// The paused elapsed readout blinks on the existing position poll: each tick
// flips the phase while the transport is paused, and a tick after Play or Stop
// restores it. A stopped tick still holds the flag true, but the view draws the
// stopped readout transparent regardless of the phase except for the `--:--`
// placeholder. A loaded preview is the paused state Play can resume.
#[test]
fn position_tick_blinks_the_elapsed_readout_only_while_paused() {
    let (mut player, state) = player_with_audio(Arc::new(RecordingAudio::default()));
    futures::executor::block_on(player.apple_music_service.play_track(
        "song-1",
        Some(PREVIEW_URL),
        || true,
    ))
    .unwrap();
    state.blocking_lock().pause();
    assert_eq!(
        state.blocking_lock().transport_mode(),
        TransportMode::Paused
    );
    assert!(player.blink_visible);

    let _ = update(&mut player, Message::PositionTick);
    assert!(
        !player.blink_visible,
        "the first paused tick hides the readout"
    );
    let _ = update(&mut player, Message::PositionTick);
    assert!(
        player.blink_visible,
        "the second paused tick shows it again"
    );

    state.blocking_lock().play();
    let _ = update(&mut player, Message::PositionTick);
    assert!(
        player.blink_visible,
        "a playing tick always shows the readout"
    );

    state.blocking_lock().pause();
    let _ = update(&mut player, Message::PositionTick);
    assert!(!player.blink_visible, "a paused tick resumes the blink");
    state.blocking_lock().stop();
    let _ = update(&mut player, Message::PositionTick);
    assert!(
        player.blink_visible,
        "a stopped tick holds the blink phase visible, though the view clears a real readout"
    );
}

#[test]
fn position_tick_skips_the_format_and_length_once_both_are_known() {
    let (_recording, mut player, _state) = player_with_recording();
    player.playback_format = Some(PlaybackFormat {
        sample_rate_hz: 44_100,
        channels: 2,
        bitrate_kbps: Some(256),
    });
    player.playback_duration = Some(Duration::from_secs(30));
    let (epoch, replies) = tick_replies(&mut player);

    assert_position_queried(&replies, epoch);
    assert!(
        !replies.iter().any(|message| matches!(
            message,
            Message::PlaybackFormat { .. } | Message::PlaybackDuration { .. }
        )),
        "a steady preview's format and length cannot change, so the tick must not query them: {replies:?}"
    );
}

// The two optional values are polled independently, so a preview whose source
// cannot supply one of them still polls for that one without re-querying the
// known one.
#[test]
fn position_tick_queries_only_the_missing_value() {
    let (_recording, mut player, _state) = player_with_recording();
    player.playback_format = Some(PlaybackFormat {
        sample_rate_hz: 44_100,
        channels: 2,
        bitrate_kbps: Some(256),
    });
    let (epoch, replies) = tick_replies(&mut player);
    assert!(
        replies.iter().any(|message| matches!(
            message,
            Message::PlaybackDuration { epoch: reply_epoch, duration: Some(_) }
                if *reply_epoch == epoch
        )),
        "an unknown length must still be queried: {replies:?}"
    );
    assert!(
        !replies
            .iter()
            .any(|message| matches!(message, Message::PlaybackFormat { .. })),
        "a known format must not be queried again: {replies:?}"
    );

    player.playback_format = None;
    player.playback_duration = Some(Duration::from_secs(30));
    let (epoch, replies) = tick_replies(&mut player);
    assert!(
        replies.iter().any(|message| matches!(
            message,
            Message::PlaybackFormat { epoch: reply_epoch, .. } if *reply_epoch == epoch
        )),
        "an unknown format must still be queried: {replies:?}"
    );
    assert!(
        !replies
            .iter()
            .any(|message| matches!(message, Message::PlaybackDuration { .. })),
        "a known length must not be queried again: {replies:?}"
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
