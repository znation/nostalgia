use super::browse::ordering::shuffled_order;
use super::*;

fn artist(name: &str) -> Artist {
    Artist {
        id: name.to_string(),
        name: name.to_string(),
        album_count: 0,
    }
}

fn album(id: &str) -> Album {
    Album {
        id: id.to_string(),
        title: id.to_string(),
        artist_id: "artist-1".to_string(),
        artist: "Band".to_string(),
        duration_ms: 0,
    }
}

const SEEDS: [u64; 5] = [0, 1, 42, 0xDEAD_BEEF, u64::MAX];

#[test]
fn shuffled_order_is_a_deterministic_permutation() {
    for len in 0..=6 {
        for seed in SEEDS {
            let order = shuffled_order(len, seed);
            let mut sorted = order.clone();
            sorted.sort_unstable();
            assert_eq!(
                sorted,
                (0..len).collect::<Vec<_>>(),
                "len {len} seed {seed}"
            );
            assert_eq!(order, shuffled_order(len, seed));
        }
    }
}

#[test]
fn shuffled_order_pins_a_known_seed() {
    assert_eq!(shuffled_order(5, 0), [2, 3, 1, 4, 0]);
}

#[test]
fn shuffled_order_varies_with_the_seed() {
    let orders: std::collections::HashSet<Vec<usize>> =
        (0..8).map(|seed| shuffled_order(5, seed)).collect();
    assert!(orders.len() >= 2, "{orders:?}");
}

#[test]
fn shuffle_rows_moves_songs_with_their_cached_text() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    let ids: Vec<String> = player.songs.items.iter().map(|s| s.id.clone()).collect();
    let titles = player.songs.titles.clone();
    let labels = player.songs.labels.clone();
    let seed = SEEDS
        .into_iter()
        .find(|&seed| {
            let order = shuffled_order(ids.len(), seed);
            order.iter().enumerate().any(|(at, &from)| at != from)
        })
        .expect("some seed moves a row");
    let order = shuffled_order(ids.len(), seed);

    player.songs.shuffle_rows(seed);

    let got: Vec<String> = player.songs.items.iter().map(|s| s.id.clone()).collect();
    assert_eq!(
        got,
        order.iter().map(|&i| ids[i].clone()).collect::<Vec<_>>()
    );
    assert_eq!(
        player.songs.titles,
        order.iter().map(|&i| titles[i].clone()).collect::<Vec<_>>()
    );
    assert_eq!(
        player.songs.labels,
        order.iter().map(|&i| labels[i].clone()).collect::<Vec<_>>()
    );
}

#[test]
fn randomize_bumps_the_epoch_clears_the_selection_and_keeps_every_row() {
    let (mut player, _state) = test_player();
    let _ = update(
        &mut player,
        Message::ArtistsLoaded(vec![artist("a"), artist("b"), artist("c")]),
    );
    let old_epoch = player.artists.epoch;
    let _ = update(
        &mut player,
        Message::SelectRow {
            epoch: old_epoch,
            index: 0,
        },
    );
    assert_eq!(player.artists.selected(), Some(0));

    let _ = update(&mut player, Message::RandomizeBrowse);

    assert_ne!(player.artists.epoch, old_epoch);
    assert_eq!(player.artists.selected(), None);
    let _ = update(
        &mut player,
        Message::ArtistSelected {
            epoch: old_epoch,
            index: 0,
        },
    );
    assert_view(&player, CurrentView::Artists);
    let mut names = artist_names(&player);
    names.sort_unstable();
    assert_eq!(names, ["a", "b", "c"]);
}

fn assert_randomize_is_a_noop_on(view: CurrentView) {
    let (mut player, _state) = test_player();
    let name = format!("{view:?}");
    player.current_view = view;
    let epochs = browse_list_epochs(&player);
    let ids = browse_list_ids(&player);

    let _ = update(&mut player, Message::RandomizeBrowse);

    assert_eq!(browse_list_epochs(&player), epochs, "{name}");
    assert_eq!(browse_list_ids(&player), ids, "{name}");
}

#[test]
fn randomize_with_an_empty_or_loading_list_is_a_noop() {
    assert_randomize_is_a_noop_on(CurrentView::Artists);
    assert_randomize_is_a_noop_on(CurrentView::Albums);
    assert_randomize_is_a_noop_on(CurrentView::Songs);
    assert_randomize_is_a_noop_on(CurrentView::AllSongs);
    assert_randomize_is_a_noop_on(CurrentView::AllAlbums);
    assert_randomize_is_a_noop_on(CurrentView::Playlists);

    let (mut player, _state) = test_player();
    player.artists.items = vec![artist("a"), artist("b")];
    player.artists.loading = true;
    let epoch = player.artists.epoch;

    let _ = update(&mut player, Message::RandomizeBrowse);

    assert_eq!(artist_names(&player), ["a", "b"]);
    assert_eq!(player.artists.epoch, epoch);
}

#[test]
fn back_restores_a_randomized_albums_list_to_library_order() {
    let mut player = player_in_albums();
    let _ = update(
        &mut player,
        Message::AlbumsLoaded(vec![album("b"), album("c"), album("a"), album("d")]),
    );

    player.albums.shuffle_rows(7);
    let epoch = player.albums.epoch;
    let _ = update(&mut player, Message::AlbumSelected { epoch, index: 0 });
    assert_view(&player, CurrentView::Songs);
    let _ = update(&mut player, Message::Back);

    let ids: Vec<&str> = player.albums.items.iter().map(|a| a.id.as_str()).collect();
    assert_eq!(ids, ["b", "c", "a", "d"]);
}
