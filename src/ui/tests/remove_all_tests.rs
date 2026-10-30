use super::*;

fn song_ids(list: &BrowseList<Song>) -> Vec<&str> {
    list.items.iter().map(|song| song.id.as_str()).collect()
}

fn songs_view(player: &mut WinampPlayer) -> u64 {
    player.current_view = CurrentView::Songs;
    let _ = update(player, Message::SongsLoaded(stepping_songs()));
    player.songs.epoch
}

/// Ctrl+Shift+Delete outside the shared songs buffer is a no-op, like the
/// other one-shots that only edit the songs view's list: the handler routes on
/// `CurrentView::Songs` and never touches a list the view did not compose.
#[test]
fn remove_all_is_a_noop_outside_the_songs_view() {
    for view in [
        CurrentView::Artists,
        CurrentView::Albums,
        CurrentView::AllSongs,
        CurrentView::AllAlbums,
        CurrentView::RecentlyAdded,
        CurrentView::RecentlyPlayed,
        CurrentView::Playlists,
        CurrentView::Genres,
        CurrentView::Years,
        CurrentView::Composers,
    ] {
        let view_name = format!("{view:?}");
        let (mut player, _state) = test_player();
        let epoch = songs_view(&mut player);

        player.current_view = view;
        let _ = update(&mut player, Message::RemoveAllBrowse);

        assert_eq!(
            song_ids(&player.songs),
            ["song-1", "song-2", "song-3"],
            "{view_name}"
        );
        assert_eq!(
            player.songs.epoch, epoch,
            "{view_name} bumped the songs epoch"
        );
    }

    // Once the showing view is the songs list again, the same message clears
    // the list the guard kept standing.
    let (mut player, _state) = test_player();
    let _ = songs_view(&mut player);

    let _ = update(&mut player, Message::RemoveAllBrowse);

    assert_eq!(song_ids(&player.songs), Vec::<&str>::new());
}

#[test]
fn remove_all_clears_the_showing_songs_list_and_rejects_a_pre_clear_press() {
    let (mut player, _state) = test_player();
    let old_epoch = songs_view(&mut player);
    let _ = update(
        &mut player,
        Message::SelectRow {
            epoch: old_epoch,
            index: 1,
        },
    );
    assert_eq!(player.songs.selected(), Some(1));
    assert!(!player.songs.titles.is_empty());

    let _ = update(&mut player, Message::RemoveAllBrowse);

    // No rows, no cached titles or labels, no selection, no permutation, and
    // the list is neither loading nor failed (its empty wording shows).
    assert!(player.songs.items.is_empty());
    assert!(player.songs.titles.is_empty());
    assert!(player.songs.labels.is_empty());
    assert_eq!(player.songs.selected(), None);
    // `restore_library_order` is the permutation's only public read side:
    // dropping it at the clear means a later restore has nothing to
    // resurrect (a kept permutation would rebuild the three rows here).
    let before = player.songs.epoch;
    player.songs.restore_library_order();
    assert!(player.songs.items.is_empty());
    assert_eq!(player.songs.epoch, before, "restore must be a no-op");
    assert!(!player.songs.loading);
    assert_eq!(player.songs.error, None);
    // The epoch bumped exactly once, so a press rendered before the clear
    // cannot open a row of the emptied list under a stale index.
    assert_eq!(player.songs.epoch, old_epoch.wrapping_add(1));
    let _ = update(
        &mut player,
        Message::SelectRow {
            epoch: old_epoch,
            index: 0,
        },
    );
    assert_eq!(player.songs.selected(), None);
}

// The playing track is not named by any browse-list index, so clearing the
// list the user is listening through must not touch playback, the Now Playing
// line, or the played history.
#[test]
fn clearing_a_playing_list_leaves_playback_and_the_now_playing_line_alone() {
    let (mut player, state) = test_player();
    player.current_view = CurrentView::Songs;
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    let epoch = player.songs.epoch;
    let _ = update(&mut player, Message::TrackSelected { epoch, index: 0 });
    {
        let mut shared = state.blocking_lock();
        shared.current_track = Some("song-1".to_string());
        shared.is_playing = true;
    }
    let generation = player
        .plays_generation
        .load(std::sync::atomic::Ordering::SeqCst);
    let _ = update(
        &mut player,
        Message::TrackPlayed {
            generation,
            committed: true,
        },
    );
    assert_eq!(player.now_playing_label(Some("song-1")), "One");
    let history_before = player.played_songs.newest_first_ids();

    let _ = update(&mut player, Message::RemoveAllBrowse);

    assert!(player.songs.items.is_empty());
    let shared = state.blocking_lock();
    assert_eq!(shared.current_track.as_deref(), Some("song-1"));
    assert!(shared.is_playing);
    assert_eq!(
        player.played_songs.newest_first_ids(),
        history_before,
        "the played history grew or shrank with the clear"
    );
    assert_eq!(
        player.now_playing_label(shared.current_track.as_deref()),
        "One"
    );
}

// Sort composes with the clear the way it does with Back: Back restores the
// parent list and touches nothing about the emptied songs buffer, and no row
// is resurrected by a later restore of the cleared list.
#[test]
fn sort_then_clear_then_back_restores_the_parent_and_leaves_the_list_cleared() {
    let (mut player, _state) = test_player();
    player.current_view = CurrentView::Songs;
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));

    let _ = update(&mut player, Message::SortBrowse);
    assert_eq!(song_ids(&player.songs), ["song-1", "song-3", "song-2"]);
    let _ = update(&mut player, Message::RemoveAllBrowse);
    assert!(player.songs.items.is_empty());

    player.songs_origin = SongsOrigin::Album;
    player.albums.items = vec![sample_album()];
    player.albums.loading = false;
    let _ = update(&mut player, Message::Back);
    assert_view(&player, CurrentView::Albums);

    // The cleared songs buffer stays cleared: the list holds no permutation
    // to resurrect a row from.
    assert!(player.songs.items.is_empty());
    player.songs.restore_library_order();
    assert!(player.songs.items.is_empty());
}

// A clear on an already emptied, loading, or failed list is a no-op that
// bumps no epoch, exactly like Sort's (message-level, through the real
// handler and `update` dispatch).
#[test]
fn remove_all_on_an_emptied_loading_or_failed_list_is_a_noop() {
    let (mut player, _state) = test_player();
    let first_epoch = songs_view(&mut player);
    let _ = update(&mut player, Message::RemoveAllBrowse);
    assert!(player.songs.items.is_empty());
    let cleared_epoch = player.songs.epoch;

    // An already emptied list.
    let _ = update(&mut player, Message::RemoveAllBrowse);
    assert!(player.songs.items.is_empty());
    assert_eq!(player.songs.epoch, cleared_epoch);

    // A loading list (its rows are about to be replaced by a fetch).
    player.current_view = CurrentView::Songs;
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    let epoch = player.songs.epoch;
    player.songs.loading = true;
    let _ = update(&mut player, Message::RemoveAllBrowse);
    player.songs.loading = false;
    assert_eq!(song_ids(&player.songs), ["song-1", "song-2", "song-3"]);
    assert_eq!(player.songs.epoch, epoch);

    // A failed list still holding rows.
    player.songs.error = Some("boom".to_string());
    let _ = update(&mut player, Message::RemoveAllBrowse);
    assert_eq!(song_ids(&player.songs), ["song-1", "song-2", "song-3"]);
    assert_eq!(player.songs.epoch, epoch);

    // Nothing else moved: the settings are not involved anywhere above.
    assert_ne!(first_epoch, cleared_epoch);
}
