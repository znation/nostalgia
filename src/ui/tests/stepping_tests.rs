use super::*;

// `Message::TrackPlayed` is the other half of the `TrackSelected` handoff:
// the playback task reports the track started, and `update`'s `TrackPlayed`
// arm receives that completion. The test above only *produces* the message
// (it drives the task and asserts its output); nothing feeds `TrackPlayed`
// back through `update`, so this arm — the one update branch the suite
// never reaches — could drift (e.g. scheduling a follow-up task or touching
// shared state) with no test catching it. Pin the arm's contract: it
// schedules no work and leaves the shared state untouched, pruning only the
// player-local title index. `blocking_lock` panics inside an async runtime,
// so this stays a plain test.
#[test]
fn track_played_handoff_schedules_no_work_and_leaves_shared_state_untouched() {
    let (mut player, state) = test_player();

    // A track is mid-playback when the completion handoff arrives.
    {
        let mut state = state.blocking_lock();
        state.current_track = Some("song-1".to_string());
        state.is_playing = true;
        state.set_volume(0.7);
    }

    let task = update(
        &mut player,
        Message::TrackPlayed {
            generation: 0,
            committed: true,
        },
    );

    // The arm schedules no follow-up work...
    assert_no_task(task);

    // ...and leaves the shared state exactly as it was.
    let state = state.blocking_lock();
    assert_eq!(state.current_track.as_deref(), Some("song-1"));
    assert!(state.is_playing);
    assert_eq!(state.volume(), 0.7);
}

#[test]
fn next_track_steps_to_the_following_song() {
    let (mut player, _state) = player_stepping_from(Some("song-1"));

    let task = update(&mut player, Message::NextTrack);

    assert_track_selected(task, 0, 1);
}

#[test]
fn previous_track_steps_to_the_preceding_song() {
    let (mut player, _state) = player_stepping_from(Some("song-2"));

    let task = update(&mut player, Message::PreviousTrack);

    assert_track_selected(task, 0, 0);
}

#[test]
fn next_track_with_no_current_track_starts_at_the_first_song() {
    let (mut player, _state) = player_stepping_from(None);

    let task = update(&mut player, Message::NextTrack);

    assert_track_selected(task, 0, 0);
}

// `next_track_with_no_current_track_starts_at_the_first_song` pins the
// Next half of the no-current case; this pins the Previous half — pressing
// Previous before anything has played must land on the last song, not do
// nothing. The transport arithmetic for a `None` current is pinned in
// `transport.rs` (`previous_without_current_starts_at_last`); this pins
// the `PreviousTrack` arm's wiring over the same case, which no other
// test drives (the only other `previous` wiring tests set a current track
// or load no songs).
#[test]
fn previous_track_with_no_current_track_starts_at_the_last_song() {
    let (mut player, _state) = player_stepping_from(None);

    let task = update(&mut player, Message::PreviousTrack);

    assert_track_selected(task, 0, 2);
}

// Browsing to a different album replaces `player.songs.items` with the new
// album's list while the shared `current_track` still names a song from
// the album just left — the same browse-away state the now-playing-label
// regression (`now_playing_label_keeps_the_track_name_after_browsing_to_another_album`)
// is about. Pressing Next or Previous must then step within the newly
// browsed album's songs — Next onto its first, Previous onto its last —
// not do nothing because the playing track is no longer in the buffer.
// `transport.rs` pins the arithmetic for an unknown current
// (`next_with_unknown_current_starts_at_first` and
// `previous_with_unknown_current_starts_at_last`), but no arm-level test
// drives this exact browse-away-then-step flow: the other stepping wiring
// tests set a current track that IS in `player.songs.items` or load no songs, so
// a guard in `step_track` that bailed on a current unknown to this album
// would clear every existing test.
#[test]
fn stepping_after_browsing_away_steps_within_the_new_albums_songs() {
    let (mut player, _state) = player_stepping_from(Some("song-4"));
    // Played song-4 (an album-2 song) from the library, then browsed to
    // album-1's songs (now the current buffer): the current track names a
    // song that is not in the buffer, exactly as after a browse-away.

    let next = update(&mut player, Message::NextTrack);
    assert_track_selected(next, 0, 0);

    // The buttons only schedule a step — neither mutates the current
    // track — so Previous still sees the same browse-away state.
    let previous = update(&mut player, Message::PreviousTrack);
    assert_track_selected(previous, 0, 2);
}

#[test]
fn next_track_with_no_songs_loaded_does_nothing() {
    let (mut player, _state) = test_player();

    assert_message_schedules_no_work(&mut player, Message::NextTrack);
}

#[test]
fn previous_track_with_no_songs_loaded_does_nothing() {
    let (mut player, _state) = test_player();

    assert_message_schedules_no_work(&mut player, Message::PreviousTrack);
}

// Classic Winamp's restart rule: once a loaded preview has played at least
// three seconds, Previous restarts it from zero instead of stepping. The
// expected threshold is the literal `Duration::from_secs(3)` rather than the
// handler's constant, so a changed threshold fails this test.
#[test]
fn previous_restarts_the_preview_after_three_seconds() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) =
        futures::executor::block_on(player_playing_preview(Arc::clone(&recording)));
    player.songs.items = stepping_songs();
    player.playback_position = Some(Duration::from_secs(3));
    player.seek_preview = Some(Duration::from_secs(1));
    let epoch = player.position_epoch;

    let task = update(&mut player, Message::PreviousTrack);

    assert_no_task(task);
    assert_preview_calls(&recording.calls(), &[AudioCall::SeekTo(Duration::ZERO)]);
    assert_eq!(player.playback_position, Some(Duration::ZERO));
    assert_eq!(player.seek_preview, None);
    assert_eq!(player.position_epoch, epoch.wrapping_add(1));
    assert_eq!(
        state.blocking_lock().current_track.as_deref(),
        Some("song-1")
    );
}

// Before the three-second threshold the same Previous press steps through the
// showing list exactly as it always has, sending no absolute seek.
#[test]
fn previous_steps_before_the_restart_threshold() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, _state) =
        futures::executor::block_on(player_playing_preview(Arc::clone(&recording)));
    player.songs.items = stepping_songs();
    player.playback_position = Some(Duration::from_secs(2));

    let task = update(&mut player, Message::PreviousTrack);

    assert_track_selected(task, 0, 0);
    assert_preview_calls(&recording.calls(), &[]);
}

// The restart branch precedes Shuffle's recorded-play retrace, so a late
// Previous restarts instead of walking the history and consuming an entry.
#[test]
fn previous_with_shuffle_restarts_after_the_threshold_instead_of_retracing() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) =
        futures::executor::block_on(player_playing_preview(Arc::clone(&recording)));
    player.songs.items = stepping_songs();
    state.blocking_lock().shuffle = true;
    record_a_play(&mut player, &state, "song-1");
    record_a_play(&mut player, &state, "song-3");
    record_a_play(&mut player, &state, "song-2");
    player.playback_position = Some(Duration::from_secs(3));

    let task = update(&mut player, Message::PreviousTrack);

    assert_no_task(task);
    assert_preview_calls(&recording.calls(), &[AudioCall::SeekTo(Duration::ZERO)]);
    assert_eq!(
        state.blocking_lock().current_track.as_deref(),
        Some("song-2")
    );
}

// A rejected restart seek is only logged: the track stays queued with its
// position and epoch unchanged, and no step is scheduled. The fake counts its
// rejection, proving the restart reached the failing backend.
#[test]
fn a_failed_previous_restart_leaves_the_track_queued() {
    let (audio, mut player, state) = player_with_failing_audio(FailingAudio::failing_seek());
    futures::executor::block_on(player.apple_music_service.play_track(
        "song-1",
        Some(PREVIEW_URL),
        || true,
    ))
    .unwrap();
    player.songs.items = stepping_songs();
    player.playback_position = Some(Duration::from_secs(3));
    let epoch = player.position_epoch;

    let task = update(&mut player, Message::PreviousTrack);

    assert_no_task(task);
    assert_eq!(audio.attempts(FailingCommand::Seek), 1);
    assert_eq!(player.playback_position, Some(Duration::from_secs(3)));
    assert_eq!(player.position_epoch, epoch);
    assert_eq!(
        state.blocking_lock().current_track.as_deref(),
        Some("song-1")
    );
}

// The rejected-restart log names the track and the position that triggered
// the restart, so a developer can tell which preview failed. The id is
// `Debug`-escaped like the play log, so a hostile library id cannot inject
// terminal output.
#[test]
fn a_failed_restart_report_names_the_track_and_position() {
    let report = crate::ui::handlers::playback::stepping::restart_failure_report(
        Some("song\n1"),
        Duration::from_secs(3),
        &"the audio thread is gone",
    );

    assert_eq!(
        report,
        "restarting track \"song\\n1\" after 3s failed: the audio thread is gone"
    );
}

// The restart branch works from the loaded preview even when shared state
// names no current track, so the report still says which preview the failed
// seek belonged to.
#[test]
fn a_failed_restart_report_without_a_track_names_the_loaded_preview() {
    let report = crate::ui::handlers::playback::stepping::restart_failure_report(
        None,
        Duration::from_secs(5),
        &"no seek",
    );

    assert_eq!(
        report,
        "restarting the loaded preview after 5s failed: no seek"
    );
}

// Next and Previous can step the playing row outside the visible list, so
// while a song list is showing each step reveals the landed row, scrolling
// only as far as needed and not at all when it is already visible. From
// song-2 in a one-row viewport showing song-2's top, Next lands on song-3
// (index 2) and scrolls to its bottom edge, while Previous lands on song-1
// (index 0) and scrolls up to its top.
#[test]
fn next_and_previous_reveal_the_landed_row_only_when_it_leaves_the_viewport() {
    const ROW: f32 = views::BROWSE_ROW_HEIGHT;
    const PAD: f32 = views::BROWSE_LIST_PADDING;

    let (mut player, _state) = player_stepping_from(Some("song-2"));
    player.current_view = CurrentView::Songs;

    futures::executor::block_on(assert_task_reveals_browse_row(
        update(&mut player, Message::NextTrack),
        "next",
        ROW,
        PAD + ROW,
        PAD + 3.0 * ROW - ROW,
    ));

    let (mut player, _state) = player_stepping_from(Some("song-2"));
    player.current_view = CurrentView::Songs;
    futures::executor::block_on(assert_task_reveals_browse_row(
        update(&mut player, Message::PreviousTrack),
        "previous",
        ROW,
        PAD + ROW,
        PAD,
    ));

    let (mut player, _state) = player_stepping_from(Some("song-2"));
    player.current_view = CurrentView::Songs;
    let effects = futures::executor::block_on(browse_scroll_effects_at(
        update(&mut player, Message::NextTrack),
        "next within a tall viewport",
        10.0 * ROW,
        0.0,
    ));
    assert!(!effects.any());
}

// A step only follows the playing row when a song list is showing: the other
// browse levels carry no playing marker, and their lists keep their offset.
#[test]
fn stepping_while_another_browse_level_shows_does_not_scroll() {
    let (mut player, _state) = player_stepping_from(Some("song-1"));
    assert_view(&player, CurrentView::Artists);

    futures::executor::block_on(assert_task_scrolls_nothing(
        update(&mut player, Message::NextTrack),
        "next from artists",
    ));
}

// The Next arm forwards the shared Repeat flag to the transport helper:
// from the last song, Next wraps to the first when Repeat is on and
// re-lands on the last when it is off. The arithmetic is pinned in
// `transport.rs` (`next_wraps_from_last_to_first` and
// `next_stays_on_last_without_repeat`); this pins the arm's wiring over
// the shared flag, which no other test drives.
#[test]
fn next_track_follows_the_shared_repeat_flag_at_the_albums_end() {
    assert_repeat_wraps_at_the_edge(Message::NextTrack, "song-3", 2, 0);
}

// The Previous arm forwards the shared Repeat flag to the transport
// helper, the mirror of the Next test above: from the first song,
// Previous wraps to the last when Repeat is on and re-lands on the first
// when it is off. The arithmetic is pinned in `transport.rs`
// (`previous_wraps_from_first_to_last` and
// `previous_stays_on_first_without_repeat`); this pins the Previous arm's
// wiring over the shared flag, which no other test drives — the other
// Previous wiring tests set a current track with Repeat off, start from
// no current track, or load no songs, so an arm that dropped the shared
// flag (or passed a stale one) would clear every existing test and only
// fail here.
#[test]
fn previous_track_follows_the_shared_repeat_flag_at_the_albums_start() {
    assert_repeat_wraps_at_the_edge(Message::PreviousTrack, "song-1", 0, 2);
}

// With Shuffle on, the Next arm must land somewhere other than the current
// song. `shuffled_track_id`'s arithmetic is pinned in `transport.rs`; this
// pins the arm's wiring over the shared Shuffle flag and that the picked id
// resolves back to an index. The roll is random, so the test asserts only the
// invariant that matters — the index is not the current song's (index 1).
#[test]
fn next_with_shuffle_never_reselects_the_current_track() {
    let (mut player, state) = player_stepping_from(Some("song-2"));
    state.blocking_lock().shuffle = true;

    let task = update(&mut player, Message::NextTrack);
    futures::executor::block_on(drive_task(
        task,
        "shuffled stepping",
        |message| match message {
            Message::TrackSelected { index, .. } => {
                assert_ne!(index, 1, "shuffle re-selected the current song");
            }
            other => panic!("unexpected shuffled stepping task output: {other:?}"),
        },
    ));
}

// Shuffle plays every song once before repeating: with song-1 and song-2
// already played, the only unplayed song is song-3 (index 2).
#[test]
fn next_with_shuffle_plays_the_unplayed_song() {
    let (mut player, state) = player_stepping_from(None);
    state.blocking_lock().shuffle = true;
    record_a_play(&mut player, &state, "song-1");
    record_a_play(&mut player, &state, "song-2");

    let task = update(&mut player, Message::NextTrack);
    assert_track_selected(task, 0, 2);
}

// With Shuffle on, Previous retraces the recorded play order: the history
// here is song-1, song-3, song-2, so the first press must land on song-3
// (index 2), not the sequential song-1 (index 0), and the second press —
// after the jump's play commits — continues back to song-1.
#[test]
fn previous_with_shuffle_retraces_recorded_plays() {
    let (mut player, state) = player_stepping_from(None);
    state.blocking_lock().shuffle = true;
    record_a_play(&mut player, &state, "song-1");
    record_a_play(&mut player, &state, "song-3");
    record_a_play(&mut player, &state, "song-2");

    let task = update(&mut player, Message::PreviousTrack);
    assert_track_selected(task, 0, 2);

    record_a_play(&mut player, &state, "song-3");
    let task = update(&mut player, Message::PreviousTrack);
    assert_track_selected(task, 0, 0);
}

// The jump's play has not committed yet, so `current` still names song-3;
// two Previous presses must keep walking back from the pending target rather
// than both resolving against the same current track.
#[test]
fn previous_with_shuffle_walks_back_before_the_jump_commits() {
    let (mut player, state) = player_stepping_from(None);
    state.blocking_lock().shuffle = true;
    record_a_play(&mut player, &state, "song-1");
    record_a_play(&mut player, &state, "song-2");
    record_a_play(&mut player, &state, "song-3");

    let first = update(&mut player, Message::PreviousTrack);
    assert_track_selected(first, 0, 1);
    let second = update(&mut player, Message::PreviousTrack);
    assert_track_selected(second, 0, 0);
}

// With no recorded history there is nothing to retrace, so a shuffled
// Previous falls back to the sequential row before the current song.
#[test]
fn previous_with_shuffle_falls_back_to_the_row_before_the_current_song() {
    let (mut player, state) = player_stepping_from(Some("song-2"));
    state.blocking_lock().shuffle = true;

    let task = update(&mut player, Message::PreviousTrack);
    assert_track_selected(task, 0, 0);
}

// A recorded track the visible list no longer shows is skipped: the history
// is song-1, song-3, song-2, but song-3 was removed from the browsed songs, so
// Previous lands on song-1 (index 0) rather than scheduling a hidden row.
#[test]
fn previous_with_shuffle_skips_history_entries_outside_the_visible_list() {
    let (mut player, state) = player_stepping_from(None);
    state.blocking_lock().shuffle = true;
    record_a_play(&mut player, &state, "song-1");
    record_a_play(&mut player, &state, "song-3");
    record_a_play(&mut player, &state, "song-2");
    player.songs.items.retain(|song| song.id != "song-3");

    let task = update(&mut player, Message::PreviousTrack);
    assert_track_selected(task, 0, 0);
}

// A history jump whose play fails must be retried, not left behind: the
// backend rolls `is_playing` back but leaves the jump target in
// `current_track`, so the failed completion names song-2 and `TrackPlayed`
// marks the jump for retry. The next shuffled Previous must select song-2
// again instead of walking past it to song-1.
#[test]
fn a_failed_history_jump_retries_the_target() {
    let (mut player, state) = player_stepping_from(None);
    state.blocking_lock().shuffle = true;
    record_a_play(&mut player, &state, "song-1");
    record_a_play(&mut player, &state, "song-2");
    record_a_play(&mut player, &state, "song-3");

    let jump = update(&mut player, Message::PreviousTrack);
    assert_track_selected(jump, 0, 1);

    {
        let mut state = state.blocking_lock();
        state.current_track = Some("song-2".to_string());
        state.is_playing = false;
    }
    let generation = player
        .plays_generation
        .load(std::sync::atomic::Ordering::SeqCst);
    let _ = update(
        &mut player,
        Message::TrackPlayed {
            generation,
            committed: false,
        },
    );

    let task = update(&mut player, Message::PreviousTrack);
    assert_track_selected(task, 0, 1);
}

// A new committed play supersedes the pending jump; from song-4 the next
// Previous steps from the last actually recorded track, song-3 (index 2),
// rather than from the abandoned jump's target.
#[test]
fn previous_with_shuffle_resumes_from_the_last_recorded_play_after_a_new_track() {
    let (mut player, state) = player_stepping_from(None);
    state.blocking_lock().shuffle = true;
    record_a_play(&mut player, &state, "song-1");
    record_a_play(&mut player, &state, "song-2");
    record_a_play(&mut player, &state, "song-3");

    let jump = update(&mut player, Message::PreviousTrack);
    assert_track_selected(jump, 0, 1);
    record_a_play(&mut player, &state, "song-4");

    let task = update(&mut player, Message::PreviousTrack);
    assert_track_selected(task, 0, 2);
}

// A failed play reports a current track but is not playing, so `TrackPlayed`
// must not record it. With history song-1, song-2 and the failed song-3 left
// out, `previous(.., None)` returns song-2; a wrongly recorded song-3 would
// return song-3 instead.
#[test]
fn track_played_ignores_a_failed_completion() {
    let (mut player, state) = player_stepping_from(None);
    record_a_play(&mut player, &state, "song-1");
    record_a_play(&mut player, &state, "song-2");
    {
        let mut state = state.blocking_lock();
        state.current_track = Some("song-3".to_string());
        state.is_playing = false;
    }

    let _ = update(
        &mut player,
        Message::TrackPlayed {
            generation: 0,
            committed: false,
        },
    );

    assert_eq!(
        player.play_history.previous(&stepping_songs(), None),
        Some("song-2".to_string())
    );
}

// A play can commit and then be paused before its completion reaches
// `update`: the Pause's `TransportSettled` may be processed first, leaving
// `is_playing` false although the play committed and the preview is merely
// paused. `TrackPlayed` must still record that play, or Shuffle's Previous
// retraces past it. The completion carries the play's own outcome for exactly
// this case; the shared flags cannot stand in, because Pause clears
// `is_playing`. Without the carried outcome this test sees history song-1,
// song-2 only and the paused song-3 is treated as a failed play.
#[test]
fn track_played_records_a_play_paused_before_its_completion() {
    let (mut player, state) = player_stepping_from(None);
    record_a_play(&mut player, &state, "song-1");
    record_a_play(&mut player, &state, "song-2");
    {
        let mut state = state.blocking_lock();
        state.current_track = Some("song-3".to_string());
        state.pause();
    }

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

    record_a_play(&mut player, &state, "song-4");
    assert_eq!(
        player
            .play_history
            .previous(&stepping_songs(), Some("song-4")),
        Some("song-3".to_string())
    );
}

// A completion from an older generation belongs to a superseded play and must
// not record its track. The same song-2 assertion distinguishes the expected
// history from one that wrongly recorded song-3.
#[test]
fn track_played_ignores_a_superseded_completion() {
    let (mut player, state) = player_stepping_from(None);
    record_a_play(&mut player, &state, "song-1");
    record_a_play(&mut player, &state, "song-2");
    player
        .plays_generation
        .store(1, std::sync::atomic::Ordering::SeqCst);
    {
        let mut state = state.blocking_lock();
        state.current_track = Some("song-3".to_string());
        state.is_playing = true;
    }

    let _ = update(
        &mut player,
        Message::TrackPlayed {
            generation: 0,
            committed: true,
        },
    );

    assert_eq!(
        player.play_history.previous(&stepping_songs(), None),
        Some("song-2".to_string())
    );
}

// Songs view draws from `all_songs`, so Next there moves through the flat
// rows, not the browsed `songs` buffer. `songs` holds a single song while the
// flat list holds the three-song stepping album, so a Next that stepped
// `songs` could not yield the flat list's second row (epoch 1, index 1).
#[test]
fn next_track_in_all_songs_steps_the_flat_list() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(second_album_songs()));
    let _ = update(&mut player, Message::AllSongsLoaded(stepping_songs()));
    player.current_view = CurrentView::AllSongs;
    player.state.blocking_lock().current_track = Some("song-1".to_string());

    let task = update(&mut player, Message::NextTrack);
    assert_track_selected(task, 1, 1);
}

/// Drives a natural preview end, asserts it schedules a selection of `index`
/// at `epoch`, and plays that selection to completion.
fn advance_on_preview_end(player: &mut WinampPlayer, epoch: u64, index: usize) {
    let selected = drive_playback_finished(player);
    assert!(
        matches!(
            selected,
            Message::TrackSelected { epoch: e, index: i } if e == epoch && i == index
        ),
        "preview end must select index {index} at epoch {epoch}"
    );
    let task = update(player, selected);
    futures::executor::block_on(drive_task(task, "preview end play", |_| {}));
}

// A preview that ends while playing must step forward exactly as Next would:
// from song-1, the showing list's next song is song-2. The scheduled selection
// resolves against the same list, and its play commits the new current track.
#[test]
fn preview_end_advances_to_the_next_song() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_stepping_song(&recording, "song-1");

    advance_on_preview_end(&mut player, 0, 1);

    let state = state.blocking_lock();
    assert_eq!(state.current_track.as_deref(), Some("song-2"));
    assert!(state.is_playing);
}

// The All Songs view draws from `all_songs`, so a natural end there must step
// the flat list, not the browsed `songs` buffer, and carry the flat list's
// epoch. `songs` holds only "song-4" while `all_songs` holds the three-song
// stepping album, so a preview end that read `songs` could not select index 1.
#[test]
fn preview_end_in_all_songs_steps_the_flat_list() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_stepping_song(&recording, "song-1");
    player.songs.items = second_album_songs();
    let _ = update(&mut player, Message::AllSongsLoaded(stepping_songs()));
    player.current_view = CurrentView::AllSongs;
    let all_songs_epoch = player.all_songs.epoch;

    advance_on_preview_end(&mut player, all_songs_epoch, 1);

    let state = state.blocking_lock();
    assert_eq!(state.current_track.as_deref(), Some("song-2"));
    assert!(state.is_playing);
}

// A natural preview end follows the advanced row exactly as Next does: from
// song-1 it reveals song-2's row, scrolling only as far as the one-row
// viewport needs, and not at all when the row is already visible.
#[test]
fn a_finished_preview_scrolls_the_song_list_to_the_advanced_row() {
    const ROW: f32 = views::BROWSE_ROW_HEIGHT;
    const PAD: f32 = views::BROWSE_LIST_PADDING;

    let recording = Arc::new(RecordingAudio::default());
    let (mut player, _state) = player_playing_stepping_song(&recording, "song-1");
    player.current_view = CurrentView::Songs;
    let epoch = player.position_epoch;

    futures::executor::block_on(assert_task_reveals_browse_row(
        update(
            &mut player,
            Message::PlaybackFinished {
                epoch,
                finished: true,
            },
        ),
        "preview end",
        ROW,
        0.0,
        PAD + 2.0 * ROW - ROW,
    ));
}

#[test]
fn a_finished_preview_within_the_viewport_does_not_scroll() {
    const ROW: f32 = views::BROWSE_ROW_HEIGHT;

    let recording = Arc::new(RecordingAudio::default());
    let (mut player, _state) = player_playing_stepping_song(&recording, "song-1");
    player.current_view = CurrentView::Songs;
    let epoch = player.position_epoch;

    let effects = futures::executor::block_on(browse_scroll_effects_at(
        update(
            &mut player,
            Message::PlaybackFinished {
                epoch,
                finished: true,
            },
        ),
        "preview end within the viewport",
        10.0 * ROW,
        0.0,
    ));
    assert!(!effects.any());
}

// Repeat wraps a natural end at the last song back to the first, just as the
// Next button does.
#[test]
fn preview_end_wraps_to_the_first_song_with_repeat_on() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_stepping_song(&recording, "song-3");
    state.blocking_lock().repeat = true;

    advance_on_preview_end(&mut player, 0, 0);

    assert_eq!(
        state.blocking_lock().current_track.as_deref(),
        Some("song-1")
    );
}

// Repeat wraps a one-song list back to itself: the pick equals the current
// song, which is still a step that replays it rather than the Repeat-off Stop
// path. The three-song fixture above never exercises this equal-id pick.
#[test]
fn preview_end_repeats_a_single_song_with_repeat_on() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_stepping_song(&recording, "song-1");
    player.songs.items.truncate(1);
    state.blocking_lock().repeat = true;

    advance_on_preview_end(&mut player, 0, 0);

    let state = state.blocking_lock();
    assert_eq!(state.current_track.as_deref(), Some("song-1"));
    assert!(state.is_playing);
}

// With Repeat off, a natural end at the last song takes the Stop path: the
// transport becomes stopped, the song stays queued for Play, and the readout,
// status row's format, visualizer, and seek bar clear together.
#[test]
fn preview_end_at_the_last_song_stops_when_repeat_is_off() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_stepping_song(&recording, "song-3");
    player.playback_position = Some(Duration::from_secs(95));
    player.playback_format = Some(PlaybackFormat {
        sample_rate_hz: 44_100,
        channels: 2,
        bitrate_kbps: Some(256),
    });
    player.playback_duration = Some(Duration::from_secs(30));
    player.spectrum = Some(recording_spectrum());
    player.seek_preview = Some(Duration::from_secs(5));

    let settled = drive_playback_finished(&mut player);
    assert!(matches!(settled, Message::TransportSettled));

    assert_eq!(recording.calls(), vec![AudioCall::Stop]);
    let state = state.blocking_lock();
    assert!(!state.is_playing);
    assert_eq!(state.current_track.as_deref(), Some("song-3"));
    assert_eq!(player.playback_position, None);
    assert_eq!(player.playback_format, None);
    assert_eq!(player.playback_duration, None);
    assert_eq!(player.spectrum, None);
    assert_eq!(player.seek_preview, None);
}

// Shuffle picks a non-current song on a natural end, as it does for Next. A
// two-song list makes the pick deterministic: song-1 is current, so the only
// candidate is song-2.
#[test]
fn preview_end_with_shuffle_on_picks_the_other_song() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_stepping_song(&recording, "song-1");
    player.songs.items.truncate(2);
    state.blocking_lock().shuffle = true;

    advance_on_preview_end(&mut player, 0, 1);

    assert_eq!(
        state.blocking_lock().current_track.as_deref(),
        Some("song-2")
    );
}

// The preview-end advance shares Next's round: with song-1 and song-2
// played, a natural end on song-2 lands on song-3.
#[test]
fn preview_end_with_shuffle_on_plays_the_unplayed_song() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_stepping_song(&recording, "song-1");
    state.blocking_lock().shuffle = true;
    record_a_play(&mut player, &state, "song-1");
    record_a_play(&mut player, &state, "song-2");

    advance_on_preview_end(&mut player, 0, 2);

    assert_eq!(
        state.blocking_lock().current_track.as_deref(),
        Some("song-3")
    );
}

// Repeat alone repeats a one-song list, but Shuffle changes the forward step:
// `shuffled_track_id` can only pick the current song, so the advance filters it
// out and stops rather than replaying, whether Repeat is on or off. The
// two-song Shuffle test above never reaches this no-other-song path.
#[test]
fn preview_end_with_shuffle_on_a_single_song_stops() {
    for repeat in [false, true] {
        let recording = Arc::new(RecordingAudio::default());
        let (mut player, state) = player_playing_stepping_song(&recording, "song-1");
        player.songs.items.truncate(1);
        {
            let mut state = state.blocking_lock();
            state.shuffle = true;
            state.repeat = repeat;
        }

        let settled = drive_playback_finished(&mut player);
        assert!(
            matches!(settled, Message::TransportSettled),
            "Shuffle with a single song and Repeat {repeat} must take the Stop path"
        );
        assert_eq!(recording.calls(), vec![AudioCall::Stop]);
        let state = state.blocking_lock();
        assert!(!state.is_playing);
        assert_eq!(state.current_track.as_deref(), Some("song-1"));
    }
}

// With Shuffle on and Repeat off, the natural end of the song that completed
// the round stops like the last song of an unshuffled list, keeping it queued.
#[test]
fn preview_end_with_shuffle_on_and_repeat_off_stops_after_the_round() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_stepping_song(&recording, "song-3");
    state.blocking_lock().shuffle = true;
    for id in ["song-1", "song-2", "song-3"] {
        record_a_play(&mut player, &state, id);
    }

    let settled = drive_playback_finished(&mut player);
    assert!(matches!(settled, Message::TransportSettled));
    assert_eq!(recording.calls(), vec![AudioCall::Stop]);
    let state = state.blocking_lock();
    assert!(!state.is_playing);
    assert_eq!(state.current_track.as_deref(), Some("song-3"));
}

// Repeat on starts a new round instead of stopping.
#[test]
fn preview_end_with_shuffle_and_repeat_on_starts_a_new_round() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_stepping_song(&recording, "song-3");
    {
        let mut state = state.blocking_lock();
        state.shuffle = true;
        state.repeat = true;
    }
    for id in ["song-1", "song-2", "song-3"] {
        record_a_play(&mut player, &state, id);
    }

    match drive_playback_finished(&mut player) {
        Message::TrackSelected { index, .. } => assert!(index == 0 || index == 1),
        other => panic!("expected a new round's TrackSelected, got {other:?}"),
    }
}

// Next is not a preview end: it starts a new round and never stops.
#[test]
fn next_with_shuffle_after_a_completed_round_keeps_playing() {
    let (mut player, state) = player_stepping_from(None);
    state.blocking_lock().shuffle = true;
    for id in ["song-1", "song-2", "song-3"] {
        record_a_play(&mut player, &state, id);
    }

    let task = update(&mut player, Message::NextTrack);
    match futures::executor::block_on(task_output(task, "next")) {
        Message::TrackSelected { index, .. } => assert!(index == 0 || index == 1),
        other => panic!("expected TrackSelected, got {other:?}"),
    }
}

// Pause and Stop never advance: the finished signal may still arrive from a
// poll issued before the transport changed, and a paused or stopped player has
// nothing to step past.
#[test]
fn preview_end_while_paused_or_stopped_does_not_advance() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_stepping_song(&recording, "song-1");

    state.blocking_lock().pause();
    let epoch = player.position_epoch;
    assert_message_schedules_no_work(
        &mut player,
        Message::PlaybackFinished {
            epoch,
            finished: true,
        },
    );

    state.blocking_lock().stop();
    let epoch = player.position_epoch;
    assert_message_schedules_no_work(
        &mut player,
        Message::PlaybackFinished {
            epoch,
            finished: true,
        },
    );
}

// A finished reply in flight across a transport change must not advance: a
// stale epoch is dropped, and a reply that lands while a transition is still
// reaching the backend is dropped too, so it cannot step from the previous
// track's list. A not-finished reply is likewise a no-op.
#[test]
fn preview_end_with_a_stale_epoch_or_pending_transition_is_dropped() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, _state) = player_playing_stepping_song(&recording, "song-1");

    let stale = player.position_epoch.wrapping_add(1);
    assert_message_schedules_no_work(
        &mut player,
        Message::PlaybackFinished {
            epoch: stale,
            finished: true,
        },
    );

    let epoch = player.position_epoch;
    player.pending_position_transitions = 1;
    assert_message_schedules_no_work(
        &mut player,
        Message::PlaybackFinished {
            epoch,
            finished: true,
        },
    );

    player.pending_position_transitions = 0;
    assert_message_schedules_no_work(
        &mut player,
        Message::PlaybackFinished {
            epoch,
            finished: false,
        },
    );
}

#[test]
fn first_track_selects_the_first_song() {
    let (mut player, _state) = player_stepping_from(Some("song-1"));

    let task = update(&mut player, Message::FirstTrack);

    assert_track_selected(task, 0, 0);
}

#[test]
fn last_track_selects_the_last_song() {
    let (mut player, _state) = player_stepping_from(Some("song-0"));

    let task = update(&mut player, Message::LastTrack);

    assert_track_selected(task, 0, stepping_songs().len() - 1);
}

#[test]
fn edge_jumps_ignore_shuffle_and_repeat() {
    let (mut player, state) = player_stepping_from(Some("song-1"));
    {
        let mut state = state.blocking_lock();
        state.shuffle = true;
        state.repeat = true;
    }

    let task = update(&mut player, Message::FirstTrack);
    assert_track_selected(task, 0, 0);
    let task = update(&mut player, Message::LastTrack);
    assert_track_selected(task, 0, stepping_songs().len() - 1);
}

#[test]
fn edge_jumps_with_no_songs_loaded_do_nothing() {
    let (mut player, _state) = test_player();

    assert_message_schedules_no_work(&mut player, Message::FirstTrack);
    assert_message_schedules_no_work(&mut player, Message::LastTrack);
}

#[test]
fn edge_jumps_in_all_songs_use_the_flat_list() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(second_album_songs()));
    let _ = update(&mut player, Message::AllSongsLoaded(stepping_songs()));
    player.current_view = CurrentView::AllSongs;

    let task = update(&mut player, Message::FirstTrack);
    assert_track_selected(task, 1, 0);
    let task = update(&mut player, Message::LastTrack);
    assert_track_selected(task, 1, stepping_songs().len() - 1);
}

#[test]
fn edge_jumps_scroll_the_showing_song_list() {
    const ROW: f32 = views::BROWSE_ROW_HEIGHT;
    const PAD: f32 = views::BROWSE_LIST_PADDING;

    let (mut player, _state) = player_stepping_from(Some("song-1"));
    player.current_view = CurrentView::Songs;
    let last = stepping_songs().len() - 1;

    futures::executor::block_on(assert_task_reveals_browse_row(
        update(&mut player, Message::FirstTrack),
        "first",
        ROW,
        PAD + last as f32 * ROW,
        PAD,
    ));
    futures::executor::block_on(assert_task_reveals_browse_row(
        update(&mut player, Message::LastTrack),
        "last",
        ROW,
        0.0,
        PAD + (last as f32 + 1.0) * ROW - ROW,
    ));
}

#[test]
fn preview_end_with_stop_after_current_stops_and_clears_it() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_stepping_song(&recording, "song-1");
    player.stop_after_current = true;

    let settled = drive_playback_finished(&mut player);
    assert!(matches!(settled, Message::TransportSettled));

    let state = state.blocking_lock();
    assert!(!state.is_playing);
    assert_eq!(state.current_track.as_deref(), Some("song-1"));
    assert!(!player.stop_after_current);
}

#[test]
fn preview_end_after_stop_after_current_fired_advances_again() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_stepping_song(&recording, "song-1");
    player.stop_after_current = true;
    let settled = drive_playback_finished(&mut player);
    assert!(matches!(settled, Message::TransportSettled));
    let _ = update(&mut player, settled);

    {
        let mut state = state.blocking_lock();
        state.current_track = Some("song-1".to_string());
        state.play();
    }
    let advanced = drive_playback_finished(&mut player);
    assert!(matches!(advanced, Message::TrackSelected { .. }));
}

#[test]
fn stop_after_current_survives_a_dropped_finished_reply() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_stepping_song(&recording, "song-1");
    player.stop_after_current = true;
    let stale = player.position_epoch.wrapping_add(1);

    let task = update(
        &mut player,
        Message::PlaybackFinished {
            epoch: stale,
            finished: true,
        },
    );

    assert!(iced_runtime::task::into_stream(task).is_none());
    assert!(player.stop_after_current);
    assert!(state.blocking_lock().is_playing);
}

#[test]
fn toggle_stop_after_current_flips_only_the_flag() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_stepping_song(&recording, "song-1");

    let task = update(&mut player, Message::ToggleStopAfterCurrent);
    assert!(player.stop_after_current);
    assert!(iced_runtime::task::into_stream(task).is_none());
    let _ = update(&mut player, Message::ToggleStopAfterCurrent);
    assert!(!player.stop_after_current);
    assert!(state.blocking_lock().is_playing);
    assert!(recording.calls().is_empty());
}

// With Manual playlist advance on, a natural end takes the Stop path even
// though a next song exists: playback stops, the ended song stays queued for
// Play, and the flag stays on.
#[test]
fn preview_end_with_manual_advance_stops_and_keeps_the_song() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_stepping_song(&recording, "song-1");
    state.blocking_lock().manual_advance = true;

    let settled = drive_playback_finished(&mut player);
    assert!(matches!(settled, Message::TransportSettled));

    let state = state.blocking_lock();
    assert!(!state.is_playing);
    assert_eq!(state.current_track.as_deref(), Some("song-1"));
    assert!(state.manual_advance);
}

// Manual advance outranks Repeat: Repeat would replay the one-song list, but
// the natural end stops instead.
#[test]
fn preview_end_with_manual_advance_overrides_repeat() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_stepping_song(&recording, "song-1");
    player.songs.items.truncate(1);
    {
        let mut state = state.blocking_lock();
        state.repeat = true;
        state.manual_advance = true;
    }

    let settled = drive_playback_finished(&mut player);
    assert!(matches!(settled, Message::TransportSettled));

    let state = state.blocking_lock();
    assert!(!state.is_playing);
    assert_eq!(state.current_track.as_deref(), Some("song-1"));
}

// Manual advance outranks Shuffle: Shuffle would advance to the other song of
// the list, but the natural end stops instead.
#[test]
fn preview_end_with_manual_advance_overrides_shuffle() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_stepping_song(&recording, "song-1");
    {
        let mut state = state.blocking_lock();
        state.shuffle = true;
        state.manual_advance = true;
    }

    let settled = drive_playback_finished(&mut player);
    assert!(matches!(settled, Message::TransportSettled));

    let state = state.blocking_lock();
    assert!(!state.is_playing);
    assert_eq!(state.current_track.as_deref(), Some("song-1"));
}

// After the options row disables Manual playlist advance, the natural end
// follows the ordinary advance rules again.
#[test]
fn preview_end_after_disabling_manual_advance_advances_again() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_stepping_song(&recording, "song-1");
    state.blocking_lock().manual_advance = true;

    // The options row toggles through the shared state method; flip it off
    // exactly as the row does, closing the menu like every choice does.
    player.options_visible = true;
    let _ = update(
        &mut player,
        Message::OptionsChoice(OptionsChoice::ManualAdvance),
    );
    assert!(!player.options_visible);
    assert!(!state.blocking_lock().manual_advance);

    let advanced = drive_playback_finished(&mut player);
    assert!(matches!(advanced, Message::TrackSelected { .. }));
}

// The shared toggle flips only the manual-advance flag: nothing else in the
// state or the transport moves, and no audio command is issued.
#[test]
fn stepping_toggle_manual_advance_flips_only_the_flag() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, state) = player_playing_stepping_song(&recording, "song-1");

    let task = crate::ui::handlers::playback::toggle_manual_advance(&mut player);
    assert!(iced_runtime::task::into_stream(task).is_none());
    assert!(state.blocking_lock().manual_advance);

    let task = crate::ui::handlers::playback::toggle_manual_advance(&mut player);
    assert!(iced_runtime::task::into_stream(task).is_none());
    {
        let state = state.blocking_lock();
        assert!(!state.manual_advance);
        assert!(state.is_playing);
        assert_eq!(state.current_track.as_deref(), Some("song-1"));
    }
    assert!(recording.calls().is_empty());
}
