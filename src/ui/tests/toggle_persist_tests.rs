use super::*;

#[test]
fn toggle_time_display_reaches_the_saved_snapshot() {
    let (mut player, _state) = test_player();
    let store = MemorySettingsStore::default();
    player.settings_store = Some(Box::new(store.clone()));

    let _ = update(&mut player, Message::ToggleTimeDisplay);
    let snapshots = store.snapshots();
    assert_eq!(snapshots.len(), 1);
    assert!(snapshots[0].time_display_remaining());

    let _ = update(&mut player, Message::Ignored);
    assert_eq!(store.snapshots().len(), 1);

    let _ = update(&mut player, Message::ToggleTimeDisplay);
    let snapshots = store.snapshots();
    assert_eq!(snapshots.len(), 2);
    assert!(!snapshots[1].time_display_remaining());
}

#[test]
fn saved_time_display_mode_is_restored_on_boot() {
    let (mut player, _state) = test_player();
    let mut remaining = Settings::default();
    remaining.set_time_display_remaining(true);

    player.adopt_saved_settings(remaining.clone());
    assert_eq!(player.time_display, views::TimeDisplay::Remaining);
    assert_eq!(player.saved_settings, remaining);

    player.adopt_saved_settings(Settings::default());
    assert_eq!(player.time_display, views::TimeDisplay::Elapsed);
    assert_eq!(player.saved_settings, Settings::default());
}

#[test]
fn toggle_always_on_top_reaches_the_saved_snapshot() {
    let (mut player, _state) = test_player();
    let store = MemorySettingsStore::default();
    player.settings_store = Some(Box::new(store.clone()));

    let _ = update(&mut player, Message::ToggleAlwaysOnTop);
    let snapshots = store.snapshots();
    assert_eq!(snapshots.len(), 1);
    assert!(snapshots[0].always_on_top());

    let _ = update(&mut player, Message::Ignored);
    assert_eq!(store.snapshots().len(), 1);

    let _ = update(&mut player, Message::ToggleAlwaysOnTop);
    let snapshots = store.snapshots();
    assert_eq!(snapshots.len(), 2);
    assert!(!snapshots[1].always_on_top());
}

#[test]
fn saved_always_on_top_is_restored_on_boot() {
    let (mut player, _state) = test_player();
    let mut pinned = Settings::default();
    pinned.set_always_on_top(true);

    player.adopt_saved_settings(pinned.clone());
    assert!(player.always_on_top);
    assert_eq!(player.saved_settings, pinned);

    player.adopt_saved_settings(Settings::default());
    assert!(!player.always_on_top);
    assert_eq!(player.saved_settings, Settings::default());
}

#[test]
fn toggle_visualizer_reaches_the_saved_snapshot() {
    let (mut player, _state) = test_player();
    let store = MemorySettingsStore::default();
    player.settings_store = Some(Box::new(store.clone()));

    let _ = update(&mut player, Message::ToggleVisualizer);
    let snapshots = store.snapshots();
    assert_eq!(snapshots.len(), 1);
    assert!(!snapshots[0].visualizer_visible());

    let _ = update(&mut player, Message::Ignored);
    assert_eq!(store.snapshots().len(), 1);

    let _ = update(&mut player, Message::ToggleVisualizer);
    let snapshots = store.snapshots();
    assert_eq!(snapshots.len(), 2);
    assert!(snapshots[1].visualizer_visible());
}

#[test]
fn saved_visualizer_visibility_is_restored_on_boot() {
    let (mut player, _state) = test_player();
    let mut hidden = Settings::default();
    hidden.set_visualizer_visible(false);

    player.adopt_saved_settings(hidden.clone());
    assert!(!player.visualizer_visible);
    assert_eq!(player.saved_settings, hidden);

    player.adopt_saved_settings(Settings::default());
    assert!(player.visualizer_visible);
    assert_eq!(player.saved_settings, Settings::default());
}

#[test]
fn toggle_panels_reach_the_saved_snapshot() {
    let (mut player, _state) = test_player();
    let store = MemorySettingsStore::default();
    player.settings_store = Some(Box::new(store.clone()));

    let _ = update(&mut player, Message::ToggleEqualizerPanel);
    let snapshots = store.snapshots();
    assert_eq!(snapshots.len(), 1);
    assert!(!snapshots[0].eq_panel_visible());
    assert!(snapshots[0].library_panel_visible());

    let _ = update(&mut player, Message::Ignored);
    assert_eq!(store.snapshots().len(), 1);

    let _ = update(&mut player, Message::ToggleLibraryPanel);
    let snapshots = store.snapshots();
    assert_eq!(snapshots.len(), 2);
    assert!(!snapshots[1].eq_panel_visible());
    assert!(!snapshots[1].library_panel_visible());
}

#[test]
fn saved_panel_visibility_is_restored_on_boot() {
    let (mut player, _state) = test_player();
    let mut hidden = Settings::default();
    hidden.set_eq_panel_visible(false);
    hidden.set_library_panel_visible(false);

    player.adopt_saved_settings(hidden.clone());
    assert!(!player.eq_panel_visible);
    assert!(!player.library_panel_visible);
    assert_eq!(player.saved_settings, hidden);

    player.adopt_saved_settings(Settings::default());
    assert!(player.eq_panel_visible);
    assert!(player.library_panel_visible);
    assert_eq!(player.saved_settings, Settings::default());
}

#[test]
fn toggle_window_shade_reaches_the_saved_snapshot() {
    let (mut player, _state) = test_player();
    let store = MemorySettingsStore::default();
    player.settings_store = Some(Box::new(store.clone()));

    let _ = update(&mut player, Message::ToggleWindowShade);
    let snapshots = store.snapshots();
    assert_eq!(snapshots.len(), 1);
    assert!(snapshots[0].shaded());
    assert!(player.shaded);

    let _ = update(&mut player, Message::Ignored);
    assert_eq!(store.snapshots().len(), 1);

    let _ = update(&mut player, Message::ToggleWindowShade);
    let snapshots = store.snapshots();
    assert_eq!(snapshots.len(), 2);
    assert!(!snapshots[1].shaded());
    assert!(!player.shaded);
}

#[test]
fn saved_shade_is_restored_on_boot() {
    let (mut player, _state) = test_player();
    let mut shaded = Settings::default();
    shaded.set_shaded(true);

    player.adopt_saved_settings(shaded.clone());
    assert!(player.shaded);
    assert_eq!(player.saved_settings, shaded);

    player.adopt_saved_settings(Settings::default());
    assert!(!player.shaded);
    assert_eq!(player.saved_settings, Settings::default());
}

#[test]
fn shaded_eq_toggle_saves_nothing() {
    let (mut player, _state) = test_player();
    let store = MemorySettingsStore::default();
    player.settings_store = Some(Box::new(store.clone()));
    player.shaded = true;
    player.saved_settings.set_shaded(true);

    let _ = update(&mut player, Message::ToggleEqualizerPanel);
    assert!(player.eq_panel_visible);
    assert!(store.snapshots().is_empty());
}

#[test]
fn saved_autoscroll_choice_is_restored_on_boot() {
    let (mut player, _state) = test_player();
    assert!(player.marquee_autoscroll);
    let mut stopped = Settings::default();
    stopped.set_autoscroll(false);

    player.adopt_saved_settings(stopped);
    assert!(!player.marquee_autoscroll);

    player.adopt_saved_settings(Settings::default());
    assert!(player.marquee_autoscroll);
}

#[test]
fn toggle_autoscroll_reaches_the_saved_snapshot() {
    let (mut player, _state) = test_player();
    let store = MemorySettingsStore::default();
    player.settings_store = Some(Box::new(store.clone()));

    let _ = update(&mut player, Message::ToggleAutoscroll);
    assert!(!player.marquee_autoscroll);
    assert!(
        !store.snapshots().last().expect("a snapshot").autoscroll(),
        "the autoscroll choice must reach the saved snapshot"
    );
}
