use super::*;

fn song_ids(list: &[Song]) -> Vec<&str> {
    list.iter().map(|song| song.id.as_str()).collect()
}

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

#[test]
fn reverse_flips_the_artists_list() {
    let (mut player, _state) = test_player();
    let _ = update(
        &mut player,
        Message::ArtistsLoaded(vec![artist("b"), artist("a"), artist("c")]),
    );

    let _ = update(&mut player, Message::ReverseBrowse);
    assert_eq!(artist_names(&player), ["c", "a", "b"]);

    let _ = update(&mut player, Message::ReverseBrowse);
    assert_eq!(artist_names(&player), ["b", "a", "c"]);
}

#[test]
fn reverse_flips_songs_and_keeps_their_cached_text_aligned() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    player.current_view = CurrentView::Songs;
    let ids_before: Vec<String> = player.songs.items.iter().map(|s| s.id.clone()).collect();
    let titles_before = player.songs.titles.clone();
    let labels_before = player.songs.labels.clone();
    assert!(!titles_before.is_empty());

    let _ = update(&mut player, Message::ReverseBrowse);

    let mut ids = ids_before.clone();
    ids.reverse();
    let mut titles = titles_before.clone();
    titles.reverse();
    let mut labels = labels_before.clone();
    labels.reverse();
    assert_eq!(song_ids(&player.songs.items), ids);
    assert_eq!(player.songs.titles, titles);
    assert_eq!(player.songs.labels, labels);

    let _ = update(&mut player, Message::AllSongsLoaded(stepping_songs()));
    player.current_view = CurrentView::AllSongs;
    let _ = update(&mut player, Message::ReverseBrowse);
    assert_eq!(song_ids(&player.all_songs.items), ids);
    assert_eq!(player.all_songs.titles, titles);
    assert_eq!(player.all_songs.labels, labels);
}

#[test]
fn reverse_bumps_the_epoch_and_clears_the_selection() {
    let (mut player, _state) = test_player();
    let _ = update(
        &mut player,
        Message::ArtistsLoaded(vec![artist("a"), artist("b")]),
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

    let _ = update(&mut player, Message::ReverseBrowse);

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
}

fn assert_reverse_is_a_noop_on(view: CurrentView) {
    let (mut player, _state) = test_player();
    let name = format!("{view:?}");
    player.current_view = view;
    let epochs = browse_list_epochs(&player);
    let ids = browse_list_ids(&player);

    let _ = update(&mut player, Message::ReverseBrowse);

    assert_eq!(browse_list_epochs(&player), epochs, "{name}");
    assert_eq!(browse_list_ids(&player), ids, "{name}");
}

#[test]
fn reverse_with_an_empty_or_loading_list_is_a_noop() {
    assert_reverse_is_a_noop_on(CurrentView::Artists);
    assert_reverse_is_a_noop_on(CurrentView::Albums);
    assert_reverse_is_a_noop_on(CurrentView::Songs);
    assert_reverse_is_a_noop_on(CurrentView::AllSongs);
    assert_reverse_is_a_noop_on(CurrentView::AllAlbums);
    assert_reverse_is_a_noop_on(CurrentView::Playlists);

    let (mut player, _state) = test_player();
    player.artists.items = vec![artist("a"), artist("b")];
    player.artists.loading = true;
    let epoch = player.artists.epoch;

    let _ = update(&mut player, Message::ReverseBrowse);

    assert_eq!(artist_names(&player), ["a", "b"]);
    assert_eq!(player.artists.epoch, epoch);
}

#[test]
fn back_restores_a_reversed_then_sorted_albums_list_to_library_order() {
    let mut player = player_in_albums();
    let _ = update(
        &mut player,
        Message::AlbumsLoaded(vec![album("b"), album("c"), album("a")]),
    );

    let _ = update(&mut player, Message::ReverseBrowse);
    let _ = update(&mut player, Message::SortBrowse);
    let epoch = player.albums.epoch;
    let _ = update(&mut player, Message::AlbumSelected { epoch, index: 0 });
    assert_view(&player, CurrentView::Songs);
    let _ = update(&mut player, Message::Back);

    let ids: Vec<&str> = player.albums.items.iter().map(|a| a.id.as_str()).collect();
    assert_eq!(ids, ["b", "c", "a"]);
}

#[test]
fn reverse_leaves_other_lists_view_search_and_playback_untouched() {
    let (mut player, state) = test_player();
    seed_lists_for_sort_locality(&mut player);
    player.current_view = CurrentView::Playlists;
    player.search_query = "echo".to_string();
    player.search_active = true;
    state.blocking_lock().current_track = Some("song-2".to_string());
    let state_before = format!("{:?}", *state.blocking_lock());
    let ids_before = browse_list_ids(&player);
    let epochs_before = browse_list_epochs(&player);

    let _ = update(&mut player, Message::ReverseBrowse);

    let ids_after = browse_list_ids(&player);
    let epochs_after = browse_list_epochs(&player);
    assert_eq!(ids_after[5], ["alpha", "zulu"]);
    assert_eq!(epochs_after[5], epochs_before[5].wrapping_add(1));
    for index in 0..5 {
        assert_eq!(ids_after[index], ids_before[index], "list {index}");
        assert_eq!(epochs_after[index], epochs_before[index], "list {index}");
    }
    assert_view(&player, CurrentView::Playlists);
    assert_eq!(player.search_query, "echo");
    assert!(player.search_active);
    assert_eq!(format!("{:?}", *state.blocking_lock()), state_before);
}
