//! Plain J's Jump to File panel: opening, filtering, playing a match, and
//! closing.

use super::window_tests::make_current_track_known;
use super::*;
use crate::ui::handlers::jump::jump_file_row_top;
use crate::ui::views::{
    JUMP_FILE_LIST_HEIGHT, JUMP_FILE_LIST_ID, JUMP_FILE_LIST_PADDING, JUMP_FILE_ROW_HEIGHT,
    JUMP_FILE_ROW_SPACING,
};

fn player_in_songs() -> (WinampPlayer, Arc<Mutex<AppState>>) {
    let (mut player, state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    player.current_view = CurrentView::Songs;
    (player, state)
}

fn submit(player: &mut WinampPlayer, query: &str) -> Task<Message> {
    let _ = update(player, Message::JumpFileChanged(query.to_string()));
    update(player, Message::JumpFileSubmitted)
}

#[test]
fn toggle_opens_with_an_empty_query_and_closes_again() {
    let (mut player, _state) = player_in_songs();
    player.jump_file_query = "stale".to_string();

    assert_message_schedules_work(&mut player, Message::ToggleJumpFile);
    assert!(player.jump_file_visible);
    assert!(player.jump_file_query.is_empty());

    assert_message_schedules_no_work(&mut player, Message::ToggleJumpFile);
    assert!(!player.jump_file_visible);
}

#[test]
fn toggle_needs_a_showing_song_list() {
    let (mut player, _state) = test_player();
    for view in [
        CurrentView::Artists,
        CurrentView::Albums,
        CurrentView::AllAlbums,
        CurrentView::Playlists,
    ] {
        player.current_view = view;
        assert_message_schedules_no_work(&mut player, Message::ToggleJumpFile);
        assert!(!player.jump_file_visible);
    }
}

#[test]
fn submit_plays_the_first_match_and_closes() {
    let (mut player, _state) = player_in_songs();
    let _ = update(&mut player, Message::ToggleJumpFile);
    let epoch = player.songs.epoch;

    let task = submit(&mut player, "two");

    assert_track_selected(task, epoch, 1);
    assert!(!player.jump_file_visible);
}

#[test]
fn submit_matches_the_artist_and_title_words_together() {
    let (mut player, _state) = player_in_songs();
    let _ = update(&mut player, Message::ToggleJumpFile);
    let epoch = player.songs.epoch;

    let task = submit(&mut player, "band two");

    assert_track_selected(task, epoch, 1);
    assert!(!player.jump_file_visible);
}

#[test]
fn submit_uses_the_all_songs_list_when_it_is_showing() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::AllSongsLoaded(stepping_songs()));
    player.current_view = CurrentView::AllSongs;
    let epoch = player.all_songs.epoch;
    let _ = update(&mut player, Message::ToggleJumpFile);

    let task = submit(&mut player, "three");

    assert_track_selected(task, epoch, 2);
    assert!(!player.jump_file_visible);
}

#[test]
fn down_and_up_move_the_selection_through_the_matches() {
    let (mut player, _state) = player_in_songs();
    // Closed, Up and Down change nothing.
    assert_message_schedules_no_work(&mut player, Message::JumpFileSelectionDown);
    assert_eq!(player.jump_file_selected, None);

    let _ = update(&mut player, Message::ToggleJumpFile);
    assert_eq!(player.jump_file_selected, None);

    // Down selects the second match, then the last, and stops there. Every
    // move asks the panel to reveal its landed row.
    assert_message_schedules_work(&mut player, Message::JumpFileSelectionDown);
    assert_eq!(player.jump_file_selected, Some(1));
    assert_message_schedules_work(&mut player, Message::JumpFileSelectionDown);
    assert_eq!(player.jump_file_selected, Some(2));
    assert_message_schedules_work(&mut player, Message::JumpFileSelectionDown);
    assert_eq!(player.jump_file_selected, Some(2));

    // Up walks back and stops at the first match.
    assert_message_schedules_work(&mut player, Message::JumpFileSelectionUp);
    assert_eq!(player.jump_file_selected, Some(1));
    assert_message_schedules_work(&mut player, Message::JumpFileSelectionUp);
    assert_eq!(player.jump_file_selected, Some(0));
    assert_message_schedules_work(&mut player, Message::JumpFileSelectionUp);
    assert_eq!(player.jump_file_selected, Some(0));

    // Closing clears the highlight, so the next opening starts at the first.
    let _ = update(&mut player, Message::ToggleJumpFile);
    assert_eq!(player.jump_file_selected, None);
    assert_message_schedules_no_work(&mut player, Message::JumpFileSelectionDown);
    assert_eq!(player.jump_file_selected, None);
}

#[test]
fn page_up_and_down_move_the_highlight_by_one_page_and_clamp() {
    let (mut player, _state) = test_player();
    let songs: Vec<Song> = (0..12)
        .map(|index| {
            let mut song = stepping_songs()[0].clone();
            song.id = format!("song-{index}");
            song.title = format!("Track {index}");
            song
        })
        .collect();
    let _ = update(&mut player, Message::SongsLoaded(songs));
    player.current_view = CurrentView::Songs;

    // Closed, the page keys leave the highlight alone and schedule nothing.
    assert_message_schedules_no_work(&mut player, Message::JumpFileSelectionPageDown);
    assert_message_schedules_no_work(&mut player, Message::JumpFileSelectionPageUp);
    assert_eq!(player.jump_file_selected, None);

    let _ = update(&mut player, Message::ToggleJumpFile);
    assert_eq!(player.jump_file_selected, None);

    // Page Down starts from the first match and moves five rows a page.
    assert_message_schedules_work(&mut player, Message::JumpFileSelectionPageDown);
    assert_eq!(player.jump_file_selected, Some(5));
    assert_message_schedules_work(&mut player, Message::JumpFileSelectionPageDown);
    assert_eq!(player.jump_file_selected, Some(10));
    // A page past the last match clamps there.
    assert_message_schedules_work(&mut player, Message::JumpFileSelectionPageDown);
    assert_eq!(player.jump_file_selected, Some(11));

    // Page Up walks back by one page and clamps at the first match.
    assert_message_schedules_work(&mut player, Message::JumpFileSelectionPageUp);
    assert_eq!(player.jump_file_selected, Some(6));
    assert_message_schedules_work(&mut player, Message::JumpFileSelectionPageUp);
    assert_eq!(player.jump_file_selected, Some(1));
    assert_message_schedules_work(&mut player, Message::JumpFileSelectionPageUp);
    assert_eq!(player.jump_file_selected, Some(0));
    assert_message_schedules_work(&mut player, Message::JumpFileSelectionPageUp);
    assert_eq!(player.jump_file_selected, Some(0));
}

#[test]
fn jump_file_row_top_starts_at_the_padding_and_steps_by_one_row_and_gap() {
    let pitch = JUMP_FILE_ROW_HEIGHT + JUMP_FILE_ROW_SPACING;
    assert_eq!(jump_file_row_top(0), JUMP_FILE_LIST_PADDING);
    assert_eq!(jump_file_row_top(3), JUMP_FILE_LIST_PADDING + 3.0 * pitch);
}

/// Drives a Jump to File highlight move's task until it runs its scroll
/// operation and applies that operation to a match list scrolled to
/// `scroll_top`, asserting it touches only the match list's id.
async fn jump_file_scroll_effects(task: Task<Message>, scroll_top: f32) -> BrowseScrollEffects {
    use iced::advanced::widget::Id;

    let bounds = iced::Rectangle {
        height: JUMP_FILE_LIST_HEIGHT,
        ..iced::Rectangle::default()
    };
    let translation = iced::Vector::new(0.0, scroll_top);
    let mut operation = widget_operation(task, "a Jump to File highlight move").await;
    let effects = browse_scroll_effects_by(
        &mut *operation,
        &Id::new(JUMP_FILE_LIST_ID),
        bounds,
        translation,
    );
    let unrelated = browse_scroll_effects_by(
        &mut *operation,
        &Id::new("not-the-jump-file-list"),
        bounds,
        translation,
    );
    assert!(
        !unrelated.any(),
        "a Jump to File highlight move must scroll only the match list id"
    );
    effects
}

#[test]
fn a_highlight_move_reveals_only_the_landed_row_in_the_match_list() {
    use iced::advanced::widget::operation::scrollable::AbsoluteOffset;

    let (mut player, _state) = test_player();
    let songs: Vec<Song> = (0..12)
        .map(|index| {
            let mut song = stepping_songs()[0].clone();
            song.id = format!("song-{index}");
            song.title = format!("Track {index}");
            song
        })
        .collect();
    let _ = update(&mut player, Message::SongsLoaded(songs));
    player.current_view = CurrentView::Songs;
    let _ = update(&mut player, Message::ToggleJumpFile);

    // Page Down lands on row 5, below the fixed-height list, so the task
    // scrolls the match list to that row's bottom edge and nothing else.
    let effects = futures::executor::block_on(jump_file_scroll_effects(
        update(&mut player, Message::JumpFileSelectionPageDown),
        0.0,
    ));
    assert_eq!(player.jump_file_selected, Some(5));
    let expected = jump_file_row_top(5) + JUMP_FILE_ROW_HEIGHT - JUMP_FILE_LIST_HEIGHT;
    assert_eq!(
        effects.scrolled,
        Some(AbsoluteOffset {
            x: None,
            y: Some(expected),
        })
    );
    assert_eq!(effects.snapped, None);
    assert_eq!(effects.scrolled_by, None);

    // From row 5 to row 4 stays inside the view the first scroll left, so the
    // move asks for no scroll at all.
    let effects = futures::executor::block_on(jump_file_scroll_effects(
        update(&mut player, Message::JumpFileSelectionUp),
        expected,
    ));
    assert_eq!(player.jump_file_selected, Some(4));
    assert_eq!(effects.scrolled, None);
    assert_eq!(effects.snapped, None);
    assert_eq!(effects.scrolled_by, None);
}

#[test]
fn typing_a_filter_or_reopening_resets_the_selection_to_the_first_match() {
    let (mut player, _state) = player_in_songs();
    let _ = update(&mut player, Message::ToggleJumpFile);
    assert_message_schedules_work(&mut player, Message::JumpFileSelectionDown);
    assert_eq!(player.jump_file_selected, Some(1));

    let _ = update(&mut player, Message::JumpFileChanged("two".to_string()));
    assert_eq!(player.jump_file_selected, None);

    assert_message_schedules_work(&mut player, Message::JumpFileSelectionDown);
    assert_eq!(player.jump_file_selected, Some(1));
    let _ = update(&mut player, Message::ToggleJumpFile);
    let _ = update(&mut player, Message::ToggleJumpFile);
    assert_eq!(player.jump_file_selected, None);
}

#[test]
fn submit_plays_the_selected_match() {
    let (mut player, _state) = player_in_songs();
    let _ = update(&mut player, Message::ToggleJumpFile);
    let epoch = player.songs.epoch;
    let _ = update(&mut player, Message::JumpFileSelectionDown);
    let _ = update(&mut player, Message::JumpFileSelectionDown);

    let task = update(&mut player, Message::JumpFileSubmitted);

    assert_track_selected(task, epoch, 2);
    assert!(!player.jump_file_visible);
    assert_eq!(player.jump_file_selected, None);
}

#[test]
fn a_no_match_submit_keeps_the_panel_and_query() {
    let (mut player, _state) = player_in_songs();
    let _ = update(&mut player, Message::ToggleJumpFile);

    assert_message_schedules_no_work(&mut player, Message::JumpFileChanged("zzz".to_string()));
    assert_message_schedules_no_work(&mut player, Message::JumpFileSubmitted);

    assert!(player.jump_file_visible);
    assert_eq!(player.jump_file_query, "zzz");
}

#[test]
fn a_row_press_dispatches_its_clicked_index() {
    let (mut player, _state) = player_in_songs();
    let _ = update(&mut player, Message::ToggleJumpFile);
    let epoch = player.songs.epoch;

    let task = update(
        &mut player,
        Message::JumpFileRowSelected { epoch, index: 2 },
    );

    assert_track_selected(task, epoch, 2);
    assert!(!player.jump_file_visible);
}

#[test]
fn jump_file_and_the_other_panels_close_each_other() {
    let (mut player, state) = player_in_songs();
    make_current_track_known(&mut player, &state);
    player.playback_duration = Some(Duration::from_secs(30));

    let _ = update(&mut player, Message::ToggleJumpFile);
    assert!(player.jump_file_visible);

    let _ = update(&mut player, Message::ToggleAbout);
    assert!(player.about_visible && !player.jump_file_visible);

    let _ = update(&mut player, Message::ToggleJumpFile);
    assert!(player.jump_file_visible && !player.about_visible);

    let _ = update(&mut player, Message::ToggleFileInfo);
    assert!(player.file_info_visible && !player.jump_file_visible);

    let _ = update(&mut player, Message::ToggleJumpFile);
    assert!(player.jump_file_visible && !player.file_info_visible);

    let _ = update(&mut player, Message::ToggleJumpTime);
    assert!(player.jump_time_visible && !player.jump_file_visible);

    let _ = update(&mut player, Message::ToggleJumpFile);
    assert!(player.jump_file_visible && !player.jump_time_visible);
}

/// Every browse navigation that replaces the showing list clears the panel:
/// each arm in `handlers::browse` and `handlers::browse::search` that changes
/// `current_view` away from a song list. Each case starts from a fresh Songs
/// view with the panel open and feeds the one row the handler needs.
fn assert_navigation_closes_the_panel(setup: impl FnOnce(&mut WinampPlayer), message: Message) {
    let (mut player, _state) = player_in_songs();
    setup(&mut player);
    let _ = update(&mut player, Message::ToggleJumpFile);
    assert!(player.jump_file_visible);

    let _ = update(&mut player, message);

    assert!(!player.jump_file_visible);
}

#[test]
fn every_navigation_that_replaces_the_showing_list_closes_the_panel() {
    assert_navigation_closes_the_panel(|_| {}, Message::ReloadLibrary);
    assert_navigation_closes_the_panel(|_| {}, Message::Back);
    assert_navigation_closes_the_panel(|_| {}, Message::ShowAllSongs);
    assert_navigation_closes_the_panel(|_| {}, Message::ShowAllAlbums);
    assert_navigation_closes_the_panel(|_| {}, Message::ShowPlaylists);
    assert_navigation_closes_the_panel(
        |player| player.artists.items = vec![sample_artist()],
        Message::ArtistSelected { epoch: 0, index: 0 },
    );
    assert_navigation_closes_the_panel(
        |player| player.albums.items = vec![sample_album()],
        Message::AlbumSelected { epoch: 0, index: 0 },
    );
    assert_navigation_closes_the_panel(
        |player| player.playlists.items = vec![sample_playlist()],
        Message::PlaylistSelected { epoch: 0, index: 0 },
    );
    assert_navigation_closes_the_panel(
        |player| player.search_query = "echo".to_string(),
        Message::SearchSubmitted,
    );
}

#[test]
fn toggle_while_shaded_restores_the_window_first() {
    let (mut player, _state) = player_with_window_id();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    player.current_view = CurrentView::Songs;
    shade_with_known_size(&mut player);

    assert_message_schedules_work(&mut player, Message::ToggleJumpFile);

    assert!(!player.shaded);
    assert!(player.jump_file_visible);
}

#[test]
fn the_root_view_draws_only_the_filtered_row_in_the_panel() {
    let (mut player, _state) = player_in_songs();
    // The base browse list is hidden so the capture names only the panel's
    // rows; the panel itself still filters the same cached list.
    player.library_panel_visible = false;
    player.jump_file_visible = true;
    player.jump_file_query = "two".to_string();

    let capture = captured_root_view_text_bounds(&player);
    let texts: Vec<&str> = capture.texts().collect();
    assert!(texts.contains(&"JUMP TO FILE"), "{texts:?}");
    assert!(texts.contains(&"2. The Sample Band - Two"), "{texts:?}");
    assert!(!texts.contains(&"1. The Sample Band - One"), "{texts:?}");
    assert!(!texts.contains(&"3. The Sample Band - Three"), "{texts:?}");
}

// The root view picks the list the panel filters from the showing view, so
// each flat list must draw its own titles, and a view without a song list
// (or a shaded window) must draw no panel at all.
#[test]
fn the_root_view_filters_the_all_songs_and_recently_played_lists() {
    for (label, view) in [
        ("all songs", CurrentView::AllSongs),
        ("recently played", CurrentView::RecentlyPlayed),
    ] {
        let (mut player, _state) = test_player();
        // Only the showing view's list is loaded, so drawing another
        // view's list would draw nothing.
        let loaded = match label {
            "all songs" => Message::AllSongsLoaded(stepping_songs()),
            _ => Message::RecentlyPlayedLoaded(stepping_songs()),
        };
        let _ = update(&mut player, loaded);
        player.current_view = view;
        player.library_panel_visible = false;
        player.jump_file_visible = true;
        player.jump_file_query = "three".to_string();

        let capture = captured_root_view_text_bounds(&player);
        let texts: Vec<&str> = capture.texts().collect();
        assert!(texts.contains(&"JUMP TO FILE"), "{label}: {texts:?}");
        assert!(
            texts.contains(&"3. The Sample Band - Three"),
            "{label}: {texts:?}"
        );
        assert!(
            !texts.contains(&"1. The Sample Band - One"),
            "{label}: {texts:?}"
        );
    }
}

#[test]
fn the_root_view_draws_no_jump_file_panel_without_a_song_list_or_when_shaded() {
    let (mut player, _state) = player_in_songs();
    player.library_panel_visible = false;
    player.jump_file_visible = true;

    player.current_view = CurrentView::Genres;
    let capture = captured_root_view_text_bounds(&player);
    assert!(!capture.texts().any(|text| text == "JUMP TO FILE"));

    player.current_view = CurrentView::Songs;
    player.shaded = true;
    let capture = captured_root_view_text_bounds(&player);
    assert!(!capture.texts().any(|text| text == "JUMP TO FILE"));
}
