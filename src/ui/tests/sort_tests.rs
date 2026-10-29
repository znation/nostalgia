use super::*;

fn album_ids(albums: &[Album]) -> Vec<&str> {
    albums.iter().map(|album| album.id.as_str()).collect()
}

/// Two albums whose rendered titles ("Zulu - Alpha", "alpha - Zulu") sort
/// opposite to their `title` fields, in library order (`zulu`, then `alpha`).
fn display_title_albums(first_duration_ms: u64, second_duration_ms: u64) -> Vec<Album> {
    vec![
        Album {
            id: "zulu".to_string(),
            title: "Alpha".to_string(),
            artist_id: "artist-1".to_string(),
            artist: "Zulu".to_string(),
            duration_ms: first_duration_ms,
        },
        Album {
            id: "alpha".to_string(),
            title: "Zulu".to_string(),
            artist_id: "artist-1".to_string(),
            artist: "alpha".to_string(),
            duration_ms: second_duration_ms,
        },
    ]
}

#[test]
fn sort_reorders_artists_case_insensitively() {
    let (mut player, _state) = test_player();
    player.artists.items = vec![
        Artist {
            id: "banana".to_string(),
            name: "banana".to_string(),
            album_count: 0,
        },
        Artist {
            id: "apple".to_string(),
            name: "Apple".to_string(),
            album_count: 0,
        },
        Artist {
            id: "cherry".to_string(),
            name: "cherry".to_string(),
            album_count: 0,
        },
    ];
    // A list with rows has finished its fetch; the startup list is still
    // marked loading until a reply lands.
    player.artists.loading = false;

    let _ = update(&mut player, Message::SortBrowse);

    let names: Vec<&str> = player
        .artists
        .items
        .iter()
        .map(|artist| artist.name.as_str())
        .collect();
    assert_eq!(names, ["Apple", "banana", "cherry"]);
}

#[test]
fn sort_reorders_artists_and_keeps_their_cached_labels_aligned() {
    let (mut player, _state) = test_player();
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

    let _ = update(&mut player, Message::SortBrowse);

    let names: Vec<&str> = player
        .artists
        .items
        .iter()
        .map(|artist| artist.name.as_str())
        .collect();
    assert_eq!(names, ["Apple", "banana", "cherry"]);
    assert_eq!(
        player.artists.labels,
        ["2 albums", "", "1 album"],
        "each sorted artist row keeps its own cached album count"
    );
}

// Back restores the artists list from its saved permutation, and it must move
// the cached album-count labels with the rows just as Sort does. A restore
// that reordered only `items` would leave every artist showing a neighbour's
// count, the failure mode the cache's own risk note names.
#[test]
fn back_restores_sorted_artists_and_their_cached_album_labels() {
    let (mut player, _state) = test_player();
    let artist = |id: &str, name: &str, album_count: usize| Artist {
        id: id.to_string(),
        name: name.to_string(),
        album_count,
    };
    // Load through the real `ArtistsLoaded` arm so the labels come from
    // `store_with_labels`, in this library order.
    let _ = update(
        &mut player,
        Message::ArtistsLoaded(vec![
            artist("banana", "banana", 0),
            artist("apple", "Apple", 2),
            artist("cherry", "cherry", 1),
        ]),
    );
    assert_eq!(player.artists.labels, ["", "2 albums", "1 album"]);

    let _ = update(&mut player, Message::SortBrowse);
    assert_eq!(artist_names(&player), ["Apple", "banana", "cherry"]);
    // A second Sort must still let Back undo both reorders, labels included.
    let _ = update(&mut player, Message::SortBrowse);

    player.current_view = CurrentView::Albums;
    let _ = update(&mut player, Message::Back);

    assert_eq!(artist_names(&player), ["banana", "Apple", "cherry"]);
    assert_eq!(
        player.artists.labels,
        ["", "2 albums", "1 album"],
        "each restored artist row keeps its own cached album count"
    );
}

// The documented sort is stable: rows whose rendered text compares equal
// (ignoring case) keep the library order. Every other sort test uses distinct
// keys, so a regression to an unstable sort could reorder these equal-key rows
// and leave them green. Enough rows are used that an unstable sort cannot keep
// them in place by luck on a short list. The same rows also pin the saved
// permutation when keys collide: Back must still restore their stored order.
#[test]
fn sort_is_stable_for_equal_keys_and_back_restores_them() {
    const EQUAL_ROWS: usize = 32;
    let same_rows = || {
        (0..EQUAL_ROWS).map(|index| Artist {
            id: format!("same-{index:02}"),
            name: "Same".to_string(),
            album_count: 0,
        })
    };
    let (mut player, _state) = test_player();
    player.artists.items = std::iter::once(Artist {
        id: "zulu".to_string(),
        name: "Zulu".to_string(),
        album_count: 0,
    })
    .chain(same_rows())
    .chain(std::iter::once(Artist {
        id: "alpha".to_string(),
        name: "alpha".to_string(),
        album_count: 0,
    }))
    .collect();
    player.artists.loading = false;

    let _ = update(&mut player, Message::SortBrowse);

    let sorted_ids: Vec<String> = player
        .artists
        .items
        .iter()
        .map(|artist| artist.id.clone())
        .collect();
    let mut expected_sorted = vec!["alpha".to_string()];
    expected_sorted.extend((0..EQUAL_ROWS).map(|index| format!("same-{index:02}")));
    expected_sorted.push("zulu".to_string());
    assert_eq!(
        sorted_ids, expected_sorted,
        "equal keys must keep library order"
    );

    player.current_view = CurrentView::Albums;
    let _ = update(&mut player, Message::Back);

    let restored_ids: Vec<String> = player
        .artists
        .items
        .iter()
        .map(|artist| artist.id.clone())
        .collect();
    let mut expected_restored = vec!["zulu".to_string()];
    expected_restored.extend((0..EQUAL_ROWS).map(|index| format!("same-{index:02}")));
    expected_restored.push("alpha".to_string());
    assert_eq!(restored_ids, expected_restored);
}

#[test]
fn sort_reorders_albums_by_their_display_title() {
    let (mut player, _state) = test_player();
    player.current_view = CurrentView::Albums;
    // The display titles are "Zulu - Alpha" and "alpha - Zulu", so sorting
    // by the rendered text puts the second row first; sorting by `title`
    // alone would leave this order unchanged.
    player.albums.items = display_title_albums(0, 0);

    let _ = update(&mut player, Message::SortBrowse);

    let ids = album_ids(&player.albums.items);
    assert_eq!(ids, ["alpha", "zulu"]);
}

#[test]
fn sort_reorders_playlists_by_name() {
    let (mut player, _state) = test_player();
    player.current_view = CurrentView::Playlists;
    player.playlists.items = vec![
        Playlist {
            id: "banana".to_string(),
            name: "banana".to_string(),
        },
        Playlist {
            id: "apple".to_string(),
            name: "Apple".to_string(),
        },
        Playlist {
            id: "cherry".to_string(),
            name: "cherry".to_string(),
        },
    ];

    let _ = update(&mut player, Message::SortBrowse);

    let names: Vec<&str> = player
        .playlists
        .items
        .iter()
        .map(|playlist| playlist.name.as_str())
        .collect();
    assert_eq!(names, ["Apple", "banana", "cherry"]);
}

#[test]
fn sort_reorders_genres_by_name() {
    let (mut player, _state) = test_player();
    player.current_view = CurrentView::Genres;
    let genre = |name: &str| Genre {
        name: name.to_string(),
        songs: Vec::new(),
    };
    player.genres.items = vec![genre("rock"), genre("Blues"), genre("Pop")];

    let _ = update(&mut player, Message::SortBrowse);

    let names: Vec<&str> = player
        .genres
        .items
        .iter()
        .map(|genre| genre.name.as_str())
        .collect();
    assert_eq!(names, ["Blues", "Pop", "rock"]);
}

/// Asserts a sorted `stepping_songs()` list holds its rows in title order with each row's cached
/// title and running time still beside it.
fn assert_sorted_stepping_songs(list: &BrowseList<Song>) {
    let ids: Vec<&str> = list.items.iter().map(|song| song.id.as_str()).collect();
    assert_eq!(ids, ["song-1", "song-3", "song-2"]);
    assert_eq!(
        list.titles,
        [
            "The Sample Band - One",
            "The Sample Band - Three",
            "The Sample Band - Two"
        ]
    );
    assert_eq!(list.labels, ["3:30", "1:35", "2:05"]);
}

#[test]
fn sort_reorders_songs_and_keeps_their_cached_text_aligned() {
    let (mut player, _state) = test_player();
    player.current_view = CurrentView::Songs;
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));

    let _ = update(&mut player, Message::SortBrowse);

    assert_sorted_stepping_songs(&player.songs);
}

#[test]
fn sort_reorders_all_songs_and_keeps_their_cached_text_aligned() {
    let (mut player, _state) = test_player();
    player.current_view = CurrentView::AllSongs;
    let _ = update(&mut player, Message::AllSongsLoaded(stepping_songs()));

    let _ = update(&mut player, Message::SortBrowse);

    assert_sorted_stepping_songs(&player.all_songs);
}

#[test]
fn sort_reorders_all_albums_by_their_display_title() {
    let (mut player, _state) = test_player();
    player.current_view = CurrentView::AllAlbums;
    player.all_albums.items = display_title_albums(0, 0);

    let _ = update(&mut player, Message::SortBrowse);

    let ids = album_ids(&player.all_albums.items);
    assert_eq!(ids, ["alpha", "zulu"]);
}

#[test]
fn sort_reorders_all_albums_and_keeps_their_cached_text_aligned() {
    let (mut player, _state) = test_player();
    player.current_view = CurrentView::AllAlbums;
    let albums = display_title_albums(60_000, 120_000);
    let _ = update(&mut player, Message::AllAlbumsLoaded(albums));

    let _ = update(&mut player, Message::SortBrowse);

    let ids = album_ids(&player.all_albums.items);
    assert_eq!(ids, ["alpha", "zulu"]);
    assert_eq!(
        player.all_albums.titles,
        ["alpha - Zulu", "Zulu - Alpha"],
        "each row keeps its own cached display title"
    );
    assert_eq!(
        player.all_albums.labels,
        ["2:00", "1:00"],
        "each row keeps its own cached running time"
    );
}

#[test]
fn sort_bumps_the_epoch_so_a_pre_sort_press_is_rejected() {
    let (mut player, _state) = test_player();
    player.artists.items = vec![
        Artist {
            id: "banana".to_string(),
            name: "banana".to_string(),
            album_count: 0,
        },
        Artist {
            id: "apple".to_string(),
            name: "Apple".to_string(),
            album_count: 0,
        },
    ];
    // A list with rows has finished its fetch; the startup list is still
    // marked loading until a reply lands.
    player.artists.loading = false;
    let old_epoch = player.artists.epoch;

    let _ = update(&mut player, Message::SortBrowse);

    assert_eq!(player.artists.epoch, old_epoch.wrapping_add(1));
    let _ = update(
        &mut player,
        Message::ArtistSelected {
            epoch: old_epoch,
            index: 0,
        },
    );
    assert_view(&player, CurrentView::Artists);

    let new_epoch = player.artists.epoch;
    let _ = update(
        &mut player,
        Message::ArtistSelected {
            epoch: new_epoch,
            index: 0,
        },
    );
    assert_view(&player, CurrentView::Albums);
}

#[test]
fn sort_with_an_empty_or_loading_list_is_a_noop() {
    assert_sort_is_a_noop_on_an_unloaded_view(CurrentView::Artists);
    assert_sort_is_a_noop_on_an_unloaded_view(CurrentView::Albums);
    assert_sort_is_a_noop_on_an_unloaded_view(CurrentView::Songs);
    assert_sort_is_a_noop_on_an_unloaded_view(CurrentView::AllSongs);
    assert_sort_is_a_noop_on_an_unloaded_view(CurrentView::AllAlbums);
    assert_sort_is_a_noop_on_an_unloaded_view(CurrentView::Playlists);

    // A loading list is a no-op even if its buffer still holds rows: the
    // documented guarantee is about the list's state, not just its emptiness,
    // so a future transition that keeps rows while a fetch is in flight must
    // not let Sort reorder them or bump the epoch. (The current transitions
    // empty the rows in `clear` before setting `loading`, so this state is
    // constructed here to pin that rule.)
    let (mut player, _state) = test_player();
    player.artists.items = vec![
        Artist {
            id: "banana".to_string(),
            name: "banana".to_string(),
            album_count: 0,
        },
        Artist {
            id: "apple".to_string(),
            name: "Apple".to_string(),
            album_count: 0,
        },
    ];
    player.artists.loading = true;
    let epoch = player.artists.epoch;

    let _ = update(&mut player, Message::SortBrowse);

    let names: Vec<&str> = player
        .artists
        .items
        .iter()
        .map(|artist| artist.name.as_str())
        .collect();
    assert_eq!(names, ["banana", "Apple"]);
    assert_eq!(player.artists.epoch, epoch);
}

// The "Browse row order" invariant says a sort leaves every other list, the
// current view, the search state, and playback alone. Each test above seeds
// only the list it sorts, so a handler that also sorted another list (or
// cleared a search or playback) could pass them all. Seed every list and both
// kinds of state, then sort each view in turn and check only its own list's
// rows and epoch moved.
#[test]
fn sort_leaves_other_lists_view_search_and_playback_untouched() {
    let views = [
        CurrentView::Artists,
        CurrentView::Albums,
        CurrentView::Songs,
        CurrentView::AllSongs,
        CurrentView::AllAlbums,
        CurrentView::Playlists,
    ];
    for view in views {
        let target = match &view {
            CurrentView::Artists => 0,
            CurrentView::Albums => 1,
            CurrentView::Songs => 2,
            CurrentView::AllSongs => 3,
            CurrentView::AllAlbums => 4,
            CurrentView::Playlists => 5,
            CurrentView::RecentlyAdded | CurrentView::RecentlyPlayed | CurrentView::Genres => {
                unreachable!("not in the views iterated above")
            }
        };
        let expected_sorted = match &view {
            CurrentView::Artists => vec!["alpha", "bravo", "Zulu"],
            CurrentView::Albums | CurrentView::AllAlbums => vec!["alpha", "zulu"],
            CurrentView::Songs | CurrentView::AllSongs => vec!["song-1", "song-3", "song-2"],
            CurrentView::Playlists => vec!["alpha", "zulu"],
            CurrentView::RecentlyAdded | CurrentView::RecentlyPlayed | CurrentView::Genres => {
                unreachable!("not in the views iterated above")
            }
        };
        let view_name = format!("{view:?}");

        let (mut player, state) = test_player();
        seed_lists_for_sort_locality(&mut player);
        player.current_view = view;
        player.search_query = "echo".to_string();
        player.search_active = true;
        {
            let mut shared = state.blocking_lock();
            shared.current_track = Some("song-2".to_string());
            shared.is_playing = true;
            shared.repeat = true;
            shared.shuffle = true;
            shared.set_volume(0.3);
            shared.set_balance(-0.4);
            shared.eq_enabled = true;
        }
        let state_before = format!("{:?}", *state.blocking_lock());
        let ids_before = browse_list_ids(&player);
        let epochs_before = browse_list_epochs(&player);
        let view_before = std::mem::discriminant(&player.current_view);

        let _ = update(&mut player, Message::SortBrowse);

        let ids_after = browse_list_ids(&player);
        let epochs_after = browse_list_epochs(&player);
        assert_eq!(ids_after[target], expected_sorted, "sorting {view_name}");
        assert_eq!(
            epochs_after[target],
            epochs_before[target].wrapping_add(1),
            "sorting {view_name} must bump its own epoch"
        );
        for index in 0..ids_after.len() {
            if index == target {
                continue;
            }
            assert_eq!(
                ids_after[index], ids_before[index],
                "sorting {view_name} touched list {index}'s rows"
            );
            assert_eq!(
                epochs_after[index], epochs_before[index],
                "sorting {view_name} bumped list {index}'s epoch"
            );
        }
        assert_eq!(
            std::mem::discriminant(&player.current_view),
            view_before,
            "sorting {view_name} changed the view"
        );
        assert_eq!(player.search_query, "echo", "sorting {view_name}");
        assert!(
            player.search_active,
            "sorting {view_name} cleared the search"
        );
        assert_eq!(
            format!("{:?}", *state.blocking_lock()),
            state_before,
            "sorting {view_name} changed playback state"
        );
    }
}

// The reorder is one-shot: leaving a sorted parent list and returning with
// Back must show the library's order again (README, "reopening the list",
// and the "Browse row order" invariant). Back does not refetch the parent, so
// the list has to restore the order it had before Sort.
#[test]
fn back_restores_a_sorted_artists_list_to_library_order() {
    let (mut player, _state) = test_player();
    player.artists.items = vec![
        Artist {
            id: "zulu".to_string(),
            name: "Zulu".to_string(),
            album_count: 0,
        },
        Artist {
            id: "alpha".to_string(),
            name: "alpha".to_string(),
            album_count: 0,
        },
        Artist {
            id: "bravo".to_string(),
            name: "bravo".to_string(),
            album_count: 0,
        },
    ];
    player.artists.loading = false;
    player.current_view = CurrentView::Artists;

    let _ = update(&mut player, Message::SortBrowse);
    assert_eq!(artist_names(&player), ["alpha", "bravo", "Zulu"]);
    // A second Sort must not lose the library order: the saved permutation
    // composes with the second reorder instead of being overwritten.
    let _ = update(&mut player, Message::SortBrowse);

    // Opening an artist replaces the albums list, not the artists one.
    player.current_view = CurrentView::Albums;
    let epoch = player.artists.epoch;
    let _ = update(&mut player, Message::Back);

    assert_eq!(artist_names(&player), ["Zulu", "alpha", "bravo"]);
    assert_eq!(player.artists.epoch, epoch.wrapping_add(1));
}

// The same one-shot rule holds for the other in-memory parent list: songs
// opened from an album return to that album's list, whose library order Back
// must restore after a Sort.
#[test]
fn back_from_songs_restores_a_sorted_albums_list_to_library_order() {
    let mut player = player_in_albums();
    let _ = update(
        &mut player,
        Message::AlbumsLoaded(vec![
            Album {
                id: "zulu".to_string(),
                title: "Zulu".to_string(),
                artist_id: "artist-1".to_string(),
                artist: "Zulu".to_string(),
                duration_ms: 0,
            },
            Album {
                id: "alpha".to_string(),
                title: "alpha".to_string(),
                artist_id: "artist-1".to_string(),
                artist: "alpha".to_string(),
                duration_ms: 0,
            },
        ]),
    );
    let _ = update(&mut player, Message::SortBrowse);
    assert_eq!(player.albums.items[0].id, "alpha");

    let epoch = player.albums.epoch;
    let _ = update(&mut player, Message::AlbumSelected { epoch, index: 0 });
    assert_view(&player, CurrentView::Songs);
    let _ = update(&mut player, Message::Back);

    let ids = album_ids(&player.albums.items);
    assert_eq!(ids, ["zulu", "alpha"]);
}

// The flat All Albums list is the other parent Back can return to after a
// Sort: opening an album leaves it in memory, so Back must restore its
// library order rather than leave the sorted order showing.
#[test]
fn back_from_all_albums_songs_restores_a_sorted_flat_list_to_library_order() {
    let (mut player, _state) = test_player();
    player.current_view = CurrentView::AllAlbums;
    player.all_albums.loading = false;
    player.all_albums.items = vec![
        Album {
            id: "zulu".to_string(),
            title: "Zulu".to_string(),
            artist_id: "artist-1".to_string(),
            artist: "Zulu".to_string(),
            duration_ms: 0,
        },
        Album {
            id: "alpha".to_string(),
            title: "alpha".to_string(),
            artist_id: "artist-1".to_string(),
            artist: "alpha".to_string(),
            duration_ms: 0,
        },
    ];

    let _ = update(&mut player, Message::SortBrowse);
    let sorted = album_ids(&player.all_albums.items);
    assert_eq!(sorted, ["alpha", "zulu"]);

    let epoch = player.all_albums.epoch;
    let _ = update(&mut player, Message::AlbumSelected { epoch, index: 0 });
    assert_view(&player, CurrentView::Songs);
    assert_eq!(player.songs_origin, SongsOrigin::AllAlbums);

    let _ = update(&mut player, Message::Back);

    let restored = album_ids(&player.all_albums.items);
    assert_view(&player, CurrentView::AllAlbums);
    assert_eq!(restored, ["zulu", "alpha"]);
}

// Playlists is the remaining in-memory parent Back restores: a sorted list
// whose playlist was opened must come back in library order.
#[test]
fn back_from_a_playlists_songs_restores_a_sorted_list_to_library_order() {
    let (mut player, _state) = test_player();
    player.current_view = CurrentView::Playlists;
    player.playlists.loading = false;
    player.playlists.items = vec![
        Playlist {
            id: "zulu".to_string(),
            name: "Zulu".to_string(),
        },
        Playlist {
            id: "alpha".to_string(),
            name: "alpha".to_string(),
        },
    ];

    let _ = update(&mut player, Message::SortBrowse);
    let sorted: Vec<&str> = player
        .playlists
        .items
        .iter()
        .map(|playlist| playlist.id.as_str())
        .collect();
    assert_eq!(sorted, ["alpha", "zulu"]);

    let epoch = player.playlists.epoch;
    let _ = update(&mut player, Message::PlaylistSelected { epoch, index: 0 });
    assert_view(&player, CurrentView::Songs);
    assert_eq!(player.songs_origin, SongsOrigin::Playlist);

    let _ = update(&mut player, Message::Back);

    let restored: Vec<&str> = player
        .playlists
        .items
        .iter()
        .map(|playlist| playlist.id.as_str())
        .collect();
    assert_view(&player, CurrentView::Playlists);
    assert_eq!(restored, ["zulu", "alpha"]);
}
