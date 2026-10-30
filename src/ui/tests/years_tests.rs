use super::flat_lists_tests::{
    assert_back_returns_to_artists, assert_group_opens_its_songs_and_back_unwinds,
    assert_loaded_stores_rows_with_song_count_labels, assert_navigation_clears_the_search,
    assert_show_fetches_the_list_and_switches_view, assert_stale_group_press_does_nothing,
    assert_view_footer_counts_and_totals,
};
use super::*;

/// Songs released in 2005, none, and 2001, in library order.
fn year_songs() -> Vec<Song> {
    let mut songs = stepping_songs();
    songs[0].year = Some(2005);
    songs[1].year = None;
    songs[2].year = Some(2005);
    songs
}

/// A player showing the Years list loaded from [`year_songs`]: `2005`
/// (songs 0 and 2), then `Unknown` (song 1).
fn player_with_years() -> WinampPlayer {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::ShowYears);
    let _ = update(
        &mut player,
        Message::YearsLoaded(group_by_year(year_songs())),
    );
    player
}

#[tokio::test]
async fn show_years_fetches_and_switches_view() {
    assert_show_fetches_the_list_and_switches_view(
        Message::ShowYears,
        "load library years",
        |player| &mut player.years.items,
        |year| year.name.as_str(),
        &["2001", "2003", "2005"],
        CurrentView::Years,
    )
    .await;
}

#[test]
fn years_loaded_stores_rows_with_song_count_labels() {
    let player = player_with_years();
    assert_loaded_stores_rows_with_song_count_labels(
        &player,
        |player| &player.years,
        &["2005", "Unknown"],
        &["2 songs - 5:05", "1 song - 2:05"],
    );
}

#[test]
fn show_years_clears_an_active_search() {
    assert_navigation_clears_the_search(Message::ShowYears, CurrentView::Years);
}

#[test]
fn back_from_years_returns_to_the_artists_list() {
    assert_back_returns_to_artists(CurrentView::Years);
}

#[test]
fn year_selected_shows_that_years_songs_and_back_returns_to_years() {
    let player = player_with_years();
    let epoch = player.years.epoch;
    assert_group_opens_its_songs_and_back_unwinds(
        player,
        epoch,
        |epoch, index| Message::YearSelected { epoch, index },
        CurrentView::Years,
        SongsOrigin::Year,
    );
}

#[test]
fn a_stale_year_press_does_nothing() {
    let player = player_with_years();
    let epoch = player.years.epoch;
    assert_stale_group_press_does_nothing(
        player,
        epoch,
        |epoch, index| Message::YearSelected { epoch, index },
        CurrentView::Years,
    );
}

#[test]
fn years_view_footer_counts_years_and_totals_their_songs() {
    let player = player_with_years();
    assert_view_footer_counts_and_totals(&player, CurrentView::Years, "2 years - 7:10");
}
