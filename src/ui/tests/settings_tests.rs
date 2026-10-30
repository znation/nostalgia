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

    let _ = update(
        &mut player,
        Message::EqPresetSelected(PresetChoice::BuiltIn(rock_preset())),
    );
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
fn toggle_eq_auto_reaches_the_saved_snapshot() {
    let (mut player, state) = test_player();
    let store = MemorySettingsStore::default();
    player.settings_store = Some(Box::new(store.clone()));

    let _ = update(&mut player, Message::ToggleEqAuto);
    let on = {
        let state = state.blocking_lock();
        assert!(state.eq_auto);
        Settings::from_state(&state)
    };
    assert_eq!(store.snapshots(), vec![on.clone()]);

    let _ = update(&mut player, Message::Ignored);
    assert_eq!(store.snapshots().len(), 1);

    let _ = update(&mut player, Message::ToggleEqAuto);
    let off = {
        let state = state.blocking_lock();
        assert!(!state.eq_auto);
        Settings::from_state(&state)
    };
    assert_eq!(store.snapshots().len(), 2);
    assert_eq!(store.snapshots()[1], off);
    assert_ne!(store.snapshots()[0], store.snapshots()[1]);
}

#[test]
fn saved_eq_auto_is_restored_through_apply_to() {
    let mut auto = AppState::default();
    auto.eq_auto = true;
    let saved = Settings::from_state(&auto);
    let mut restored = AppState::default();
    saved.apply_to(&mut restored);
    assert!(restored.eq_auto);

    Settings::from_state(&AppState::default()).apply_to(&mut restored);
    assert!(!restored.eq_auto);
}

#[test]
fn saved_current_track_survives_the_persist_round_trip() {
    let mut state = AppState::default();
    state.current_track = Some("song-1".to_string());
    assert!(!state.is_playing && !state.paused, "the track sits stopped");

    // The save path, end to end: snapshot, render, parse.
    let loaded = Settings::from_text(&Settings::from_state(&state).to_text());

    // The boot path: apply the loaded snapshot to a fresh state.
    let mut booted = AppState::default();
    loaded.apply_to(&mut booted);
    assert_eq!(booted.current_track.as_deref(), Some("song-1"));
    assert!(!booted.is_playing, "the restored track sits stopped");
    assert!(!booted.paused, "the restored track is not paused");

    // A missing settings file boots with no track.
    let mut from_defaults = AppState::default();
    Settings::default().apply_to(&mut from_defaults);
    assert_eq!(from_defaults.current_track, None);
}

#[test]
fn the_stored_sessions_token_reaches_the_saved_snapshot() {
    let (mut player, _state) = test_player();
    let store = MemorySettingsStore::default();
    player.settings_store = Some(Box::new(store.clone()));

    // The service holds a session only after a browser sign-in (or a test
    // seed standing in for one); the save path reads the token out of that
    // same session, so its change alone triggers the write — no other value
    // moved.
    player
        .apple_music_service
        .seed_session("ey1.dev.ey", "ey1.user.ey")
        .unwrap();

    let _ = update(&mut player, Message::Ignored);

    let snapshots = store.snapshots();
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].music_kit_user_token(), Some("ey1.user.ey"));
}

#[test]
fn a_saved_song_list_reaches_the_saved_snapshot() {
    let (mut player, _state) = test_player();
    let store = MemorySettingsStore::default();
    player.settings_store = Some(Box::new(store.clone()));

    let _ = update(&mut player, Message::SongsLoaded(numbered_songs(2)));
    player.current_view = CurrentView::Songs;
    let _ = update(&mut player, Message::ToggleSaveList);
    let _ = update(
        &mut player,
        Message::SaveListNameChanged("Morning Drive".to_string()),
    );
    let _ = update(&mut player, Message::SaveListSubmitted);
    let _ = update(&mut player, Message::Ignored);

    let snapshots = store.snapshots();
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].saved_playlists().len(), 1);
    assert_eq!(snapshots[0].saved_playlists()[0].name, "Morning Drive");
    assert_eq!(
        snapshots[0].saved_playlists()[0].song_ids,
        vec!["song-0", "song-1"]
    );
    assert!(
        snapshots[0]
            .to_text()
            .contains("user_playlist=Morning Drive:song-0,song-1")
    );

    // An unchanged follow-up writes nothing more.
    let _ = update(&mut player, Message::Ignored);
    assert_eq!(store.snapshots().len(), 1);
}

#[test]
fn saved_song_lists_are_restored_on_boot() {
    let (mut player, _state) = test_player();
    let mut saved = Settings::default();
    saved.set_saved_playlists(vec![crate::settings::SavedPlaylist {
        name: "Morning Drive".to_string(),
        song_ids: vec!["song-1".to_string()],
    }]);

    player.adopt_saved_settings(saved);

    assert_eq!(player.saved_playlists.len(), 1);
    assert_eq!(player.saved_playlists[0].name, "Morning Drive");
}

// A launch with a saved signed-out Recently Played list seeds its ids bit
// for bit, and the seed holds snapshot parity: the boot's persist saves
// nothing back, so the whole-file write the first message would otherwise
// schedule cannot erase the `recently_played=` line. When the resolve
// answers, the rows write the (possibly shortened, here unanswerable
// `song-9` drops) list back — like a stale saved-playlist id disappears.
#[test]
fn saved_recently_played_is_restored_on_boot_and_seeded_until_the_answer() {
    let (mut player, _state) = test_player();
    let store = MemorySettingsStore::default();
    player.settings_store = Some(Box::new(store.clone()));

    let mut saved = Settings::default();
    saved.set_recently_played(vec![
        "song-3".to_string(),
        "song-9".to_string(),
        "song-1".to_string(),
    ]);
    player.adopt_saved_settings(saved);
    assert_eq!(
        player.recently_played_seeded,
        vec![
            "song-3".to_string(),
            "song-9".to_string(),
            "song-1".to_string()
        ]
    );

    // The boot's persist carries the seed, so the saved file and the
    // snapshot compare equal and nothing is written.
    let _ = update(&mut player, Message::Ignored);
    assert_eq!(store.snapshots().len(), 0);

    // The resolve answers: `song-9` is unanswerable, the rows land, and the
    // next persist shortens the file's list to what the library names.
    let songs = stepping_songs();
    let _ = update(
        &mut player,
        Message::RecentlyPlayedResolved {
            songs: vec![Some(songs[2].clone()), None, Some(songs[0].clone())],
        },
    );
    let _ = update(&mut player, Message::Ignored);
    let snapshots = store.snapshots();
    assert_eq!(snapshots.len(), 1);
    assert_eq!(
        snapshots[0].recently_played(),
        &["song-3".to_string(), "song-1".to_string()]
    );
    assert!(
        snapshots[0]
            .to_text()
            .contains("recently_played=song-3,song-1")
    );

    // The written snapshot is now the remembered one; nothing more saves.
    let _ = update(&mut player, Message::Ignored);
    assert_eq!(store.snapshots().len(), 1);
}

// A play that commits while the boot restore is in flight joins the seed in
// the pre-answer persist: the save carries the seeded ids with the play's
// newest-first ids on top, so a quit during the answer's window keeps every
// row the previous file held plus the run's own play — and when the reply
// then lands, the restored composition equals that remembered snapshot, so
// no extra write happens.
// A play that commits while the boot restore is in flight joins the seed in
// the pre-answer persist: the save carries the seeded ids with the play's
// newest-first ids on top, so a quit during the answer's window keeps every
// row the previous file held plus the run's own play — and when the reply
// then lands, the restored composition equals that remembered snapshot, so
// no extra write happens.
#[test]
fn a_mid_restore_play_persists_the_seed_rows_with_the_play_on_top() {
    let (mut player, _state) = test_player();
    let store = MemorySettingsStore::default();
    player.settings_store = Some(Box::new(store.clone()));

    let mut saved = Settings::default();
    saved.set_recently_played(vec!["song-3".to_string(), "song-1".to_string()]);
    player.adopt_saved_settings(saved);

    // The boot issues the restore's resolve, as `main`'s task chain does.
    let _ = update(&mut player, Message::ResolveRecentlyPlayed);

    // The boot's persist holds the seed: nothing is written yet.
    let _ = update(&mut player, Message::Ignored);
    assert_eq!(store.snapshots().len(), 0);

    // A play commits while the resolve is still in flight.
    let songs = stepping_songs();
    player.played_songs.record(songs[1].clone());
    let _ = update(&mut player, Message::Ignored);

    // The pre-answer persist keeps the file whole: the play's ids on top of
    // the seed, not a replace-with-one-row write that would erase the seed.
    let snapshots = store.snapshots();
    assert_eq!(snapshots.len(), 1);
    assert_eq!(
        snapshots[0].recently_played(),
        &[
            "song-2".to_string(),
            "song-3".to_string(),
            "song-1".to_string()
        ]
    );
    assert!(
        snapshots[0]
            .to_text()
            .contains("recently_played=song-2,song-3,song-1")
    );

    // The resolve answers in seed order (newest first, song-9-style
    // unanswerable id absent here); the restored composition is the same
    // list the pre-answer persist saved, so nothing more writes.
    let _ = update(
        &mut player,
        Message::RecentlyPlayedResolved {
            songs: vec![Some(songs[2].clone()), Some(songs[0].clone())],
        },
    );
    let _ = update(&mut player, Message::Ignored);
    assert_eq!(store.snapshots().len(), 1);
}

// A mid-restore play that replays an id the seed already names must not let
// the pre-answer persist write that id twice: the composition folds the
// seed's naming of it under the play's newest-first row instead of standing
// it again, so the file never gains a duplicated `recently_played=` row.
#[test]
fn a_mid_restore_replay_of_a_seed_row_persists_it_once() {
    let (mut player, _state) = test_player();
    let store = MemorySettingsStore::default();
    player.settings_store = Some(Box::new(store.clone()));

    let mut saved = Settings::default();
    saved.set_recently_played(vec!["song-3".to_string(), "song-1".to_string()]);
    player.adopt_saved_settings(saved);

    // The boot issues the restore's resolve, as `main`'s task chain does.
    let _ = update(&mut player, Message::ResolveRecentlyPlayed);
    let _ = update(&mut player, Message::Ignored);
    assert_eq!(store.snapshots().len(), 0);

    // A replay of the seed's OLDEST row (song-1) commits while the resolve
    // is still in flight: the play's id now tops the composition, so the
    // snapshot differs from the file and exactly one save happens — the
    // seed's naming of song-1 folded under the play, not stood a second
    // time (`song-1,song-3,song-1` would read as a doubled row).
    let songs = stepping_songs();
    player.played_songs.record(songs[0].clone());
    let _ = update(&mut player, Message::Ignored);
    let snapshots = store.snapshots();
    assert_eq!(snapshots.len(), 1);
    assert_eq!(
        snapshots[0].recently_played(),
        &["song-1".to_string(), "song-3".to_string()]
    );
    assert!(
        snapshots[0]
            .to_text()
            .contains("recently_played=song-1,song-3")
    );

    // The reply answers both seed ids in save order: the replay drops back
    // to its restored position and the re-recorded mid-flight play pins the
    // composition at the list the pre-answer persist already wrote, so
    // nothing more saves.
    let _ = update(
        &mut player,
        Message::RecentlyPlayedResolved {
            songs: vec![Some(songs[2].clone()), Some(songs[0].clone())],
        },
    );
    let _ = update(&mut player, Message::Ignored);
    assert_eq!(store.snapshots().len(), 1);
}

#[test]
fn an_unchanged_token_writes_nothing() {
    let (mut player, _state) = test_player();
    let store = MemorySettingsStore::default();
    player.settings_store = Some(Box::new(store.clone()));

    player
        .apple_music_service
        .seed_session("ey1.dev.ey", "ey1.user.ey")
        .unwrap();
    let _ = update(&mut player, Message::Ignored);

    // A seeded launch holds the same token its snapshot already saved, so a
    // later poll message must not rewrite the file for the token's sake.
    let _ = update(&mut player, Message::Ignored);
    assert_eq!(store.snapshots().len(), 1);
}

#[test]
fn a_sessionless_run_keeps_the_adopted_saved_token() {
    let (mut player, _state) = test_player();
    let store = MemorySettingsStore::default();
    player.settings_store = Some(Box::new(store.clone()));

    // A launch that passed a saved token to `init_service` but whose session
    // never arrives — sign-in skipped with the variable unset, still in
    // flight, or failed — must keep that token in every write it makes: the
    // docs promise the saved line is left alone until a fresh sign-in
    // overwrites it, so an ordinary persist before the sign-in lands cannot
    // erase it.
    let mut adopted = Settings::from_state(&player.state.blocking_lock());
    adopted.set_music_kit_user_token(Some("ey1.user.ey".to_string()));
    player.adopt_saved_settings(adopted);

    let _ = update(&mut player, Message::VolumeChange(0.8));

    let snapshots = store.snapshots();
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].music_kit_user_token(), Some("ey1.user.ey"));
}

#[test]
fn the_landed_sessions_token_replaces_the_adopted_one() {
    let (mut player, _state) = test_player();
    let store = MemorySettingsStore::default();
    player.settings_store = Some(Box::new(store.clone()));

    // A saved line from an earlier sign-in stays through session-less
    // persists, and the session that finally lands replaces it in the next
    // write, so a rotated token is never shadowed by the stale value.
    let mut adopted = Settings::from_state(&player.state.blocking_lock());
    adopted.set_music_kit_user_token(Some("old.user".to_string()));
    player.adopt_saved_settings(adopted);
    player
        .apple_music_service
        .seed_session("ey1.dev.ey", "ey1.user.ey")
        .unwrap();
    let _ = update(&mut player, Message::VolumeChange(0.8));

    let snapshots = store.snapshots();
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].music_kit_user_token(), Some("ey1.user.ey"));
}

#[test]
fn a_sessionless_run_saves_no_token_line() {
    let (mut player, _state) = test_player();
    let store = MemorySettingsStore::default();
    player.settings_store = Some(Box::new(store.clone()));

    let _ = update(&mut player, Message::VolumeChange(0.8));

    let snapshots = store.snapshots();
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].music_kit_user_token(), None);
    assert!(!snapshots[0].to_text().contains("music_kit_user_token"));
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

#[test]
fn window_moved_reaches_the_saved_snapshot() {
    let (mut player, _state) = test_player();
    let store = MemorySettingsStore::default();
    player.settings_store = Some(Box::new(store.clone()));

    let _ = update(
        &mut player,
        Message::WindowMoved(iced::Point::new(300.0, 200.0)),
    );
    let snapshots = store.snapshots();
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].window_position(), Some((300.0, 200.0)));

    let _ = update(
        &mut player,
        Message::WindowMoved(iced::Point::new(300.0, 200.0)),
    );
    assert_eq!(store.snapshots().len(), 1);
}

#[test]
fn saved_window_position_is_restored_on_boot() {
    let (mut player, _state) = test_player();
    let mut moved = Settings::default();
    moved.set_window_position(Some((10.0, -20.0)));

    player.adopt_saved_settings(moved);
    assert_eq!(player.window_position, Some((10.0, -20.0)));

    player.adopt_saved_settings(Settings::default());
    assert_eq!(player.window_position, None);
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

#[test]
fn launch_position_clamps_the_saved_point_or_uses_the_default() {
    use crate::ui::handlers::window::launch_position;
    assert!(matches!(
        launch_position(Some((10.0, 20.0))),
        iced::window::Position::SpecificWith(_)
    ));
    assert!(matches!(
        launch_position(None),
        iced::window::Position::Default
    ));
}

#[test]
fn clamp_into_monitor_passes_an_in_bounds_point_through() {
    use crate::ui::handlers::window::clamp_into_monitor;
    let point = iced::Point::new(40.0, 60.0);
    assert_eq!(
        clamp_into_monitor(
            point,
            iced::Size::new(200.0, 100.0),
            iced::Size::new(1280.0, 720.0)
        ),
        point
    );
}

#[test]
fn clamp_into_monitor_snaps_points_beyond_each_edge_independently() {
    use crate::ui::handlers::window::clamp_into_monitor;
    // x is past the right edge, y is fine: only x clamps.
    assert_eq!(
        clamp_into_monitor(
            iced::Point::new(2000.0, 60.0),
            iced::Size::new(200.0, 100.0),
            iced::Size::new(1280.0, 720.0)
        ),
        iced::Point::new(1080.0, 60.0)
    );
    // y is past the bottom edge, x is fine: only y clamps.
    assert_eq!(
        clamp_into_monitor(
            iced::Point::new(40.0, 2000.0),
            iced::Size::new(200.0, 100.0),
            iced::Size::new(1280.0, 720.0)
        ),
        iced::Point::new(40.0, 620.0)
    );
}

#[test]
fn clamp_into_monitor_snaps_a_negative_coordinate_to_zero() {
    use crate::ui::handlers::window::clamp_into_monitor;
    assert_eq!(
        clamp_into_monitor(
            iced::Point::new(-50.0, -1.0),
            iced::Size::new(200.0, 100.0),
            iced::Size::new(1280.0, 720.0)
        ),
        iced::Point::ORIGIN
    );
}

#[test]
fn clamp_into_monitor_pins_a_monitor_smaller_than_the_window_at_zero() {
    use crate::ui::handlers::window::clamp_into_monitor;
    assert_eq!(
        clamp_into_monitor(
            iced::Point::new(960.0, 540.0),
            iced::Size::new(1280.0, 720.0),
            iced::Size::new(800.0, 600.0)
        ),
        iced::Point::ORIGIN
    );
}

fn saved_library_view(store: &MemorySettingsStore) -> crate::settings::library_view::LibraryView {
    store
        .snapshots()
        .last()
        .expect("a snapshot must have been saved")
        .library_view()
}

#[test]
fn library_view_reaches_the_saved_snapshot() {
    use crate::settings::library_view::LibraryView;

    let (mut player, _state) = test_player();
    let store = MemorySettingsStore::default();
    player.settings_store = Some(Box::new(store.clone()));

    let _ = update(&mut player, Message::ShowAllSongs);
    assert_eq!(saved_library_view(&store), LibraryView::AllSongs);

    let _ = update(&mut player, Message::ShowPlaylists);
    player.current_view = CurrentView::Songs;
    player.songs_origin = SongsOrigin::Playlist;
    let _ = update(&mut player, Message::Ignored);
    assert_eq!(saved_library_view(&store), LibraryView::Playlists);

    player.songs_origin = SongsOrigin::Album;
    let _ = update(&mut player, Message::Ignored);
    assert_eq!(saved_library_view(&store), LibraryView::Artists);

    let _ = update(&mut player, Message::ShowAllAlbums);
    assert_eq!(saved_library_view(&store), LibraryView::AllAlbums);
    player.search_active = true;
    let _ = update(&mut player, Message::Ignored);
    assert_eq!(saved_library_view(&store), LibraryView::Artists);
    player.search_active = false;

    let _ = update(&mut player, Message::ShowGenres);
    assert_eq!(saved_library_view(&store), LibraryView::Genres);
    player.current_view = CurrentView::Songs;
    player.songs_origin = SongsOrigin::Genre;
    let _ = update(&mut player, Message::Ignored);
    assert_eq!(saved_library_view(&store), LibraryView::Genres);

    let _ = update(&mut player, Message::ShowYears);
    assert_eq!(saved_library_view(&store), LibraryView::Years);
    player.current_view = CurrentView::Songs;
    player.songs_origin = SongsOrigin::Year;
    let _ = update(&mut player, Message::Ignored);
    assert_eq!(saved_library_view(&store), LibraryView::Years);

    let _ = update(&mut player, Message::ShowComposers);
    assert_eq!(saved_library_view(&store), LibraryView::Composers);
    player.current_view = CurrentView::Songs;
    player.songs_origin = SongsOrigin::Composer;
    let _ = update(&mut player, Message::Ignored);
    assert_eq!(saved_library_view(&store), LibraryView::Composers);
}

#[test]
fn restore_view_opens_the_saved_list() {
    use crate::settings::library_view::LibraryView;

    let (mut player, _state) = test_player();
    let _ =
        crate::ui::handlers::browse::lists::restore_view(&mut player, LibraryView::RecentlyAdded);
    assert_eq!(player.current_view, CurrentView::RecentlyAdded);
    let _ = crate::ui::handlers::browse::lists::restore_view(&mut player, LibraryView::Artists);
    assert_eq!(player.current_view, CurrentView::RecentlyAdded);

    let (mut player, _state) = test_player();
    let _ = crate::ui::handlers::browse::lists::restore_view(&mut player, LibraryView::Artists);
    assert_eq!(player.current_view, CurrentView::Artists);

    let (mut player, _state) = test_player();
    let _ = crate::ui::handlers::browse::lists::restore_view(&mut player, LibraryView::Genres);
    assert_eq!(player.current_view, CurrentView::Genres);
    assert!(player.genres.loading);

    let (mut player, _state) = test_player();
    let _ = crate::ui::handlers::browse::lists::restore_view(&mut player, LibraryView::Years);
    assert_eq!(player.current_view, CurrentView::Years);
    assert!(player.years.loading);

    let (mut player, _state) = test_player();
    let _ = crate::ui::handlers::browse::lists::restore_view(&mut player, LibraryView::Composers);
    assert_eq!(player.current_view, CurrentView::Composers);
    assert!(player.composers.loading);
}

#[test]
fn session_ready_refetches_only_the_flat_list_showing() {
    let (mut player, _state) = test_player();
    player.current_view = CurrentView::AllAlbums;
    player.artists.loading = false;
    let _ = update(&mut player, Message::SessionReady);
    assert_eq!(player.current_view, CurrentView::AllAlbums);
    assert!(player.all_albums.loading);
    assert!(player.artists.loading);

    let (mut player, _state) = test_player();
    player.current_view = CurrentView::Genres;
    player.artists.loading = false;
    let _ = update(&mut player, Message::SessionReady);
    assert_eq!(player.current_view, CurrentView::Genres);
    assert!(player.genres.loading);

    let (mut player, _state) = test_player();
    player.current_view = CurrentView::Years;
    player.artists.loading = false;
    let _ = update(&mut player, Message::SessionReady);
    assert_eq!(player.current_view, CurrentView::Years);
    assert!(player.years.loading);

    let (mut player, _state) = test_player();
    player.current_view = CurrentView::Composers;
    player.artists.loading = false;
    let _ = update(&mut player, Message::SessionReady);
    assert_eq!(player.current_view, CurrentView::Composers);
    assert!(player.composers.loading);

    let (mut player, _state) = test_player();
    player.current_view = CurrentView::Albums;
    player.artists.loading = false;
    let _ = update(&mut player, Message::SessionReady);
    assert_eq!(player.current_view, CurrentView::Albums);
    assert!(player.artists.loading);

    let (mut player, _state) = test_player();
    player.current_view = CurrentView::AllSongs;
    player.search_active = true;
    let _ = update(&mut player, Message::SessionReady);
    assert_eq!(player.current_view, CurrentView::AllSongs);
    assert!(player.search_active);
}

// Saving a preset through the naming panel reaches the saved snapshot as an
// `eq_custom_preset=` row: the player carries its `custom_eq_presets` list
// onto the snapshot before `to_text`, so one save writes exactly one new
// snapshot and the panel open/close and typing alone write none.
#[test]
fn saving_a_preset_reaches_the_saved_snapshot() {
    let (mut player, state) = test_player();
    let store = MemorySettingsStore::default();
    player.settings_store = Some(Box::new(store.clone()));
    let _ = update(
        &mut player,
        Message::EqPresetSelected(PresetChoice::BuiltIn(rock_preset())),
    );

    let _ = update(
        &mut player,
        Message::SavePresetNameChanged("Very Fuzzed".to_string()),
    );
    let _ = update(&mut player, Message::SavePresetSubmitted);

    let expected = {
        let state = state.blocking_lock();
        let mut snapshot = Settings::from_state(&state);
        snapshot.set_eq_custom_presets(player.custom_eq_presets.clone());
        snapshot
    };
    assert_eq!(store.snapshots(), vec![expected]);

    // Opening and closing the panel alone writes nothing.
    let _ = update(&mut player, Message::ToggleSavePreset);
    let _ = update(&mut player, Message::ToggleSavePreset);
    assert_eq!(store.snapshots().len(), 1);
}

// A relaunch adopts the saved custom curves: `adopt_saved_settings` reads
// them off the loaded snapshot, so the pick list offers them again.
#[test]
fn the_saved_custom_presets_are_adopted_at_launch() {
    let (mut player, _state) = test_player();
    let mut bands: [f32; BAND_COUNT] = [0.0; BAND_COUNT];
    bands[0] = GAIN_MAX_DB;
    let mut saved = Settings::default();
    saved.set_eq_custom_presets(vec![CustomPreset::from_curve(
        String::from("Very Fuzzed"),
        2.5,
        bands,
    )]);

    player.adopt_saved_settings(saved);

    assert_eq!(player.custom_eq_presets.len(), 1);
    assert_eq!(player.custom_eq_presets[0].name, "Very Fuzzed");
    assert_eq!(player.custom_eq_presets[0].preamp, 2.5);
    assert_eq!(player.custom_eq_presets[0].bands, bands);
}
