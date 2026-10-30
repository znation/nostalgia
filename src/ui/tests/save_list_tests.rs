//! The `Save list` button, its naming panel, and the saved song lists the
//! Playlists view reopens.

use super::*;

/// A player whose shared songs list shows two loaded rows under the Songs
/// view — the shape the `Save list` button acts on.
fn player_showing_songs() -> (WinampPlayer, Arc<Mutex<AppState>>) {
    let (mut player, state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(numbered_songs(2)));
    player.current_view = CurrentView::Songs;
    (player, state)
}

// The button is a no-op like Sort's no-ops: off the shared songs list, and
// while the list is loading, failed, or empty, it opens no panel, schedules
// no work, and saves nothing.
#[test]
fn toggle_save_list_is_a_noop_without_a_saveable_songs_list() {
    let (mut player, _state) = test_player();

    let _ = update(&mut player, Message::ToggleSaveList);
    assert!(!player.save_list_open);

    let _ = update(&mut player, Message::SongsLoaded(numbered_songs(2)));
    player.current_view = CurrentView::Songs;

    player.songs.loading = true;
    let _ = update(&mut player, Message::ToggleSaveList);
    assert!(!player.save_list_open);

    player.songs.loading = false;
    player.songs.error = Some("boom".to_string());
    let _ = update(&mut player, Message::ToggleSaveList);
    assert!(!player.save_list_open);

    player.songs.error = None;
    let _ = update(&mut player, Message::SongsLoaded(Vec::new()));
    let _ = update(&mut player, Message::ToggleSaveList);
    assert!(!player.save_list_open);
    assert!(player.saved_playlists.is_empty());
}

// On a loaded, non-empty songs list the button opens the panel with an empty
// box, closing every other transient panel and clearing their flags.
#[test]
fn the_save_list_panel_opens_empty_and_closes_the_other_panels() {
    let (mut player, _state) = player_showing_songs();
    player.jump_time_visible = true;

    let _ = update(&mut player, Message::ToggleSaveList);

    assert!(player.save_list_open);
    assert!(player.transient_panel_open());
    assert_eq!(player.save_list_name, "");
    assert_eq!(player.save_list_note, None);
    assert!(
        !player.jump_time_visible,
        "the save-list panel must close Jump to time on opening"
    );

    player.save_list_open = false;
    let _ = update(&mut player, Message::ToggleSaveList);
    assert!(player.save_list_open);
    assert_eq!(player.save_list_name, "");
}

// Enter saves the showing list's song ids, in showing order, under the
// trimmed name and closes the panel. The showing list is untouched.
#[test]
fn save_list_submitted_saves_the_showing_songs_ids() {
    let (mut player, _state) = player_showing_songs();
    let _ = update(&mut player, Message::ToggleSaveList);
    let _ = update(
        &mut player,
        Message::SaveListNameChanged("  Morning Drive ".to_string()),
    );

    let _ = update(&mut player, Message::SaveListSubmitted);

    assert!(!player.save_list_open);
    assert_eq!(player.save_list_note, None);
    assert_eq!(player.saved_playlists.len(), 1);
    let saved = &player.saved_playlists[0];
    assert_eq!(saved.name, "Morning Drive");
    assert_eq!(
        saved.song_ids,
        numbered_songs(2)
            .iter()
            .map(|song| song.id.clone())
            .collect::<Vec<_>>()
    );
    assert_eq!(player.songs.items.len(), 2);
    assert!(player.current_view == CurrentView::Songs);
}

// Typing is refused inline and saves nothing: an empty box keeps the panel
// open with `Type a name`, a colon shows `Use it without a colon` because the
// settings line's separator is the colon, and a closed panel ignores a
// submit.
#[test]
fn save_list_submitted_refuses_empty_colon_and_closed_panels() {
    let (mut player, _state) = player_showing_songs();
    let _ = update(&mut player, Message::ToggleSaveList);

    let _ = update(&mut player, Message::SaveListSubmitted);
    assert!(player.save_list_open);
    assert_eq!(
        player.save_list_note,
        Some(views::SAVE_LIST_EMPTY_NAME_NOTE)
    );
    assert!(player.saved_playlists.is_empty());

    let _ = update(
        &mut player,
        Message::SaveListNameChanged("A:name".to_string()),
    );
    let _ = update(&mut player, Message::SaveListSubmitted);
    assert!(player.save_list_open);
    assert_eq!(
        player.save_list_note,
        Some(views::SAVE_LIST_COLON_NAME_NOTE)
    );
    assert!(player.saved_playlists.is_empty());

    let _ = update(
        &mut player,
        Message::SaveListNameChanged("My List\neq_auto=on".to_string()),
    );
    let _ = update(&mut player, Message::SaveListSubmitted);
    assert!(player.save_list_open);
    assert_eq!(
        player.save_list_note,
        Some(views::SAVE_LIST_LINE_BREAK_NAME_NOTE)
    );
    assert!(player.saved_playlists.is_empty());

    // A keystroke clears the note, so the next submit starts clean.
    let _ = update(
        &mut player,
        Message::SaveListNameChanged("Morning Drive".to_string()),
    );
    assert_eq!(player.save_list_note, None);

    player.save_list_open = false;
    let _ = update(&mut player, Message::SaveListSubmitted);
    assert!(
        !player.save_list_open,
        "a closed panel must not reopen by saving"
    );
}

// A name equal to an existing save in any case replaces that list's ids in
// place: one row stays and its name is kept as first saved.
#[test]
fn saving_the_same_name_again_replaces_the_saved_ids() {
    let (mut player, _state) = player_showing_songs();
    let _ = update(&mut player, Message::ToggleSaveList);
    let _ = update(
        &mut player,
        Message::SaveListNameChanged("Morning Drive".to_string()),
    );
    let _ = update(&mut player, Message::SaveListSubmitted);
    let _ = update(&mut player, Message::SongsLoaded(numbered_songs(3)));

    let _ = update(&mut player, Message::ToggleSaveList);
    let _ = update(
        &mut player,
        Message::SaveListNameChanged("MORNING DRIVE".to_string()),
    );
    let _ = update(&mut player, Message::SaveListSubmitted);

    assert!(!player.save_list_open);
    assert_eq!(player.saved_playlists.len(), 1);
    assert_eq!(player.saved_playlists[0].name, "Morning Drive");
    assert_eq!(
        player.saved_playlists[0].song_ids,
        numbered_songs(3)
            .iter()
            .map(|song| song.id.clone())
            .collect::<Vec<_>>()
    );
}

// The panel's `Delete` button removes the saved list matching the typed
// name in any case, in its stored position: the other list keeps its stored
// position and its ids.
#[test]
fn delete_list_submitted_removes_the_named_saved_list() {
    let (mut player, _state) = player_showing_songs();
    player.saved_playlists = vec![
        crate::settings::SavedPlaylist {
            name: "Morning Drive".to_string(),
            song_ids: vec!["song-1".to_string()],
        },
        crate::settings::SavedPlaylist {
            name: "Evening Drive".to_string(),
            song_ids: vec!["song-2".to_string(), "song-3".to_string()],
        },
    ];
    let _ = update(&mut player, Message::ToggleSaveList);
    let _ = update(
        &mut player,
        Message::SaveListNameChanged("  MORNING drive ".to_string()),
    );

    let _ = update(&mut player, Message::DeleteListSubmitted);

    assert!(!player.save_list_open);
    assert_eq!(player.save_list_note, None);
    assert_eq!(player.saved_playlists.len(), 1);
    let kept = &player.saved_playlists[0];
    assert_eq!(kept.name, "Evening Drive");
    assert_eq!(
        kept.song_ids,
        vec!["song-2".to_string(), "song-3".to_string()]
    );
}

// Deleting refuses inline and removes nothing: an empty box keeps the panel
// open with `Type a name`, a colon shows `Use it without a colon`, and a
// name no saved list carries shows `No such playlist` with the panel staying
// open and the typed name kept.
#[test]
fn delete_list_submitted_refuses_blank_colon_and_unknown_names() {
    let (mut player, _state) = player_showing_songs();
    player.saved_playlists = vec![crate::settings::SavedPlaylist {
        name: "Morning Drive".to_string(),
        song_ids: vec!["song-1".to_string()],
    }];
    let _ = update(&mut player, Message::ToggleSaveList);

    let _ = update(&mut player, Message::DeleteListSubmitted);
    assert!(player.save_list_open);
    assert_eq!(
        player.save_list_note,
        Some(views::SAVE_LIST_EMPTY_NAME_NOTE)
    );
    assert_eq!(player.saved_playlists.len(), 1);

    let _ = update(
        &mut player,
        Message::SaveListNameChanged("A:name".to_string()),
    );
    let _ = update(&mut player, Message::DeleteListSubmitted);
    assert!(player.save_list_open);
    assert_eq!(
        player.save_list_note,
        Some(views::SAVE_LIST_COLON_NAME_NOTE)
    );
    assert_eq!(player.saved_playlists.len(), 1);

    let _ = update(
        &mut player,
        Message::SaveListNameChanged("Nocturne".to_string()),
    );
    let _ = update(&mut player, Message::DeleteListSubmitted);
    assert!(
        player.save_list_open,
        "an unknown name must keep the panel open"
    );
    assert_eq!(player.save_list_note, Some(views::SAVE_LIST_NO_SUCH_NOTE));
    assert_eq!(player.save_list_name, "Nocturne");
    assert_eq!(
        player.saved_playlists[0].name, "Morning Drive",
        "an unknown name must remove nothing"
    );
}

// A closed panel ignores a delete: nothing is removed and the flag stays
// closed.
#[test]
fn delete_list_submitted_is_a_noop_with_the_panel_closed() {
    let (mut player, _state) = player_showing_songs();
    player.saved_playlists = vec![crate::settings::SavedPlaylist {
        name: "Morning Drive".to_string(),
        song_ids: vec!["song-1".to_string()],
    }];
    player.save_list_name = "Morning Drive".to_string();
    player.save_list_open = false;

    let _ = update(&mut player, Message::DeleteListSubmitted);

    assert!(!player.save_list_open);
    assert_eq!(player.saved_playlists.len(), 1);
}

// Deleting the saved list whose songs are currently showing touches only
// the saved list: the showing rows, the playing track, and the view stay
// exactly as they were, like a removed preset's applied curve.
#[test]
fn deleting_the_showing_saved_list_leaves_the_showing_rows_untouched() {
    let (mut player, _state) = player_showing_songs();
    player.saved_playlists = vec![crate::settings::SavedPlaylist {
        name: "Morning Drive".to_string(),
        song_ids: numbered_songs(2).iter().map(|s| s.id.clone()).collect(),
    }];
    let showing_ids: Vec<String> = player
        .songs
        .items
        .iter()
        .map(|song| song.id.clone())
        .collect();
    let _ = update(&mut player, Message::ToggleSaveList);
    let _ = update(
        &mut player,
        Message::SaveListNameChanged("Morning Drive".to_string()),
    );

    let _ = update(&mut player, Message::DeleteListSubmitted);

    assert!(player.saved_playlists.is_empty());
    assert!(player.current_view == CurrentView::Songs);
    let still_showing: Vec<String> = player
        .songs
        .items
        .iter()
        .map(|song| song.id.clone())
        .collect();
    assert_eq!(still_showing, showing_ids, "the showing rows must stay");
}

// The Playlists fetch lands with one sentinel row appended per saved list,
// after the library's rows, so they render with continued numbering and the
// footer counts them.
#[test]
fn playlists_loaded_appends_a_sentinel_row_per_saved_list() {
    let (mut player, _state) = test_player();
    player.saved_playlists.push(crate::settings::SavedPlaylist {
        name: "Morning Drive".to_string(),
        song_ids: vec!["song-1".to_string()],
    });

    let _ = update(
        &mut player,
        Message::PlaylistsLoaded(vec![sample_playlist()]),
    );

    assert_eq!(player.playlists.items.len(), 2);
    assert_eq!(player.playlists.items[0].id, "playlist-1");
    assert_eq!(player.playlists.items[1].id, SAVED_PLAYLIST_ROW_ID);
    assert_eq!(player.playlists.items[1].name, "Morning Drive");
}

// Pressing a saved list's sentinel row opens its songs under `SongsOrigin
// ::UserPlaylist`: the ids resolve through `resolve_song` in saved order, so
// the songs are stored on the reply like a genre's group. Back returns to
// the Playlists view.
#[test]
fn a_sentinel_row_press_opens_the_saved_lists_songs() {
    let (mut player, _state) = test_player();
    player.saved_playlists.push(crate::settings::SavedPlaylist {
        name: "Morning Drive".to_string(),
        song_ids: vec!["song-2".to_string(), "song-1".to_string()],
    });
    let _ = update(
        &mut player,
        Message::PlaylistsLoaded(vec![sample_playlist()]),
    );
    let epoch = player.playlists.epoch;

    let task = update(&mut player, Message::PlaylistSelected { epoch, index: 1 });
    assert!(player.current_view == CurrentView::Songs);
    assert_eq!(player.songs_origin, SongsOrigin::UserPlaylist);
    let reply = futures::executor::block_on(task_output(task, "saved playlist open"));
    let _ = update(&mut player, reply);
    // The ids resolve in saved order, so the songs are stored in that
    // order, not the library's.
    assert_eq!(player.current_view, CurrentView::Songs);
    assert_eq!(
        player
            .songs
            .items
            .iter()
            .map(|song| song.id.clone())
            .collect::<Vec<_>>(),
        vec!["song-2".to_string(), "song-1".to_string()]
    );
    let _ = update(&mut player, Message::Back);
    assert_eq!(player.current_view, CurrentView::Playlists);
}

// An id the library no longer answers is dropped at open time: the songs the
// ids still resolve to keep their saved order.
#[test]
fn unresolvable_saved_ids_are_dropped_when_the_list_opens() {
    let (mut player, _state) = test_player();
    player.saved_playlists.push(crate::settings::SavedPlaylist {
        name: "Morning Drive".to_string(),
        song_ids: vec!["song-999".to_string(), "song-1".to_string()],
    });
    let _ = update(
        &mut player,
        Message::PlaylistsLoaded(vec![sample_playlist()]),
    );

    let epoch = player.playlists.epoch;
    let task = update(&mut player, Message::PlaylistSelected { epoch, index: 1 });
    let reply = futures::executor::block_on(task_output(task, "saved playlist resolution"));
    let _ = update(&mut player, reply);
    assert!(player.current_view == CurrentView::Songs);
    assert_eq!(
        player
            .songs
            .items
            .iter()
            .map(|song| song.id.clone())
            .collect::<Vec<_>>(),
        vec!["song-1".to_string()]
    );
}

// A reply a newer songs request superseded is dropped, as every browse reply
// is: `BrowseReply`'s UI-thread re-check names the songs list in its own arm,
// so a slower replay of the first save cannot overwrite the second save's
// resolution. Issue the first resolution, drive it while it is still
// current, issue a newer one before `update` processes the reply, and assert
// the stamped reply is dropped.
#[tokio::test]
async fn a_superseded_saved_list_reply_is_dropped() {
    let (mut player, _state) = test_player();
    player.saved_playlists.push(crate::settings::SavedPlaylist {
        name: "Morning Drive".to_string(),
        song_ids: vec!["song-1".to_string()],
    });
    let _ = update(
        &mut player,
        Message::PlaylistsLoaded(vec![sample_playlist()]),
    );

    let epoch = player.playlists.epoch;
    let task = update(&mut player, Message::PlaylistSelected { epoch, index: 1 });
    let reply = task_output(task, "first saved-list resolution").await;
    assert!(
        matches!(
            &reply,
            Message::BrowseReply { reply, .. }
                if matches!(reply.as_ref(), Message::UserPlaylistLoaded(_))
        ),
        "the driven resolution must produce a stamped reply, got {reply:?}"
    );

    // A newer songs request is issued before `update` processes the reply —
    // a second saved list opened while the first was still resolving.
    let _ = player.songs.begin_fetch();

    let _ = update(&mut player, reply);
    assert!(player.songs.items.is_empty());
    assert!(player.songs.loading);
}

// A Sort of the Playlists view can move each sentinel row, so the press
// must resolve the saved list by the row's name, not by its position among
// trailing rows: sorting the Playlists rows, then pressing a sentinel row,
// opens the list the row's name names.
#[test]
fn a_sorted_playlists_view_still_opens_the_named_saved_list() {
    let (mut player, _state) = test_player();
    player.saved_playlists.push(crate::settings::SavedPlaylist {
        name: "Morning Drive".to_string(),
        song_ids: vec!["song-1".to_string()],
    });
    let _ = update(
        &mut player,
        Message::PlaylistsLoaded(vec![sample_playlist()]),
    );
    let _ = update(&mut player, Message::SortBrowse);

    let mut sentinel_index = usize::MAX;
    for (index, row) in player.playlists.items.iter().enumerate() {
        if row.id == SAVED_PLAYLIST_ROW_ID {
            sentinel_index = index;
        }
    }
    assert_ne!(sentinel_index, usize::MAX, "the sentinel must still exist");

    let epoch = player.playlists.epoch;
    let task = update(
        &mut player,
        Message::PlaylistSelected {
            epoch,
            index: sentinel_index,
        },
    );
    let _ = update(
        &mut player,
        futures::executor::block_on(task_output(task, "sorted press")),
    );
    assert_eq!(player.current_view, CurrentView::Songs);
    assert_eq!(
        player
            .songs
            .items
            .iter()
            .map(|song| song.id.clone())
            .collect::<Vec<_>>(),
        vec!["song-1".to_string()]
    );
}

// The Save list button renders in the Top/Sort/Reload row so a list a user
// assembles can be named without leaving the level it shows on.
#[test]
fn the_save_list_button_lays_out_in_the_nav_row() {
    let (player, _state) = test_player();
    let capture = captured_root_view_text_bounds(&player);
    let reload = capture.find("Reload");
    let save_list = capture.find("Save list");

    assert_eq!(
        save_list.y,
        reload.y,
        "Save list must share the Reload row: {:?}",
        capture.fragments()
    );
    assert!(
        save_list.x >= reload.x + reload.width,
        "Save list must render to the right of Reload: {:?}",
        capture.fragments()
    );
}
