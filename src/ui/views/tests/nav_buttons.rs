use super::super::{
    view_composers_button, view_genres_button, view_recently_added_button,
    view_recently_played_button, view_years_button,
};
use super::*;

// The browse toolbar, sidebar, and Clear buttons are iced `Button`s whose
// pressed message is opaque once built: only a headless click recovers which
// `Message` a press publishes. A miswired button — Sort opening the save-list
// panel, Genres stepping the hierarchy — would pass every construct-only and
// placement guard, so each button's label and its one published message are
// pinned here the way the title bar's M button is in `chrome.rs`.

/// One (label, button view, expected-press-message p) check.
type ButtonCheck = (
    &'static str,
    Element<'static, Message>,
    fn(&Message) -> bool,
);

fn presses_publish_one_message(
    label: &'static str,
    view: Element<'static, Message>,
    expected: fn(&Message) -> bool,
) {
    let mut renderer = layout_capture::headless_renderer();
    let mut element = view;
    let capture = layout_capture::capture_text(&mut element, &renderer);
    let label_bounds = capture.find(label);
    let messages = layout_capture::click_at(
        element,
        &mut renderer,
        iced::Point::new(label_bounds.center_x(), label_bounds.center_y()),
    );
    assert_eq!(
        1,
        messages.iter().filter(|message| expected(message)).count(),
        "a press on {label} must publish exactly one expected message: {messages:?}"
    );
}

#[test]
fn every_browse_toolbar_button_publishes_its_message() {
    let checks: Vec<ButtonCheck> = vec![
        ("Back", view_back_button(), |m| matches!(m, Message::Back)),
        ("Top", view_top_button(), |m| {
            matches!(m, Message::ScrollBrowseToTop)
        }),
        ("Sort", view_sort_button(), |m| {
            matches!(m, Message::SortBrowse)
        }),
        ("Save list", view_save_list_button(), |m| {
            matches!(m, Message::ToggleSaveList)
        }),
        ("Reload", view_reload_button(), |m| {
            matches!(m, Message::ReloadLibrary)
        }),
        ("Open", view_open_button(), |m| {
            matches!(m, Message::OpenSelected)
        }),
        ("Retry", view_retry_button(), |m| {
            matches!(m, Message::LoadArtists)
        }),
        ("Clear", view_clear_search_button(), |m| {
            matches!(m, Message::ClearSearch)
        }),
    ];
    for (label, view, expected) in checks {
        presses_publish_one_message(label, view, expected);
    }
}

#[test]
fn every_artists_sidebar_button_publishes_its_message() {
    let checks: Vec<ButtonCheck> = vec![
        ("All Songs", view_all_songs_button(), |m| {
            matches!(m, Message::ShowAllSongs)
        }),
        ("All Albums", view_all_albums_button(), |m| {
            matches!(m, Message::ShowAllAlbums)
        }),
        ("Playlists", view_playlists_button(), |m| {
            matches!(m, Message::ShowPlaylists)
        }),
        ("Genres", view_genres_button(), |m| {
            matches!(m, Message::ShowGenres)
        }),
        ("Years", view_years_button(), |m| {
            matches!(m, Message::ShowYears)
        }),
        ("Composers", view_composers_button(), |m| {
            matches!(m, Message::ShowComposers)
        }),
        ("Recently Added", view_recently_added_button(), |m| {
            matches!(m, Message::ShowRecentlyAdded)
        }),
        ("Recently Played", view_recently_played_button(), |m| {
            matches!(m, Message::ShowRecentlyPlayed)
        }),
    ];
    for (label, view, expected) in checks {
        presses_publish_one_message(label, view, expected);
    }
}
