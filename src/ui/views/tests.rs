use super::super::{CurrentView, Message, style, theme};
use super::browse::{
    BROWSE_VIEWS, album_row, artist_row, browse_placeholder, current_row_style, empty_list_label,
    list_count_summary, playlist_row, row_prefix, song_list_summary, song_row,
};
use super::equalizer::{
    RESPONSE_CURVE_COLUMNS, RESPONSE_CURVE_HEIGHT, RESPONSE_CURVE_LINE_HEIGHT, eq_enabled_label,
    response_curve_offsets, response_curve_row,
};
use super::now_playing::{
    KnownTrack, MARQUEE_WIDTH, TimeDisplay, display_len, format_lcd_elapsed, format_lcd_remaining,
    format_playback_status, format_track_time, lcd_indicator_strip, marquee_text, needs_marquee,
    now_playing_artist, now_playing_content, now_playing_elapsed, now_playing_label,
    now_playing_row, now_playing_status_row, now_playing_time, now_playing_visualizer_row,
    visualizer_bar_heights,
};
use super::shade::{
    SHADE_STRIP_HEIGHT, SHADE_VOLUME_ID, SHADE_VOLUME_WIDTH, SHADED_HEIGHT, play_pause_glyph,
    shade_controls, shade_volume_spec, view_shade_strip,
};
use super::transport::{play_pause_label, repeat_label, shuffle_label, transport_buttons};
use super::{
    BALANCE_MAX, BALANCE_MIN, BALANCE_STEP, EQ_STEP, VOLUME_MAX, VOLUME_MIN, VOLUME_STEP,
    can_go_back, can_retry_artists, seek_bar_value, view_albums, view_all_albums,
    view_all_albums_button, view_all_songs, view_all_songs_button, view_artists, view_back_button,
    view_browse_nav_buttons, view_equalizer, view_now_playing, view_playlists,
    view_playlists_button, view_retry_button, view_search_box, view_seek_bar, view_songs,
    view_transport_controls,
};
use crate::audio::{PlaybackFormat, Spectrum};
use crate::equalizer::{
    BAND_COUNT, BAND_FREQUENCIES, GAIN_MAX_DB, GAIN_MIN_DB, PRESETS, clamp_gain,
};
use crate::library::{Album, Artist, Playlist, Song};
use crate::sample_library::sample_library;
use crate::state::{clamp_balance, clamp_volume};
use crate::test_support::{sample_album, sample_artist, sample_playlist, sample_song};
use crate::ui::layout_capture;
use iced::advanced::{
    Layout,
    layout::Limits,
    widget::{Id, Tree, operation::Operation},
};
use iced::{Length, Rectangle, Size};
use std::borrow::Cow;
use std::collections::HashMap;
use std::time::Duration;

// Each row maps one library entry to the (title, secondary label, press
// message, current-track flag) tuple that `scrollable_list` renders as a
// button. Wrong label text or a swapped selection message would silently
// break the browse UI, so the mapping is the contract these tests pin
// down — the flag is pinned too, since it drives the `▶`/highlight
// marker.

#[test]
fn artist_row_uses_name_and_selects_the_artist_by_index() {
    // The row carries the position `view_artists` enumerates and the
    // list's epoch, not the id: the update loop resolves that index back
    // to the artist. The index-to-id mapping is pinned by
    // `artist_selected_fetches_the_artists_albums_into_the_player` in
    // `ui::tests`.
    let artist = sample_artist();
    let (title, label, message, is_current) = artist_row(0, 0, &artist);
    assert_eq!(title, "The Sample Band");
    assert_eq!(label, "View Albums");
    assert!(matches!(
        message,
        Message::ArtistSelected { epoch: 0, index: 0 }
    ));
    assert!(!is_current);
}

#[test]
fn album_row_uses_title_and_selects_the_album_by_index() {
    // As with `artist_row`: the message carries the row's position and the
    // list's epoch, and
    // `album_selected_fetches_the_albums_songs_into_the_player` in
    // `ui::tests` pins the index-to-id mapping.
    let album = sample_album();
    let (title, label, message, is_current) = album_row(0, 0, &album);
    assert_eq!(title, "The Sample Band - First Record");
    assert_eq!(label, "View Songs");
    assert!(matches!(
        message,
        Message::AlbumSelected { epoch: 0, index: 0 }
    ));
    assert!(!is_current);
}

// An album with no artist has nothing to join, so the row must not render a
// dangling `" - "` prefix; it shows the bare title and borrows it.
#[test]
fn album_row_shows_the_title_alone_when_the_artist_is_empty() {
    let mut album = sample_album();
    album.artist = String::new();
    let (title, label, message, is_current) = album_row(0, 0, &album);
    assert_eq!(title, "First Record");
    assert!(matches!(title, std::borrow::Cow::Borrowed(_)));
    assert_eq!(label, "View Songs");
    assert!(matches!(
        message,
        Message::AlbumSelected { epoch: 0, index: 0 }
    ));
    assert!(!is_current);
}

#[test]
fn playlist_row_uses_name_and_selects_the_playlist_by_index() {
    // As with the other rows: the message carries the row's position and
    // the list's epoch, and
    // `playlist_selected_in_playlists_loads_its_songs_and_back_returns_to_playlists`
    // in `ui::tests` pins the index-to-id mapping.
    let playlist = sample_playlist();
    let (title, label, message, is_current) = playlist_row(0, 0, &playlist);
    assert_eq!(title, "Morning Drive");
    assert_eq!(label, "View Songs");
    assert!(matches!(
        message,
        Message::PlaylistSelected { epoch: 0, index: 0 }
    ));
    assert!(!is_current);
}

#[test]
fn song_row_uses_title_and_selects_the_song_by_index() {
    // As with the other rows: the message carries the row's position and
    // the list's epoch, and
    // `track_selected_starts_playback_of_the_selected_track` in
    // `ui::tests` pins the index-to-id mapping. Without a cached title the
    // row composes the display title itself, so a list stored without caches
    // still reads `<artist> - <title>`.
    let song = sample_song();
    let (title, label, message, is_current) = song_row(0, 0, &song, None, None, None);
    assert_eq!(title, "The Sample Band - Opening");
    // `sample_song` is 210_000 ms long, so the secondary label is its
    // `m:ss` length rather than the old static "Play" hint.
    assert_eq!(label, "3:30");
    assert!(matches!(
        message,
        Message::TrackSelected { epoch: 0, index: 0 }
    ));
    assert!(!is_current);
}

#[test]
fn song_row_prefers_the_lists_cached_title() {
    // The loaded song list caches each row's display title, and the row
    // builder must use it rather than composing the artist and title again.
    // A cached value that differs from the song's fields proves the row
    // borrows the cache; the `None` tests pin the fallback.
    let song = sample_song();
    let (title, _, _, _) = song_row(0, 0, &song, Some("Cached - Title"), None, None);
    assert_eq!(title, "Cached - Title");
}

#[test]
fn song_row_uses_the_title_alone_when_the_artist_is_empty() {
    // An artist-less song has nothing to join, so the row must not render a
    // dangling `" - "` prefix.
    let mut song = sample_song();
    song.artist = String::new();
    let (title, _, _, _) = song_row(0, 0, &song, None, None, None);
    assert_eq!(title, "Opening");
}

#[test]
fn song_row_prefers_the_lists_cached_length_label() {
    // The loaded song list caches each row's formatted length, and the row
    // builder must use it rather than formatting the duration again. A
    // cached label that differs from the duration's formatting proves the
    // row borrows the cache; the `None` tests above pin the fallback.
    let song = sample_song();
    let (_, label, _, _) = song_row(0, 0, &song, None, Some("1:23"), None);
    assert_eq!(label, "1:23");
}

#[test]
fn song_row_shows_an_unknown_duration_as_blank_time() {
    // A song whose source supplied no duration (`0`) renders the same
    // `--:--` placeholder as the Now Playing bar, not `0:00`.
    let mut song = sample_song();
    song.duration_ms = 0;
    let (_, label, _, _) = song_row(0, 0, &song, None, None, None);
    assert_eq!(label, "--:--");
}

#[test]
fn song_row_marks_the_current_track() {
    // The flag is true only when `current_track` names exactly this
    // song; a different id — or no track at all — leaves the row
    // unmarked, so the `▶`/highlight marker follows the playback state.
    let (_, _, _, is_current) = song_row(0, 0, &sample_song(), None, None, Some("song-1"));
    assert!(is_current);

    let (_, _, _, is_current) = song_row(0, 0, &sample_song(), None, None, Some("other-song"));
    assert!(!is_current);

    let (_, _, _, is_current) = song_row(0, 0, &sample_song(), None, None, None);
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

/// Builds the id→track index the player maintains for the Now Playing
/// bar, from `(id, artist, title)` triples — the shape `now_playing_label`
/// and `now_playing_artist` resolve against. Durations are `0` here, so the
/// cached time is the `--:--` placeholder; the length formatting is pinned
/// separately through `format_track_time` and the time-resolution test sets a
/// known label of its own.
fn known_tracks(entries: &[(&str, &str, &str)]) -> HashMap<String, KnownTrack> {
    entries
        .iter()
        .map(|(id, artist, title)| {
            (
                id.to_string(),
                KnownTrack {
                    title: title.to_string(),
                    artist: artist.to_string(),
                    time: "--:--".to_string(),
                    duration_ms: 0,
                },
            )
        })
        .collect()
}

#[test]
fn now_playing_label_shows_nothing_with_no_current_track() {
    assert_eq!(now_playing_label(&known_tracks(&[]), None), "Nothing");
}

#[test]
fn now_playing_label_resolves_known_track_to_title() {
    let tracks = known_tracks(&[("song-1", "The Sample Band", "Opening")]);
    assert_eq!(now_playing_label(&tracks, Some("song-1")), "Opening");
}

#[test]
fn now_playing_label_falls_back_to_id_when_track_not_in_tracks() {
    let tracks = known_tracks(&[("song-1", "The Sample Band", "Opening")]);
    assert_eq!(
        now_playing_label(&tracks, Some("no-such-song")),
        "no-such-song"
    );
}

#[test]
fn now_playing_label_falls_back_to_id_when_tracks_is_empty() {
    assert_eq!(
        now_playing_label(&known_tracks(&[]), Some("song-1")),
        "song-1"
    );
}

// The Now Playing bar renders `<artist> - <title>`; the artist resolves
// through the same id→track index as the title. Pin the known case, both
// fallbacks (unknown id and no current track), and — as with the label —
// that the known artist borrows from the caller's map rather than cloning
// per frame.
#[test]
fn now_playing_artist_resolves_known_track_to_artist() {
    let tracks = known_tracks(&[("song-1", "The Sample Band", "Opening")]);
    assert_eq!(
        now_playing_artist(&tracks, Some("song-1")),
        "The Sample Band"
    );
}

#[test]
fn now_playing_artist_is_empty_when_track_not_in_tracks() {
    let tracks = known_tracks(&[("song-1", "The Sample Band", "Opening")]);
    assert_eq!(now_playing_artist(&tracks, Some("no-such-song")), "");
}

#[test]
fn now_playing_artist_is_empty_with_no_current_track() {
    assert_eq!(now_playing_artist(&known_tracks(&[]), None), "");
}

#[test]
fn now_playing_artist_borrows_the_artist() {
    let tracks = known_tracks(&[("song-1", "The Sample Band", "Opening")]);

    let artist = now_playing_artist(&tracks, Some("song-1"));
    assert!(matches!(&artist, Cow::Borrowed(_)));
    assert_eq!(artist.as_ptr(), tracks["song-1"].artist.as_ptr());

    let artist = now_playing_artist(&tracks, None);
    assert!(matches!(&artist, Cow::Borrowed(_)));
}

#[test]
fn now_playing_time_resolves_known_track_to_cached_label() {
    let mut tracks = known_tracks(&[("song-1", "The Sample Band", "Opening")]);
    tracks.get_mut("song-1").expect("seed track").time = "3:30".to_string();

    let time = now_playing_time(&tracks, Some("song-1"));
    assert_eq!(time, "3:30");
    assert!(matches!(&time, Cow::Borrowed(_)));
    assert_eq!(time.as_ptr(), tracks["song-1"].time.as_ptr());

    let missing = now_playing_time(&tracks, Some("no-such-song"));
    assert_eq!(missing, "--:--");
    assert!(matches!(&missing, Cow::Borrowed(_)));

    let none = now_playing_time(&tracks, None);
    assert_eq!(none, "--:--");
    assert!(matches!(&none, Cow::Borrowed(_)));
}

// The right edge always shows the plain cached total; the backend-reported
// position belongs to the separate left-hand elapsed readout, never to this
// string.
#[test]
fn now_playing_time_returns_the_plain_total_even_with_a_position() {
    let mut tracks = known_tracks(&[("song-1", "The Sample Band", "Opening")]);
    tracks.get_mut("song-1").expect("seed track").time = "3:30".to_string();

    assert_eq!(now_playing_time(&tracks, Some("song-1")), "3:30");
    assert_eq!(
        now_playing_elapsed(
            &tracks,
            Some("song-1"),
            Some(Duration::from_secs(192)),
            TimeDisplay::Elapsed,
        ),
        "03:12"
    );
    assert_eq!(now_playing_time(&tracks, Some("song-1")), "3:30");
}

#[test]
fn now_playing_elapsed_reads_placeholder_zero_or_position() {
    let mut tracks = known_tracks(&[("song-1", "The Sample Band", "Opening")]);
    tracks.get_mut("song-1").expect("seed track").time = "3:30".to_string();

    let unknown = now_playing_elapsed(
        &tracks,
        Some("no-such-song"),
        Some(Duration::from_secs(53)),
        TimeDisplay::Elapsed,
    );
    assert_eq!(unknown, "--:--");
    assert!(matches!(&unknown, Cow::Borrowed(_)));

    let none = now_playing_elapsed(
        &tracks,
        None,
        Some(Duration::from_secs(53)),
        TimeDisplay::Elapsed,
    );
    assert_eq!(none, "--:--");
    assert!(matches!(&none, Cow::Borrowed(_)));

    let zero = now_playing_elapsed(&tracks, Some("song-1"), None, TimeDisplay::Elapsed);
    assert_eq!(zero, "00:00");
    assert!(matches!(&zero, Cow::Borrowed(_)));

    let position = now_playing_elapsed(
        &tracks,
        Some("song-1"),
        Some(Duration::from_secs(53)),
        TimeDisplay::Elapsed,
    );
    assert_eq!(position, "00:53");
    assert!(matches!(&position, Cow::Owned(_)));
}

#[test]
fn format_lcd_elapsed_pads_minutes_and_seconds() {
    assert_eq!(format_lcd_elapsed(0), "00:00");
    assert_eq!(format_lcd_elapsed(53_000), "00:53");
    assert_eq!(format_lcd_elapsed(192_000), "03:12");
    // Minutes stay unbounded, so an hour reads as 60:00 rather than rolling
    // over.
    assert_eq!(format_lcd_elapsed(3_600_000), "60:00");
}

#[test]
fn format_lcd_remaining_prefixes_a_minus_and_pads() {
    assert_eq!(format_lcd_remaining(0), "-00:00");
    assert_eq!(format_lcd_remaining(157_000), "-02:37");
    assert_eq!(format_lcd_remaining(3_600_000), "-60:00");
}

// The remaining mode counts down from the known track's own length, treating a
// missing position as zero; the length and position share milliseconds, so a
// position past the length saturates at -00:00 instead of underflowing. A
// known track with no length has nothing to count down from, and an unknown or
// absent track keeps the placeholder.
#[test]
fn now_playing_elapsed_remaining_counts_down_from_the_total() {
    let mut tracks = known_tracks(&[("song-1", "The Sample Band", "Opening")]);
    tracks.get_mut("song-1").expect("seed track").duration_ms = 210_000;

    assert_eq!(
        now_playing_elapsed(
            &tracks,
            Some("song-1"),
            Some(Duration::from_secs(53)),
            TimeDisplay::Remaining,
        ),
        "-02:37"
    );
    assert_eq!(
        now_playing_elapsed(&tracks, Some("song-1"), None, TimeDisplay::Remaining),
        "-03:30"
    );
    assert_eq!(
        now_playing_elapsed(
            &tracks,
            Some("song-1"),
            Some(Duration::from_secs(300)),
            TimeDisplay::Remaining,
        ),
        "-00:00"
    );

    tracks.get_mut("song-1").expect("seed track").duration_ms = 0;
    let no_length = now_playing_elapsed(
        &tracks,
        Some("song-1"),
        Some(Duration::from_secs(53)),
        TimeDisplay::Remaining,
    );
    assert_eq!(no_length, "--:--");
    assert!(matches!(&no_length, Cow::Borrowed(_)));

    let unknown = now_playing_elapsed(
        &tracks,
        Some("no-such-song"),
        Some(Duration::from_secs(53)),
        TimeDisplay::Remaining,
    );
    assert_eq!(unknown, "--:--");
    assert!(matches!(&unknown, Cow::Borrowed(_)));
}

#[test]
fn format_track_time_shows_placeholders_when_the_source_supplied_no_duration() {
    assert_eq!(format_track_time(0), "--:--");
}

// The marquee window is a pure character window: a line that fits is left
// alone so the common case renders borrowed and still, and an overlong line
// starts at the requested offset, wraps through the gap, and counts
// characters rather than bytes.
#[test]
fn marquee_text_keeps_a_line_that_fits() {
    assert_eq!(marquee_text("Opening", 9, 28), "Opening");
}

#[test]
fn marquee_text_windows_from_the_offset() {
    let text = "The Sample Band - Opening";
    assert_eq!(marquee_text(text, 0, 10), "The Sample");
    assert_eq!(marquee_text(text, 2, 10), "e Sample B");
}

#[test]
fn marquee_text_wraps_through_the_gap_to_the_start() {
    assert_eq!(
        marquee_text("The Sample Band - Opening", 25, 10),
        "   The Sam"
    );
}

#[test]
fn marquee_text_counts_characters_not_bytes() {
    assert_eq!(marquee_text("Café — Ok", 0, 5), "Café ");
}

/// The bounds of every container a built widget draws, collected through a
/// widget operation. The visualizer's bars are borderless containers with no
/// text, so counting their containers is how a test pins the bar count.
#[derive(Default)]
struct CapturedContainers(Vec<Rectangle>);

impl Operation for CapturedContainers {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn container(&mut self, _id: Option<&Id>, bounds: Rectangle) {
        self.0.push(bounds);
    }
}

/// Lays out the Now Playing row for `elapsed`, `label`, and `artist` at
/// `marquee_offset` and returns the text fragments it would draw.
///
/// The total readout is fixed at `"3:30"`, which the row draws at the right
/// edge of the LCD well. The first fragment is asserted to be `elapsed`: the
/// large elapsed readout leads the row.
fn captured_now_playing_texts(
    elapsed: &str,
    label: &str,
    artist: &str,
    offset: usize,
) -> Vec<String> {
    let mut element: iced::Element<'_, Message, iced::Theme, ()> = now_playing_row(
        Cow::Borrowed(elapsed),
        Cow::Borrowed(label),
        Cow::Borrowed(artist),
        Cow::Borrowed("3:30"),
        offset,
    )
    .into();
    let capture = layout_capture::capture_text(&mut element, &());
    assert_eq!(
        capture.texts().next(),
        Some(elapsed),
        "the elapsed readout must lead the row: {:?}",
        capture.fragments()
    );
    capture
        .fragments()
        .iter()
        .map(|(text, _)| text.clone())
        .collect()
}

/// Lays out the decorative indicator strip and returns the text fragments it
/// would draw and the width the column occupies.
fn captured_indicator_texts() -> (Vec<String>, f32) {
    let mut element: iced::Element<'_, Message, iced::Theme, ()> = lcd_indicator_strip().into();
    let capture = layout_capture::capture_text(&mut element, &());
    (
        capture.texts().map(str::to_string).collect(),
        capture.size().width,
    )
}

// The LCD's left edge draws the five classic indicator letters in the
// mockup's order, one per line, at the fixed strip width so the content
// beside it never shifts.
#[test]
fn lcd_indicator_strip_draws_each_label_in_order_at_a_fixed_width() {
    let (texts, width) = captured_indicator_texts();
    assert_eq!(texts, vec!["O", "A", "I", "D", "V"]);
    assert_eq!(width, 12.0, "the strip's fixed width");
}

// The whole bar's content carries the strip as well as the elapsed, title,
// and total readouts, so a refactor cannot drop the strip while its builder
// alone stays tested. The content is laid out directly because the concrete
// `lcd_well` wrapper ties `view_now_playing` to iced's fallback renderer.
// Every letter sits left of the readouts, in order down the left edge, and
// the total keeps its right-edge place beside them. The null renderer gives
// text no size, so the letters' order is read from the draw sequence and
// their `x` positions from the row layout, not from their zero-height bounds.
#[test]
fn now_playing_content_draws_the_indicator_strip_beside_the_title() {
    let mut element: iced::Element<'_, Message, iced::Theme, ()> = now_playing_content(
        Cow::Borrowed("00:53"),
        Cow::Borrowed("Opening"),
        Cow::Borrowed("Band"),
        Cow::Borrowed("3:30"),
        0,
        None,
        None,
    )
    .into();
    let capture = layout_capture::capture_text(&mut element, &());

    assert_eq!(
        capture.texts().take(5).collect::<Vec<_>>(),
        vec!["O", "A", "I", "D", "V"]
    );
    let find = |text: &str| capture.find(text);
    let elapsed = find("00:53");
    let title = find("Opening");
    let total = find("3:30");

    for label in ["O", "A", "I", "D", "V"] {
        let letter = find(label);
        assert!(
            letter.x < elapsed.x && letter.x < title.x && letter.x < total.x,
            "indicator {label} must sit left of the readouts: {letter:?}"
        );
    }

    assert!(
        (total.x + total.width - capture.size().width).abs() < 0.5,
        "the total must stay at the right edge: {total:?} in {:?}",
        capture.size()
    );
}

// The bar's own measure of the line: the label's characters plus the
// artist's and the `" - "` separator between them. The view windows the line
// when the measure exceeds the LCD's budget, and the marquee timer runs only
// while it does — so a measure that disagreed with the view's would leave the
// timer scrolling a line the bar shows whole, or idle while the bar windows
// one.
#[test]
fn display_len_counts_the_label_the_artist_and_the_separator() {
    assert_eq!(display_len("Opening", ""), 7);
    assert_eq!(display_len("Opening", "The Sample Band"), 25);
    // The separator is three characters, and a non-ASCII artist counts
    // characters rather than bytes.
    assert_eq!(display_len("Café", "Ångström"), 4 + 8 + 3);
}

// The line needs the marquee only when it does not fit the LCD's character
// budget: a fitting line is shown whole, so the timer would have nothing to
// scroll.
#[test]
fn needs_marquee_only_for_a_line_too_long_for_the_lcd() {
    // A label alone fills the budget exactly and still fits.
    assert!(!needs_marquee(&"x".repeat(MARQUEE_WIDTH), ""));
    assert!(needs_marquee(&"x".repeat(MARQUEE_WIDTH + 1), ""));
    // With an artist, the artist and the `" - "` separator count towards the
    // budget: "The Sample Band" is 15 characters, so a 4-character label fits
    // the 22-character budget and a 5-character one does not.
    let artist = "The Sample Band";
    assert!(!needs_marquee(&"x".repeat(4), artist));
    assert!(needs_marquee(&"x".repeat(5), artist));
}

// The row the Now Playing bar renders must use that window: an overlong line
// shows a different LCD text as the offset advances, while a fitting line
// still renders the separate borrowed artist and title at every offset.
#[test]
fn overlong_now_playing_line_renders_a_new_window_as_the_offset_advances() {
    let label = "A Title Much Longer Than The Display";
    let artist = "The Sample Band";
    let display = format!("{artist} - {label}");
    let first_window = marquee_text(&display, 0, MARQUEE_WIDTH);
    let second_window = marquee_text(&display, 1, MARQUEE_WIDTH);
    assert_ne!(first_window, second_window);

    let first = captured_now_playing_texts("00:53", label, artist, 0);
    let second = captured_now_playing_texts("00:53", label, artist, 1);
    assert!(first.iter().any(|text| text == &first_window), "{first:?}");
    assert!(
        second.iter().any(|text| text == &second_window),
        "{second:?}"
    );

    // A line that fits still renders the separate artist and title at every
    // offset: "Band" (4) plus the 3-character separator plus "Opening" (7)
    // is 14 characters, within the 22-character budget.
    let still_artist = "Band";
    let still_a = captured_now_playing_texts("00:00", "Opening", still_artist, 0);
    let still_b = captured_now_playing_texts("00:00", "Opening", still_artist, 5);
    assert_eq!(still_a, still_b);
    assert!(still_a.iter().any(|text| text == still_artist));
    assert!(still_a.iter().any(|text| text == "Opening"));
}

// The status row's text names the encoded bitrate, the sample rate in whole
// kilohertz, and the channel mode: 44.1 kHz truncates to 44, 48 kHz reads 48,
// one channel is mono while any other count is stereo, and an unknown bitrate
// is omitted.
#[test]
fn format_playback_status_formats_khz_and_channel_mode() {
    assert_eq!(
        format_playback_status(PlaybackFormat {
            sample_rate_hz: 44_100,
            channels: 2,
            bitrate_kbps: Some(256),
        }),
        "256 KBPS  44 KHZ  STEREO"
    );
    assert_eq!(
        format_playback_status(PlaybackFormat {
            sample_rate_hz: 44_100,
            channels: 2,
            bitrate_kbps: None,
        }),
        "44 KHZ  STEREO"
    );
    assert_eq!(
        format_playback_status(PlaybackFormat {
            sample_rate_hz: 44_100,
            channels: 1,
            bitrate_kbps: None,
        }),
        "44 KHZ  MONO"
    );
    assert_eq!(
        format_playback_status(PlaybackFormat {
            sample_rate_hz: 48_000,
            channels: 2,
            bitrate_kbps: Some(192),
        }),
        "192 KBPS  48 KHZ  STEREO"
    );
}

/// Lays out the LCD status row for `format` and returns the text fragments it
/// would draw and the height the row occupies.
fn captured_status_row(format: Option<PlaybackFormat>) -> (Vec<String>, f32) {
    let mut element: iced::Element<'_, Message, iced::Theme, ()> =
        now_playing_status_row(format).into();
    let capture = layout_capture::capture_text(&mut element, &());
    (
        capture.texts().map(str::to_string).collect(),
        capture.size().height,
    )
}

// The status row draws the formatted format when the backend reports one, and
// draws nothing when it does not, while keeping the same fixed height either
// way so the bar does not jump when a preview loads.
#[test]
fn now_playing_status_row_draws_the_format_or_nothing() {
    let (texts, height) = captured_status_row(Some(PlaybackFormat {
        sample_rate_hz: 44_100,
        channels: 2,
        bitrate_kbps: Some(256),
    }));
    assert_eq!(texts, vec!["256 KBPS  44 KHZ  STEREO".to_string()]);

    let (blank, blank_height) = captured_status_row(None);
    assert!(
        blank.is_empty(),
        "no reported format must draw no status text"
    );
    assert_eq!(
        height, blank_height,
        "the status row must keep its height whether or not a format is reported"
    );
    assert_eq!(height, 16.0, "the status row's fixed height");
}

/// The visualizer's bar heights come straight from the reported bands: a
/// missing spectrum is all zero, each band scales the row height, and values
/// outside `0.0..=1.0` clamp to the strip's bounds.
#[test]
fn visualizer_bar_heights_scale_and_clamp_the_reported_bands() {
    let mut bands = [0.0; BAND_COUNT];
    bands[0] = 0.0;
    bands[1] = 0.5;
    bands[2] = 1.0;
    bands[3] = 2.0;
    let heights = visualizer_bar_heights(Some(Spectrum { bands }));

    assert_eq!(heights.len(), BAND_COUNT);
    assert_eq!(heights[0], 0.0);
    assert_eq!(heights[1], 12.0);
    assert_eq!(heights[2], 24.0);
    assert_eq!(heights[3], 24.0);
    assert_eq!(&heights[4..], &[0.0; BAND_COUNT - 4]);
    assert_eq!(visualizer_bar_heights(None), [0.0; BAND_COUNT]);
}

/// Lays out the LCD content for `spectrum` and returns the text fragments it
/// would draw, each fragment's bounds, the height the content occupies, and
/// the bounds of every container it draws.
fn captured_content_with_spectrum(
    spectrum: Option<Spectrum>,
) -> (Vec<String>, Vec<Rectangle>, f32, Vec<Rectangle>) {
    let mut element: iced::Element<'_, Message, iced::Theme, ()> = now_playing_content(
        Cow::Borrowed("00:53"),
        Cow::Borrowed("Opening"),
        Cow::Borrowed("Band"),
        Cow::Borrowed("3:30"),
        0,
        Some(PlaybackFormat {
            sample_rate_hz: 44_100,
            channels: 2,
            bitrate_kbps: Some(256),
        }),
        spectrum,
    )
    .into();
    let capture = layout_capture::capture_text(&mut element, &());
    let height = capture.size().height;
    let texts = capture.texts().map(str::to_string).collect();
    let bounds = capture
        .fragments()
        .iter()
        .map(|(_, bounds)| *bounds)
        .collect();
    let mut tree = Tree::new(element.as_widget());
    let limits = Limits::new(Size::ZERO, Size::new(1000.0, 1000.0));
    let node = element.as_widget_mut().layout(&mut tree, &(), &limits);
    let mut containers = CapturedContainers::default();
    element
        .as_widget_mut()
        .operate(&mut tree, Layout::new(&node), &(), &mut containers);
    (texts, bounds, height, containers.0)
}

// The visualizer row keeps its fixed height whether a preview reports a
// spectrum or not, so loading one leaves the title and status text in place.
// The status text sits at least one visualizer-row height below the title, so
// the row is really wired between them rather than merely adding height.
#[test]
fn now_playing_content_keeps_the_visualizer_height_with_and_without_a_spectrum() {
    let mut bands = [0.0; BAND_COUNT];
    bands[0] = 1.0;
    let (reporting, reporting_bounds, reporting_height, reporting_containers) =
        captured_content_with_spectrum(Some(Spectrum { bands }));
    let (blank, _blank_bounds, blank_height, _blank_containers) =
        captured_content_with_spectrum(None);

    assert!(
        reporting.iter().any(|text| text == "Opening"),
        "the title must still draw: {reporting:?}"
    );
    assert!(
        reporting
            .iter()
            .any(|text| text == "256 KBPS  44 KHZ  STEREO"),
        "the status row must still draw: {reporting:?}"
    );
    assert!(
        blank.iter().any(|text| text == "Opening"),
        "the title must draw without a spectrum: {blank:?}"
    );

    let fragment_y = |text: &str| -> f32 {
        reporting
            .iter()
            .position(|drawn| drawn == text)
            .map(|index| reporting_bounds[index].y)
            .unwrap_or_else(|| panic!("missing {text}: {reporting:?}"))
    };
    let title_y = fragment_y("Opening");
    let status_y = fragment_y("256 KBPS  44 KHZ  STEREO");
    assert!(
        status_y - title_y >= 24.0,
        "the 24px visualizer row must sit between the title and status rows: \
         title y {title_y}, status y {status_y}"
    );

    // The bars themselves, not just the extra height, must sit between the
    // title and status text. Moving the visualizer row above the title or
    // below the status row leaves the spacing above intact, so check the bar
    // containers' own bounds as well.
    let bars: Vec<Rectangle> = reporting_containers
        .iter()
        .copied()
        .filter(|bounds| bounds.width == 8.0)
        .collect();
    assert_eq!(
        bars.len(),
        BAND_COUNT,
        "the visualizer row must draw one bar per band: {bars:?}"
    );
    assert!(
        bars.iter().all(|bar| bar.y >= title_y),
        "the visualizer bars must sit below the title: title y {title_y}, bars {bars:?}"
    );
    assert!(
        bars.iter().all(|bar| bar.y + bar.height <= status_y),
        "the visualizer bars must sit above the status row: status y {status_y}, bars {bars:?}"
    );
    assert_eq!(
        reporting_height, blank_height,
        "the visualizer row must keep its height whether or not a spectrum is reported"
    );
}

// The visualizer row keeps the fixed height even when every bar is empty; the
// silent bands still occupy a zero-height fixed-width slot inside the row.
#[test]
fn now_playing_visualizer_row_keeps_its_height_without_a_spectrum() {
    for spectrum in [
        None,
        Some(Spectrum {
            bands: [0.0; BAND_COUNT],
        }),
    ] {
        let mut element: iced::Element<'_, Message, iced::Theme, ()> =
            now_playing_visualizer_row(spectrum).into();
        let mut tree = Tree::new(element.as_widget());
        let limits = Limits::new(Size::ZERO, Size::new(1000.0, 1000.0));
        let node = element.as_widget_mut().layout(&mut tree, &(), &limits);
        assert_eq!(node.bounds().height, 24.0);
    }
}

// The row draws exactly one fixed-width bar per band, scaled by the band
// level, so a refactor cannot collapse the visualizer to a single bar while
// the pure height helper stays tested. The bands include zeros because a real
// spectrum does: iced's `Row` drops a `Fixed(0.0)`-height child, so this is
// the state that would silently lose the silent bars.
#[test]
fn now_playing_visualizer_row_draws_one_fixed_width_bar_per_band() {
    let mut bands = [0.0; BAND_COUNT];
    for (index, band) in bands.iter_mut().enumerate() {
        *band = (index % 3) as f32 * 0.25;
    }
    let mut element: iced::Element<'_, Message, iced::Theme, ()> =
        now_playing_visualizer_row(Some(Spectrum { bands })).into();
    let mut tree = Tree::new(element.as_widget());
    let limits = Limits::new(Size::ZERO, Size::new(1000.0, 1000.0));
    let node = element.as_widget_mut().layout(&mut tree, &(), &limits);

    let mut captured = CapturedContainers::default();
    element
        .as_widget_mut()
        .operate(&mut tree, Layout::new(&node), &(), &mut captured);
    let bars: Vec<_> = captured
        .0
        .iter()
        .filter(|bounds| bounds.width == 8.0)
        .collect();
    assert_eq!(
        bars.len(),
        BAND_COUNT,
        "the row must draw one bar per band, including silent bands"
    );
    for (bar, band) in bars.iter().zip(bands) {
        assert_eq!(bar.width, 8.0, "each bar keeps its fixed width: {bar:?}");
        assert_eq!(
            bar.height,
            band * 24.0,
            "a {band} band draws a proportionally tall bar: {bar:?}"
        );
    }
}

// The seek bar's thumb never exceeds the preview's own length, and a missing
// position, a missing length, or a zero length hides the bar entirely: the
// bar only appears once both a position and a scale are reported.
#[test]
fn seek_bar_value_clamps_the_thumb_and_hides_without_a_length() {
    assert_eq!(
        seek_bar_value(Some(Duration::from_secs(45)), Some(Duration::from_secs(30))),
        Some((30.0, 30.0))
    );
    assert_eq!(seek_bar_value(None, Some(Duration::from_secs(30))), None);
    assert_eq!(seek_bar_value(Some(Duration::from_secs(5)), None), None);
    assert_eq!(
        seek_bar_value(Some(Duration::from_secs(5)), Some(Duration::ZERO)),
        None
    );
    assert_eq!(seek_bar_value(None, None), None);
}

// Both shapes of the strip must construct: the slider when a preview length
// is known and the empty fixed-height stand-in when it is not.
#[test]
fn view_seek_bar_constructs_with_and_without_a_length() {
    let _known = view_seek_bar(Some(Duration::from_secs(10)), Some(Duration::from_secs(30)));
    let _unknown = view_seek_bar(Some(Duration::from_secs(10)), None);
}

// A track the library supplied without an artist must read as `Now Playing:
// <title>`: the row pushes the artist and its " - " separator only when the
// artist is non-empty, so an artist-less track shows no dangling separator.
// The app's empty state takes the same path — `now_playing_artist` answers ""
// for an unknown or an absent track (both pinned above), so the bar reads
// "Now Playing: Nothing" before anything is played. Nothing pinned the row's
// half of that contract: the test above always passed an artist, and the
// empty-artist row was only constructed by the smoke test, never read back.
#[test]
fn now_playing_row_omits_the_artist_separator_when_the_track_has_no_artist() {
    let texts = captured_now_playing_texts("00:00", "Opening", "", 0);

    assert!(
        texts.iter().any(|text| text == "Opening"),
        "the title must be drawn: {texts:?}"
    );
    assert!(
        !texts.iter().any(|text| text == " - "),
        "a track without an artist must not draw the separator: {texts:?}"
    );
    assert!(
        texts.iter().any(|text| text == "3:30"),
        "the length readout must be drawn: {texts:?}"
    );
}

// The same artist-less track with a title too long for the LCD scrolls the
// title alone: the window is taken over the title, not over a " - <title>"
// string, so the scrolling line never starts with the separator the empty
// artist dropped. A regression that composed the window from the
// artist-prefixed string would show the separator and lose the title's first
// character instead.
#[test]
fn an_overlong_title_without_an_artist_scrolls_the_title_alone() {
    let label = "A Title Much Longer Than The Display";
    let window = marquee_text(label, 0, MARQUEE_WIDTH);

    let texts = captured_now_playing_texts("00:00", label, "", 0);

    assert!(
        texts.iter().any(|text| text == &window),
        "the window must be taken over the title alone: {texts:?}"
    );
    assert!(
        !texts.iter().any(|text| text == " - "),
        "a track without an artist must not draw the separator: {texts:?}"
    );
}

#[test]
fn format_track_time_formats_minutes_and_zero_padded_seconds() {
    assert_eq!(format_track_time(210_000), "3:30");
    assert_eq!(format_track_time(45_000), "0:45");
}

// The doc comment promises a track past an hour keeps counting minutes
// (`60:00`, `61:05`) rather than rolling into an `h:mm:ss` hours field,
// and the tests above stop at 3:30. Pin the boundary: an exact hour is
// `60:00` (not `0:00`, and not `1:00:00`) and an hour plus a remainder
// keeps the same `m:ss` shape, so a regression that special-cased hours or
// reset the minute counter at 60 fails here.
#[test]
fn format_track_time_keeps_counting_minutes_past_an_hour() {
    assert_eq!(format_track_time(3_600_000), "60:00");
    assert_eq!(format_track_time(3_665_000), "61:05");
}

// `0` is the only "source supplied no duration" sentinel (see
// `Song::duration_ms`); every nonzero value is a real length and formats by
// truncating its sub-second remainder. Pin the boundary so a regression that
// treated a duration under one second as missing would show `--:--` for a
// real short track instead of `0:00`, while the first full second still
// advances the display.
#[test]
fn format_track_time_distinguishes_no_duration_from_a_sub_second_track() {
    assert_eq!(format_track_time(1), "0:00");
    assert_eq!(format_track_time(999), "0:00");
    assert_eq!(format_track_time(1_000), "0:01");
}

// The bar is rebuilt on every frame, and `now_playing_label` documents
// that the common cases borrow — the title straight out of the id→title
// index, or the static "Nothing" literal — so only the rare unknown-id
// fallback allocates. The value-equality tests above pass even if the
// function clones the title into a `Cow::Owned`, so pin the variants
// (and, for the known title, that the borrow points into the caller's
// map) here; otherwise a reintroduced clone would silently allocate per
// frame.
#[test]
fn now_playing_label_borrows_the_title_and_nothing_literal() {
    let tracks = known_tracks(&[("song-1", "The Sample Band", "Opening")]);

    let label = now_playing_label(&tracks, Some("song-1"));
    assert!(matches!(&label, Cow::Borrowed(_)));
    assert_eq!(label.as_ptr(), tracks["song-1"].title.as_ptr());

    let label = now_playing_label(&tracks, None);
    assert!(matches!(&label, Cow::Borrowed(_)));

    // The one allocating case stays owned: the unknown id is copied into
    // the fallback label.
    let label = now_playing_label(&tracks, Some("no-such-song"));
    assert!(matches!(&label, Cow::Owned(_)));
}

#[test]
fn play_pause_label_mirrors_playing_state() {
    assert_eq!(play_pause_label(true), "Pause");
    assert_eq!(play_pause_label(false), "Play");
}

#[test]
fn repeat_label_mirrors_repeat_state() {
    assert_eq!(repeat_label(true), "Repeat: On");
    assert_eq!(repeat_label(false), "Repeat: Off");
}

#[test]
fn shuffle_label_mirrors_shuffle_state() {
    assert_eq!(shuffle_label(true), "Shuffle: On");
    assert_eq!(shuffle_label(false), "Shuffle: Off");
}

#[test]
fn eq_enabled_label_mirrors_enabled_state() {
    assert_eq!(eq_enabled_label(true), "EQ: On");
    assert_eq!(eq_enabled_label(false), "EQ: Off");
}

// The `view_*` builders are the code that runs on every frame, and no
// other test reaches them: the row tests stop at the (title, label,
// message) tuples and the `update` tests stop before the view layer, so
// a regression that made a builder panic — a bad slider range, an
// out-of-bounds index in the `scrollable_list` loop — would take the
// window down on every refresh with no test catching it. iced `Element`s
// expose no tree introspection, so the observable contract here is that
// each builder constructs its widget tree without panicking over the
// input space the app actually produces: every browse view over both the
// loaded library and the pre-load empty buffer, both play states, the
// volume endpoints the update arm can store, and both repeat states.

#[test]
fn row_prefix_numbers_rows_and_puts_the_play_marker_first() {
    assert_eq!(row_prefix(0, false), "1. ");
    assert_eq!(row_prefix(9, false), "10. ");
    assert_eq!(row_prefix(99, false), "100. ");
    assert_eq!(row_prefix(0, true), "▶ 1. ");
}

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
    // marker) and with one set (the `▶`/highlight marker builds).
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
    let _artists = view_artists(artists, epoch, loading, None);
    let _albums = view_albums(albums, epoch, loading, None);
    let _songs = view_songs(songs, &[], &[], epoch, loading, None, None, false);
    let _songs_marked = view_songs(songs, &[], &[], epoch, loading, None, Some("song-1"), false);
    let _songs_searched = view_songs(songs, &[], &[], epoch, loading, None, None, true);
    let _all_songs = view_all_songs(songs, &[], &[], epoch, loading, None, None);
    let _all_songs_marked = view_all_songs(songs, &[], &[], epoch, loading, None, Some("song-1"));
    let _all_albums = view_all_albums(albums, epoch, loading, None);
    let _playlists = view_playlists(playlists, epoch, loading, None);
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
fn song_list_summary_counts_songs_and_sums_durations() {
    assert_eq!(song_list_summary(&[]), "0 songs - 0:00");
    let songs = [summary_song(210_000), summary_song(45_000)];
    assert_eq!(song_list_summary(&songs), "2 songs - 4:15");
}

#[test]
fn song_list_summary_uses_the_singular_for_one_song() {
    let songs = [summary_song(210_000)];
    assert_eq!(song_list_summary(&songs), "1 song - 3:30");
}

#[test]
fn song_list_summary_counts_unknown_durations_without_adding_time() {
    let songs = [summary_song(210_000), summary_song(0)];
    assert_eq!(song_list_summary(&songs), "2 songs - 3:30");
}

#[test]
fn song_list_summary_keeps_counting_minutes_past_an_hour() {
    let songs = [summary_song(3_665_000)];
    assert_eq!(song_list_summary(&songs), "1 song - 61:05");
}

// The total is summed with `saturating_add`, so a payload carrying a duration
// near the u64 ceiling pins the total at u64::MAX instead of wrapping back to
// a small time (a plain `+` would wrap in release and panic in debug).
#[test]
fn song_list_summary_saturates_a_total_past_the_u64_ceiling() {
    let songs = [summary_song(u64::MAX), summary_song(1)];
    assert_eq!(song_list_summary(&songs), "2 songs - 307445734561825:51");
}

// The footer's time is whole seconds, so a sub-second total truncates rather
// than rounding: a 999 ms list reads 0:00 and 1_999 ms reads 0:01.
#[test]
fn song_list_summary_truncates_a_sub_second_total() {
    assert_eq!(song_list_summary(&[summary_song(999)]), "1 song - 0:00");
    assert_eq!(song_list_summary(&[summary_song(1_999)]), "1 song - 0:01");
}

// The currently playing row is marked by a selection bar. iced's `Element`
// API exposes no way to read a built `Button`'s style, and the row-style
// closure only runs at render time, so building the marked row (above)
// never executes the override. `current_row_style` is the pure seam that
// pins it: every hover/press state keeps the highlight background rather
// than the unmarked row's face ladder, and the row's light text is kept so
// the title stays legible on the bar.
#[test]
fn current_row_style_paints_the_selection_bar_over_every_row_state() {
    use iced::widget::button::Status;

    for status in [
        Status::Active,
        Status::Hovered,
        Status::Pressed,
        Status::Disabled,
    ] {
        let marked = current_row_style(status);
        assert_eq!(
            marked.background,
            Some(iced::Background::Color(theme::PLAYING_ROW_HIGHLIGHT))
        );
        assert_eq!(
            marked.text_color,
            style::playlist_row_style(status).text_color
        );
    }

    // The bar must differ from the unmarked resting row, or the marker
    // would be invisible against the well.
    assert_ne!(
        current_row_style(Status::Active).background,
        style::playlist_row_style(Status::Active).background
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
        "3:30".into(),
        0,
        None,
        None,
    );
    let _bar = view_now_playing(
        "00:53".into(),
        "Opening".into(),
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
    );
    let _overlong = view_now_playing(
        "03:12".into(),
        "A Title Much Longer Than The Display".into(),
        "The Sample Band".into(),
        "3:30".into(),
        7,
        None,
        None,
    );
    let _back = view_back_button();
    let _retry = view_retry_button();
    let _nav = view_browse_nav_buttons();
    let _all_songs = view_all_songs_button();
    let _all_albums = view_all_albums_button();
    let _playlists = view_playlists_button();
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

// The transport row is rebuilt every frame, and the Play/Pause, Repeat,
// and Shuffle labels change with state. A button that sizes to its text
// resizes when its label changes, reflowing every widget after it in the
// row (the qa reproduction measured a 14-15px shift when Play swapped to
// the wider Pause). Pin that every transport button requests a fixed width
// and that the widths do not depend on the play, repeat, or shuffle state,
// so a label swap cannot move the row.
#[test]
fn transport_buttons_keep_a_fixed_width_across_label_changes() {
    let stopped = transport_buttons(false, false, false);
    let playing = transport_buttons(true, false, false);
    let repeat_on = transport_buttons(false, true, false);
    let shuffle_on = transport_buttons(false, false, true);

    for (index, button) in stopped.iter().enumerate() {
        assert!(
            matches!(button.as_widget().size().width, Length::Fixed(_)),
            "transport button {index} is not fixed-width"
        );
    }
    for (a, b) in stopped.iter().zip(playing.iter()) {
        assert_eq!(a.as_widget().size().width, b.as_widget().size().width);
    }
    for (a, b) in stopped.iter().zip(repeat_on.iter()) {
        assert_eq!(a.as_widget().size().width, b.as_widget().size().width);
    }
    for (a, b) in stopped.iter().zip(shuffle_on.iter()) {
        assert_eq!(a.as_widget().size().width, b.as_widget().size().width);
    }
}

// The label-reflow test above pins only that every transport button is
// fixed-width and that its width is independent of the play, repeat, and
// shuffle state; a zeroed, widened, or otherwise wrong width literal would
// clear it while the label clipped or the row moved. Pin the face widths
// measured from the running window when the transport row was fixed (see
// "Transport buttons resize to their label text" in BUGS-FIXED.md), in row
// order: Play/Pause 65, Stop 54, Previous 84, Next 54, Repeat 104, Shuffle
// 112. Each is the width of that button's widest label, so a deliberate
// label or width change must update this pin.
#[test]
fn transport_buttons_keep_their_measured_face_widths() {
    let widths: Vec<Length> = transport_buttons(false, false, false)
        .iter()
        .map(|button| button.as_widget().size().width)
        .collect();
    assert_eq!(
        widths,
        [
            Length::Fixed(65.0),
            Length::Fixed(54.0),
            Length::Fixed(84.0),
            Length::Fixed(54.0),
            Length::Fixed(104.0),
            Length::Fixed(112.0),
        ]
    );
}

#[test]
fn transport_controls_construct_for_both_play_states_volume_endpoints_and_repeat_states() {
    // `view()` passes the shared state's `is_playing`, clamped `volume`,
    // `balance`, `repeat`, and `shuffle` straight through, so build the
    // transport row for every value the update arms can store, in every
    // play, repeat, and shuffle state.
    for volume in [0.0, 0.5, 1.0] {
        for balance in [-1.0, 0.0, 1.0] {
            for is_playing in [false, true] {
                for repeat in [false, true] {
                    for shuffle in [false, true] {
                        let _controls =
                            view_transport_controls(is_playing, volume, balance, repeat, shuffle);
                    }
                }
            }
        }
    }
}

#[test]
fn equalizer_panel_constructs_for_both_states_and_gain_endpoints() {
    // `view()` passes the shared EQ flag, preamp, and band array straight
    // through, so build the panel for both on/off states and every gain
    // the update arms can store: the range endpoints and flat. iced
    // `Element`s expose no tree introspection, so the observable contract
    // is that the builder constructs its ten-slider tree without
    // panicking over the input space the app produces.
    for enabled in [false, true] {
        for gain in [GAIN_MIN_DB, 0.0, GAIN_MAX_DB] {
            for preset in [None, Some(PRESETS[0])] {
                let bands = [gain; BAND_COUNT];
                let _panel = view_equalizer(enabled, gain, &bands, preset);
            }
        }
    }
}

// The slider specs are literals iced exposes no way to read back, so they
// are named constants the builders consume and these tests pin. The
// important one is the tie to the shared-state clamp: each slider's range
// must equal the range its state clamp stores. A slider bound the clamp
// moves means the thumb cannot reach the stored extreme; a bound the clamp
// accepts but the slider cannot reach leaves part of the scale unusable.

/// Asserts that `clamp` stores exactly `min..=max`: both endpoints survive
/// unchanged and a step beyond each is clamped back. This pins a slider's
/// range equal to its shared-state range without reading the iced widget.
fn assert_slider_range_matches_clamp(clamp: fn(f32) -> f32, min: f32, max: f32, step: f32) {
    assert_eq!(clamp(min), min);
    assert_eq!(clamp(max), max);
    assert_eq!(clamp(min - step), min);
    assert_eq!(clamp(max + step), max);
}

#[test]
fn volume_slider_spec_matches_the_state_clamp_and_pins_its_granularity() {
    assert_slider_range_matches_clamp(clamp_volume, VOLUME_MIN, VOLUME_MAX, VOLUME_STEP);

    // One-percent drag granularity, pinned so the step cannot silently
    // coarsen while the endpoints stay put.
    assert_eq!(VOLUME_STEP, 0.01);
}

// The balance twin of the volume spec: the balance slider's range must
// equal the range `set_balance`/`clamp_balance` store, so a drag to either
// end lands exactly on the stored extreme, and its step is pinned so it
// cannot silently coarsen.
#[test]
fn balance_slider_spec_matches_the_state_clamp_and_pins_its_granularity() {
    assert_slider_range_matches_clamp(clamp_balance, BALANCE_MIN, BALANCE_MAX, BALANCE_STEP);

    assert_eq!(BALANCE_STEP, 0.01);
}

// The equalizer twin: every preamp and band slider spans
// `GAIN_MIN_DB..=GAIN_MAX_DB` and must equal the range `clamp_gain`
// stores, so a drag to either end lands exactly on the stored extreme.
#[test]
fn equalizer_slider_spec_matches_the_gain_clamp_and_steps_whole_decibels() {
    assert_slider_range_matches_clamp(clamp_gain, GAIN_MIN_DB, GAIN_MAX_DB, EQ_STEP);

    assert_eq!(EQ_STEP, 1.0);
}

// The ten band labels are the equalizer's only frequency readout: when a
// pair touches, two frequencies read as one number (`1703106001K`). This was
// confirmed only from a captured window, so lay the panel out with the
// headless tiny-skia renderer — which measures text for real — and require a
// visible gap between every pair of neighbouring labels.
#[test]
fn equalizer_band_labels_leave_a_gap_between_frequencies() {
    let bands = [0.0; BAND_COUNT];
    let mut element = view_equalizer(false, 0.0, &bands, None);
    let capture = layout_capture::capture_text(&mut element, &layout_capture::headless_renderer());

    let bounds: Vec<(&str, Rectangle)> = BAND_FREQUENCIES
        .iter()
        .map(|frequency| (*frequency, capture.find(frequency)))
        .collect();
    for pair in bounds.windows(2) {
        let (left_label, left) = pair[0];
        let (right_label, right) = pair[1];
        assert!(
            left.x + left.width + 1.0 <= right.x,
            "{left_label} and {right_label} labels touch: {:?}",
            capture.fragments()
        );
    }
}

// The response curve is the panel's only readout of the whole stored shape,
// so its pure offset helper must trace the same gains the sliders show: flat
// in the middle, the extremes at the plot's edges, and linear between the ten
// bands. Pin the exact values, the endpoint correspondence, and the two
// degenerate column counts.
#[test]
fn response_curve_offsets_follow_the_band_gains() {
    assert!(
        response_curve_offsets(&[0.0; BAND_COUNT], 0).is_empty(),
        "no columns means no segments"
    );
    assert_eq!(
        response_curve_offsets(&[0.0; BAND_COUNT], 1),
        vec![0.5],
        "one column sits on the first band instead of dividing by zero"
    );

    let flat = response_curve_offsets(&[0.0; BAND_COUNT], RESPONSE_CURVE_COLUMNS);
    assert_eq!(flat.len(), RESPONSE_CURVE_COLUMNS);
    assert!(
        flat.iter().all(|offset| *offset == 0.5),
        "a flat curve is centered: {flat:?}"
    );
    assert!(
        response_curve_offsets(&[GAIN_MIN_DB; BAND_COUNT], RESPONSE_CURVE_COLUMNS)
            .iter()
            .all(|offset| *offset == 0.0),
        "all-min gains sit on the plot's bottom edge"
    );
    assert!(
        response_curve_offsets(&[GAIN_MAX_DB; BAND_COUNT], RESPONSE_CURVE_COLUMNS)
            .iter()
            .all(|offset| *offset == 1.0),
        "all-max gains sit on the plot's top edge"
    );

    let mut ramp = [0.0; BAND_COUNT];
    ramp[0] = GAIN_MIN_DB;
    ramp[BAND_COUNT - 1] = GAIN_MAX_DB;
    let endpoints = response_curve_offsets(&ramp, RESPONSE_CURVE_COLUMNS);
    assert_eq!(
        endpoints.first().copied(),
        Some((clamp_gain(GAIN_MIN_DB) - GAIN_MIN_DB) / (GAIN_MAX_DB - GAIN_MIN_DB))
    );
    assert_eq!(
        endpoints.last().copied(),
        Some((clamp_gain(GAIN_MAX_DB) - GAIN_MIN_DB) / (GAIN_MAX_DB - GAIN_MIN_DB))
    );

    let three = response_curve_offsets(&ramp, 3);
    assert_eq!(
        three,
        vec![0.0, 0.5, 1.0],
        "columns interpolate between bands"
    );
}

// Lay the curve row out under the null renderer: it must keep its fixed
// height for any curve and draw exactly one green segment per column, first
// column at the bottom and last at the top for a rising curve. The zero-offset
// first column is the one iced would silently drop if it were `Fixed(0.0)`.
#[test]
fn response_curve_row_keeps_its_height_and_draws_one_segment_per_column() {
    let mut bands = [0.0; BAND_COUNT];
    bands[0] = GAIN_MIN_DB;
    bands[BAND_COUNT - 1] = GAIN_MAX_DB;
    let mut element: iced::Element<'_, Message, iced::Theme, ()> = response_curve_row(&bands);
    let mut tree = Tree::new(element.as_widget());
    let node = element
        .as_widget_mut()
        .layout(&mut tree, &(), &layout_capture::limits());
    assert_eq!(node.bounds().height, RESPONSE_CURVE_HEIGHT);

    let mut captured = CapturedContainers::default();
    element
        .as_widget_mut()
        .operate(&mut tree, Layout::new(&node), &(), &mut captured);
    let segments: Vec<Rectangle> = captured
        .0
        .iter()
        .filter(|bounds| bounds.height == RESPONSE_CURVE_LINE_HEIGHT)
        .copied()
        .collect();
    assert_eq!(
        segments.len(),
        RESPONSE_CURVE_COLUMNS,
        "one segment per column, including the zero-offset first column"
    );
    let first = segments[0];
    let last = segments[segments.len() - 1];
    assert_eq!(
        first.y + first.height,
        RESPONSE_CURVE_HEIGHT,
        "the first band's minimum gain draws on the bottom edge: {first:?}"
    );
    assert_eq!(
        last.y, 0.0,
        "the last band's maximum gain draws on the top edge: {last:?}"
    );

    // The mockup's flat preset: every segment stays on the plot's midline.
    let mut flat_element: iced::Element<'_, Message, iced::Theme, ()> =
        response_curve_row(&[0.0; BAND_COUNT]);
    let mut flat_tree = Tree::new(flat_element.as_widget());
    let flat_node =
        flat_element
            .as_widget_mut()
            .layout(&mut flat_tree, &(), &layout_capture::limits());
    let mut flat_captured = CapturedContainers::default();
    flat_element.as_widget_mut().operate(
        &mut flat_tree,
        Layout::new(&flat_node),
        &(),
        &mut flat_captured,
    );
    let flat_segments: Vec<Rectangle> = flat_captured
        .0
        .iter()
        .filter(|bounds| bounds.height == RESPONSE_CURVE_LINE_HEIGHT)
        .copied()
        .collect();
    assert_eq!(flat_segments.len(), RESPONSE_CURVE_COLUMNS);
    let midline = (RESPONSE_CURVE_HEIGHT - RESPONSE_CURVE_LINE_HEIGHT) / 2.0;
    assert!(
        flat_segments.iter().all(|segment| segment.y == midline),
        "a flat curve draws a centered line: {flat_segments:?}"
    );
}

// The shade strip's Play/Pause face is the compact twin of the transport
// row's label: it switches with playback so the rolled-up window still shows
// what pressing it will do.
#[test]
fn shade_play_pause_glyph_switches_with_playback_state() {
    assert_eq!(play_pause_glyph(false), "▶");
    assert_eq!(play_pause_glyph(true), "II");
}

// The four compact buttons must emit the same messages as their full
// transport-row twins, and each keeps a fixed width so the glyph swap cannot
// reflow the strip. Pin both, in row order.
#[test]
fn shade_controls_map_to_transport_messages_and_keep_fixed_widths() {
    let stopped = shade_controls(false);
    let playing = shade_controls(true);

    let labels: Vec<&str> = stopped.iter().map(|(label, _, _)| *label).collect();
    assert_eq!(labels, vec!["◀◀", "▶", "■", "▶▶"]);
    let widths: Vec<f32> = stopped.iter().map(|(_, width, _)| *width).collect();
    assert_eq!(widths, vec![34.0, 30.0, 30.0, 34.0]);

    let messages: Vec<&Message> = stopped.iter().map(|(_, _, message)| message).collect();
    assert!(matches!(messages[0], Message::PreviousTrack));
    assert!(matches!(messages[1], Message::PlayPause));
    assert!(matches!(messages[2], Message::Stop));
    assert!(matches!(messages[3], Message::NextTrack));

    assert_eq!(playing[1].0, "II");
    let playing_widths: Vec<f32> = playing.iter().map(|(_, width, _)| *width).collect();
    assert_eq!(
        playing_widths, widths,
        "the glyph swap must not resize a button"
    );
}

// The rolled-up window is exactly the title bar plus the strip; both the
// view and `resize_to_shade` read `SHADED_HEIGHT`, so pin its composition and
// its positive, fixed size.
#[test]
fn shade_height_pins_the_rolled_up_window() {
    assert_eq!(SHADE_STRIP_HEIGHT, 28.0);
    assert_eq!(SHADED_HEIGHT, super::TITLE_BAR_HEIGHT + SHADE_STRIP_HEIGHT);
}

// Laying the strip out with the headless renderer resolves its fixed height
// for both playback states, so a browser or layout change cannot shrink the
// rolled-up window to an empty band.
#[test]
fn view_shade_strip_keeps_its_fixed_height_for_both_playback_states() {
    let renderer = layout_capture::headless_renderer();

    for is_playing in [false, true] {
        let mut element = view_shade_strip(is_playing, 0.5, Cow::Borrowed("00:53"));
        let mut tree = Tree::new(element.as_widget());
        let limits = Limits::new(Size::ZERO, Size::new(1000.0, 1000.0));
        let node = element
            .as_widget_mut()
            .layout(&mut tree, &renderer, &limits);
        assert_eq!(
            node.size().height,
            SHADE_STRIP_HEIGHT,
            "the strip must keep its fixed height while is_playing = {is_playing}"
        );
    }
}

/// Lays the shade strip out with the headless tiny-skia renderer and records
/// every text fragment plus the bounds of the container tagged
/// [`SHADE_VOLUME_ID`], so a test can see the time readout and the volume
/// slider's place in the real laid-out row.
fn captured_shade_strip(elapsed: &str) -> (Vec<String>, Option<Rectangle>, Size) {
    #[derive(Default)]
    struct CapturedContainer {
        slider: Option<Rectangle>,
    }
    impl Operation for CapturedContainer {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }
        fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
            if id == Some(&Id::new(SHADE_VOLUME_ID)) {
                self.slider = Some(bounds);
            }
        }
    }

    let renderer = layout_capture::headless_renderer();
    let mut element = view_shade_strip(false, 0.5, Cow::Borrowed(elapsed));
    let capture = layout_capture::capture_text(&mut element, &renderer);
    let texts = capture.texts().map(str::to_string).collect();

    let mut tree = Tree::new(element.as_widget());
    let limits = Limits::new(Size::ZERO, Size::new(1000.0, 1000.0));
    let node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &limits);
    let mut captured = CapturedContainer::default();
    element
        .as_widget_mut()
        .operate(&mut tree, Layout::new(&node), &renderer, &mut captured);
    (texts, captured.slider, capture.size())
}

// The strip's two non-button controls must actually reach the laid-out row:
// the time readout the caller gives it (elapsed or remaining) and the volume
// slider. The fill spacer pushes the slider to the right edge, so it does not
// sit immediately after the readout on the left.
#[test]
fn shade_strip_draws_the_time_readout_and_puts_the_volume_slider_at_the_right_edge() {
    for time in ["00:53", "-02:37"] {
        let (texts, slider, size) = captured_shade_strip(time);
        assert!(
            texts.iter().any(|text| text == time),
            "missing {time} readout: {texts:?}"
        );
        let slider = slider.unwrap_or_else(|| panic!("missing volume slider: {texts:?}"));
        assert_eq!(slider.width, SHADE_VOLUME_WIDTH);
        assert!(
            (slider.x + slider.width - (size.width - 6.0)).abs() < 0.5,
            "slider must sit at the strip's right edge: {slider:?} in {size:?}"
        );
    }
}

// The strip's volume slider must read the shared volume range and write
// `Message::VolumeChange`, the same contract as the full transport row's
// volume slider.
#[test]
fn shade_volume_slider_spec_matches_the_shared_volume_range_and_message() {
    let (min, max, step, width, on_change) = shade_volume_spec();
    assert_eq!((min, max, step), (VOLUME_MIN, VOLUME_MAX, VOLUME_STEP));
    assert_eq!(width, SHADE_VOLUME_WIDTH);
    assert_eq!(SHADE_VOLUME_WIDTH, 80.0);
    assert!(matches!(
        on_change(0.5),
        Message::VolumeChange(value) if value == 0.5
    ));
}
