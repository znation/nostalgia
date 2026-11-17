use super::*;

// A roll over the drawn Now Playing seek slider seeks instead of nudging the
// volume.
#[test]
fn wheel_over_the_root_seek_slider_seeks_without_nudging_the_volume() {
    let (mut player, _state) = test_player();
    player.playback_position = Some(Duration::from_secs(45));
    player.playback_duration = Some(Duration::from_secs(180));
    let bar = captured_root_view_container(&player, views::SEEK_BAR_ID)
        .expect("a drawn seek slider is laid out");

    for (y, forward) in [(1.0, true), (-1.0, false)] {
        let messages = root_wheel_messages(&player, bar, y);
        let seeks: Vec<_> = messages
            .iter()
            .filter(|m| matches!(m, Message::SeekForward | Message::SeekBackward))
            .collect();
        assert_eq!(seeks.len(), 1, "exactly one seek expected: {messages:?}");
        assert_eq!(matches!(seeks[0], Message::SeekForward), forward);
        assert!(
            !messages
                .iter()
                .any(|m| matches!(m, Message::VolumeWheel(_))),
            "the roll must not nudge the volume: {messages:?}"
        );
    }
}

// With no position or length the strip is blank, so a roll over it is still
// the panel's volume nudge.
#[test]
fn wheel_over_a_blank_seek_strip_still_nudges_the_volume() {
    let (mut player, _state) = test_player();
    player.playback_position = None;
    player.playback_duration = None;
    assert!(captured_root_view_container(&player, views::SEEK_BAR_ID).is_none());
    let strip = captured_root_view_container(&player, views::SEEK_STRIP_ID)
        .expect("the blank strip is laid out");

    let messages = root_wheel_messages(&player, strip, 1.0);
    let volume = messages
        .iter()
        .filter(|m| matches!(m, Message::VolumeWheel(_)))
        .count();
    assert_eq!(volume, 1, "one volume nudge expected: {messages:?}");
    assert!(
        !messages
            .iter()
            .any(|m| matches!(m, Message::SeekForward | Message::SeekBackward)),
        "a blank strip must not seek: {messages:?}"
    );
}
