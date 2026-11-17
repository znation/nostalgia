use super::*;

#[test]
fn dragging_the_seek_slider_shows_the_target_until_release() {
    let (mut player, state) = test_player();
    seed_current_title(&mut player, &state, "Tune");
    player.playback_duration = Some(Duration::from_secs(30));
    let _ = update(&mut player, Message::SeekPreviewChanged(12.5));
    assert!(root_view_draws(&player, "Seek to: 00:12/00:30 (41%)"));
    assert!(!root_view_draws(&player, "Tune"));

    let _ = update(&mut player, Message::SeekPreviewReleased);
    assert!(root_view_draws(&player, "Tune"));
    assert!(!root_view_draws(&player, "Seek to: 00:12/00:30 (41%)"));
}

#[test]
fn a_seek_drag_without_a_duration_shows_no_seek_text() {
    let (mut player, state) = test_player();
    seed_current_title(&mut player, &state, "Tune");
    player.playback_duration = None;
    player.seek_preview = Some(Duration::from_secs(12));
    assert!(!root_view_draws(&player, "Seek to: 00:12/00:00 (0%)"));
    assert!(root_view_draws(&player, "Tune"));
}

#[test]
fn the_marquee_pauses_during_a_seek_drag() {
    let (mut player, state) = test_player();
    seed_current_title(&mut player, &state, "A Title Too Long For The LCD Bar");
    player.playback_duration = Some(Duration::from_secs(30));
    assert!(player.needs_marquee());
    let _ = update(&mut player, Message::SeekPreviewChanged(12.5));
    assert!(!player.needs_marquee());
    let _ = update(&mut player, Message::SeekPreviewReleased);
    assert!(player.needs_marquee());
}
