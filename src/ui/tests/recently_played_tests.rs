use super::*;

fn titles(player: &WinampPlayer) -> Vec<String> {
    player
        .recently_played
        .items
        .iter()
        .map(|song| song.title.clone())
        .collect()
}

// The Recently Played button fetches the sample library's songs in reverse
// order; pressing the first row plays the library's last song, and Back
// returns to Artists.
#[tokio::test]
async fn show_recently_played_lists_reversed_sample_songs_and_plays_the_first() {
    let (mut player, _state) = test_player();

    let task = update(&mut player, Message::ShowRecentlyPlayed);
    let message = task_output(task, "load recently played songs").await;
    let _ = update(&mut player, message);
    assert_view(&player, CurrentView::RecentlyPlayed);

    let expected: Vec<String> = player
        .apple_music_service
        .get_all_songs()
        .await
        .expect("sample songs")
        .into_iter()
        .rev()
        .map(|song| song.title)
        .collect();
    assert_eq!(titles(&player), expected);

    let epoch = player.recently_played.epoch;
    let first_id = player.recently_played.items[0].id.clone();
    let task = update(&mut player, Message::TrackSelected { epoch, index: 0 });
    drive_task(task, "recently played play", |_| {}).await;
    assert_eq!(
        player.state.lock().await.current_track.as_deref(),
        Some(first_id.as_str())
    );

    let _ = update(&mut player, Message::Back);
    assert_view(&player, CurrentView::Artists);
}

#[test]
fn recently_played_load_failed_shows_its_report() {
    let (mut player, _state) = test_player();
    let _ = update(
        &mut player,
        Message::RecentlyPlayedLoadFailed("boom".to_string()),
    );
    assert_eq!(player.recently_played.error.as_deref(), Some("boom"));
}

#[test]
fn a_stale_recently_played_reply_is_ignored() {
    let (mut player, _state) = test_player();
    let stale = player.recently_played.begin_fetch().issued();
    let _newer = player.recently_played.begin_fetch();
    let reply = Box::new(Message::RecentlyPlayedLoaded(stepping_songs()));
    let _ = update(
        &mut player,
        Message::BrowseReply {
            issued: stale,
            reply,
        },
    );
    assert!(player.recently_played.items.is_empty());
}

#[test]
fn recently_played_reverse_and_sort_reorder_only_that_list() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::RecentlyPlayedLoaded(stepping_songs()));
    player.current_view = CurrentView::RecentlyPlayed;
    let before = titles(&player);

    let _ = update(&mut player, Message::ReverseBrowse);
    let mut reversed = before.clone();
    reversed.reverse();
    assert_eq!(titles(&player), reversed);

    let _ = update(&mut player, Message::SortBrowse);
    let mut sorted = before;
    sorted.sort_by_key(|title| title.to_lowercase());
    assert_eq!(titles(&player), sorted);
    assert!(player.songs.items.is_empty());
}

#[test]
fn next_track_in_recently_played_steps_that_list() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(second_album_songs()));
    let _ = update(&mut player, Message::RecentlyPlayedLoaded(stepping_songs()));
    player.current_view = CurrentView::RecentlyPlayed;
    player.state.blocking_lock().current_track = Some("song-1".to_string());

    let task = update(&mut player, Message::NextTrack);
    assert_track_selected(task, player.recently_played.epoch, 1);
}

#[test]
fn preview_end_in_recently_played_steps_that_list() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, _state) = player_playing_stepping_song(&recording, "song-1");
    player.songs.items = second_album_songs();
    let _ = update(&mut player, Message::RecentlyPlayedLoaded(stepping_songs()));
    player.current_view = CurrentView::RecentlyPlayed;
    let epoch = player.recently_played.epoch;

    let selected = drive_playback_finished(&mut player);
    assert!(
        matches!(selected, Message::TrackSelected { epoch: e, index: 1 } if e == epoch),
        "preview end must step the recently played list, got {selected:?}"
    );
}

#[test]
fn jump_to_file_submits_against_the_recently_played_list() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::RecentlyPlayedLoaded(stepping_songs()));
    player.current_view = CurrentView::RecentlyPlayed;
    let epoch = player.recently_played.epoch;
    let _ = update(&mut player, Message::ToggleJumpFile);

    let _ = update(&mut player, Message::JumpFileChanged("three".to_string()));
    let task = update(&mut player, Message::JumpFileSubmitted);

    assert_track_selected(task, epoch, 2);
}

// Open (Enter) on a selected Recently Played row dispatches that row's
// track selection against the list's epoch, like the other flat song list.
#[test]
fn open_selected_in_recently_played_plays_the_selected_row() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::RecentlyPlayedLoaded(stepping_songs()));
    player.current_view = CurrentView::RecentlyPlayed;
    let epoch = player.recently_played.epoch;
    assert_message_schedules_no_work(&mut player, Message::SelectRow { epoch, index: 1 });

    let task = update(&mut player, Message::OpenSelected);

    assert_track_selected(task, epoch, 1);
}

// Alt+Down moves the selected row within Recently Played only and keeps it
// selected.
#[test]
fn move_selected_row_reorders_only_recently_played() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::RecentlyPlayedLoaded(stepping_songs()));
    player.current_view = CurrentView::RecentlyPlayed;
    let before = titles(&player);
    let epoch = player.recently_played.epoch;
    let _ = update(&mut player, Message::SelectRow { epoch, index: 0 });

    let _ = update(&mut player, Message::MoveSelectedRowDown);

    let mut expected = before;
    expected.swap(0, 1);
    assert_eq!(titles(&player), expected);
    assert_eq!(player.recently_played.selected(), Some(1));
    assert!(player.songs.items.is_empty());
}
