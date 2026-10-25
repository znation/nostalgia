use super::*;

#[test]
fn track_selected_records_the_played_tracks_title_and_time() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));

    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 1 });

    let track = player.known_tracks.get("song-2").expect("song-2 recorded");
    assert_eq!(track.title, "Two");
    assert_eq!(track.number, "2. ");
    assert_eq!(track.time, "2:05");
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
    let (player, state) = player_playing_preview(Arc::new(RecordingAudio::default())).await;
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
#[test]
fn playback_format_reply_with_a_stale_epoch_or_a_pending_transition_is_dropped() {
    let (mut player, _current_track) = play_song_1_then_browse_to_album_2();
    let old_epoch = player.position_epoch;

    let _ = update(&mut player, Message::Stop);
    let _ = update(&mut player, Message::TransportSettled);
    let stale = PlaybackFormat {
        sample_rate_hz: 48_000,
        channels: 2,
        bitrate_kbps: None,
    };
    let _ = update(
        &mut player,
        Message::PlaybackFormat {
            epoch: old_epoch,
            format: Some(stale),
        },
    );
    assert_eq!(player.playback_format, None);

    // A pending transition drops even a reply at the current epoch.
    let current_epoch = player.position_epoch;
    let kept = PlaybackFormat {
        sample_rate_hz: 44_100,
        channels: 1,
        bitrate_kbps: None,
    };
    player.playback_format = Some(kept);
    let _ = update(&mut player, Message::Pause);
    let _ = update(
        &mut player,
        Message::PlaybackFormat {
            epoch: current_epoch,
            format: Some(stale),
        },
    );
    assert_eq!(player.playback_format, Some(kept));

    // Once the transition settles, the same reply is stored.
    let _ = update(&mut player, Message::TransportSettled);
    let _ = update(
        &mut player,
        Message::PlaybackFormat {
            epoch: current_epoch,
            format: Some(stale),
        },
    );
    assert_eq!(player.playback_format, Some(stale));
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
fn spectrum_reply_stores_the_loaded_previews_spectrum() {
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

    let _ = update(
        &mut player,
        Message::Spectrum {
            epoch,
            spectrum: None,
        },
    );
    assert_eq!(player.spectrum, None);
}

// A spectrum reply is dropped when it was issued before a Stop, or when it
// lands while a transport transition is still reaching the backend: either
// way it could describe the previous player. A reply at the current epoch
// with no transition pending is stored, so the drops above come from the
// guards and not from the message being ignored entirely.
#[test]
fn spectrum_reply_with_a_stale_epoch_or_a_pending_transition_is_dropped() {
    let (mut player, _current_track) = play_song_1_then_browse_to_album_2();
    let old_epoch = player.position_epoch;

    let _ = update(&mut player, Message::Stop);
    let _ = update(&mut player, Message::TransportSettled);
    let stale = Spectrum {
        bands: [0.25; BAND_COUNT],
    };
    let _ = update(
        &mut player,
        Message::Spectrum {
            epoch: old_epoch,
            spectrum: Some(stale),
        },
    );
    assert_eq!(player.spectrum, None);

    // A pending transition drops even a reply at the current epoch.
    let current_epoch = player.position_epoch;
    let kept = recording_spectrum();
    player.spectrum = Some(kept);
    let _ = update(&mut player, Message::Pause);
    let _ = update(
        &mut player,
        Message::Spectrum {
            epoch: current_epoch,
            spectrum: Some(stale),
        },
    );
    assert_eq!(player.spectrum, Some(kept));

    // Once the transition settles, the same reply is stored.
    let _ = update(&mut player, Message::TransportSettled);
    let _ = update(
        &mut player,
        Message::Spectrum {
            epoch: current_epoch,
            spectrum: Some(stale),
        },
    );
    assert_eq!(player.spectrum, Some(stale));
}

// Pause keeps the loaded preview's bars frozen, while Stop clears them
// immediately, like the elapsed readout and the status row's format.
#[test]
fn stop_clears_the_spectrum_while_pause_keeps_it() {
    let (mut player, _current_track) = play_song_1_then_browse_to_album_2();
    let spectrum = recording_spectrum();
    player.spectrum = Some(spectrum);

    let _ = update(&mut player, Message::Pause);
    assert_eq!(player.spectrum, Some(spectrum));

    let _ = update(&mut player, Message::Stop);
    assert_eq!(player.spectrum, None);
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
    let (mut player, _current_track) = play_song_1_then_browse_to_album_2();
    let old_epoch = player.position_epoch;

    let _ = update(&mut player, Message::Stop);
    let _ = update(&mut player, Message::TransportSettled);
    let _ = update(
        &mut player,
        Message::PlaybackDuration {
            epoch: old_epoch,
            duration: Some(Duration::from_secs(5)),
        },
    );
    assert_eq!(player.playback_duration, None);

    // A pending transition drops even a reply at the current epoch.
    let current_epoch = player.position_epoch;
    player.playback_duration = Some(Duration::from_secs(30));
    let _ = update(&mut player, Message::Pause);
    let _ = update(
        &mut player,
        Message::PlaybackDuration {
            epoch: current_epoch,
            duration: Some(Duration::from_secs(5)),
        },
    );
    assert_eq!(player.playback_duration, Some(Duration::from_secs(30)));

    // Once the transition settles, the same reply is stored.
    let _ = update(&mut player, Message::TransportSettled);
    let _ = update(
        &mut player,
        Message::PlaybackDuration {
            epoch: current_epoch,
            duration: Some(Duration::from_secs(5)),
        },
    );
    assert_eq!(player.playback_duration, Some(Duration::from_secs(5)));
}

// Dragging the seek bar stores the thumb value in seconds and sends nothing
// to the backend: only a release seeks.
#[test]
fn seek_preview_changed_tracks_the_drag_without_touching_the_backend() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, _state) =
        player_with_audio(Arc::clone(&recording) as Arc<dyn crate::audio::AudioOutput>);

    assert_no_task(update(&mut player, Message::SeekPreviewChanged(12.5)));

    assert_eq!(player.seek_preview, Some(Duration::from_millis(12_500)));
    assert_eq!(recording.calls(), vec![]);
}

// Releasing the seek bar sends exactly one absolute seek, records the dragged
// position optimistically, and bumps the epoch so replies already in flight
// cannot restore the old position.
#[test]
fn seek_preview_released_seeks_once_to_the_dragged_position() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, _state) =
        player_with_audio(Arc::clone(&recording) as Arc<dyn crate::audio::AudioOutput>);
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
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, _state) =
        player_with_audio(Arc::clone(&recording) as Arc<dyn crate::audio::AudioOutput>);
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
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, _state) =
        player_with_audio(Arc::clone(&recording) as Arc<dyn crate::audio::AudioOutput>);
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
    let audio = Arc::new(FailingAudio::failing_seek());
    let (mut player, _state) =
        player_with_audio(Arc::clone(&audio) as Arc<dyn crate::audio::AudioOutput>);
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
    let epoch = player.position_epoch;
    let replies = task_outputs(update(&mut player, Message::PositionTick), "position tick");

    assert!(
        replies.iter().any(|message| matches!(
            message,
            Message::PlaybackPosition { epoch: reply_epoch, .. } if *reply_epoch == epoch
        )),
        "the tick must query the position: {replies:?}"
    );
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
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, _state) =
        player_with_audio(Arc::clone(&recording) as Arc<dyn crate::audio::AudioOutput>);
    let epoch = player.position_epoch;
    let replies = task_outputs(update(&mut player, Message::PositionTick), "position tick");

    assert!(
        replies.iter().any(|message| matches!(
            message,
            Message::PlaybackPosition { epoch: reply_epoch, .. } if *reply_epoch == epoch
        )),
        "the tick must query the position: {replies:?}"
    );
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
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, _state) =
        player_with_audio(Arc::clone(&recording) as Arc<dyn crate::audio::AudioOutput>);
    for finished in [true, false] {
        recording.set_finished(finished);
        let epoch = player.position_epoch;
        let replies = task_outputs(update(&mut player, Message::PositionTick), "position tick");

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
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, _state) =
        player_with_audio(Arc::clone(&recording) as Arc<dyn crate::audio::AudioOutput>);
    player.playback_format = Some(PlaybackFormat {
        sample_rate_hz: 44_100,
        channels: 2,
        bitrate_kbps: Some(256),
    });
    player.playback_duration = Some(Duration::from_secs(30));
    let epoch = player.position_epoch;
    let replies = task_outputs(update(&mut player, Message::PositionTick), "position tick");

    assert!(
        replies.iter().any(|message| matches!(
            message,
            Message::PlaybackPosition { epoch: reply_epoch, .. } if *reply_epoch == epoch
        )),
        "the tick must still query the position: {replies:?}"
    );
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
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, _state) =
        player_with_audio(Arc::clone(&recording) as Arc<dyn crate::audio::AudioOutput>);
    player.playback_format = Some(PlaybackFormat {
        sample_rate_hz: 44_100,
        channels: 2,
        bitrate_kbps: Some(256),
    });
    let epoch = player.position_epoch;
    let replies = task_outputs(update(&mut player, Message::PositionTick), "position tick");
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
    let epoch = player.position_epoch;
    let replies = task_outputs(update(&mut player, Message::PositionTick), "position tick");
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
