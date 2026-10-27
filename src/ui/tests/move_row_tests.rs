use super::*;

fn artist(name: &str) -> Artist {
    Artist {
        id: name.to_string(),
        name: name.to_string(),
        album_count: 0,
    }
}

fn select(player: &mut WinampPlayer, epoch: u64, index: usize) {
    let _ = update(player, Message::SelectRow { epoch, index });
}

#[test]
fn move_down_swaps_a_song_with_the_next_and_bumps_the_epoch() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    player.current_view = CurrentView::Songs;
    let old_epoch = player.songs.epoch;
    let ids: Vec<String> = player.songs.items.iter().map(|s| s.id.clone()).collect();
    let titles = player.songs.titles.clone();
    let labels = player.songs.labels.clone();
    select(&mut player, old_epoch, 0);

    let _ = update(&mut player, Message::MoveSelectedRowDown);

    let got: Vec<String> = player.songs.items.iter().map(|s| s.id.clone()).collect();
    assert_eq!(got[0], ids[1]);
    assert_eq!(got[1], ids[0]);
    assert_eq!(
        player.songs.titles[..2],
        [titles[1].clone(), titles[0].clone()]
    );
    assert_eq!(
        player.songs.labels[..2],
        [labels[1].clone(), labels[0].clone()]
    );
    assert_eq!(player.songs.selected(), Some(1));
    assert_ne!(player.songs.epoch, old_epoch);
    select(&mut player, old_epoch, 2);
    assert_eq!(player.songs.selected(), Some(1));
}

#[test]
fn two_moves_up_carry_an_artist_up_two_places() {
    let (mut player, _state) = test_player();
    let _ = update(
        &mut player,
        Message::ArtistsLoaded(vec![artist("a"), artist("b"), artist("c")]),
    );
    let epoch = player.artists.epoch;
    select(&mut player, epoch, 2);

    let _ = update(&mut player, Message::MoveSelectedRowUp);
    let _ = update(&mut player, Message::MoveSelectedRowUp);

    assert_eq!(artist_names(&player), ["c", "a", "b"]);
    assert_eq!(player.artists.selected(), Some(0));
}

#[test]
fn back_restores_a_moved_artists_list_to_library_order() {
    let mut player = player_in_albums();
    let _ = update(
        &mut player,
        Message::ArtistsLoaded(vec![artist("a"), artist("b"), artist("c")]),
    );
    player.current_view = CurrentView::Artists;
    let epoch = player.artists.epoch;
    select(&mut player, epoch, 1);
    let _ = update(&mut player, Message::MoveSelectedRowDown);
    assert_eq!(artist_names(&player), ["a", "c", "b"]);
    player.current_view = CurrentView::Albums;

    let _ = update(&mut player, Message::Back);

    assert_view(&player, CurrentView::Artists);
    assert_eq!(artist_names(&player), ["a", "b", "c"]);
}

// The Artists list caches only its album-count labels, not its titles, so a
// move takes the labels-only path. A move that carried the labels on the same
// condition as the titles would leave every artist showing a neighbour's
// count. Pin the alignment through a move and through Back's restore.
#[test]
fn a_moved_artist_keeps_its_cached_album_label_and_back_restores_it() {
    let mut player = player_in_albums();
    let artist = |id: &str, name: &str, album_count: usize| Artist {
        id: id.to_string(),
        name: name.to_string(),
        album_count,
    };
    let _ = update(
        &mut player,
        Message::ArtistsLoaded(vec![
            artist("banana", "banana", 0),
            artist("apple", "Apple", 2),
            artist("cherry", "cherry", 1),
        ]),
    );
    assert_eq!(player.artists.labels, ["", "2 albums", "1 album"]);
    player.current_view = CurrentView::Artists;
    let epoch = player.artists.epoch;
    select(&mut player, epoch, 0);

    let _ = update(&mut player, Message::MoveSelectedRowDown);

    assert_eq!(artist_names(&player), ["Apple", "banana", "cherry"]);
    assert_eq!(
        player.artists.labels,
        ["2 albums", "", "1 album"],
        "each moved artist row keeps its own cached album count"
    );
    assert_eq!(player.artists.selected(), Some(1));

    player.current_view = CurrentView::Albums;
    let _ = update(&mut player, Message::Back);

    assert_eq!(artist_names(&player), ["banana", "Apple", "cherry"]);
    assert_eq!(
        player.artists.labels,
        ["", "2 albums", "1 album"],
        "Back restores each artist's own cached album count"
    );
}

#[test]
fn a_move_leaves_the_view_and_the_other_lists_alone() {
    let (mut player, _state) = test_player();
    let _ = update(
        &mut player,
        Message::ArtistsLoaded(vec![artist("a"), artist("b")]),
    );
    let epoch = player.artists.epoch;
    select(&mut player, epoch, 0);
    let songs_epoch = player.songs.epoch;

    let _ = update(&mut player, Message::MoveSelectedRowDown);

    assert_view(&player, CurrentView::Artists);
    assert_eq!(player.songs.epoch, songs_epoch);
    assert_eq!(artist_names(&player), ["b", "a"]);
}

const ROW: f32 = views::BROWSE_ROW_HEIGHT;
const PAD: f32 = views::BROWSE_LIST_PADDING;

fn twenty_artists_with_row_ten_selected() -> WinampPlayer {
    let (mut player, _state) = test_player();
    let names: Vec<Artist> = (0..20).map(|i| artist(&format!("a{i:02}"))).collect();
    let _ = update(&mut player, Message::ArtistsLoaded(names));
    let epoch = player.artists.epoch;
    select(&mut player, epoch, 10);
    player
}

#[tokio::test]
async fn a_move_past_the_viewport_bottom_scrolls_the_row_into_view() {
    let mut player = twenty_artists_with_row_ten_selected();
    let task = update(&mut player, Message::MoveSelectedRowDown);

    let effects =
        browse_scroll_effects_at(task, "a move past the viewport bottom", 5.0 * ROW, 0.0).await;

    assert_eq!(
        effects.scrolled,
        Some(
            iced::advanced::widget::operation::scrollable::AbsoluteOffset {
                x: None,
                y: Some(PAD + 12.0 * ROW - 5.0 * ROW),
            }
        )
    );
}

#[tokio::test]
async fn a_move_within_the_viewport_does_not_scroll() {
    let mut player = twenty_artists_with_row_ten_selected();
    let task = update(&mut player, Message::MoveSelectedRowDown);

    let effects =
        browse_scroll_effects_at(task, "a move within the viewport", 100.0 * ROW, 0.0).await;

    assert!(!effects.any());
}

#[tokio::test]
async fn a_move_above_the_viewport_top_scrolls_up_to_the_row() {
    let mut player = twenty_artists_with_row_ten_selected();
    let task = update(&mut player, Message::MoveSelectedRowUp);

    let effects = browse_scroll_effects_at(
        task,
        "a move above the viewport top",
        5.0 * ROW,
        PAD + 12.0 * ROW,
    )
    .await;

    assert_eq!(
        effects.scrolled,
        Some(
            iced::advanced::widget::operation::scrollable::AbsoluteOffset {
                x: None,
                y: Some(PAD + 9.0 * ROW),
            }
        )
    );
}

#[test]
fn a_no_op_move_schedules_no_task() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::ArtistsLoaded(vec![artist("a")]));
    assert_no_task(update(&mut player, Message::MoveSelectedRowDown));
}

// A move records its swap in the saved library permutation, so a later
// reorder composes with it. Back must still undo both.
#[test]
fn back_restores_library_order_after_a_move_then_a_reverse() {
    let mut player = player_in_albums();
    let _ = update(
        &mut player,
        Message::ArtistsLoaded(vec![artist("a"), artist("b"), artist("c")]),
    );
    player.current_view = CurrentView::Artists;
    let epoch = player.artists.epoch;
    select(&mut player, epoch, 0);
    let _ = update(&mut player, Message::MoveSelectedRowDown);
    assert_eq!(artist_names(&player), ["b", "a", "c"]);
    let _ = update(&mut player, Message::ReverseBrowse);
    assert_eq!(artist_names(&player), ["c", "a", "b"]);
    player.current_view = CurrentView::Albums;

    let _ = update(&mut player, Message::Back);

    assert_eq!(artist_names(&player), ["a", "b", "c"]);
}

// A row already at the list's edge cannot move past it: the order, the
// selection, and the epoch stay as they were, so rendered presses stay valid.
#[test]
fn a_move_past_either_edge_changes_nothing() {
    let mut player = player_in_albums();
    let _ = update(
        &mut player,
        Message::ArtistsLoaded(vec![artist("a"), artist("b"), artist("c")]),
    );
    player.current_view = CurrentView::Artists;
    let epoch = player.artists.epoch;

    select(&mut player, epoch, 0);
    let _ = update(&mut player, Message::MoveSelectedRowUp);
    assert_eq!(artist_names(&player), ["a", "b", "c"]);
    assert_eq!(player.artists.selected(), Some(0));
    assert_eq!(player.artists.epoch, epoch);

    select(&mut player, epoch, 2);
    let _ = update(&mut player, Message::MoveSelectedRowDown);
    assert_eq!(artist_names(&player), ["a", "b", "c"]);
    assert_eq!(player.artists.selected(), Some(2));
    assert_eq!(player.artists.epoch, epoch);
}
