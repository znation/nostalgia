use super::*;

/// `count` songs with ids `song-0`… and matching titles, enough for a
/// ten-song jump that `stepping_songs` is too short for.
fn numbered_songs(count: usize) -> Vec<Song> {
    (0..count)
        .map(|number| Song {
            id: format!("song-{number}"),
            title: format!("Song {number}"),
            ..sample_song()
        })
        .collect()
}

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
