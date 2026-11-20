use super::*;

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
