use super::flat_lists_tests::{
    assert_back_returns_to_artists, assert_group_opens_its_songs_and_back_unwinds,
    assert_loaded_stores_rows_with_song_count_labels, assert_navigation_clears_the_search,
    assert_show_fetches_the_list_and_switches_view, assert_stale_group_press_does_nothing,
    assert_view_footer_counts_and_totals,
};
use super::*;

/// Songs by Zed, nobody, and Zed again, in library order.
fn composer_songs() -> Vec<Song> {
    let mut songs = stepping_songs();
    songs[0].composer = "Zed".to_string();
    songs[1].composer = String::new();
    songs[2].composer = " zed ".to_string();
    songs
}

/// A player showing the Composers list loaded from [`composer_songs`]: `Zed`
/// (songs 0 and 2), then `Unknown` (song 1).
fn player_with_composers() -> WinampPlayer {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::ShowComposers);
    let _ = update(
        &mut player,
        Message::ComposersLoaded(group_by_composer(composer_songs())),
    );
    player
}

#[tokio::test]
async fn show_composers_fetches_and_switches_view() {
    assert_show_fetches_the_list_and_switches_view(
        Message::ShowComposers,
        "load library composers",
        |player| &mut player.composers.items,
        |composer| composer.name.as_str(),
        &["Sam Ample", "Unknown"],
        CurrentView::Composers,
    )
    .await;
}

#[test]
fn composers_loaded_stores_rows_with_song_count_labels() {
    let player = player_with_composers();
    assert_loaded_stores_rows_with_song_count_labels(
        &player,
        |player| &player.composers,
        &["Zed", "Unknown"],
        &["2 songs - 5:05", "1 song - 2:05"],
    );
}

#[test]
fn show_composers_clears_an_active_search() {
    assert_navigation_clears_the_search(Message::ShowComposers, CurrentView::Composers);
}

#[test]
fn back_from_composers_returns_to_the_artists_list() {
    assert_back_returns_to_artists(CurrentView::Composers);
}

#[test]
fn composer_selected_shows_that_composers_songs_and_back_returns_to_composers() {
    let player = player_with_composers();
    let epoch = player.composers.epoch;
    assert_group_opens_its_songs_and_back_unwinds(
        player,
        epoch,
        |epoch, index| Message::ComposerSelected { epoch, index },
        CurrentView::Composers,
        SongsOrigin::Composer,
    );
}

#[test]
fn a_stale_composer_press_does_nothing() {
    let player = player_with_composers();
    let epoch = player.composers.epoch;
    assert_stale_group_press_does_nothing(
        player,
        epoch,
        |epoch, index| Message::ComposerSelected { epoch, index },
        CurrentView::Composers,
    );
}

#[test]
fn composers_view_footer_counts_composers_and_totals_their_songs() {
    let player = player_with_composers();
    assert_view_footer_counts_and_totals(&player, CurrentView::Composers, "2 composers - 7:10");
}
