use super::flat_lists_tests::{
    assert_back_returns_to_artists, assert_navigation_clears_the_search,
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
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));

    let task = update(&mut player, Message::ShowComposers);
    drive_fetch_and_assert_loaded(
        &mut player,
        task,
        "load library composers",
        |player| &mut player.composers.items,
        |composer| composer.name.as_str(),
        &["Sam Ample", "Unknown"],
    )
    .await;

    assert_view(&player, CurrentView::Composers);
    assert!(!player.search_active);
    assert_eq!(player.songs.items.len(), 3);
}

#[test]
fn composers_loaded_stores_rows_with_song_count_labels() {
    let player = player_with_composers();

    let names: Vec<&str> = player
        .composers
        .items
        .iter()
        .map(|composer| composer.name.as_str())
        .collect();
    assert_eq!(names, ["Zed", "Unknown"]);
    assert_eq!(player.composers.labels, ["2 songs - 5:05", "1 song - 2:05"]);
    assert!(!player.composers.loading);
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
    let mut player = player_with_composers();
    let epoch = player.composers.epoch;

    let _ = update(&mut player, Message::ComposerSelected { epoch, index: 0 });

    assert_view(&player, CurrentView::Songs);
    assert_eq!(player.songs_origin, SongsOrigin::Composer);
    let ids: Vec<&str> = player
        .songs
        .items
        .iter()
        .map(|song| song.id.as_str())
        .collect();
    assert_eq!(ids, ["song-1", "song-3"]);

    let _ = update(&mut player, Message::Back);
    assert_view(&player, CurrentView::Composers);
    let _ = update(&mut player, Message::Back);
    assert_view(&player, CurrentView::Artists);
}

#[test]
fn a_stale_composer_press_does_nothing() {
    let mut player = player_with_composers();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    let epoch = player.composers.epoch;

    assert_message_schedules_no_work(
        &mut player,
        Message::ComposerSelected {
            epoch: epoch - 1,
            index: 0,
        },
    );
    assert_message_schedules_no_work(&mut player, Message::ComposerSelected { epoch, index: 9 });

    assert_view(&player, CurrentView::Composers);
    assert_eq!(player.songs.items.len(), 3);
}

#[test]
fn a_superseded_composers_load_is_dropped() {
    let (mut player, _state) = test_player();
    let stale = player.composers.begin_fetch();
    let _ = player.composers.begin_fetch();

    let reply = Message::BrowseReply {
        issued: stale.issued(),
        reply: Box::new(Message::ComposersLoaded(
            group_by_composer(composer_songs()),
        )),
    };
    let _ = update(&mut player, reply);

    assert!(player.composers.items.is_empty());
}
