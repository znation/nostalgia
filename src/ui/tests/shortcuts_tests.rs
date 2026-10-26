use super::*;

// The system media Play/Pause key dispatches the same toggle as the on-screen
// button and the X/C pair: it pauses a playing preview and resumes it. The
// second press holds Shift, pinning that the media keys ignore that modifier.
#[tokio::test]
async fn media_play_pause_key_toggles_playback() {
    use iced::keyboard::{Event, Key, Modifiers, key};

    let (mut player, state) = player_playing_preview(Arc::new(RecordingAudio::default())).await;
    assert!(state.lock().await.is_playing);

    for (index, modifiers) in [Modifiers::NONE, Modifiers::SHIFT].into_iter().enumerate() {
        let media_play_pause = Event::KeyPressed {
            key: Key::Named(key::Named::MediaPlayPause),
            modified_key: Key::Named(key::Named::MediaPlayPause),
            physical_key: key::Physical::Code(key::Code::KeyA),
            location: iced::keyboard::Location::Standard,
            modifiers,
            text: None,
            repeat: false,
        };
        let message = super::shortcuts::message_for(media_play_pause)
            .expect("MediaPlayPause must map to a message");
        assert!(matches!(message, Message::PlayPause));

        let task = update(&mut player, message);
        assert_transport_settled(task, "media play/pause toggle").await;
        assert_eq!(state.lock().await.is_playing, index == 1);
    }
}

// Enter is the keyboard activation route for a selected browse row: the
// shortcut table maps a plain press to OpenSelected, whose update arm dispatches
// the same TrackSelected playback the Open button does.
#[tokio::test]
async fn enter_opens_the_selected_song() {
    use iced::keyboard::{Event, Key, Modifiers, key};

    let (mut player, state) = test_player();
    seed_browse_lists(&mut player);
    player.current_view = CurrentView::Songs;
    player.songs.loading = false;
    let epoch = player.songs.epoch;
    assert_message_schedules_no_work(&mut player, Message::SelectRow { epoch, index: 0 });

    let enter = Event::KeyPressed {
        key: Key::Named(key::Named::Enter),
        modified_key: Key::Named(key::Named::Enter),
        physical_key: key::Physical::Code(key::Code::Enter),
        location: iced::keyboard::Location::Standard,
        modifiers: Modifiers::NONE,
        text: None,
        repeat: false,
    };
    let message = super::shortcuts::message_for(enter).expect("Enter must map to a message");
    assert!(matches!(message, Message::OpenSelected));

    let task = update(&mut player, message);
    let opened = futures::executor::block_on(task_output(task, "Enter on selected song"));
    match &opened {
        Message::TrackSelected { epoch: e, index: 0 } if *e == epoch => {}
        other => panic!("Enter must dispatch the selected song: {other:?}"),
    }
    drive_task(
        update(&mut player, opened),
        "playback",
        |message| match message {
            Message::TrackPlayed { committed, .. } => assert!(committed),
            other => panic!("unexpected playback task output: {other:?}"),
        },
    )
    .await;

    let state = state.lock().await;
    assert_eq!(state.current_track.as_deref(), Some("song-1"));
    assert!(state.is_playing);
}

// Backspace is the keyboard route to the same single-step Back action the Back
// button dispatches, so it pins both the shortcut table and the handler wiring.
#[test]
fn backspace_steps_back_from_albums() {
    use iced::keyboard::{Event, Key, Modifiers, key};

    let mut player = player_in_albums();
    assert_view(&player, CurrentView::Albums);

    let backspace = Event::KeyPressed {
        key: Key::Named(key::Named::Backspace),
        modified_key: Key::Named(key::Named::Backspace),
        physical_key: key::Physical::Code(key::Code::Backspace),
        location: iced::keyboard::Location::Standard,
        modifiers: Modifiers::NONE,
        text: None,
        repeat: false,
    };
    let message =
        super::shortcuts::message_for(backspace).expect("Backspace must map to a message");
    assert!(matches!(message, Message::Back));

    let _ = update(&mut player, message);
    assert_view(&player, CurrentView::Artists);
}

// The classic A and T keys dispatch the same window and readout toggles the
// title bar's and LCD strip's A and a readout click already press, so they pin
// both the shortcut table and the handler wiring.
#[test]
fn a_toggles_always_on_top_from_the_keyboard() {
    use iced::keyboard::{Event, Key, Modifiers, key};

    let (mut player, _state) = player_with_window_id();
    assert!(!player.always_on_top);

    for (index, modifiers) in [Modifiers::NONE, Modifiers::SHIFT].into_iter().enumerate() {
        let a = Event::KeyPressed {
            key: Key::Character("a".into()),
            modified_key: Key::Character("a".into()),
            physical_key: key::Physical::Code(key::Code::KeyA),
            location: iced::keyboard::Location::Standard,
            modifiers,
            text: None,
            repeat: false,
        };
        let message = super::shortcuts::message_for(a).expect("A must map to a message");
        assert!(matches!(message, Message::ToggleAlwaysOnTop));

        assert_message_schedules_work(&mut player, message);
        assert_eq!(player.always_on_top, index == 0);
    }
}

#[test]
fn t_toggles_the_time_display_from_the_keyboard() {
    use iced::keyboard::{Event, Key, Modifiers, key};

    let (mut player, _state) = test_player();
    assert_eq!(player.time_display, views::TimeDisplay::Elapsed);

    for (index, modifiers) in [Modifiers::NONE, Modifiers::SHIFT].into_iter().enumerate() {
        let t = Event::KeyPressed {
            key: Key::Character("t".into()),
            modified_key: Key::Character("t".into()),
            physical_key: key::Physical::Code(key::Code::KeyT),
            location: iced::keyboard::Location::Standard,
            modifiers,
            text: None,
            repeat: false,
        };
        let message = super::shortcuts::message_for(t).expect("T must map to a message");
        assert!(matches!(message, Message::ToggleTimeDisplay));

        let _ = update(&mut player, message);
        let expected = if index == 0 {
            views::TimeDisplay::Remaining
        } else {
            views::TimeDisplay::Elapsed
        };
        assert_eq!(player.time_display, expected);
    }
}

// The classic E key dispatches the same equalizer-panel show/hide message the
// EQ button beside the balance slider presses, for plain and Shift-held E.
#[test]
fn e_toggles_the_equalizer_panel_from_the_keyboard() {
    use iced::keyboard::{Event, Key, Modifiers, key};

    let (mut player, _state) = test_player();
    assert!(player.eq_panel_visible);

    for (index, modifiers) in [Modifiers::NONE, Modifiers::SHIFT].into_iter().enumerate() {
        let e = Event::KeyPressed {
            key: Key::Character("e".into()),
            modified_key: Key::Character("e".into()),
            physical_key: key::Physical::Code(key::Code::KeyE),
            location: iced::keyboard::Location::Standard,
            modifiers,
            text: None,
            repeat: false,
        };
        let message = super::shortcuts::message_for(e).expect("E must map to a message");
        assert!(matches!(message, Message::ToggleEqualizerPanel));

        assert_message_schedules_no_work(&mut player, message);
        assert_eq!(player.eq_panel_visible, index == 1);
    }
}

#[test]
fn e_in_shade_mode_leaves_the_equalizer_panel_alone() {
    use iced::keyboard::{Event, Key, Modifiers, key};

    let (mut player, _state) = test_player();
    player.shaded = true;
    assert!(player.eq_panel_visible);

    let e = Event::KeyPressed {
        key: Key::Character("e".into()),
        modified_key: Key::Character("e".into()),
        physical_key: key::Physical::Code(key::Code::KeyE),
        location: iced::keyboard::Location::Standard,
        modifiers: Modifiers::NONE,
        text: None,
        repeat: false,
    };
    let message = super::shortcuts::message_for(e).expect("E must map to a message");
    assert!(matches!(message, Message::ToggleEqualizerPanel));

    assert_message_schedules_no_work(&mut player, message);
    assert!(player.eq_panel_visible);
}

// F5 is the keyboard route to the Reload button's library refresh, so it pins
// both the shortcut table and the handler wiring: the mapped message must
// clear an active search, return to Artists, drop the stale rows and error,
// show the loading placeholder, refetch the artists, and leave the typed
// query, the other browse buffers, and playback alone.
#[tokio::test]
async fn f5_reloads_the_library_from_the_keyboard() {
    use iced::keyboard::{Event, Key, Modifiers, key};

    let (mut player, state) = test_player();
    player.current_view = CurrentView::AllSongs;
    player.search_active = true;
    player.search_query = "echo".to_string();
    player.artists.items = vec![sample_artist()];
    player.artists.error = Some("music-library fetch failed: boom".to_string());
    player.artists.loading = false;
    player.all_songs.items = vec![sample_song()];
    let flat_song_id = player.all_songs.items[0].id.clone();
    {
        let mut shared = state.lock().await;
        shared.current_track = Some("song-1".to_string());
        shared.is_playing = true;
    }

    let f5 = Event::KeyPressed {
        key: Key::Named(key::Named::F5),
        modified_key: Key::Named(key::Named::F5),
        physical_key: key::Physical::Code(key::Code::F5),
        location: iced::keyboard::Location::Standard,
        modifiers: Modifiers::NONE,
        text: None,
        repeat: false,
    };
    let message = super::shortcuts::message_for(f5).expect("F5 must map to a message");
    assert!(matches!(message, Message::ReloadLibrary));

    let task = update(&mut player, message);

    assert_view(&player, CurrentView::Artists);
    assert!(!player.search_active);
    assert_eq!(player.search_query, "echo");
    assert!(player.artists.items.is_empty());
    assert!(player.artists.loading);
    assert_eq!(player.artists.error, None);
    assert_eq!(player.all_songs.items.len(), 1);
    assert_eq!(player.all_songs.items[0].id, flat_song_id);

    drive_fetch_and_assert_loaded(
        &mut player,
        task,
        "reload the library's artists with F5",
        |player| &mut player.artists.items,
        |artist| artist.id.as_str(),
        &["artist-1", "artist-2", "artist-3"],
    )
    .await;

    assert_eq!(player.all_songs.items.len(), 1);
    assert_eq!(player.all_songs.items[0].id, flat_song_id);
    {
        let shared = state.lock().await;
        assert_eq!(shared.current_track.as_deref(), Some("song-1"));
        assert!(shared.is_playing);
    }
}

// Ctrl+Shift+1 reaches the same sort as the `Sort` button: the logical key is
// `!` with Shift held, so the mapping must come from the physical key.
#[test]
fn ctrl_shift_1_sorts_the_showing_list_from_the_keyboard() {
    use iced::keyboard::{Event, Key, Location, Modifiers, key};

    let (mut player, _state) = test_player();
    player.artists.items = ["banana", "Apple", "cherry"]
        .into_iter()
        .map(|name| Artist {
            id: name.to_lowercase(),
            name: name.to_string(),
            album_count: 0,
        })
        .collect();
    player.artists.loading = false;

    let key = Key::Character("!".into());
    let event = Event::KeyPressed {
        key: key.clone(),
        modified_key: key,
        physical_key: key::Physical::Code(key::Code::Digit1),
        location: Location::Standard,
        modifiers: Modifiers::CTRL | Modifiers::SHIFT,
        text: None,
        repeat: false,
    };
    let message = super::shortcuts::message_for(event).expect("Ctrl+Shift+1 is bound");
    assert!(matches!(message, Message::SortBrowse));
    let _ = update(&mut player, message);

    let names: Vec<&str> = player
        .artists
        .items
        .iter()
        .map(|artist| artist.name.as_str())
        .collect();
    assert_eq!(names, ["Apple", "banana", "cherry"]);
}
