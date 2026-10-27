use super::*;

#[test]
fn a_changed_setting_is_saved_once_and_unchanged_updates_are_not() {
    let (mut player, state) = test_player();
    let store = MemorySettingsStore::default();
    player.settings_store = Some(Box::new(store.clone()));

    let _ = update(&mut player, Message::VolumeChange(0.8));
    let expected = Settings::from_state(&state.blocking_lock());
    assert_eq!(store.snapshots(), vec![expected]);
    assert_eq!(state.blocking_lock().volume(), 0.8);

    let _ = update(&mut player, Message::Ignored);
    assert_eq!(store.snapshots().len(), 1);
}

#[test]
fn repeat_and_equalizer_changes_reach_the_saved_snapshot() {
    let (mut player, state) = test_player();
    let store = MemorySettingsStore::default();
    player.settings_store = Some(Box::new(store.clone()));

    let _ = update(&mut player, Message::ToggleRepeat);
    assert_eq!(store.snapshots().len(), 1);
    assert_eq!(
        store.snapshots()[0],
        Settings::from_state(&state.blocking_lock())
    );

    let _ = update(&mut player, Message::EqPresetSelected(rock_preset()));
    let current = {
        let state = state.blocking_lock();
        assert!(state.repeat);
        assert_eq!(state.eq_preset(), Some(rock_preset()));
        Settings::from_state(&state)
    };
    assert_eq!(store.snapshots().len(), 2);
    assert_eq!(store.snapshots()[1], current);
}

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
fn window_id_resolved_applies_a_saved_always_on_top_level() {
    let (mut player, _state) = test_player();
    player.always_on_top = true;
    let id = iced::window::Id::unique();

    assert_message_schedules_work(&mut player, Message::WindowIdResolved(Some(id)));
    assert_eq!(player.window_id, Some(id));

    player.always_on_top = false;
    let id = iced::window::Id::unique();

    assert_message_schedules_no_work(&mut player, Message::WindowIdResolved(Some(id)));
    assert_eq!(player.window_id, Some(id));
}

// A fresh fetch that replaces a sorted parent list must drop the saved sort
// permutation: the replacement's rows are already in library order, so Back
// must not reapply the old sort's inverse to them. The sign-in re-issuing
// `LoadArtists` is the production path that can replace the list the user
// already sorted while a child list is open.
#[test]
fn a_fresh_artists_reply_drops_the_saved_sort_permutation() {
    let (mut player, _state) = test_player();
    player.artists.items = vec![
        Artist {
            id: "zulu".to_string(),
            name: "Zulu".to_string(),
            album_count: 0,
        },
        Artist {
            id: "alpha".to_string(),
            name: "alpha".to_string(),
            album_count: 0,
        },
        Artist {
            id: "bravo".to_string(),
            name: "bravo".to_string(),
            album_count: 0,
        },
    ];
    player.artists.loading = false;
    let _ = update(&mut player, Message::SortBrowse);
    assert_eq!(artist_names(&player), ["alpha", "bravo", "Zulu"]);

    // The refresh lands with a different same-length order while the child
    // list is showing, so the saved permutation no longer describes these
    // rows.
    let _ = update(
        &mut player,
        Message::ArtistsLoaded(vec![
            Artist {
                id: "bravo".to_string(),
                name: "bravo".to_string(),
                album_count: 0,
            },
            Artist {
                id: "zulu".to_string(),
                name: "Zulu".to_string(),
                album_count: 0,
            },
            Artist {
                id: "alpha".to_string(),
                name: "alpha".to_string(),
                album_count: 0,
            },
        ]),
    );
    assert_eq!(artist_names(&player), ["bravo", "Zulu", "alpha"]);

    player.current_view = CurrentView::Albums;
    let epoch = player.artists.epoch;
    let _ = update(&mut player, Message::Back);

    assert_eq!(
        artist_names(&player),
        ["bravo", "Zulu", "alpha"],
        "a fresh reply's library order must survive Back"
    );
    assert_eq!(
        player.artists.epoch, epoch,
        "restoring a list with no saved sort must be a no-op"
    );
}
