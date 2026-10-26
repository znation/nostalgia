//! Ctrl+J's Jump to time panel: opening, closing, and seeking from it.

use super::window_tests::make_current_track_known;
use super::*;

fn player_with_duration() -> WinampPlayer {
    let (mut player, _state) = test_player();
    player.playback_duration = Some(Duration::from_secs(30));
    player
}

fn submit(player: &mut WinampPlayer, query: &str) {
    let _ = update(player, Message::JumpTimeChanged(query.to_string()));
    let _ = update(player, Message::JumpTimeSubmitted);
}

#[test]
fn toggle_opens_with_an_empty_query_and_closes_again() {
    let mut player = player_with_duration();
    player.jump_time_query = "stale".to_string();

    assert_message_schedules_work(&mut player, Message::ToggleJumpTime);
    assert!(player.jump_time_visible);
    assert!(player.jump_time_query.is_empty());

    assert_message_schedules_no_work(&mut player, Message::ToggleJumpTime);
    assert!(!player.jump_time_visible);
}

#[test]
fn toggle_needs_a_known_preview_length() {
    let (mut player, _state) = test_player();
    assert_message_schedules_no_work(&mut player, Message::ToggleJumpTime);
    assert!(!player.jump_time_visible);
}

#[test]
fn submit_seeks_records_the_position_and_closes() {
    let mut player = player_with_duration();
    let _ = update(&mut player, Message::ToggleJumpTime);
    let epoch = player.position_epoch;

    submit(&mut player, "0:12");

    assert_eq!(player.playback_position, Some(Duration::from_secs(12)));
    assert_eq!(player.position_epoch, epoch.wrapping_add(1));
    assert!(!player.jump_time_visible);
}

#[test]
fn submit_clamps_to_the_preview_length() {
    let mut player = player_with_duration();
    let _ = update(&mut player, Message::ToggleJumpTime);

    submit(&mut player, "5:00");

    assert_eq!(player.playback_position, Some(Duration::from_secs(30)));
}

#[test]
fn submit_of_an_unparsable_query_keeps_the_panel_open() {
    let mut player = player_with_duration();
    let _ = update(&mut player, Message::ToggleJumpTime);
    let epoch = player.position_epoch;

    submit(&mut player, "abc");

    assert!(player.jump_time_visible);
    assert_eq!(player.jump_time_query, "abc");
    assert_eq!(player.playback_position, None);
    assert_eq!(player.position_epoch, epoch);
}

#[test]
fn submit_with_the_panel_closed_seeks_nothing() {
    let mut player = player_with_duration();
    submit(&mut player, "0:12");
    assert_eq!(player.playback_position, None);
}

#[test]
fn jump_time_and_the_other_panels_close_each_other() {
    let (mut player, state) = test_player();
    make_current_track_known(&mut player, &state);
    player.playback_duration = Some(Duration::from_secs(30));

    let _ = update(&mut player, Message::ToggleAbout);
    let _ = update(&mut player, Message::ToggleJumpTime);
    assert!(player.jump_time_visible && !player.about_visible);

    let _ = update(&mut player, Message::ToggleAbout);
    assert!(player.about_visible && !player.jump_time_visible);

    let _ = update(&mut player, Message::ToggleJumpTime);
    assert!(player.jump_time_visible && !player.about_visible);
    let _ = update(&mut player, Message::ToggleFileInfo);
    assert!(player.file_info_visible && !player.jump_time_visible);

    let _ = update(&mut player, Message::ToggleJumpTime);
    assert!(player.jump_time_visible && !player.file_info_visible);
}

#[test]
fn toggle_while_shaded_restores_the_window_first() {
    let (mut player, _state) = player_with_window_id();
    player.playback_duration = Some(Duration::from_secs(30));
    player.shaded = true;
    player.unshaded_size = Some(iced::Size::new(800.0, 600.0));

    assert_message_schedules_work(&mut player, Message::ToggleJumpTime);

    assert!(!player.shaded);
    assert!(player.jump_time_visible);
}

#[test]
fn stop_closes_the_panel_whose_preview_length_it_clears() {
    let mut player = player_with_duration();
    let _ = update(&mut player, Message::ToggleJumpTime);
    assert!(player.jump_time_visible);

    let _ = update(&mut player, Message::Stop);

    assert!(player.playback_duration.is_none());
    assert!(!player.jump_time_visible);
}
