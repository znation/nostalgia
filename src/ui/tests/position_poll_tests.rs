use super::*;

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
