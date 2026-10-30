use super::*;

// Each row maps one library entry to the (title, secondary label,
// current-track flag) tuple that `scrollable_list` renders as a button. Wrong
// label text would silently break the browse UI, so the mapping is the
// contract these tests pin down. The press message is the list's, built from
// the row position and list epoch (see `scrollable_list`), and the flag is
// pinned too, since it drives the `▶` prefix and white playing text.

#[test]
fn artist_row_uses_name_and_album_count() {
    let mut artist = sample_artist();
    artist.album_count = 2;
    let (title, label, is_current) = artist_row(&artist, None);
    assert_eq!(title, "The Sample Band");
    assert_eq!(label, "2 albums");
    assert!(!is_current);
}

#[test]
fn artist_row_prefers_the_lists_cached_album_count() {
    // The artists list caches each row's label at store time; a refresh must
    // borrow it instead of formatting the count again. A stale cache would
    // show the wrong count, so the cached text wins over `album_count`.
    let mut artist = sample_artist();
    artist.album_count = 2;
    let (title, label, is_current) = artist_row(&artist, Some("7 albums"));
    assert_eq!(title, "The Sample Band");
    assert_eq!(label, "7 albums");
    assert!(!is_current);
}

// An overlong title must stay inside its own column instead of drawing
// across the spacer into the right-aligned label. The shared list puts the
// title in a clipping container, so lay it out with one very long unbroken
// title, draw it, and read the pixels in the spacer gap: the container clips
// the title there, leaving only the dark well behind.
#[test]
fn scrollable_list_clips_an_overlong_title_to_its_column() {
    let long_title = "W".repeat(400);
    let mut element = scrollable_list(
        [(
            Cow::Borrowed(long_title.as_str()),
            Cow::Borrowed("3:30"),
            false,
        )],
        &CurrentView::Songs,
        0,
        None,
        "No songs",
    );
    let mut renderer = layout_capture::headless_renderer();
    let (capture, pixels) =
        layout_capture::capture_text_and_pixels(&mut element, &mut renderer, &iced::Theme::Dark);
    let title = capture.find(&long_title);
    let label = capture.find("3:30");
    let gap = Rectangle {
        x: label.x - 9.0,
        y: title.y,
        width: 8.0,
        height: title.height,
    };
    let green = pixels.max_green_in(gap);
    assert!(
        green < 64,
        "the title must be clipped at its column, not drawn into the label's gap: {title:?} vs {label:?} (green {green})"
    );
}

// A playback poll rebuilds the whole view, so an unchanged browse list must
// reuse each row's cached button instead of constructing it again. Build the
// real list twice, diff the second element into the first tree, and count
// `row_button` calls: no new calls on an identical list, exactly the row a
// selection change marks, and every row when the list epoch changes. A text
// change rides an epoch bump, because `BrowseList` bumps the epoch whenever it
// stores or reorders the cached titles and labels; the key therefore need not
// rescan the text on every refresh.
#[test]
fn scrollable_list_reuses_unchanged_rows_and_rebuilds_only_changed_keys() {
    fn items(
        first_title: &'static str,
        first_label: &'static str,
    ) -> [(Cow<'static, str>, Cow<'static, str>, bool); 2] {
        [
            (
                Cow::Borrowed(first_title),
                Cow::Borrowed(first_label),
                false,
            ),
            (Cow::Borrowed("Second"), Cow::Borrowed("4:00"), true),
        ]
    }

    let before = row_build_count();
    let first = scrollable_list(
        items("First", "3:30"),
        &CurrentView::Songs,
        7,
        None,
        "No songs",
    );
    let mut tree = Tree::new(first.as_widget());
    assert_eq!(
        row_build_count() - before,
        2,
        "the first view builds each browse row once"
    );

    let same = scrollable_list(
        items("First", "3:30"),
        &CurrentView::Songs,
        7,
        None,
        "No songs",
    );
    tree.diff(same.as_widget());
    assert_eq!(
        row_build_count() - before,
        2,
        "an unchanged list and selection reuses every row"
    );

    let selected = scrollable_list(
        items("First", "3:30"),
        &CurrentView::Songs,
        7,
        Some(1),
        "No songs",
    );
    tree.diff(selected.as_widget());
    assert_eq!(
        row_build_count() - before,
        3,
        "changing the selection rebuilds exactly the row it marks"
    );

    let renamed = scrollable_list(
        items("Renamed", "3:31"),
        &CurrentView::Songs,
        8,
        Some(1),
        "No songs",
    );
    tree.diff(renamed.as_widget());
    assert_eq!(
        row_build_count() - before,
        5,
        "a new list epoch rebuilds every row, including new cached text"
    );

    let reloaded = scrollable_list(
        items("Renamed", "3:31"),
        &CurrentView::Songs,
        9,
        Some(1),
        "No songs",
    );
    tree.diff(reloaded.as_widget());
    assert_eq!(
        row_build_count() - before,
        7,
        "a new list epoch rebuilds every row"
    );
}

#[test]
fn album_count_label_singular_plural_and_none() {
    assert_eq!(album_count_label(0), "");
    assert_eq!(album_count_label(1), "1 album");
    assert_eq!(album_count_label(2), "2 albums");
}

#[test]
fn switching_lists_at_the_same_epoch_rebuilds_cached_rows() {
    let before = row_build_count();
    let album = sample_album();
    let titles = vec!["Shared title".to_string()];
    let labels = vec!["3:30".to_string()];
    let albums = [album];
    let first = view_albums(&albums, &titles, &labels, 1, None, false, None);
    let mut tree = Tree::new(first.as_widget());
    assert_eq!(row_build_count() - before, 1);

    // Identical text must still invalidate the old list's cache. This also
    // fails with the original text-hashing key, which lacked list identity.
    let flat = view_all_albums(&albums, &titles, &labels, 1, None, false, None);
    tree.diff(flat.as_widget());
    assert_eq!(row_build_count() - before, 2);

    let renamed = vec!["Different title".to_string()];
    let relabeled = vec!["4:00".to_string()];
    let back = view_albums(&albums, &renamed, &relabeled, 1, None, false, None);
    tree.diff(back.as_widget());
    assert_eq!(row_build_count() - before, 3);
    tree.diff(back.as_widget());
    assert_eq!(
        row_build_count() - before,
        3,
        "the same list reuses its row"
    );
}

// The mockup puts the album count at the artist row's right edge, not
// directly after the name. Lay the real list out and require the count to
// reach the right of the row while the name stays on the left.
#[test]
fn artist_row_puts_its_album_count_at_the_right_edge() {
    let mut artist = sample_artist();
    artist.album_count = 2;
    let mut element = scrollable_list(
        [artist_row(&artist, None)],
        &CurrentView::Artists,
        0,
        None,
        "No artists",
    );
    let capture = layout_capture::capture_text(&mut element, &layout_capture::headless_renderer());

    let title = capture.find("The Sample Band");
    let count = capture.find("2 albums");
    assert!(
        count.x > title.x + title.width,
        "the count must follow the name: {title:?}, {count:?}"
    );
    assert!(
        capture.size().width - (count.x + count.width) < 40.0,
        "the count must sit at the right edge: {count:?} in {:?}",
        capture.size()
    );
}

#[test]
fn album_row_uses_title_and_duration() {
    let mut album = sample_album();
    album.duration_ms = 630_000;
    let (title, label, is_current) = album_row(&album, None, None);
    assert_eq!(title, "The Sample Band - First Record");
    assert_eq!(label, "10:30");
    assert!(!is_current);
}

#[test]
fn album_row_shows_the_release_year_before_the_running_time() {
    let mut album = sample_album();
    album.year = Some(2001);
    album.duration_ms = 630_000;
    assert_eq!(album_row(&album, None, None).1, "2001  10:30");
    album.duration_ms = 0;
    assert_eq!(album_row(&album, None, None).1, "2001  --:--");
}

#[test]
fn album_row_prefers_the_lists_cached_title_and_running_time() {
    // The loaded album list caches each row's display title and formatted
    // running time, and the row builder must borrow them rather than joining
    // the artist and title and formatting the duration again. Cached values
    // that differ from the album's fields prove the row uses the cache; the
    // `None` tests pin the fallback.
    let album = sample_album();
    let (title, label, _) = album_row(&album, Some("Cached - Title"), Some("1:23"));
    assert_eq!(title, "Cached - Title");
    assert_eq!(label, "1:23");
}

// An album with no artist has nothing to join, so the row must not render a
// dangling `" - "` prefix; it shows the bare title and borrows it.
#[test]
fn album_row_shows_the_title_alone_when_the_artist_is_empty() {
    let mut album = sample_album();
    album.artist = String::new();
    album.duration_ms = 95_000;
    let (title, label, is_current) = album_row(&album, None, None);
    assert_eq!(title, "First Record");
    assert!(matches!(title, std::borrow::Cow::Borrowed(_)));
    assert_eq!(label, "1:35");
    assert!(!is_current);
}

// An album the source reported no songs for has no known running time, so the
// row must show the same `--:--` placeholder a song with no duration does
// rather than a misleading `0:00`.
#[test]
fn album_row_shows_a_placeholder_when_the_duration_is_unknown() {
    let album = sample_album();
    let (_, label, is_current) = album_row(&album, None, None);
    assert_eq!(label, "--:--");
    assert!(!is_current);
}

#[test]
fn playlist_row_uses_name_and_the_view_songs_hint() {
    let playlist = sample_playlist();
    let (title, label, is_current) = playlist_row(&playlist);
    assert_eq!(title, "Morning Drive");
    assert_eq!(label, "View Songs");
    assert!(!is_current);
}

#[test]
fn song_row_uses_title_and_length() {
    // Without a cached title the row composes the display title itself, so a
    // list stored without caches still reads `<artist> - <title>`.
    let song = sample_song();
    let (title, label, is_current) = song_row(&song, None, None, None);
    assert_eq!(title, "The Sample Band - Opening");
    // `sample_song` is 210_000 ms long, so the secondary label is its
    // `m:ss` length rather than the old static "Play" hint.
    assert_eq!(label, "3:30");
    assert!(!is_current);
}

#[test]
fn song_row_prefers_the_lists_cached_title() {
    // The loaded song list caches each row's display title, and the row
    // builder must use it rather than composing the artist and title again.
    // A cached value that differs from the song's fields proves the row
    // borrows the cache; the `None` tests pin the fallback.
    let song = sample_song();
    let (title, _, _) = song_row(&song, Some("Cached - Title"), None, None);
    assert_eq!(title, "Cached - Title");
}

#[test]
fn song_row_uses_the_title_alone_when_the_artist_is_empty() {
    // An artist-less song has nothing to join, so the row must not render a
    // dangling `" - "` prefix.
    let mut song = sample_song();
    song.artist = String::new();
    let (title, _, _) = song_row(&song, None, None, None);
    assert_eq!(title, "Opening");
}

#[test]
fn song_row_prefers_the_lists_cached_length_label() {
    // The loaded song list caches each row's formatted length, and the row
    // builder must use it rather than formatting the duration again. A
    // cached label that differs from the duration's formatting proves the
    // row borrows the cache; the `None` tests above pin the fallback.
    let song = sample_song();
    let (_, label, _) = song_row(&song, None, Some("1:23"), None);
    assert_eq!(label, "1:23");
}

#[test]
fn song_row_shows_an_unknown_duration_as_blank_time() {
    // A song whose source supplied no duration (`0`) renders the same
    // `--:--` placeholder as the Now Playing bar, not `0:00`.
    let mut song = sample_song();
    song.duration_ms = 0;
    let (_, label, _) = song_row(&song, None, None, None);
    assert_eq!(label, "--:--");
}

#[test]
fn song_row_marks_the_current_track() {
    // The flag is true only when `current_track` names exactly this
    // song; a different id — or no track at all — leaves the row
    // unmarked, so the `▶` prefix and white text follow the playback state.
    let (_, _, is_current) = song_row(&sample_song(), None, None, Some("song-1"));
    assert!(is_current);

    let (_, _, is_current) = song_row(&sample_song(), None, None, Some("other-song"));
    assert!(!is_current);

    let (_, _, is_current) = song_row(&sample_song(), None, None, None);
    assert!(!is_current);
}

// The Back button is a browse-navigation control: it must exist only
// where there is a level above to return to, so the visibility predicate
// is pinned alongside the other pure browse helpers. Every view below
// Artists must offer Back, including the flat lists and Playlists: the
// update arms route their Back presses, but a hidden button would leave the
// user with no way to press it. Drive the shared view list so a newly added
// level is covered here too.
#[test]
fn back_button_is_available_only_below_the_artist_list() {
    for view in BROWSE_VIEWS {
        assert_eq!(
            can_go_back(&view),
            view != CurrentView::Artists,
            "Back visibility for {view:?}"
        );
    }
}

// Retry exists only for the one failed fetch navigation cannot re-issue:
// the top-level artists fetch. The predicate is pinned alongside the other
// pure browse helpers.
#[test]
fn retry_button_is_available_only_for_a_failed_artists_fetch() {
    assert!(can_retry_artists(&CurrentView::Artists, Some("boom")));
    assert!(!can_retry_artists(&CurrentView::Artists, None));
    assert!(!can_retry_artists(&CurrentView::Albums, Some("boom")));
    assert!(!can_retry_artists(&CurrentView::Songs, Some("boom")));
}

// The Now Playing bar and transport controls are built in this module
// (grouped with the other widget builders), so their pure label logic is
// pinned here alongside the browse-row mappings.

#[test]
fn list_count_summary_uses_the_singular_and_plural() {
    assert_eq!(list_count_summary(1, "artist", "artists"), "1 artist");
    assert_eq!(list_count_summary(3, "artist", "artists"), "3 artists");
    assert_eq!(list_count_summary(1, "playlist", "playlists"), "1 playlist");
    assert_eq!(list_count_summary(0, "album", "albums"), "0 albums");
}

#[test]
fn browse_views_construct_over_the_loaded_library() {
    // The populated branch of `scrollable_list`, built from the real
    // sample library so each row maps actual titles and selection
    // messages into buttons. Rendered both with no current track (no
    // playing row) and with one set (the `▶` prefix and white-text style
    // build).
    let library = sample_library();
    construct_every_browse_view(
        &library.artists,
        &library.albums_by_artist["artist-1"],
        &library.songs_by_album["album-1"],
        &library.playlists,
        1,
        false,
    );
}

#[test]
fn browse_views_construct_over_an_empty_list() {
    // Every browse view renders its pre-load state — an empty buffer —
    // before the first fetch lands, so `scrollable_list` must build a
    // scrollable over zero rows, with or without a current track set.
    construct_every_browse_view(&[], &[], &[], &[], 0, true);
}

/// Builds every browse view over `artists`, `albums`, `songs`, and
/// `playlists`, each with the given `epoch`, `loading`, and no error: the
/// Artists, Albums, All Albums, and Playlists views, and the Songs and All
/// Songs views with no current track, with `song-1` marked as playing, and
/// (for Songs) with `search_active` set. iced `Element`s
/// expose no tree introspection, so the only observable contract is that each
/// builder constructs without panicking; the two tests above call this over
/// the loaded sample library and over the pre-load empty buffers.
fn construct_every_browse_view(
    artists: &[Artist],
    albums: &[Album],
    songs: &[Song],
    playlists: &[Playlist],
    epoch: u64,
    loading: bool,
) {
    let _artists = view_artists(artists, &[], epoch, None, loading, None);
    let _albums = view_albums(albums, &[], &[], epoch, None, loading, None);
    let _songs = view_songs(songs, &[], &[], epoch, None, loading, None, None, false);
    let _songs_marked = view_songs(
        songs,
        &[],
        &[],
        epoch,
        None,
        loading,
        None,
        Some("song-1"),
        false,
    );
    let _songs_selected = view_songs(songs, &[], &[], epoch, Some(0), loading, None, None, false);
    let _songs_searched = view_songs(songs, &[], &[], epoch, None, loading, None, None, true);
    let _all_songs = view_all_songs(songs, &[], &[], epoch, None, loading, None, None);
    let _all_songs_marked =
        view_all_songs(songs, &[], &[], epoch, None, loading, None, Some("song-1"));
    let _all_albums = view_all_albums(albums, &[], &[], epoch, None, loading, None);
    let _playlists = view_playlists(playlists, epoch, None, loading, None);
}

// The footer beneath a loaded song list must report the exact count and total
// it was given, including a zero-duration entry as a song with no time, and
// must keep counting minutes past an hour instead of formatting an hour field.
fn summary_song(duration_ms: u64) -> Song {
    let mut song = sample_song();
    song.duration_ms = duration_ms;
    song
}

#[test]
fn song_list_summary_puts_the_selected_length_before_the_total() {
    let songs = [summary_song(210_000), summary_song(45_000)];
    assert_eq!(song_list_summary(&songs, Some(1)), "2 songs - 0:45/4:15");
    assert_eq!(song_list_summary(&songs, Some(0)), "2 songs - 3:30/4:15");
}

#[test]
fn song_list_summary_marks_an_unknown_selected_length() {
    let songs = [summary_song(210_000), summary_song(0)];
    assert_eq!(song_list_summary(&songs, Some(1)), "2 songs - 0:00+/3:30+");
    assert_eq!(song_list_summary(&songs, Some(0)), "2 songs - 3:30/3:30+");
}

#[test]
fn song_list_summary_ignores_a_selection_past_the_list() {
    assert_eq!(
        song_list_summary(&[summary_song(210_000)], Some(1)),
        "1 song - 3:30"
    );
    assert_eq!(song_list_summary(&[], Some(0)), "0 songs - 0:00");
}

#[test]
fn song_list_summary_counts_songs_and_sums_durations() {
    assert_eq!(song_list_summary(&[], None), "0 songs - 0:00");
    let songs = [summary_song(210_000), summary_song(45_000)];
    assert_eq!(song_list_summary(&songs, None), "2 songs - 4:15");
}

#[test]
fn song_list_summary_uses_the_singular_for_one_song() {
    let songs = [summary_song(210_000)];
    assert_eq!(song_list_summary(&songs, None), "1 song - 3:30");
}

#[test]
fn song_list_summary_marks_a_total_that_skips_unknown_durations() {
    let songs = [summary_song(210_000), summary_song(0)];
    assert_eq!(song_list_summary(&songs, None), "2 songs - 3:30+");
    assert_eq!(
        song_list_summary(&[summary_song(0)], None),
        "1 song - 0:00+"
    );
}

#[test]
fn song_list_summary_keeps_counting_minutes_past_an_hour() {
    let songs = [summary_song(3_665_000)];
    assert_eq!(song_list_summary(&songs, None), "1 song - 61:05");
}

// The total is summed with `saturating_add`, so a payload carrying a duration
// near the u64 ceiling pins the total at u64::MAX instead of wrapping back to
// a small time (a plain `+` would wrap in release and panic in debug).
#[test]
fn song_list_summary_saturates_a_total_past_the_u64_ceiling() {
    let songs = [summary_song(u64::MAX), summary_song(1)];
    assert_eq!(
        song_list_summary(&songs, None),
        "2 songs - 307445734561825:51"
    );
}

// The footer's time is whole seconds, so a sub-second total truncates rather
// than rounding: a 999 ms list reads 0:00 and 1_999 ms reads 0:01.
#[test]
fn song_list_summary_truncates_a_sub_second_total() {
    assert_eq!(
        song_list_summary(&[summary_song(999)], None),
        "1 song - 0:00"
    );
    assert_eq!(
        song_list_summary(&[summary_song(1_999)], None),
        "1 song - 0:01"
    );
}

// The playing row is marked by its white text, not a selection bar; the
// selected row draws the row highlight. iced's `Element` API exposes no way
// to read a built `Button`'s style, and the row-style closure only runs at
// render time, so building the row never executes the override.
// `browse_row_style` is the pure seam that pins it: every hover/press state
// keeps the unmarked row's face ladder, so a resting playing row has no bar
// and the cursor still lifts and sinks it, while the selected row keeps the
// highlight background instead.
#[test]
fn browse_row_style_keeps_playing_rows_unbarred_with_white_text() {
    use iced::widget::button::Status;

    for status in [
        Status::Active,
        Status::Hovered,
        Status::Pressed,
        Status::Disabled,
    ] {
        let playing = browse_row_style(status, true, false);
        assert_eq!(
            playing.background,
            style::playlist_row_style(status).background
        );
        assert_eq!(playing.text_color, theme::PLAYING_ROW_TEXT);
        assert_ne!(
            playing.text_color,
            style::playlist_row_style(status).text_color
        );
    }
}

// Only the selected row gets the blue bar, in every hover/press state and
// whether or not it is also playing; its text is white. An ordinary row keeps
// the base background and green text, so the bar cannot bleed onto its
// neighbors.
#[test]
fn browse_row_style_paints_only_the_selected_row_blue() {
    use iced::widget::button::Status;

    for status in [
        Status::Active,
        Status::Hovered,
        Status::Pressed,
        Status::Disabled,
    ] {
        for is_current in [false, true] {
            let selected = browse_row_style(status, is_current, true);
            assert_eq!(
                selected.background,
                Some(iced::Background::Color(theme::ROW_HIGHLIGHT))
            );
            assert_eq!(selected.text_color, theme::PLAYING_ROW_TEXT);
        }

        let plain = browse_row_style(status, false, false);
        assert_eq!(
            plain.background,
            style::playlist_row_style(status).background
        );
        assert_eq!(plain.text_color, theme::LCD_GREEN);
    }

    // The bar must differ from the unmarked resting row, or the marker would
    // be invisible against the well.
    assert_ne!(
        browse_row_style(Status::Active, false, true).background,
        browse_row_style(Status::Active, false, false).background
    );
}

// Each browse level's loaded-but-empty buffer must render a label naming
// that list, not a blank panel, so the user has a clue what is missing.
// `browse_placeholder` chooses between this wording and "Loading…"; this
// pins the per-level empty wording `empty_list_label` hands back.
#[test]
fn empty_list_label_names_the_empty_browse_level() {
    assert_eq!(empty_list_label(&CurrentView::Artists), "No artists");
    assert_eq!(empty_list_label(&CurrentView::Albums), "No albums");
    assert_eq!(empty_list_label(&CurrentView::Songs), "No songs");
    assert_eq!(empty_list_label(&CurrentView::AllSongs), "No songs");
    assert_eq!(empty_list_label(&CurrentView::AllAlbums), "No albums");
    assert_eq!(empty_list_label(&CurrentView::Playlists), "No playlists");
}

// The placeholder distinguishes a list whose first reply has not landed
// yet from one that loaded empty: while `loading` is true the panel says
// it is loading instead of claiming the library is empty; once a reply
// lands (possibly an empty one) the per-level wording applies. The exact
// wording is pinned by `empty_list_label_names_the_empty_browse_level`,
// so here the loaded branch only pins that `browse_placeholder` delegates
// to `empty_list_label`.
#[test]
fn browse_placeholder_distinguishes_loading_from_an_empty_list() {
    for view in BROWSE_VIEWS {
        assert_eq!(browse_placeholder(&view, true, None, false), "Loading…");
        assert_eq!(
            browse_placeholder(&view, false, None, false),
            empty_list_label(&view)
        );
    }
}

// A loaded-empty Songs list that is showing a library search must not use
// the album-level "No songs" wording: that reads as "this album has no
// songs", blaming the album rather than reporting that the query matched
// nothing. The search wording applies only once the reply has landed, so a
// still-loading or failed search keeps its own message.
#[test]
fn browse_placeholder_names_an_empty_search() {
    assert_eq!(
        browse_placeholder(&CurrentView::Songs, false, None, true),
        "No songs match your search"
    );
    assert_eq!(
        browse_placeholder(&CurrentView::Songs, true, None, true),
        "Loading…"
    );
    let report = "music-library fetch failed (searching the library for \"zzz\"): boom";
    assert_eq!(
        browse_placeholder(&CurrentView::Songs, false, Some(report), true),
        report
    );
}

// A failed fetch must read as a failure, not as an empty library: when an
// error report is present it wins over both "Loading…" and the per-level
// empty wording, and the exact report (not a generic label) is shown so
// the panel names the failed query and the backend's cause.
#[test]
fn browse_placeholder_shows_the_fetch_error_instead_of_the_empty_wording() {
    let report = "music-library fetch failed (loading favorite artists): boom";

    // The error must win at every browse level, over both "Loading…"
    // (a fetch still in flight) and the per-level empty wording (a fetch
    // that already replied), so drive each view through both states.
    for view in BROWSE_VIEWS {
        for loading in [true, false] {
            assert_eq!(
                browse_placeholder(&view, loading, Some(report), false),
                report
            );
        }
    }
}

#[test]
fn now_playing_bar_and_browse_action_buttons_construct() {
    let _bar = view_now_playing(
        "00:00".into(),
        "Opening".into(),
        "The Sample Band".into(),
        "".into(),
        "3:30".into(),
        0,
        None,
        None,
        None,
        TransportMode::Stopped,
        true,
        None,
        IndicatorModes::default(),
    );
    let _bar = view_now_playing(
        "00:53".into(),
        "Opening".into(),
        "".into(),
        "".into(),
        "3:30".into(),
        4,
        Some(PlaybackFormat {
            sample_rate_hz: 44_100,
            channels: 2,
            bitrate_kbps: Some(256),
        }),
        Some(Spectrum {
            bands: [0.5; BAND_COUNT],
        }),
        None,
        TransportMode::Paused,
        true,
        None,
        IndicatorModes::default(),
    );
    let _overlong = view_now_playing(
        "03:12".into(),
        "A Title Much Longer Than The Display".into(),
        "The Sample Band".into(),
        "".into(),
        "3:30".into(),
        7,
        None,
        None,
        None,
        TransportMode::Playing,
        true,
        None,
        IndicatorModes::default(),
    );
    let _back = view_back_button();
    let _retry = view_retry_button();
    let _open = view_open_button();
    let _nav = view_browse_nav_buttons();
    let _reload = view_reload_button();
    let _clear = view_clear_search_button();
    let _search_row = view_search_row("Opening");
    let _all_songs = view_all_songs_button();
    let _all_albums = view_all_albums_button();
    let _playlists = view_playlists_button();
}

#[test]
fn browse_open_button_constructs() {
    let _open = view_open_button();
}

// The search box is an iced `TextInput`, whose placeholder and callbacks
// are opaque once built; like the other view-construction tests, this pins
// that the builder accepts a borrowed query — empty or filled — and
// returns an element without panicking.
#[test]
fn view_search_box_constructs() {
    let _empty = view_search_box("");
    let _filled = view_search_box("Opening");
}

#[test]
fn view_clear_search_button_constructs() {
    let _clear = view_clear_search_button();
}

// The Clear button must render beside the search box on every normal-mode
// browse level, so lay the row out and compare the input's real bounds with
// the Clear label's: Clear starts at or after the box's right edge and the two
// overlap vertically.
#[test]
fn view_search_row_draws_clear_after_the_search_box() {
    use iced::advanced::widget::operation::TextInput;

    #[derive(Default)]
    struct CapturedInput(Option<Rectangle>);
    impl Operation for CapturedInput {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }
        fn text_input(&mut self, id: Option<&Id>, bounds: Rectangle, _state: &mut dyn TextInput) {
            if id == Some(&Id::new(BROWSE_SEARCH_ID)) {
                self.0 = Some(bounds);
            }
        }
    }

    let renderer = layout_capture::headless_renderer();
    let mut element = view_search_row("Opening");
    let mut captured = CapturedInput::default();
    layout_capture::operate(&mut element, &renderer, &mut captured);
    let input = captured
        .0
        .expect("the search row must contain the search box");

    let mut element = view_search_row("Opening");
    let capture = layout_capture::capture_text(&mut element, &renderer);
    let clear = capture.find("Clear");

    assert!(
        clear.x >= input.x + input.width,
        "Clear must follow the search box: {input:?}, {clear:?}"
    );
    assert!(
        clear.y < input.y + input.height && input.y < clear.y + clear.height,
        "Clear and the search box must share the row vertically: {input:?}, {clear:?}"
    );
}

// Ctrl+F targets the search box by widget id, so the laid-out text input must
// register `BROWSE_SEARCH_ID` as a focusable; without it the key would do
// nothing.
#[test]
fn view_search_box_registers_its_focus_id() {
    use iced::advanced::widget::operation::Focusable;

    #[derive(Default)]
    struct CapturedIds(Vec<Option<Id>>);
    impl Operation for CapturedIds {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }
        fn focusable(&mut self, id: Option<&Id>, _bounds: Rectangle, _state: &mut dyn Focusable) {
            self.0.push(id.cloned());
        }
    }

    let renderer = layout_capture::headless_renderer();
    let mut element = view_search_box("Opening");
    let mut captured = CapturedIds::default();
    layout_capture::operate(&mut element, &renderer, &mut captured);

    assert!(
        captured
            .0
            .iter()
            .any(|id| id.as_ref() == Some(&Id::new(BROWSE_SEARCH_ID))),
        "the search box must register {BROWSE_SEARCH_ID} as a focusable"
    );
}

#[test]
fn row_prefix_numbers_rows_and_puts_the_play_marker_first() {
    assert_eq!(row_prefix(0, false), "1. ");
    assert_eq!(row_prefix(9, false), "10. ");
    assert_eq!(row_prefix(99, false), "100. ");
    assert_eq!(row_prefix(0, true), "▶ 1. ");
}

// The pixel-font split's part 3/4 draws the browse rows in the vendored
// pixel faces: the row text in Silkscreen (the letterform face,
// `theme::ui_font`) and the playing row's `▶` marker in Tiny5 (the shape
// face — Silkscreen's `▶` falls back to an anti-aliased shape). Serve
// both faces through `headless_renderer`, lay out a playing row, and pin
// the probed advances, so losing either face or its size fails here rather
// than in a screenshot. Sizes are f32-noisy, so each compares within 0.01.
#[test]
fn a_playing_browse_row_draws_the_vendored_pixel_faces() {
    let mut element = scrollable_list(
        [(Cow::Borrowed("H"), Cow::Borrowed("3:30"), true)],
        &CurrentView::Songs,
        0,
        None,
        "No songs",
    );
    let renderer = layout_capture::headless_renderer();
    let capture = layout_capture::capture_text(&mut element, &renderer);

    let marker = capture.find("▶");
    assert!(
        (marker.width - 8.0).abs() < 0.01,
        "the playing marker must draw Tiny5's ▶ at size 16 ({marker:?})"
    );
    let number = capture.find("1. ");
    assert!(
        (number.width - 24.0).abs() < 0.01,
        "the row number must draw Silkscreen's advance at size 16 ({number:?})"
    );
    let title = capture.find("H");
    assert!(
        (title.width - 12.0).abs() < 0.01,
        "the row title must draw Silkscreen's H advance at size 16 ({title:?})"
    );
    // The composed `row_prefix` reads `▶ 1. ` — one Tiny5 space (probed
    // 4.0 at size 16) between the marker and the number.
    assert!(
        (number.x - marker.x - marker.width - 4.0).abs() < 0.01,
        "the marker and number must sit one Tiny5 space apart: {marker:?} vs {number:?}"
    );
}

fn summary_album(duration_ms: u64) -> Album {
    let mut album = sample_album();
    album.duration_ms = duration_ms;
    album
}

#[test]
fn album_list_summary_counts_albums_and_sums_their_running_time() {
    assert_eq!(
        album_list_summary(&[summary_album(630_000)], None),
        "1 album - 10:30"
    );
    assert_eq!(
        album_list_summary(&[summary_album(630_000), summary_album(210_000)], None),
        "2 albums - 14:00"
    );
    assert_eq!(
        album_list_summary(&[summary_album(630_000), summary_album(0)], None),
        "2 albums - 10:30+"
    );
    assert_eq!(
        album_list_summary(&[summary_album(0)], None),
        "1 album - 0:00+"
    );
    assert!(
        album_list_summary(&[summary_album(u64::MAX), summary_album(1_000)], None)
            .starts_with("2 albums - ")
    );
}

#[test]
fn album_list_summary_puts_the_selected_running_time_before_the_total() {
    assert_eq!(
        album_list_summary(&[summary_album(630_000), summary_album(210_000)], Some(1)),
        "2 albums - 3:30/14:00"
    );
}

#[test]
fn album_list_summary_marks_an_unknown_selected_running_time() {
    assert_eq!(
        album_list_summary(&[summary_album(0), summary_album(210_000)], Some(0)),
        "2 albums - 0:00+/3:30+"
    );
}

#[test]
fn album_list_summary_ignores_a_selection_past_the_list() {
    assert_eq!(
        album_list_summary(&[summary_album(630_000)], Some(5)),
        "1 album - 10:30"
    );
}

fn artists_with_albums(counts: &[usize]) -> Vec<crate::library::Artist> {
    counts
        .iter()
        .map(|&album_count| {
            let mut artist = sample_artist();
            artist.album_count = album_count;
            artist
        })
        .collect()
}

#[test]
fn artist_list_summary_counts_artists_and_totals_their_albums() {
    let summary = |counts: &[usize]| artist_list_summary(&artists_with_albums(counts), None);
    assert_eq!(summary(&[1]), "1 artist - 1 album");
    assert_eq!(summary(&[2, 0, 3]), "3 artists - 5 albums");
    assert_eq!(summary(&[0]), "1 artist");
    assert_eq!(summary(&[0, 0]), "2 artists");
    assert!(summary(&[usize::MAX, 1]).starts_with("2 artists - "));
}

#[test]
fn artist_list_summary_puts_the_selected_album_count_before_the_total() {
    assert_eq!(
        artist_list_summary(&artists_with_albums(&[2, 0, 3]), Some(0)),
        "3 artists - 2/5 albums"
    );
    assert_eq!(
        artist_list_summary(&artists_with_albums(&[1]), Some(0)),
        "1 artist - 1/1 album"
    );
}

#[test]
fn artist_list_summary_keeps_the_plain_form_for_a_selected_artist_without_albums() {
    assert_eq!(
        artist_list_summary(&artists_with_albums(&[2, 0, 3]), Some(1)),
        "3 artists - 5 albums"
    );
    assert_eq!(
        artist_list_summary(&artists_with_albums(&[0, 0]), Some(0)),
        "2 artists"
    );
}

#[test]
fn artist_list_summary_ignores_a_selection_past_the_list() {
    assert_eq!(
        artist_list_summary(&artists_with_albums(&[2, 3]), Some(5)),
        "2 artists - 5 albums"
    );
}
