use super::flat_lists_tests::{
    assert_back_returns_to_artists, assert_navigation_clears_the_search,
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
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));

    let task = update(&mut player, Message::ShowYears);
    drive_fetch_and_assert_loaded(
        &mut player,
        task,
        "load library years",
        |player| &mut player.years.items,
        |year| year.name.as_str(),
        &["2001", "2003", "2005"],
    )
    .await;

    assert_view(&player, CurrentView::Years);
    assert!(!player.search_active);
    assert_eq!(player.songs.items.len(), 3);
}

#[test]
fn years_loaded_stores_rows_with_song_count_labels() {
    let player = player_with_years();

    let names: Vec<&str> = player
        .years
        .items
        .iter()
        .map(|year| year.name.as_str())
        .collect();
    assert_eq!(names, ["2005", "Unknown"]);
    assert_eq!(player.years.labels, ["2 songs", "1 song"]);
    assert!(!player.years.loading);
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
    let mut player = player_with_years();
    let epoch = player.years.epoch;

    let _ = update(&mut player, Message::YearSelected { epoch, index: 0 });

    assert_view(&player, CurrentView::Songs);
    assert_eq!(player.songs_origin, SongsOrigin::Year);
    let ids: Vec<&str> = player
        .songs
        .items
        .iter()
        .map(|song| song.id.as_str())
        .collect();
    assert_eq!(ids, ["song-1", "song-3"]);

    let _ = update(&mut player, Message::Back);
    assert_view(&player, CurrentView::Years);
    let _ = update(&mut player, Message::Back);
    assert_view(&player, CurrentView::Artists);
}

#[test]
fn a_stale_year_press_does_nothing() {
    let mut player = player_with_years();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    let epoch = player.years.epoch;

    assert_message_schedules_no_work(
        &mut player,
        Message::YearSelected {
            epoch: epoch - 1,
            index: 0,
        },
    );
    assert_message_schedules_no_work(&mut player, Message::YearSelected { epoch, index: 9 });

    assert_view(&player, CurrentView::Years);
    assert_eq!(player.songs.items.len(), 3);
}

#[test]
fn a_superseded_years_load_is_dropped() {
    let (mut player, _state) = test_player();
    let stale = player.years.begin_fetch();
    let _ = player.years.begin_fetch();

    let reply = Message::BrowseReply {
        issued: stale.issued(),
        reply: Box::new(Message::YearsLoaded(group_by_year(year_songs()))),
    };
    let _ = update(&mut player, reply);

    assert!(player.years.items.is_empty());
}
