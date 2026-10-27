use super::*;

// The library search box's text is stored on every keystroke; submitting is
// what turns it into a fetch. This pins the storing half.
#[test]
fn search_changed_stores_the_box_text() {
    let (mut player, _state) = test_player();

    let _ = update(&mut player, Message::SearchChanged("echo".to_string()));

    assert_eq!(player.search_query, "echo");
    assert!(!player.search_active);
}

// Ctrl+F is a pure focus affordance: it moves keyboard focus to the Search
// library box and leaves the browse view, list order, selection, typed query,
// search-active flag, and playback alone. The focus operation is applied to
// the real laid-out root view, then a focusable capture confirms the search
// input is the one widget focused.
#[test]
fn ctrl_f_focuses_the_search_box_and_changes_no_browse_state() {
    use iced::Rectangle;
    use iced::advanced::widget::{Id, operation::Focusable, operation::Operation};

    #[derive(Default)]
    struct FocusCapture(Vec<(Option<Id>, bool)>);
    impl Operation for FocusCapture {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }
        fn focusable(&mut self, id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
            self.0.push((id.cloned(), state.is_focused()));
        }
    }

    let (mut player, state) = test_player();
    player.current_view = CurrentView::Albums;
    player.albums.items = vec![sample_album()];
    let _ = update(&mut player, Message::SelectRow { epoch: 0, index: 0 });
    player.search_query = "echo".to_string();
    {
        let mut shared = state.blocking_lock();
        shared.current_track = Some("song-1".to_string());
        shared.is_playing = true;
    }
    let before_rows = player.albums.items.clone();

    let mut operation = futures::executor::block_on(widget_operation(
        update(&mut player, Message::FocusSearch),
        "focus search",
    ));

    assert_view(&player, CurrentView::Albums);
    assert_eq!(player.albums.items, before_rows);
    assert_eq!(player.albums.selected(), Some(0));
    assert_eq!(player.search_query, "echo");
    assert!(!player.search_active);
    {
        let shared = state.blocking_lock();
        assert_eq!(shared.current_track.as_deref(), Some("song-1"));
        assert!(shared.is_playing);
    }

    let renderer = layout_capture::headless_renderer();
    let mut element = view(&player);
    let (mut tree, node) = layout_capture::layout_tree(&mut element, &renderer);
    layout_capture::apply_operation(
        &mut element,
        &renderer,
        &mut tree,
        &node,
        operation.as_mut(),
    );

    let mut capture = FocusCapture::default();
    layout_capture::apply_operation(&mut element, &renderer, &mut tree, &node, &mut capture);
    let focused: Vec<Option<Id>> = capture
        .0
        .iter()
        .filter(|(_, is_focused)| *is_focused)
        .map(|(id, _)| id.clone())
        .collect();
    assert_eq!(
        focused,
        vec![Some(Id::new(BROWSE_SEARCH_ID))],
        "Ctrl+F must focus exactly the search box"
    );
}

// Shade mode builds no browse tree, so Ctrl+F there must be a no-op: the focus
// operation finds no `BROWSE_SEARCH_ID` focusable and leaves every focusable
// unfocused. If the rolled-up view ever kept the browse/search widgets (hidden
// or not), the key would move focus to an input the user cannot see and typed
// keys would edit a hidden query instead of doing nothing.
#[test]
fn ctrl_f_is_a_noop_in_shade_mode() {
    use iced::Rectangle;
    use iced::advanced::widget::{Id, operation::Focusable, operation::Operation};

    #[derive(Default)]
    struct FocusCapture {
        ids: Vec<Option<Id>>,
        focused: Vec<Option<Id>>,
    }
    impl Operation for FocusCapture {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }
        fn focusable(&mut self, id: Option<&Id>, _bounds: Rectangle, state: &mut dyn Focusable) {
            self.ids.push(id.cloned());
            if state.is_focused() {
                self.focused.push(id.cloned());
            }
        }
    }

    let (mut player, _state) = test_player();
    player.shaded = true;

    let mut operation = futures::executor::block_on(widget_operation(
        update(&mut player, Message::FocusSearch),
        "focus search in shade mode",
    ));

    let renderer = layout_capture::headless_renderer();
    let mut element = view(&player);
    let (mut tree, node) = layout_capture::layout_tree(&mut element, &renderer);
    layout_capture::apply_operation(
        &mut element,
        &renderer,
        &mut tree,
        &node,
        operation.as_mut(),
    );

    let mut capture = FocusCapture::default();
    layout_capture::apply_operation(&mut element, &renderer, &mut tree, &node, &mut capture);
    assert!(
        !capture.ids.contains(&Some(Id::new(BROWSE_SEARCH_ID))),
        "the shade view must build no search box: {:?}",
        capture.ids
    );
    assert!(
        capture.focused.is_empty(),
        "Ctrl+F in shade mode must leave every focusable unfocused: {:?}",
        capture.focused
    );
}

// Submitting a query replaces the songs buffer with the library matches and
// shows them under the Songs view — the same buffer the browse hierarchy
// uses, so the playing-row marker and the empty/failure wording keep working.
#[tokio::test]
async fn search_submitted_replaces_the_songs_list_with_the_matches() {
    let (mut player, _state) = test_player();
    // Seed a browsed album's songs so the search visibly replaces them.
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    // The submit resolves the text `SearchChanged` stored, so type it first.
    let _ = update(&mut player, Message::SearchChanged("  echo  ".to_string()));

    let task = update(&mut player, Message::SearchSubmitted);
    drive_fetch_and_assert_loaded(
        &mut player,
        task,
        "search the library",
        |player| &mut player.songs.items,
        |song| song.id.as_str(),
        &["song-5"],
    )
    .await;

    assert_view(&player, CurrentView::Songs);
    assert!(player.search_active);
    assert_eq!(player.search_query, "echo");
}

// A blank submit has nothing to search; it must not clear the current list,
// flip the view, or mark a search active. The error report `search_songs`
// would produce for the blank query is skipped too.
#[test]
fn search_submitted_with_a_blank_query_is_a_no_op() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    player.current_view = CurrentView::Songs;
    let _ = update(&mut player, Message::SearchChanged("   ".to_string()));

    assert_message_schedules_no_work(&mut player, Message::SearchSubmitted);

    assert_view(&player, CurrentView::Songs);
    assert!(!player.search_active);
    assert_eq!(player.songs.items.len(), 3);
}

// A search's result list hangs off the artists list, not the album it was
// issued from, so Back leaves the search straight for Artists rather than
// opening the album list the search replaced.
#[test]
fn back_leaves_an_active_search_for_the_artists_list() {
    let (mut player, _state) = test_player();
    player.search_active = true;
    player.current_view = CurrentView::Songs;

    let _ = update(&mut player, Message::Back);

    assert_view(&player, CurrentView::Artists);
    assert!(!player.search_active);
}

// Clear without an active search is only a text-box edit: it empties the
// query and leaves the view, rows, selection, search flag, and playback
// exactly as they were, scheduling no fetch or scroll.
#[test]
fn clear_search_without_an_active_search_clears_only_the_query() {
    let (mut player, state) = test_player();
    player.current_view = CurrentView::Albums;
    player.albums.items = vec![sample_album()];
    player.songs.items = vec![sample_song()];
    let _ = update(&mut player, Message::SelectRow { epoch: 0, index: 0 });
    player.search_query = "echo".to_string();
    {
        let mut shared = state.blocking_lock();
        shared.current_track = Some("song-1".to_string());
        shared.is_playing = true;
    }
    let albums = player.albums.items.clone();
    let songs = player.songs.items.clone();

    assert_message_schedules_no_work(&mut player, Message::ClearSearch);

    assert_eq!(player.search_query, "");
    assert_view(&player, CurrentView::Albums);
    assert_eq!(player.albums.items, albums);
    assert_eq!(player.songs.items, songs);
    assert_eq!(player.albums.selected(), Some(0));
    assert!(!player.search_active);
    let shared = state.blocking_lock();
    assert_eq!(shared.current_track.as_deref(), Some("song-1"));
    assert!(shared.is_playing);
}

// Clear with an empty query and no active search has nothing to do, so it
// schedules no work at all.
#[test]
fn clear_search_with_an_empty_query_schedules_no_work() {
    let (mut player, _state) = test_player();

    assert_message_schedules_no_work(&mut player, Message::ClearSearch);

    assert_eq!(player.search_query, "");
    assert!(!player.search_active);
}

// Clear while a search is showing is the Back button plus an emptied query:
// it returns to Artists, restores the artists list's library order, snaps the
// list to the top, and leaves the search-result rows in the songs buffer.
#[tokio::test]
async fn clear_search_while_a_search_is_active_returns_to_artists_and_clears_the_query() {
    let (mut player, _state) = test_player();
    let mut first_alphabetically = sample_artist();
    first_alphabetically.id = "artist-0".to_string();
    first_alphabetically.name = "A Sample Band".to_string();
    player.artists.items = vec![sample_artist(), first_alphabetically];
    // The startup artists list stays marked loading until a fetch reply
    // lands, and sorting a loading list is a documented no-op.
    player.artists.loading = false;
    let library_order = player.artists.items.clone();
    let _ = update(&mut player, Message::SortBrowse);
    assert_ne!(player.artists.items, library_order);

    player.search_active = true;
    player.current_view = CurrentView::Songs;
    player.search_query = "echo".to_string();
    player.songs.items = stepping_songs();
    let search_results = player.songs.items.clone();

    assert_task_snaps_browse_list_to_top(update(&mut player, Message::ClearSearch), "clear search")
        .await;

    assert_eq!(player.search_query, "");
    assert!(!player.search_active);
    assert_view(&player, CurrentView::Artists);
    assert_eq!(player.artists.items, library_order);
    assert_eq!(player.songs.items, search_results);
}

// Entering a browse level leaves the search: ArtistSelected and AlbumSelected
// both clear the active flag, so Back from the level they open behaves like an
// ordinary browse again.
#[test]
fn entering_a_browse_level_clears_the_active_search() {
    let (mut player, _state) = test_player();
    player.artists.items = vec![sample_artist()];
    player.search_active = true;

    let _ = update(&mut player, Message::ArtistSelected { epoch: 0, index: 0 });
    assert!(!player.search_active);

    let (mut player, _state) = test_player();
    player.albums.items = vec![sample_album()];
    player.search_active = true;

    let _ = update(&mut player, Message::AlbumSelected { epoch: 0, index: 0 });
    assert!(!player.search_active);
}
