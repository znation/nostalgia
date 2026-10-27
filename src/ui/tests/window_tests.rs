use super::*;

#[test]
fn window_id_resolved_stores_the_window_id() {
    let (mut player, _state) = test_player();
    assert_eq!(player.window_id, None);

    let id = iced::window::Id::unique();
    let _ = update(&mut player, Message::WindowIdResolved(Some(id)));
    assert_eq!(player.window_id, Some(id));

    let _ = update(&mut player, Message::WindowIdResolved(None));
    assert_eq!(player.window_id, None);
}

#[test]
fn title_bar_window_actions_are_noops_without_a_window_id() {
    let (mut player, _state) = test_player();
    assert_eq!(player.window_id, None);

    for message in TITLE_BAR_WINDOW_ACTIONS {
        assert_message_schedules_no_work(&mut player, message);
    }
}

#[test]
fn title_bar_window_actions_schedule_work_with_a_window_id() {
    let (mut player, _state) = player_with_window_id();

    for message in TITLE_BAR_WINDOW_ACTIONS {
        assert_message_schedules_work(&mut player, message);
    }
}

#[test]
fn toggle_always_on_top_flips_the_flag_and_sets_the_window_level() {
    let (mut player, _state) = player_with_window_id();
    assert!(!player.always_on_top);

    assert_message_schedules_work(&mut player, Message::ToggleAlwaysOnTop);
    assert!(player.always_on_top);

    assert_message_schedules_work(&mut player, Message::ToggleAlwaysOnTop);
    assert!(!player.always_on_top);
}

#[test]
fn toggle_always_on_top_without_a_window_id_still_flips_the_flag() {
    let (mut player, _state) = test_player();
    assert_eq!(player.window_id, None);

    assert_message_schedules_no_work(&mut player, Message::ToggleAlwaysOnTop);
    assert!(player.always_on_top);
}

#[test]
fn toggle_about_shows_and_hides_the_panel_without_touching_playback() {
    let (mut player, state) = test_player();
    seed_browse_lists(&mut player);
    let before = playback_snapshot(&state);
    let browse_before = browse_snapshot(&player);
    assert!(!player.about_visible);

    assert_message_schedules_no_work(&mut player, Message::ToggleAbout);
    assert!(player.about_visible);

    assert_message_schedules_no_work(&mut player, Message::ToggleAbout);
    assert!(!player.about_visible);

    let after = playback_snapshot(&state);
    assert_eq!(
        after, before,
        "the About toggle must leave playback, volume, and equalizer state untouched"
    );
    assert_eq!(
        browse_snapshot(&player),
        browse_before,
        "the About toggle must leave the browse and search state untouched"
    );
    assert_eq!(player.current_view, CurrentView::Artists);
}

#[test]
fn toggle_options_while_shaded_restores_the_window_first() {
    let (mut player, _state) = player_with_window_id();
    player.shaded = true;
    player.unshaded_size = Some(iced::Size::new(800.0, 600.0));

    assert_message_schedules_work(&mut player, Message::ToggleOptions);

    assert!(
        !player.shaded,
        "opening Options must restore the window first"
    );
    assert!(player.options_visible);
}

#[test]
fn toggle_about_while_shaded_restores_the_window_first() {
    let (mut player, _state) = player_with_window_id();
    player.shaded = true;
    player.unshaded_size = Some(iced::Size::new(800.0, 600.0));

    assert_message_schedules_work(&mut player, Message::ToggleAbout);

    assert!(
        !player.shaded,
        "opening About must restore the window first"
    );
    assert!(player.about_visible);
}

/// Makes `song-1` the current track and records it in `known_tracks`.
pub(super) fn make_current_track_known(player: &mut WinampPlayer, state: &Arc<Mutex<AppState>>) {
    player.known_tracks.insert(
        "song-1".to_string(),
        views::KnownTrack {
            title: "Opening".to_string(),
            artist: "The Sample Band".to_string(),
            album: "First Record".to_string(),
            year: None,
            genre: String::new(),
            track_number: None,
            number: "1. ".to_string(),
            time: "3:30".to_string(),
            duration_ms: 210_000,
        },
    );
    state.blocking_lock().current_track = Some("song-1".to_string());
}

#[test]
fn toggle_file_info_shows_and_hides_the_panel_without_touching_playback() {
    let (mut player, state) = test_player();
    seed_browse_lists(&mut player);
    make_current_track_known(&mut player, &state);
    let (playback_before, browse_before) = (playback_snapshot(&state), browse_snapshot(&player));
    assert!(!player.file_info_visible);

    assert_message_schedules_no_work(&mut player, Message::ToggleFileInfo);
    assert!(player.file_info_visible);

    assert_message_schedules_no_work(&mut player, Message::ToggleFileInfo);
    assert!(!player.file_info_visible);

    assert_eq!(playback_snapshot(&state), playback_before);
    assert_eq!(browse_snapshot(&player), browse_before);
}

#[test]
fn toggle_file_info_needs_a_known_current_track() {
    let (mut player, state) = test_player();
    assert_message_schedules_no_work(&mut player, Message::ToggleFileInfo);
    assert!(!player.file_info_visible, "no current track");

    state.blocking_lock().current_track = Some("missing".to_string());
    assert_message_schedules_no_work(&mut player, Message::ToggleFileInfo);
    assert!(!player.file_info_visible, "current id not in known_tracks");
}

#[test]
fn file_info_and_about_close_each_other() {
    let (mut player, state) = test_player();
    make_current_track_known(&mut player, &state);

    let _ = update(&mut player, Message::ToggleAbout);
    let _ = update(&mut player, Message::ToggleFileInfo);
    assert!(player.file_info_visible && !player.about_visible);

    let _ = update(&mut player, Message::ToggleAbout);
    assert!(player.about_visible && !player.file_info_visible);
}

#[test]
fn toggle_file_info_while_shaded_restores_the_window_first() {
    let (mut player, state) = player_with_window_id();
    make_current_track_known(&mut player, &state);
    player.shaded = true;
    player.unshaded_size = Some(iced::Size::new(800.0, 600.0));

    assert_message_schedules_work(&mut player, Message::ToggleFileInfo);

    assert!(!player.shaded);
    assert!(player.file_info_visible);
}

#[test]
fn window_level_selects_always_on_top() {
    assert_eq!(
        handlers::window::window_level(true),
        iced::window::Level::AlwaysOnTop
    );
    assert_eq!(
        handlers::window::window_level(false),
        iced::window::Level::Normal
    );
}

#[test]
fn toggle_window_shade_flips_the_flag_and_measures_the_window() {
    let (mut player, _state) = player_with_window_id();
    assert!(!player.shaded);

    assert_message_schedules_work(&mut player, Message::ToggleWindowShade);
    assert!(player.shaded);

    // Once the pre-shade size is known, unshading resizes back to it.
    player.unshaded_size = Some(iced::Size::new(800.0, 600.0));
    assert_message_schedules_work(&mut player, Message::ToggleWindowShade);
    assert!(!player.shaded);
}

#[test]
fn window_shade_measured_stores_the_size_and_resizes() {
    let (mut player, _state) = player_with_window_id();
    player.shaded = true;

    assert_message_schedules_work(
        &mut player,
        Message::WindowShadeMeasured(iced::Size::new(800.0, 600.0)),
    );

    assert_eq!(player.unshaded_size, Some(iced::Size::new(800.0, 600.0)));
}

#[test]
fn window_shade_measurement_after_unshade_is_ignored() {
    let (mut player, _state) = player_with_window_id();

    // Shade, then unshade before the deferred measurement lands.
    let _ = update(&mut player, Message::ToggleWindowShade);
    assert!(player.shaded);
    let _ = update(&mut player, Message::ToggleWindowShade);
    assert!(!player.shaded);

    // The stale measurement must neither record the size nor clamp the
    // restored window back down to the strip.
    assert_message_schedules_no_work(
        &mut player,
        Message::WindowShadeMeasured(iced::Size::new(800.0, 600.0)),
    );
    assert_eq!(player.unshaded_size, None);
}

#[test]
fn a_late_measurement_cannot_overwrite_the_captured_size() {
    let (mut player, _state) = player_with_window_id();

    // The first shade captures the real pre-shade size.
    let _ = update(&mut player, Message::ToggleWindowShade);
    let _ = update(
        &mut player,
        Message::WindowShadeMeasured(iced::Size::new(800.0, 600.0)),
    );
    assert_eq!(player.unshaded_size, Some(iced::Size::new(800.0, 600.0)));

    // Roll down, then up again: the known size rolls straight up without a
    // fresh measurement that could read the strip a prior shade left.
    let _ = update(&mut player, Message::ToggleWindowShade);
    assert_message_schedules_work(&mut player, Message::ToggleWindowShade);

    // A measurement from the first shade arriving now must not overwrite the
    // captured size with the 24 px strip.
    assert_message_schedules_no_work(
        &mut player,
        Message::WindowShadeMeasured(iced::Size::new(800.0, 24.0)),
    );
    assert_eq!(player.unshaded_size, Some(iced::Size::new(800.0, 600.0)));
}

#[test]
fn window_id_resolved_measures_a_restored_shade() {
    let (mut player, _state) = test_player();
    player.shaded = true;
    assert_eq!(player.unshaded_size, None);
    let id = iced::window::Id::unique();

    assert_message_schedules_work(&mut player, Message::WindowIdResolved(Some(id)));
    assert_eq!(player.window_id, Some(id));

    assert_message_schedules_work(
        &mut player,
        Message::WindowShadeMeasured(iced::Size::new(800.0, 600.0)),
    );
    assert!(player.shaded);
    assert_eq!(player.unshaded_size, Some(iced::Size::new(800.0, 600.0)));

    let (mut player, _state) = test_player();
    let id = iced::window::Id::unique();
    assert_message_schedules_no_work(&mut player, Message::WindowIdResolved(Some(id)));
}

#[test]
fn toggle_window_shade_without_a_window_id_still_flips_the_flag() {
    let (mut player, _state) = test_player();
    assert_eq!(player.window_id, None);

    assert_message_schedules_no_work(&mut player, Message::ToggleWindowShade);
    assert!(player.shaded);
}

// The iced window's title follows the Now Playing bar's known track: the same
// `known_tracks` entry the LCD names, with the row number and artist included
// only when they exist. Nothing current, or an id the index does not know,
// falls back to the plain app name, and Stop keeps the interrupted track.
#[test]
fn window_title_names_the_current_track() {
    let (mut player, state) = test_player();
    player.known_tracks.insert(
        "song-1".to_string(),
        views::KnownTrack {
            title: "Opening".to_string(),
            artist: "The Sample Band".to_string(),
            album: "First Record".to_string(),
            year: None,
            genre: String::new(),
            track_number: None,
            number: "1. ".to_string(),
            time: "3:30".to_string(),
            duration_ms: 210_000,
        },
    );
    state.blocking_lock().current_track = Some("song-1".to_string());
    assert_eq!(
        player.window_title(),
        "1. The Sample Band - Opening - nostalgia"
    );

    player
        .known_tracks
        .get_mut("song-1")
        .expect("seeded")
        .number = String::new();
    assert_eq!(
        player.window_title(),
        "The Sample Band - Opening - nostalgia"
    );

    player
        .known_tracks
        .get_mut("song-1")
        .expect("seeded")
        .artist = String::new();
    assert_eq!(player.window_title(), "Opening - nostalgia");

    state.blocking_lock().current_track = Some("no-such-song".to_string());
    assert_eq!(player.window_title(), "nostalgia");

    state.blocking_lock().current_track = None;
    assert_eq!(player.window_title(), "nostalgia");

    player
        .known_tracks
        .get_mut("song-1")
        .expect("seeded")
        .number = "1. ".to_string();
    player
        .known_tracks
        .get_mut("song-1")
        .expect("seeded")
        .artist = "The Sample Band".to_string();
    state.blocking_lock().current_track = Some("song-1".to_string());
    state.blocking_lock().stop();
    assert_eq!(
        player.window_title(),
        "1. The Sample Band - Opening - nostalgia"
    );
}
