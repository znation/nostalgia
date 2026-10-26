//! Plain J's Jump to File panel: opening, filtering, playing a match, and
//! closing.

use super::window_tests::make_current_track_known;
use super::*;

fn player_in_songs() -> (WinampPlayer, Arc<Mutex<AppState>>) {
    let (mut player, state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    player.current_view = CurrentView::Songs;
    (player, state)
}

fn submit(player: &mut WinampPlayer, query: &str) -> Task<Message> {
    let _ = update(player, Message::JumpFileChanged(query.to_string()));
    update(player, Message::JumpFileSubmitted)
}

#[test]
fn toggle_opens_with_an_empty_query_and_closes_again() {
    let (mut player, _state) = player_in_songs();
    player.jump_file_query = "stale".to_string();

    assert_message_schedules_work(&mut player, Message::ToggleJumpFile);
    assert!(player.jump_file_visible);
    assert!(player.jump_file_query.is_empty());

    assert_message_schedules_no_work(&mut player, Message::ToggleJumpFile);
    assert!(!player.jump_file_visible);
}

#[test]
fn toggle_needs_a_showing_song_list() {
    let (mut player, _state) = test_player();
    for view in [
        CurrentView::Artists,
        CurrentView::Albums,
        CurrentView::AllAlbums,
        CurrentView::Playlists,
    ] {
        player.current_view = view;
        assert_message_schedules_no_work(&mut player, Message::ToggleJumpFile);
        assert!(!player.jump_file_visible);
    }
}

#[test]
fn submit_plays_the_first_match_and_closes() {
    let (mut player, _state) = player_in_songs();
    let _ = update(&mut player, Message::ToggleJumpFile);
    let epoch = player.songs.epoch;

    let task = submit(&mut player, "two");

    assert_track_selected(task, epoch, 1);
    assert!(!player.jump_file_visible);
}

#[test]
fn submit_uses_the_all_songs_list_when_it_is_showing() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::AllSongsLoaded(stepping_songs()));
    player.current_view = CurrentView::AllSongs;
    let epoch = player.all_songs.epoch;
    let _ = update(&mut player, Message::ToggleJumpFile);

    let task = submit(&mut player, "three");

    assert_track_selected(task, epoch, 2);
    assert!(!player.jump_file_visible);
}

#[test]
fn a_no_match_submit_keeps_the_panel_and_query() {
    let (mut player, _state) = player_in_songs();
    let _ = update(&mut player, Message::ToggleJumpFile);

    assert_message_schedules_no_work(&mut player, Message::JumpFileChanged("zzz".to_string()));
    assert_message_schedules_no_work(&mut player, Message::JumpFileSubmitted);

    assert!(player.jump_file_visible);
    assert_eq!(player.jump_file_query, "zzz");
}

#[test]
fn a_row_press_dispatches_its_clicked_index() {
    let (mut player, _state) = player_in_songs();
    let _ = update(&mut player, Message::ToggleJumpFile);
    let epoch = player.songs.epoch;

    let task = update(
        &mut player,
        Message::JumpFileRowSelected { epoch, index: 2 },
    );

    assert_track_selected(task, epoch, 2);
    assert!(!player.jump_file_visible);
}

#[test]
fn jump_file_and_the_other_panels_close_each_other() {
    let (mut player, state) = player_in_songs();
    make_current_track_known(&mut player, &state);
    player.playback_duration = Some(Duration::from_secs(30));

    let _ = update(&mut player, Message::ToggleJumpFile);
    assert!(player.jump_file_visible);

    let _ = update(&mut player, Message::ToggleAbout);
    assert!(player.about_visible && !player.jump_file_visible);

    let _ = update(&mut player, Message::ToggleJumpFile);
    assert!(player.jump_file_visible && !player.about_visible);

    let _ = update(&mut player, Message::ToggleFileInfo);
    assert!(player.file_info_visible && !player.jump_file_visible);

    let _ = update(&mut player, Message::ToggleJumpFile);
    assert!(player.jump_file_visible && !player.file_info_visible);

    let _ = update(&mut player, Message::ToggleJumpTime);
    assert!(player.jump_time_visible && !player.jump_file_visible);

    let _ = update(&mut player, Message::ToggleJumpFile);
    assert!(player.jump_file_visible && !player.jump_time_visible);
}

/// Every browse navigation that replaces the showing list clears the panel:
/// each arm in `handlers::browse` and `handlers::browse::search` that changes
/// `current_view` away from a song list. Each case starts from a fresh Songs
/// view with the panel open and feeds the one row the handler needs.
fn assert_navigation_closes_the_panel(setup: impl FnOnce(&mut WinampPlayer), message: Message) {
    let (mut player, _state) = player_in_songs();
    setup(&mut player);
    let _ = update(&mut player, Message::ToggleJumpFile);
    assert!(player.jump_file_visible);

    let _ = update(&mut player, message);

    assert!(!player.jump_file_visible);
}

#[test]
fn every_navigation_that_replaces_the_showing_list_closes_the_panel() {
    assert_navigation_closes_the_panel(|_| {}, Message::ReloadLibrary);
    assert_navigation_closes_the_panel(|_| {}, Message::Back);
    assert_navigation_closes_the_panel(|_| {}, Message::ShowAllSongs);
    assert_navigation_closes_the_panel(|_| {}, Message::ShowAllAlbums);
    assert_navigation_closes_the_panel(|_| {}, Message::ShowPlaylists);
    assert_navigation_closes_the_panel(
        |player| player.artists.items = vec![sample_artist()],
        Message::ArtistSelected { epoch: 0, index: 0 },
    );
    assert_navigation_closes_the_panel(
        |player| player.albums.items = vec![sample_album()],
        Message::AlbumSelected { epoch: 0, index: 0 },
    );
    assert_navigation_closes_the_panel(
        |player| player.playlists.items = vec![sample_playlist()],
        Message::PlaylistSelected { epoch: 0, index: 0 },
    );
    assert_navigation_closes_the_panel(
        |player| player.search_query = "echo".to_string(),
        Message::SearchSubmitted,
    );
}

#[test]
fn toggle_while_shaded_restores_the_window_first() {
    let (mut player, _state) = player_with_window_id();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    player.current_view = CurrentView::Songs;
    player.shaded = true;
    player.unshaded_size = Some(iced::Size::new(800.0, 600.0));

    assert_message_schedules_work(&mut player, Message::ToggleJumpFile);

    assert!(!player.shaded);
    assert!(player.jump_file_visible);
}

#[test]
fn the_root_view_draws_only_the_filtered_row_in_the_panel() {
    let (mut player, _state) = player_in_songs();
    // The base browse list is hidden so the capture names only the panel's
    // rows; the panel itself still filters the same cached list.
    player.library_panel_visible = false;
    player.jump_file_visible = true;
    player.jump_file_query = "two".to_string();

    let capture = captured_root_view_text_bounds(&player);
    let texts: Vec<&str> = capture.texts().collect();
    assert!(texts.contains(&"JUMP TO FILE"), "{texts:?}");
    assert!(texts.contains(&"The Sample Band - Two"), "{texts:?}");
    assert!(!texts.contains(&"The Sample Band - One"), "{texts:?}");
    assert!(!texts.contains(&"The Sample Band - Three"), "{texts:?}");
}
