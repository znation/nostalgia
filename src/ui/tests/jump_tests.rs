use super::*;

/// A player showing 25 numbered songs with `current` playing.
fn player_jumping_from(current: &str) -> (WinampPlayer, Arc<Mutex<AppState>>) {
    let (mut player, state) = test_player();
    player.songs.items = numbered_songs(25);
    state.blocking_lock().current_track = Some(current.to_string());
    (player, state)
}

#[test]
fn jump_forward_selects_ten_songs_on() {
    let (mut player, _state) = player_jumping_from("song-3");
    let task = update(&mut player, Message::JumpTracksForward);
    assert_track_selected(task, 0, 13);
}

#[test]
fn jump_back_selects_ten_songs_back() {
    let (mut player, _state) = player_jumping_from("song-15");
    let task = update(&mut player, Message::JumpTracksBack);
    assert_track_selected(task, 0, 5);
}

#[test]
fn jumps_clamp_at_the_list_ends() {
    let (mut player, _state) = player_jumping_from("song-20");
    assert_track_selected(update(&mut player, Message::JumpTracksForward), 0, 24);
    let (mut player, _state) = player_jumping_from("song-4");
    assert_track_selected(update(&mut player, Message::JumpTracksBack), 0, 0);
}

#[test]
fn jumps_ignore_shuffle_and_repeat() {
    let (mut player, state) = player_jumping_from("song-3");
    {
        let mut state = state.blocking_lock();
        state.shuffle = true;
        state.repeat = true;
    }
    assert_track_selected(update(&mut player, Message::JumpTracksForward), 0, 13);
    assert_track_selected(update(&mut player, Message::JumpTracksBack), 0, 0);
}

#[test]
fn jumps_with_no_songs_loaded_do_nothing() {
    let (mut player, _state) = test_player();
    assert_message_schedules_no_work(&mut player, Message::JumpTracksBack);
    assert_message_schedules_no_work(&mut player, Message::JumpTracksForward);
}

#[test]
fn jumps_in_all_songs_use_the_flat_list() {
    let (mut player, state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(second_album_songs()));
    let _ = update(&mut player, Message::AllSongsLoaded(numbered_songs(25)));
    player.current_view = CurrentView::AllSongs;
    state.blocking_lock().current_track = Some("song-2".to_string());

    assert_track_selected(update(&mut player, Message::JumpTracksForward), 1, 12);
}

fn options_position(row: OptionsRow) -> usize {
    OptionsRow::ALL
        .iter()
        .position(|r| std::mem::discriminant(r) == std::mem::discriminant(&row))
        .expect("row is listed in OptionsRow::ALL")
}

// The ten-song rows run the keypad jumps, by mouse and by keyboard, and with
// no song list they only close the menu.
#[test]
fn options_ten_song_rows_jump_like_the_keypad() {
    for (row, expected) in [
        (OptionsRow::JumpTracksBack, 5),
        (OptionsRow::JumpTracksForward, 24),
    ] {
        for by_keyboard in [false, true] {
            let (mut player, _state) = player_jumping_from("song-15");
            player.options_visible = true;
            let task = if by_keyboard {
                player.options_selected = options_position(row);
                update(&mut player, Message::OptionsActivate)
            } else {
                update(&mut player, row.message())
            };
            assert!(!player.options_visible, "menu must close ({row:?})");
            assert_track_selected(task, 0, expected);

            let (mut player, _state) = test_player();
            player.options_visible = true;
            let task = update(&mut player, row.message());
            assert!(!player.options_visible, "menu must close empty ({row:?})");
            assert_no_task(task);
        }
    }
}

// The Jump rows close the menu and open their panel when it is available, by
// mouse and by keyboard; when it is not, they only close the menu.
#[test]
fn options_jump_rows_open_their_panels_or_just_close() {
    for (row, is_time) in [(OptionsRow::JumpTime, true), (OptionsRow::JumpFile, false)] {
        let position = options_position(row);
        for by_keyboard in [false, true] {
            for available in [true, false] {
                let (mut player, _state) = test_player();
                if available {
                    if is_time {
                        player.playback_duration = Some(Duration::from_secs(90));
                    } else {
                        let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
                        player.current_view = CurrentView::Songs;
                    }
                }
                player.options_visible = true;
                let _ = if by_keyboard {
                    player.options_selected = position;
                    update(&mut player, Message::OptionsActivate)
                } else {
                    update(&mut player, row.message())
                };
                let context =
                    format!("time: {is_time}, keyboard: {by_keyboard}, available: {available}");
                assert!(!player.options_visible, "menu must close ({context})");
                assert_eq!(
                    player.jump_time_visible,
                    is_time && available,
                    "jump time flag ({context})"
                );
                assert_eq!(
                    player.jump_file_visible,
                    !is_time && available,
                    "jump file flag ({context})"
                );
            }
        }
    }
}
