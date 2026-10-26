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
