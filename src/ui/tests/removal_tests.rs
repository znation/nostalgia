use super::*;

fn song_ids(list: &BrowseList<Song>) -> Vec<&str> {
    list.items.iter().map(|song| song.id.as_str()).collect()
}

/// Delete outside the shared songs buffer is a no-op, like Sort's reorder is
/// the only row edit the other views offer: the songs view is the
/// playlist-shaped artifact, and the Artists/Albums views must never cut a row
/// their composed-from-the-library list did not invite. The guard is the
/// handler's `CurrentView` check, not the list's own state, so the tests start
/// from a fully loaded songs list and drive every other view over it.
#[test]
fn remove_selected_is_a_noop_outside_the_songs_view() {
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
        player.current_view = CurrentView::Songs;
        let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
        let epoch = player.songs.epoch;
        let _ = update(&mut player, Message::SelectRow { epoch, index: 1 });
        player.current_view = view;

        let _ = update(&mut player, Message::RemoveSelectedBrowse);

        assert_eq!(
            song_ids(&player.songs),
            ["song-1", "song-2", "song-3"],
            "{view_name}"
        );
        assert_eq!(
            player.songs.epoch, epoch,
            "{view_name} bumped the songs epoch"
        );
        assert_eq!(
            player.songs.selected(),
            Some(1),
            "{view_name} dropped the selection"
        );
    }

    // Once the showing view is the songs list again, the same message removes
    // the row the guard kept standing.
    let (mut player, _state) = test_player();
    player.current_view = CurrentView::Songs;
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    let epoch = player.songs.epoch;
    let _ = update(&mut player, Message::SelectRow { epoch, index: 1 });

    let _ = update(&mut player, Message::RemoveSelectedBrowse);

    assert_eq!(song_ids(&player.songs), ["song-1", "song-3"]);
    assert_eq!(player.songs.selected(), None);
}

#[test]
fn remove_selected_bumps_the_epoch_so_a_pre_removal_press_is_rejected() {
    let (mut player, _state) = test_player();
    player.current_view = CurrentView::Songs;
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    let old_epoch = player.songs.epoch;
    let _ = update(
        &mut player,
        Message::SelectRow {
            epoch: old_epoch,
            index: 1,
        },
    );
    assert_eq!(player.songs.selected(), Some(1));

    let _ = update(&mut player, Message::RemoveSelectedBrowse);

    assert_ne!(player.songs.epoch, old_epoch);
    assert_eq!(player.songs.selected(), None);
    // A press rendered before the removal carries the old epoch; it must not
    // open a row of the shortened list under the removed index.
    let _ = update(
        &mut player,
        Message::SelectRow {
            epoch: old_epoch,
            index: 1,
        },
    );
    assert_eq!(player.songs.selected(), None);
}

// The playing track is not named by any browse-list index: the row state and
// the Now Playing bar, played history, and playback flags live on the shared
// state and the player's `known_tracks` index, so deleting the row the user is
// listening to must not touch them.
#[test]
fn removing_a_playing_row_leaves_playback_and_the_now_playing_line_alone() {
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
    let songs_before = player.played_songs.newest_first_ids();

    // Delete the row currently playing (the loaded row's "One").
    let loaded_epoch = player.songs.epoch;
    let _ = update(
        &mut player,
        Message::SelectRow {
            epoch: loaded_epoch,
            index: 0,
        },
    );
    let _ = update(&mut player, Message::RemoveSelectedBrowse);

    assert_eq!(song_ids(&player.songs), ["song-2", "song-3"]);
    let shared = state.blocking_lock();
    assert_eq!(shared.current_track.as_deref(), Some("song-1"));
    assert!(shared.is_playing);
    assert_eq!(
        player.played_songs.newest_first_ids(),
        songs_before,
        "the played history grew or shrank with the row removal"
    );
    // The bar resolves its label through `known_tracks`, which outlived the
    // browse row, so the Now Playing line reads exactly what it did before.
    assert_eq!(
        player.now_playing_label(shared.current_track.as_deref()),
        "One"
    );
}

// The removal records its library-index adjustment for
// `restore_library_order`, so a sort followed by a delete followed by a
// restore yields library order *minus* the deleted row: no removed row is
// resurrected, and no permutation index points past the shortened list.
#[test]
fn sort_then_remove_then_restore_yields_library_order_minus_the_removed_row() {
    let (mut player, _state) = test_player();
    player.current_view = CurrentView::Songs;
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));

    let _ = update(&mut player, Message::SortBrowse);
    assert_eq!(song_ids(&player.songs), ["song-1", "song-3", "song-2"]);
    // Delete the middle sorted row: song-3 sits between the two rows the
    // library order restores to, which is exactly the index the permutation's
    // decrement pass has to survive.
    let sorted_epoch = player.songs.epoch;
    let _ = update(
        &mut player,
        Message::SelectRow {
            epoch: sorted_epoch,
            index: 1,
        },
    );
    let _ = update(&mut player, Message::RemoveSelectedBrowse);
    assert_eq!(song_ids(&player.songs), ["song-1", "song-2"]);

    // Back from the songs list steps to the parent and restores that list; it
    // changes nothing about the removed row and must not panic on a list whose
    // permutation now holds a gap.
    player.songs_origin = SongsOrigin::Album;
    player.albums.items = vec![sample_album()];
    player.albums.loading = false;
    let _ = update(&mut player, Message::Back);
    assert_view(&player, CurrentView::Albums);

    // The songs list itself restores through its recorded permutation: minus
    // the deleted row, with every surviving row in the order it was loaded in.
    player.songs.restore_library_order();
    assert_eq!(song_ids(&player.songs), ["song-1", "song-2"]);
}

// A deletion with no selection or on a list that is loading or failed is a
// no-op, exactly like Sort's (message-level, through the real handler and
// `update` dispatch, so the guard cannot be bypassed by a routing mistake).
#[test]
fn remove_selected_without_a_selection_or_while_loading_or_failed_is_a_noop() {
    let (mut player, _state) = test_player();
    player.current_view = CurrentView::Songs;
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    let epoch = player.songs.epoch;

    // Nothing selected.
    let _ = update(&mut player, Message::RemoveSelectedBrowse);
    assert_eq!(song_ids(&player.songs), ["song-1", "song-2", "song-3"]);
    assert_eq!(player.songs.epoch, epoch);

    // A loading list (its rows are about to be replaced by a fetch).
    player.songs.loading = true;
    let _ = update(&mut player, Message::RemoveSelectedBrowse);
    player.songs.loading = false;
    assert_eq!(song_ids(&player.songs), ["song-1", "song-2", "song-3"]);
    assert_eq!(player.songs.epoch, epoch);

    // A failed list.
    player.songs.fail("boom".to_string());
    let _ = update(&mut player, Message::RemoveSelectedBrowse);
    assert!(player.songs.items.is_empty());
    assert_eq!(song_ids(&player.songs), Vec::<&str>::new());

    // Nothing else moved: the epoch never left its loading-time value while
    // every message ran.
    assert_eq!(player.songs.epoch, epoch);
}
