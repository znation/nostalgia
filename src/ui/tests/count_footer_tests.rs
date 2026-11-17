use super::*;

// The Artists footer adds the total of the rows' album counts after the
// artist count. The four count footer tests below differ only in which browse
// list they seed and which footer they expect, so they share
// `assert_root_view_draws_the_count_footer`.
/// Builds a fresh player, lets `setup` seed one browse list (items, the
/// cleared loading flag, `current_view`, and any row selection — selecting the
/// list's epoch before capturing) and then captures the laid-out root view,
/// asserting it draws exactly `footer`. Used by the four `root_view_*_footer`
/// tests; the flat Genres/Years/Composers footers are pinned by
/// `flat_lists_tests::assert_view_footer_counts_and_totals`, and the empty
/// levels' missing footers by `an_empty_browse_level_draws_no_count_footer`.
fn assert_root_view_draws_the_count_footer(setup: impl FnOnce(&mut WinampPlayer), footer: &str) {
    let (mut player, _state) = test_player();
    setup(&mut player);
    let capture = captured_root_view_text_bounds(&player);
    assert!(
        capture.texts().any(|text| text == footer),
        "{footer:?} missing from the laid-out root: {:?}",
        capture.fragments()
    );
}

#[test]
fn root_view_artists_footer_totals_the_album_counts() {
    assert_root_view_draws_the_count_footer(
        |player| {
            let mut artist = sample_artist();
            artist.album_count = 2;
            player.artists.items = vec![artist];
            player.artists.loading = false;
            player.current_view = CurrentView::Artists;
        },
        "1 artist - 2 albums",
    );
}

#[test]
fn root_view_artists_footer_shows_the_selected_album_count() {
    assert_root_view_draws_the_count_footer(
        |player| {
            let artists = [2, 3].map(|album_count| {
                let mut artist = sample_artist();
                artist.album_count = album_count;
                artist
            });
            player.artists.items = artists.to_vec();
            player.artists.loading = false;
            let epoch = player.artists.epoch;
            player.artists.select_row(epoch, 1);
            player.current_view = CurrentView::Artists;
        },
        "2 artists - 3/5 albums",
    );
}

#[test]
fn root_view_genres_footer_shows_the_selected_group_time() {
    assert_root_view_draws_the_count_footer(
        |player| {
            let group = |name: &str, duration_ms| crate::ui::song_groups::SongGroup {
                name: name.to_string(),
                songs: vec![crate::library::Song {
                    duration_ms,
                    ..sample_song()
                }],
            };
            player.genres.items = vec![group("Rock", 210_000), group("Jazz", 120_000)];
            player.genres.loading = false;
            let epoch = player.genres.epoch;
            player.genres.select_row(epoch, 0);
            player.current_view = CurrentView::Genres;
        },
        "2 genres - 3:30/5:30",
    );
}

#[test]
fn root_view_album_footer_shows_the_selected_running_time() {
    assert_root_view_draws_the_count_footer(
        |player| {
            let mut first = sample_album();
            first.duration_ms = 630_000;
            let mut second = sample_album();
            second.duration_ms = 210_000;
            player.all_albums.items = vec![first, second];
            player.all_albums.loading = false;
            let epoch = player.all_albums.epoch;
            player.all_albums.select_row(epoch, 1);
            player.current_view = CurrentView::AllAlbums;
        },
        "2 albums - 3:30/14:00",
    );
}

// A browse list with no rows shows only its empty wording: `list_with_count`
// returns the list unchanged at a zero count, and `song_list` skips its
// count-and-time footer, so the loading, error, or empty placeholder stays
// alone. A footer such as "0 artists" or "0 songs - 0:00" would describe rows
// the panel does not have, so pin its absence at every level along with the
// wording that must remain.
#[test]
fn an_empty_browse_level_draws_no_count_footer() {
    let (mut player, _state) = test_player();
    // A fresh player is empty but has the boot-time artists fetch in flight;
    // mark every list loaded so each level shows its empty wording.
    player.artists.loading = false;
    player.albums.loading = false;
    player.songs.loading = false;
    player.all_songs.loading = false;
    player.all_albums.loading = false;
    player.playlists.loading = false;

    let cases = [
        (CurrentView::Artists, "No artists", "0 artists"),
        (CurrentView::Albums, "No albums", "0 albums - 0:00"),
        (CurrentView::Songs, "No songs", "0 songs - 0:00"),
        (CurrentView::AllSongs, "No songs", "0 songs - 0:00"),
        (CurrentView::AllAlbums, "No albums", "0 albums - 0:00"),
        (CurrentView::Playlists, "No playlists", "0 playlists"),
    ];

    for (view, empty_label, footer) in cases {
        let level = format!("{view:?}");
        player.current_view = view;
        let capture = captured_root_view_text_bounds(&player);
        let texts: Vec<&str> = capture.texts().collect();
        assert!(
            texts.contains(&empty_label),
            "{level} must draw its empty wording {empty_label:?}: {:?}",
            capture.fragments()
        );
        assert!(
            !texts.contains(&footer),
            "{level} must not draw a zero-count footer {footer:?}: {:?}",
            capture.fragments()
        );
    }
}
