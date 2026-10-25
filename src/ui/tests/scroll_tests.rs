use super::*;

#[tokio::test]
async fn back_schedules_the_restored_list_to_the_top() {
    let mut player = player_in_albums();

    assert_task_snaps_browse_list_to_top(update(&mut player, Message::Back), "back").await;

    assert_view(&player, CurrentView::Artists);
}

// The eight arms that open a fresh browse list must each batch the
// snap-to-top scroll with their fetch; dropping one leaves that navigation
// (or search) showing the previous list's scroll offset.
#[tokio::test]
async fn every_browse_navigation_batches_a_snap_to_top_task() {
    let (mut player, _state) = test_player();
    player.artists.items = vec![sample_artist()];
    assert_task_snaps_browse_list_to_top(
        update(&mut player, Message::ArtistSelected { epoch: 0, index: 0 }),
        "artist selection",
    )
    .await;

    let (mut player, _state) = test_player();
    player.albums.items = vec![sample_album()];
    assert_task_snaps_browse_list_to_top(
        update(&mut player, Message::AlbumSelected { epoch: 0, index: 0 }),
        "album selection",
    )
    .await;

    let (mut player, _state) = test_player();
    player.playlists.items = vec![sample_playlist()];
    assert_task_snaps_browse_list_to_top(
        update(
            &mut player,
            Message::PlaylistSelected { epoch: 0, index: 0 },
        ),
        "playlist selection",
    )
    .await;

    let (mut player, _state) = test_player();
    assert_task_snaps_browse_list_to_top(update(&mut player, Message::ShowAllSongs), "all songs")
        .await;

    let (mut player, _state) = test_player();
    assert_task_snaps_browse_list_to_top(update(&mut player, Message::ShowAllAlbums), "all albums")
        .await;

    let (mut player, _state) = test_player();
    assert_task_snaps_browse_list_to_top(update(&mut player, Message::ShowPlaylists), "playlists")
        .await;

    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SearchChanged("echo".to_string()));
    assert_task_snaps_browse_list_to_top(update(&mut player, Message::SearchSubmitted), "search")
        .await;

    let (mut player, _state) = test_player();
    assert_task_snaps_browse_list_to_top(update(&mut player, Message::ReloadLibrary), "reload")
        .await;
}

// The Top button only asks iced to snap the browse scrollable back to its
// first row; it must leave the view and the shared playback state alone.
#[test]
fn scroll_browse_to_top_schedules_a_scroll_task() {
    let (mut player, state) = test_player();

    futures::executor::block_on(assert_task_snaps_browse_list_to_top(
        update(&mut player, Message::ScrollBrowseToTop),
        "top",
    ));

    assert_view(&player, CurrentView::Artists);
    assert!(state.blocking_lock().current_track.is_none());
    assert!(!state.blocking_lock().is_playing);
}

// The End key only asks iced to snap the browse scrollable to its last row;
// it must leave the view and the shared playback state alone.
#[test]
fn scroll_browse_to_bottom_schedules_a_scroll_task() {
    let (mut player, state) = test_player();

    futures::executor::block_on(assert_task_snaps_browse_list_to_end(
        update(&mut player, Message::ScrollBrowseToBottom),
        "bottom",
    ));

    assert_view(&player, CurrentView::Artists);
    assert!(state.blocking_lock().current_track.is_none());
    assert!(!state.blocking_lock().is_playing);
}

// The Page Up and Page Down keys only ask iced to scroll the browse list by
// one fixed page, relative to its current offset; they must leave the view and
// the shared playback state alone.
#[test]
fn scroll_browse_page_keys_schedule_page_sized_scrolls() {
    let (mut player, state) = test_player();

    futures::executor::block_on(assert_task_scrolls_browse_list_by(
        update(&mut player, Message::ScrollBrowsePageDown),
        "page down",
        BROWSE_PAGE_HEIGHT,
    ));
    assert_view(&player, CurrentView::Artists);
    assert!(state.blocking_lock().current_track.is_none());
    assert!(!state.blocking_lock().is_playing);

    futures::executor::block_on(assert_task_scrolls_browse_list_by(
        update(&mut player, Message::ScrollBrowsePageUp),
        "page up",
        -BROWSE_PAGE_HEIGHT,
    ));
    assert_view(&player, CurrentView::Artists);
    assert!(state.blocking_lock().current_track.is_none());
    assert!(!state.blocking_lock().is_playing);
}

// A pressed row is already in view, so the direct-press path deliberately
// leaves the list where it is.
#[test]
fn a_row_press_does_not_scroll_the_list() {
    let (mut player, _state) = player_stepping_from(Some("song-1"));
    player.current_view = CurrentView::Songs;

    futures::executor::block_on(assert_task_scrolls_nothing(
        update(&mut player, Message::TrackSelected { epoch: 0, index: 1 }),
        "row press",
    ));
}
