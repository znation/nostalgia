use super::super::{CurrentView, Message, style, theme};
use super::browse::BROWSE_VIEWS;
use super::browse::rows::{
    album_count_label, album_row, artist_row, browse_placeholder, browse_row_style,
    empty_list_label, list_count_summary, playlist_row, row_prefix, scrollable_list,
    song_list_summary, song_row,
};
use super::equalizer::{
    EQ_SLIDER_HEIGHT, RESPONSE_CURVE_COLUMNS, RESPONSE_CURVE_HEIGHT, RESPONSE_CURVE_LINE_HEIGHT,
    eq_enabled_label, response_curve_offsets, response_curve_row,
};
use super::mini_transport::{
    MINI_TRANSPORT_HEIGHT, MINI_VOLUME_ID, MINI_VOLUME_WIDTH, mini_transport_controls,
    mini_transport_readout, mini_volume_spec, view_library_strip,
};
use super::now_playing::text::{format_lcd_elapsed, format_lcd_remaining};
use super::now_playing::{
    CHANNEL_LABELS, KnownTrack, MARQUEE_WIDTH, TimeDisplay, channel_label_color,
    channel_mode_label, display_len, elapsed_readout_clickable, elapsed_readout_color,
    format_playback_prefix, lcd_indicator_message, lcd_indicator_strip, marquee_text,
    needs_marquee, now_playing_artist, now_playing_content, now_playing_elapsed, now_playing_label,
    now_playing_number, now_playing_row, now_playing_status_row, now_playing_time,
    now_playing_visualizer_row, transport_glyph, transport_glyph_color, visible_spectrum,
    visualizer_bar_heights,
};
use super::shade::{SHADE_STRIP_HEIGHT, SHADED_HEIGHT, view_shade_strip};
use super::title_bar::{CLUTTER_BUTTON_WIDTH, TITLE_BAR_BUTTON_WIDTH};
use super::transport::{
    EQ_PANEL_BUTTON_WIDTH, eq_panel_button_style, eq_panel_toggle, play_pause_glyph,
    play_pause_word, repeat_label, shuffle_label, transport_buttons, transport_glyphs,
};
use super::{
    BALANCE_MAX, BALANCE_MIN, BALANCE_STEP, EQ_STEP, TITLE_BAR_HEIGHT, VOLUME_MAX, VOLUME_MIN,
    VOLUME_STEP, can_go_back, can_retry_artists, format_track_time, seek_bar_value, view_albums,
    view_all_albums, view_all_albums_button, view_all_songs, view_all_songs_button, view_artists,
    view_back_button, view_browse_nav_buttons, view_equalizer, view_now_playing, view_open_button,
    view_playlists, view_playlists_button, view_retry_button, view_search_box, view_seek_bar,
    view_songs, view_title_band, view_title_bar, view_transport_controls, volume_wheel_message,
};
use crate::audio::{PlaybackFormat, Spectrum};
use crate::equalizer::{
    BAND_COUNT, BAND_FREQUENCIES, GAIN_MAX_DB, GAIN_MIN_DB, PRESETS, clamp_gain,
};
use crate::library::{Album, Artist, Playlist, Song};
use crate::sample_library::sample_library;
use crate::state::{TransportMode, clamp_balance, clamp_volume};
use crate::test_support::{
    recording_spectrum, sample_album, sample_artist, sample_playlist, sample_song,
};
use crate::ui::layout_capture;
use iced::advanced::{
    Layout,
    layout::Limits,
    widget::{Id, Tree, operation::Operation},
};
use iced::{Color, Element, Length, Rectangle, Size};
use std::borrow::Cow;
use std::collections::HashMap;
use std::time::Duration;

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
    let (title, label, is_current) = artist_row(&artist);
    assert_eq!(title, "The Sample Band");
    assert_eq!(label, "2 albums");
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

#[test]
fn album_count_label_singular_plural_and_none() {
    assert_eq!(album_count_label(0), "");
    assert_eq!(album_count_label(1), "1 album");
    assert_eq!(album_count_label(2), "2 albums");
}

// The mockup puts the album count at the artist row's right edge, not
// directly after the name. Lay the real list out and require the count to
// reach the right of the row while the name stays on the left.
#[test]
fn artist_row_puts_its_album_count_at_the_right_edge() {
    let mut artist = sample_artist();
    artist.album_count = 2;
    let mut element = scrollable_list([artist_row(&artist)], 0, None, "No artists");
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
                    number: String::new(),
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

// The Now Playing bar draws the cached playlist number prefix between its
// caption and the artist. Pin the known case, both empty fallbacks (unknown id
// and no current track), and that the known prefix borrows from the caller's
// map rather than cloning per frame.
#[test]
fn now_playing_number_resolves_known_track_to_its_cached_prefix() {
    let mut tracks = known_tracks(&[("song-1", "The Sample Band", "Opening")]);
    tracks.get_mut("song-1").expect("seed track").number = "1. ".to_string();

    let number = now_playing_number(&tracks, Some("song-1"));
    assert_eq!(number, "1. ");
    assert!(matches!(&number, Cow::Borrowed(_)));
    assert_eq!(number.as_ptr(), tracks["song-1"].number.as_ptr());

    assert_eq!(now_playing_number(&tracks, Some("no-such-song")), "");
    assert_eq!(now_playing_number(&tracks, None), "");
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

/// Lays out the Now Playing row for `elapsed`, `label`, `artist`, and the
/// `number` prefix at `marquee_offset` in `mode` and returns the text fragments
/// it would draw.
///
/// The total readout is fixed at `"3:30"`, which the row draws at the right
/// edge of the LCD well. The transport glyph leads the row and the large
/// elapsed readout follows it.
fn captured_now_playing_texts(
    elapsed: &str,
    label: &str,
    artist: &str,
    number: &str,
    offset: usize,
    mode: TransportMode,
) -> Vec<String> {
    let mut element: iced::Element<'_, Message, iced::Theme, ()> = now_playing_row(
        Cow::Borrowed(elapsed),
        Cow::Borrowed(label),
        Cow::Borrowed(artist),
        Cow::Borrowed(number),
        Cow::Borrowed("3:30"),
        offset,
        mode,
        true,
    )
    .into();
    let capture = layout_capture::capture_text(&mut element, &());
    assert_eq!(
        capture.texts().next(),
        Some(transport_glyph(mode)),
        "the transport glyph must lead the row: {:?}",
        capture.fragments()
    );
    assert_eq!(
        capture.texts().nth(1),
        Some(elapsed),
        "the elapsed readout must follow the glyph: {:?}",
        capture.fragments()
    );
    capture
        .fragments()
        .iter()
        .map(|(text, _)| text.clone())
        .collect()
}

/// Lays out the indicator strip (with its live `A` and `V` controls) and
/// returns the text fragments it would draw and the width the column occupies.
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

// Only A and V are live: A reuses the title bar's always-on-top toggle, V
// toggles the LCD visualizer, and O, I, and D publish nothing.
#[test]
fn lcd_indicator_message_arms_a_and_v_and_leaves_o_i_d_inert() {
    assert!(matches!(
        lcd_indicator_message("A"),
        Some(Message::ToggleAlwaysOnTop)
    ));
    assert!(matches!(
        lcd_indicator_message("V"),
        Some(Message::ToggleVisualizer)
    ));
    for label in ["O", "I", "D"] {
        assert!(
            lcd_indicator_message(label).is_none(),
            "{label} must stay decorative"
        );
    }
}

// The strip must wire the mapping up, not just expose it: drive a real left
// press at each letter's bounds. A publishes the title bar's always-on-top
// toggle, V publishes the visualizer toggle, and O publishes nothing.
#[test]
fn lcd_indicator_strip_publishes_always_on_top_on_a_and_visualizer_on_v() {
    let mut renderer = layout_capture::headless_renderer();
    for label in ["A", "V", "O"] {
        let mut element: iced::Element<'_, Message, iced::Theme, iced::Renderer> =
            lcd_indicator_strip().into();
        let capture = layout_capture::capture_text(&mut element, &renderer);
        let letter = capture.find(label);
        let mut user_interface = iced_runtime::user_interface::UserInterface::build(
            element,
            layout_capture::limits().max(),
            iced_runtime::user_interface::Cache::new(),
            &mut renderer,
        );
        let mut messages = Vec::new();
        let mut clipboard = iced::advanced::clipboard::Null;
        let _ = user_interface.update(
            &[iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
                iced::mouse::Button::Left,
            ))],
            iced::mouse::Cursor::Available(iced::Point::new(letter.center_x(), letter.center_y())),
            &mut renderer,
            &mut clipboard,
            &mut messages,
        );
        match label {
            "A" => assert!(
                messages
                    .iter()
                    .any(|message| matches!(message, Message::ToggleAlwaysOnTop)),
                "a press on A must publish ToggleAlwaysOnTop: {messages:?}"
            ),
            "V" => assert!(
                messages
                    .iter()
                    .any(|message| matches!(message, Message::ToggleVisualizer)),
                "a press on V must publish ToggleVisualizer: {messages:?}"
            ),
            _ => assert!(
                messages.is_empty(),
                "a press on {label} must publish nothing: {messages:?}"
            ),
        }
    }
}

// Hiding the visualizer drops the bars from the LCD without clearing the
// stored spectrum, so showing them again draws the current ones.
#[test]
fn visible_spectrum_hides_the_stored_bars_while_the_visualizer_is_off() {
    let spectrum = recording_spectrum();
    assert_eq!(visible_spectrum(Some(spectrum), true), Some(spectrum));
    assert_eq!(visible_spectrum(Some(spectrum), false), None);
    assert_eq!(visible_spectrum(None, true), None);
    assert_eq!(visible_spectrum(None, false), None);
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
        Cow::Borrowed(""),
        Cow::Borrowed("3:30"),
        0,
        None,
        None,
        TransportMode::Playing,
        true,
    )
    .into();
    let capture = layout_capture::capture_text(&mut element, &());

    assert_eq!(
        capture.texts().take(6).collect::<Vec<_>>(),
        vec![
            "O",
            "A",
            "I",
            "D",
            "V",
            transport_glyph(TransportMode::Playing)
        ]
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

// Each transport state draws its own glyph in its documented color: green for
// playing and paused, red for stopped. Pin the exact strings and colors so a
// swap cannot ship unnoticed.
#[test]
fn transport_glyph_names_and_colors_each_mode() {
    assert_eq!(transport_glyph(TransportMode::Playing), "▶");
    assert_eq!(transport_glyph(TransportMode::Paused), "II");
    assert_eq!(transport_glyph(TransportMode::Stopped), "■");

    assert_eq!(
        transport_glyph_color(TransportMode::Playing),
        theme::LCD_GREEN
    );
    assert_eq!(
        transport_glyph_color(TransportMode::Paused),
        theme::LCD_GREEN
    );
    assert_eq!(
        transport_glyph_color(TransportMode::Stopped),
        theme::LCD_RED
    );
}

// Only the paused readout's hidden phase and a stopped readout with a real
// time are transparent: a paused visible phase, both playing phases, and the
// stopped `--:--` placeholder stay green. Pin the exact transparent color so a
// refactor cannot blank the readout by hiding the whole row.
#[test]
fn elapsed_readout_color_blinks_when_paused_and_clears_when_stopped() {
    let transparent = Color::from_rgba(0.0, 0.0, 0.0, 0.0);
    let elapsed = "00:53";

    assert_eq!(
        elapsed_readout_color(TransportMode::Paused, false, elapsed),
        transparent
    );
    assert_eq!(
        elapsed_readout_color(TransportMode::Paused, true, elapsed),
        theme::LCD_GREEN
    );
    assert_eq!(
        elapsed_readout_color(TransportMode::Playing, false, elapsed),
        theme::LCD_GREEN
    );
    assert_eq!(
        elapsed_readout_color(TransportMode::Playing, true, elapsed),
        theme::LCD_GREEN
    );
    assert_eq!(
        elapsed_readout_color(TransportMode::Stopped, false, elapsed),
        transparent
    );
    assert_eq!(
        elapsed_readout_color(TransportMode::Stopped, true, elapsed),
        transparent
    );
}

// The `--:--` placeholder says there is no track or no known time, so a
// stopped player must keep showing it instead of clearing it like a real
// elapsed value. The paused hidden phase still blinks it, as before.
#[test]
fn elapsed_readout_color_keeps_the_placeholder_visible_while_stopped() {
    let transparent = Color::from_rgba(0.0, 0.0, 0.0, 0.0);

    assert_eq!(
        elapsed_readout_color(TransportMode::Stopped, false, "--:--"),
        theme::LCD_GREEN
    );
    assert_eq!(
        elapsed_readout_color(TransportMode::Stopped, true, "--:--"),
        theme::LCD_GREEN
    );
    assert_eq!(
        elapsed_readout_color(TransportMode::Paused, false, "--:--"),
        transparent
    );
}

// The stopped readout must not react to a click, so it cannot switch the
// persisted elapsed/remaining mode while it clears the time (the `--:--`
// placeholder aside); the playing and paused readouts still toggle it.
#[test]
fn elapsed_readout_is_clickable_only_while_not_stopped() {
    assert!(elapsed_readout_clickable(TransportMode::Playing));
    assert!(elapsed_readout_clickable(TransportMode::Paused));
    assert!(!elapsed_readout_clickable(TransportMode::Stopped));
}

// The row must wire that predicate up, not just expose it: drive a real left
// press at the elapsed text's bounds and read the messages it publishes. A
// stopped press must publish none, while playing and paused publish
// `ToggleTimeDisplay`.
#[test]
fn elapsed_readout_toggles_only_while_not_stopped() {
    let mut renderer = layout_capture::headless_renderer();
    for (mode, expected) in [
        (TransportMode::Playing, true),
        (TransportMode::Paused, true),
        (TransportMode::Stopped, false),
    ] {
        let mut element: iced::Element<'_, Message, iced::Theme, iced::Renderer> = now_playing_row(
            Cow::Borrowed("00:53"),
            Cow::Borrowed("Opening"),
            Cow::Borrowed("Band"),
            Cow::Borrowed(""),
            Cow::Borrowed("3:30"),
            0,
            mode,
            true,
        )
        .into();
        let capture = layout_capture::capture_text(&mut element, &renderer);
        let elapsed = capture.find("00:53");
        let mut user_interface = iced_runtime::user_interface::UserInterface::build(
            element,
            layout_capture::limits().max(),
            iced_runtime::user_interface::Cache::new(),
            &mut renderer,
        );
        let mut messages = Vec::new();
        let mut clipboard = iced::advanced::clipboard::Null;
        let _ = user_interface.update(
            &[iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
                iced::mouse::Button::Left,
            ))],
            iced::mouse::Cursor::Available(iced::Point::new(
                elapsed.center_x(),
                elapsed.center_y(),
            )),
            &mut renderer,
            &mut clipboard,
            &mut messages,
        );
        let toggles = messages
            .iter()
            .filter(|message| matches!(message, Message::ToggleTimeDisplay))
            .count();
        assert_eq!(
            toggles,
            usize::from(expected),
            "a press on the {mode:?} readout must {} publish ToggleTimeDisplay",
            if expected { "still" } else { "not" }
        );
        if !expected {
            assert!(
                messages.is_empty(),
                "the stopped readout published {messages:?}"
            );
        }
    }
}

// The hidden blink phase draws the elapsed text transparent instead of
// removing it, so the text keeps the same bounds and the title, total, and
// status row stay in place.
#[test]
fn now_playing_row_keeps_the_elapsed_bounds_when_the_blink_hides_it() {
    let capture = |visible: bool| {
        let mut element: iced::Element<'_, Message, iced::Theme, ()> = now_playing_row(
            Cow::Borrowed("00:53"),
            Cow::Borrowed("Opening"),
            Cow::Borrowed("Band"),
            Cow::Borrowed(""),
            Cow::Borrowed("3:30"),
            0,
            TransportMode::Paused,
            visible,
        )
        .into();
        layout_capture::capture_text(&mut element, &())
    };

    let visible = capture(true);
    let hidden = capture(false);

    assert_eq!(visible.find("00:53"), hidden.find("00:53"));
    assert_eq!(
        visible.texts().collect::<Vec<_>>(),
        hidden.texts().collect::<Vec<_>>()
    );
}

// The glyph sits in a fixed-width slot, so switching from `▶` to `II` or `■`
// must not move the large elapsed readout (or, through it, the title line).
// A stopped readout with a real time is drawn transparent but still laid out
// at the same bounds, so its equal-x assertion pins that stopping does not
// move it.
#[test]
fn now_playing_row_keeps_the_elapsed_readout_in_place_across_transport_modes() {
    let mut elapsed_x: Option<f32> = None;
    for mode in [
        TransportMode::Playing,
        TransportMode::Paused,
        TransportMode::Stopped,
    ] {
        let mut element: iced::Element<'_, Message, iced::Theme, ()> = now_playing_row(
            Cow::Borrowed("00:53"),
            Cow::Borrowed("Opening"),
            Cow::Borrowed("Band"),
            Cow::Borrowed(""),
            Cow::Borrowed("3:30"),
            0,
            mode,
            true,
        )
        .into();
        let capture = layout_capture::capture_text(&mut element, &());
        let glyph = capture.find(transport_glyph(mode));
        let elapsed = capture.find("00:53");
        assert!(
            glyph.x < elapsed.x,
            "the glyph must draw before the elapsed readout in {mode:?}: {glyph:?} {elapsed:?}"
        );
        match elapsed_x {
            Some(expected) => assert!(
                (elapsed.x - expected).abs() < 0.5,
                "the elapsed readout moved in {mode:?}: {elapsed:?} vs x={expected}"
            ),
            None => elapsed_x = Some(elapsed.x),
        }
    }
}

// The bar's own measure of the line: the playlist number prefix's characters
// plus the label's, the artist's, and the `" - "` separator between them. The
// view windows the line when the measure exceeds the LCD's budget, and the
// marquee timer runs only while it does — so a measure that disagreed with the
// view's would leave the timer scrolling a line the bar shows whole, or idle
// while the bar windows one. The prefix counts, since the view draws it on the
// same line.
#[test]
fn display_len_counts_the_number_the_label_the_artist_and_the_separator() {
    assert_eq!(display_len("Opening", "", ""), 7);
    assert_eq!(display_len("Opening", "The Sample Band", ""), 25);
    // The prefix adds its own characters (`"12. "` is four).
    assert_eq!(display_len("Opening", "The Sample Band", "1. "), 28);
    assert_eq!(display_len("Opening", "", "12. "), 11);
    // The separator is three characters, and a non-ASCII artist counts
    // characters rather than bytes.
    assert_eq!(display_len("Café", "Ångström", ""), 4 + 8 + 3);
}

// The line needs the marquee only when it does not fit the LCD's character
// budget: a fitting line is shown whole, so the timer would have nothing to
// scroll.
#[test]
fn needs_marquee_only_for_a_line_too_long_for_the_lcd() {
    // A label alone fills the budget exactly and still fits.
    assert!(!needs_marquee(&"x".repeat(MARQUEE_WIDTH), "", ""));
    assert!(needs_marquee(&"x".repeat(MARQUEE_WIDTH + 1), "", ""));
    // With an artist, the artist and the `" - "` separator count toward the
    // budget: "The Sample Band" is 15 characters, so a 4-character label fits
    // the 22-character budget and a 5-character one does not.
    let artist = "The Sample Band";
    assert!(!needs_marquee(&"x".repeat(4), artist, ""));
    assert!(needs_marquee(&"x".repeat(5), artist, ""));
    // The prefix counts too: a 4-character label that just fits with this
    // artist needs the marquee once a `"1. "` prefix joins it.
    assert!(!needs_marquee(&"x".repeat(4), artist, ""));
    assert!(needs_marquee(&"x".repeat(4), artist, "1. "));
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

    let first = captured_now_playing_texts("00:53", label, artist, "", 0, TransportMode::Playing);
    let second = captured_now_playing_texts("00:53", label, artist, "", 1, TransportMode::Playing);
    assert!(first.iter().any(|text| text == &first_window), "{first:?}");
    assert!(
        second.iter().any(|text| text == &second_window),
        "{second:?}"
    );

    // A line that fits still renders the separate artist and title at every
    // offset: "Band" (4) plus the 3-character separator plus "Opening" (7)
    // is 14 characters, within the 22-character budget.
    let still_artist = "Band";
    let still_a = captured_now_playing_texts(
        "00:00",
        "Opening",
        still_artist,
        "",
        0,
        TransportMode::Playing,
    );
    let still_b = captured_now_playing_texts(
        "00:00",
        "Opening",
        still_artist,
        "",
        5,
        TransportMode::Playing,
    );
    assert_eq!(still_a, still_b);
    assert!(still_a.iter().any(|text| text == still_artist));
    assert!(still_a.iter().any(|text| text == "Opening"));
}

// The status row's prefix names the encoded bitrate and the sample rate in
// whole kilohertz: 44.1 kHz truncates to 44, 48 kHz reads 48, the bitrate
// value is kept, and an unknown bitrate is omitted.
#[test]
fn format_playback_prefix_formats_khz_and_bitrate() {
    assert_eq!(
        format_playback_prefix(PlaybackFormat {
            sample_rate_hz: 44_100,
            channels: 2,
            bitrate_kbps: Some(256),
        }),
        "256 KBPS  44 KHZ"
    );
    assert_eq!(
        format_playback_prefix(PlaybackFormat {
            sample_rate_hz: 44_100,
            channels: 2,
            bitrate_kbps: None,
        }),
        "44 KHZ"
    );
    assert_eq!(
        format_playback_prefix(PlaybackFormat {
            sample_rate_hz: 48_000,
            channels: 2,
            bitrate_kbps: None,
        }),
        "48 KHZ"
    );
}

// The channel label mapping follows the reported channel count: one channel
// is mono, and any other count is stereo.
#[test]
fn channel_mode_label_names_the_reported_channels() {
    assert_eq!(channel_mode_label(1), "MONO");
    assert_eq!(channel_mode_label(2), "STEREO");
    assert_eq!(channel_mode_label(0), "STEREO");
}

// The status row lights only the reported channel mode: stereo lights
// `STEREO` and dims `MONO`, mono does the reverse, and no reported format
// leaves both labels dim. The exact palette values are pinned here too, so a
// recolored dim green is caught rather than shipped.
#[test]
fn channel_label_color_lights_only_the_reported_mode() {
    assert_eq!(channel_label_color(Some(2), "STEREO"), theme::LCD_GREEN);
    assert_eq!(channel_label_color(Some(2), "MONO"), theme::LCD_GREEN_DIM);
    assert_eq!(channel_label_color(Some(1), "MONO"), theme::LCD_GREEN);
    assert_eq!(channel_label_color(Some(1), "STEREO"), theme::LCD_GREEN_DIM);
    assert_eq!(channel_label_color(None, "MONO"), theme::LCD_GREEN_DIM);
    assert_eq!(channel_label_color(None, "STEREO"), theme::LCD_GREEN_DIM);
    assert_eq!(theme::LCD_GREEN, iced::Color::from_rgb(0.0, 1.0, 0.0));
    assert_eq!(theme::LCD_GREEN_DIM, iced::Color::from_rgb(0.0, 0.35, 0.0));
}

/// Lays the LCD status row out with the headless tiny-skia renderer and returns
/// the text fragments it draws, the RGBA pixels it paints, and its height.
fn captured_status_row(
    format: Option<PlaybackFormat>,
) -> (layout_capture::TextCapture, layout_capture::PixelCapture) {
    let mut renderer = layout_capture::headless_renderer();
    let mut element: iced::Element<'_, Message, iced::Theme, iced::Renderer> =
        now_playing_status_row(format).into();
    layout_capture::capture_text_and_pixels(&mut element, &mut renderer, &iced::Theme::Dark)
}

// The status row lays out in the mockup's two ends: the bitrate/sample-rate
// prefix at the left edge and both channel labels at the right. It lights only
// the reported channel mode by drawing it brighter green than the other label,
// so a row that drew both labels lit would fail here even though its text
// still matched. With no reported format both labels stay dim, and the row
// keeps its fixed height either way so the bar does not jump when a preview
// loads.
#[test]
fn now_playing_status_row_lights_the_reported_channel_mode() {
    let (capture, pixels) = captured_status_row(Some(PlaybackFormat {
        sample_rate_hz: 44_100,
        channels: 2,
        bitrate_kbps: Some(256),
    }));
    assert_eq!(
        capture.texts().collect::<Vec<_>>(),
        vec!["256 KBPS  44 KHZ", "MONO", "STEREO"]
    );
    let prefix = capture.find("256 KBPS  44 KHZ");
    let mono = capture.find("MONO");
    let stereo = capture.find("STEREO");
    assert!(
        prefix.x < 10.0,
        "the prefix must sit at the row's left edge: {prefix:?}"
    );
    assert!(
        mono.x > prefix.x + prefix.width,
        "the labels must follow the prefix, not overlap it: prefix {prefix:?}, mono {mono:?}"
    );
    assert!(
        stereo.x + stereo.width > 990.0,
        "the label pair must sit at the row's right edge: {stereo:?}"
    );

    let stereo_green = pixels.max_green_in(stereo);
    let mono_green = pixels.max_green_in(mono);
    assert!(
        stereo_green > mono_green + 100,
        "stereo must be lit and mono dim: stereo green {stereo_green}, mono green {mono_green}"
    );

    let (mono_capture, mono_pixels) = captured_status_row(Some(PlaybackFormat {
        sample_rate_hz: 44_100,
        channels: 1,
        bitrate_kbps: None,
    }));
    let lit_mono = mono_pixels.max_green_in(mono_capture.find("MONO"));
    let dim_stereo = mono_pixels.max_green_in(mono_capture.find("STEREO"));
    assert!(
        lit_mono > dim_stereo + 100,
        "mono must be lit and stereo dim: mono green {lit_mono}, stereo green {dim_stereo}"
    );

    let (blank, blank_pixels) = captured_status_row(None);
    assert_eq!(blank.texts().collect::<Vec<_>>(), vec!["MONO", "STEREO"]);
    let blank_mono = blank_pixels.max_green_in(blank.find("MONO"));
    let blank_stereo = blank_pixels.max_green_in(blank.find("STEREO"));
    assert!(
        blank_mono < 128 && blank_stereo < 128,
        "no reported format must leave both labels dim: mono green {blank_mono}, \
         stereo green {blank_stereo}"
    );
    assert!(
        blank.find("STEREO").x + blank.find("STEREO").width > 990.0,
        "the dim label pair must still sit at the row's right edge"
    );
    assert_eq!(
        capture.size().height,
        blank.size().height,
        "the status row must keep its height whether or not a format is reported"
    );
    assert_eq!(capture.size().height, 16.0, "the status row's fixed height");
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

// The clamp's lower edge: a spectrum carrying a negative band must not draw a
// bar below the strip. The test above pins the upper edge, so a regression to
// `min(1.0)` would keep every existing assertion green while a negative band
// produced a negative height.
#[test]
fn visualizer_bar_heights_floors_a_negative_band_at_zero() {
    let mut bands = [0.0; BAND_COUNT];
    bands[0] = -1.0;
    bands[1] = -0.5;
    bands[2] = 0.25;
    let heights = visualizer_bar_heights(Some(Spectrum { bands }));

    assert_eq!(
        &heights[..3],
        &[0.0, 0.0, 6.0],
        "a negative band must clamp to the strip's bottom"
    );
}

// `f32::clamp` passes NaN through unchanged, and a NaN height would poison the
// visualizer row's layout. A stub backend can report one, so the documented
// `0.0..=1.0` clamp must map it to an empty bar like the shared NaN-safe clamp
// does for the sliders. Pin it beside a finite band so the rest of the row
// still scales normally.
#[test]
fn visualizer_bar_heights_maps_a_nan_band_to_empty() {
    let mut bands = [0.0; BAND_COUNT];
    bands[0] = f32::NAN;
    bands[1] = 0.5;
    let heights = visualizer_bar_heights(Some(Spectrum { bands }));

    assert_eq!(heights[0], 0.0, "a NaN band must draw an empty bar");
    assert_eq!(heights[1], 12.0);
    assert!(
        heights.iter().all(|height| height.is_finite()),
        "every bar height must be finite: {heights:?}"
    );
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
        Cow::Borrowed(""),
        Cow::Borrowed("3:30"),
        0,
        Some(PlaybackFormat {
            sample_rate_hz: 44_100,
            channels: 2,
            bitrate_kbps: Some(256),
        }),
        spectrum,
        TransportMode::Playing,
        true,
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
        reporting.iter().any(|text| text == "256 KBPS  44 KHZ"),
        "the status row must still draw its prefix: {reporting:?}"
    );
    assert!(
        CHANNEL_LABELS
            .iter()
            .all(|label| reporting.iter().any(|text| text == label)),
        "the status row must draw both channel labels: {reporting:?}"
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
    let status_y = fragment_y("256 KBPS  44 KHZ");
    assert!(
        status_y - title_y >= 24.0,
        "the 24px visualizer row must sit between the title and status rows: \
         title y {title_y}, status y {status_y}"
    );
    for label in CHANNEL_LABELS {
        let label_y = fragment_y(label);
        assert!(
            label_y - title_y >= 24.0,
            "the {label} label must also sit below the visualizer row: \
             title y {title_y}, label y {label_y}"
        );
    }

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
    let texts = captured_now_playing_texts("00:00", "Opening", "", "", 0, TransportMode::Playing);

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

    let texts = captured_now_playing_texts("00:00", label, "", "", 0, TransportMode::Playing);

    assert!(
        texts.iter().any(|text| text == &window),
        "the window must be taken over the title alone: {texts:?}"
    );
    assert!(
        !texts.iter().any(|text| text == " - "),
        "a track without an artist must not draw the separator: {texts:?}"
    );
}

// The playlist number comes between the caption and the artist, not before
// the caption or after the title: that is the mockup's `Now Playing: 1. The
// Sample Band - Opening` order. The fragments are read in draw order, so the
// indices pin the position as well as the presence of each string.
#[test]
fn now_playing_row_draws_the_number_between_the_caption_and_the_artist() {
    // A short line so the row takes its fitting branch and draws the number as
    // its own fragment; the overlong branch is pinned separately below.
    let texts =
        captured_now_playing_texts("00:00", "One", "Band", "1. ", 0, TransportMode::Playing);

    let caption = texts
        .iter()
        .position(|text| text == "Now Playing: ")
        .expect("the caption must be drawn");
    let number = texts
        .iter()
        .position(|text| text == "1. ")
        .expect("the number must be drawn");
    let artist = texts
        .iter()
        .position(|text| text == "Band")
        .expect("the artist must be drawn");
    let title = texts
        .iter()
        .position(|text| text == "One")
        .expect("the title must be drawn");
    assert!(
        caption < number && number < artist && artist < title,
        "expected caption, number, artist, title in order: {texts:?}"
    );
}

// An overlong line scrolls one window that includes the number prefix: the
// window is taken over the same `"<n>. <artist> - <title>"` string the bar
// draws whole when it fits, so the prefix cannot overflow the budget or stay
// fixed while the rest scrolls.
#[test]
fn an_overlong_now_playing_line_windows_the_number_with_the_title() {
    let label = "A Title Much Longer Than The Display";
    let artist = "The Sample Band";
    let number = "1. ";
    let display = format!("{number}{artist} - {label}");
    let window = marquee_text(&display, 0, MARQUEE_WIDTH);
    assert!(
        window.starts_with(number),
        "the window must include the prefix: {window:?}"
    );

    let texts =
        captured_now_playing_texts("00:00", label, artist, number, 0, TransportMode::Playing);

    assert!(
        texts.iter().any(|text| text == &window),
        "the window must be taken over the prefixed line: {texts:?}"
    );
    assert!(
        !texts.iter().any(|text| text == number),
        "the scrolling branch must not draw the prefix separately: {texts:?}"
    );
}

// With no known number the row draws no prefix at all, whether the track has
// no artist, is unknown, or none is current. The row only pushes the fragment
// when the resolved number is non-empty, matching `now_playing_number`'s
// empty fallbacks.
#[test]
fn now_playing_row_draws_no_number_when_none_is_known() {
    let looks_like_number = |text: &str| {
        text.strip_suffix(". ")
            .is_some_and(|digits| !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()))
    };

    let artist_less =
        captured_now_playing_texts("00:00", "Opening", "", "", 0, TransportMode::Playing);
    assert!(
        !artist_less.iter().any(|text| looks_like_number(text)),
        "an artist-less track with no number must draw no prefix: {artist_less:?}"
    );

    let tracks = known_tracks(&[("song-1", "The Sample Band", "Opening")]);
    for current_track in [Some("no-such-song"), None] {
        let label = now_playing_label(&tracks, current_track);
        let artist = now_playing_artist(&tracks, current_track);
        let number = now_playing_number(&tracks, current_track);
        assert_eq!(number, "");
        let texts = captured_now_playing_texts(
            "00:00",
            &label,
            &artist,
            &number,
            0,
            TransportMode::Playing,
        );
        assert!(
            !texts.iter().any(|text| looks_like_number(text)),
            "an unknown or absent track must draw no prefix: {texts:?}"
        );
    }
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
fn play_pause_glyph_and_word_mirror_playing_state() {
    assert_eq!(play_pause_glyph(true), "II");
    assert_eq!(play_pause_glyph(false), "▶");
    assert_eq!(play_pause_word(true), "Pause");
    assert_eq!(play_pause_word(false), "Play");
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
    let _artists = view_artists(artists, epoch, None, loading, None);
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
        TransportMode::Stopped,
        true,
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
        TransportMode::Paused,
        true,
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
        TransportMode::Playing,
        true,
    );
    let _back = view_back_button();
    let _retry = view_retry_button();
    let _open = view_open_button();
    let _nav = view_browse_nav_buttons();
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
// clear it while the glyph clipped or the row moved. Pin the face widths in
// row order: Previous 34, Play/Pause 30, Stop 30, Next 34, Repeat 104,
// Shuffle 112. Each is the width of that button's widest face, so a
// deliberate glyph or width change must update this pin.
#[test]
fn transport_buttons_keep_their_measured_face_widths() {
    let widths: Vec<Length> = transport_buttons(false, false, false)
        .iter()
        .map(|button| button.as_widget().size().width)
        .collect();
    assert_eq!(
        widths,
        [
            Length::Fixed(34.0),
            Length::Fixed(30.0),
            Length::Fixed(30.0),
            Length::Fixed(34.0),
            Length::Fixed(104.0),
            Length::Fixed(112.0),
        ]
    );
}

// The four glyph buttons share the shade strip's spec, in the mockup's order:
// Previous, Play/Pause, Stop, Next. Pin each glyph, tooltip word, width, and
// message so a swapped face or reordered pair fails here.
#[test]
fn transport_glyphs_match_the_mockup_order_and_widths() {
    let stopped = transport_glyphs(false);
    let glyphs: Vec<&str> = stopped.iter().map(|(glyph, _, _, _)| *glyph).collect();
    assert_eq!(glyphs, ["◀◀", "▶", "■", "▶▶"]);
    let words: Vec<&str> = stopped.iter().map(|(_, word, _, _)| *word).collect();
    assert_eq!(words, ["Previous", "Play", "Stop", "Next"]);
    let widths: Vec<f32> = stopped.iter().map(|(_, _, width, _)| *width).collect();
    assert_eq!(widths, [34.0, 30.0, 30.0, 34.0]);
    assert!(matches!(stopped[0].3, Message::PreviousTrack));
    assert!(matches!(stopped[1].3, Message::PlayPause));
    assert!(matches!(stopped[2].3, Message::Stop));
    assert!(matches!(stopped[3].3, Message::NextTrack));

    let playing = transport_glyphs(true);
    let playing_glyphs: Vec<&str> = playing.iter().map(|(glyph, _, _, _)| *glyph).collect();
    assert_eq!(playing_glyphs, ["◀◀", "II", "■", "▶▶"]);
    let playing_words: Vec<&str> = playing.iter().map(|(_, word, _, _)| *word).collect();
    assert_eq!(playing_words, ["Previous", "Pause", "Stop", "Next"]);
    let playing_widths: Vec<f32> = playing.iter().map(|(_, _, width, _)| *width).collect();
    assert_eq!(
        playing_widths, widths,
        "the glyph swap must not resize a button"
    );
}

#[test]
fn transport_controls_construct_for_both_play_states_volume_endpoints_and_repeat_states() {
    // `view()` passes the shared state's `is_playing`, clamped `volume`,
    // `balance`, `repeat`, and `shuffle`, plus the player's EQ-panel
    // visibility, straight through, so build the transport row for every
    // value the update arms can store, in every play, repeat, shuffle, and
    // panel-visibility state.
    for volume in [0.0, 0.5, 1.0] {
        for balance in [-1.0, 0.0, 1.0] {
            for is_playing in [false, true] {
                for repeat in [false, true] {
                    for shuffle in [false, true] {
                        for eq_panel_visible in [false, true] {
                            let _controls = view_transport_controls(
                                is_playing,
                                volume,
                                balance,
                                repeat,
                                shuffle,
                                eq_panel_visible,
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn eq_panel_toggle_targets_the_visibility_flag() {
    let (label, width, message) = eq_panel_toggle();

    assert_eq!(label, "EQ");
    assert_eq!(width, EQ_PANEL_BUTTON_WIDTH);
    assert_eq!(width, 30.0);
    assert!(
        matches!(message, Message::ToggleEqualizerPanel),
        "the EQ button must toggle the equalizer panel, not the equalizer's on/off state"
    );
}

// The EQ button follows the title bar's A toggle rule: sunken while the thing
// it controls is on, ordinary chrome while it is off. Read the style helper's
// fields against the shared chrome faces so a changed face, edge, or shadow
// cannot leave the button reading wrong.
#[test]
fn eq_panel_button_style_sinks_while_the_panel_is_open() {
    for status in [
        iced::widget::button::Status::Active,
        iced::widget::button::Status::Hovered,
        iced::widget::button::Status::Disabled,
    ] {
        let open = eq_panel_button_style(true, status);
        let pressed = style::chrome_button_style(iced::widget::button::Status::Pressed);
        assert_eq!(open.background, pressed.background);
        assert_eq!(open.text_color, pressed.text_color);
        assert_eq!(open.border, pressed.border);
        assert_eq!(open.shadow, pressed.shadow);

        let closed = eq_panel_button_style(false, status);
        let ordinary = style::chrome_button_style(status);
        assert_eq!(closed.background, ordinary.background);
        assert_eq!(closed.text_color, ordinary.text_color);
        assert_eq!(closed.border, ordinary.border);
        assert_eq!(closed.shadow, ordinary.shadow);
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

/// Asserts the vertical direction a wheel mapping produces, without needing
/// `Message` to implement equality.
fn assert_wheel_direction(message: Message, expected: f32) {
    match message {
        Message::VolumeWheel(direction) => assert_eq!(direction, expected),
        other => panic!("expected a volume wheel message, got {other:?}"),
    }
}

// Both wheel delta kinds carry their vertical direction through as a sign:
// positive scrolls up (raise), negative scrolls down (lower), and the
// magnitude is not a step count. Line and pixel deltas map alike.
#[test]
fn volume_wheel_message_maps_a_vertical_roll_to_a_direction() {
    assert_wheel_direction(
        volume_wheel_message(iced::mouse::ScrollDelta::Lines { x: 0.0, y: 3.0 }),
        1.0,
    );
    assert_wheel_direction(
        volume_wheel_message(iced::mouse::ScrollDelta::Lines { x: 0.0, y: -2.0 }),
        -1.0,
    );
    assert_wheel_direction(
        volume_wheel_message(iced::mouse::ScrollDelta::Pixels { x: 0.0, y: 120.0 }),
        1.0,
    );
    assert_wheel_direction(
        volume_wheel_message(iced::mouse::ScrollDelta::Pixels { x: 0.0, y: -45.0 }),
        -1.0,
    );
}

// A horizontal-only roll has no vertical delta, so it maps to the no-op
// direction rather than borrowing the horizontal sign.
#[test]
fn volume_wheel_message_zeroes_a_horizontal_only_roll() {
    assert_wheel_direction(
        volume_wheel_message(iced::mouse::ScrollDelta::Lines { x: 4.0, y: 0.0 }),
        0.0,
    );
    assert_wheel_direction(
        volume_wheel_message(iced::mouse::ScrollDelta::Pixels { x: 30.0, y: 0.0 }),
        0.0,
    );
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

// The mockup draws the preamp as a vertical slider in the band row with a dB
// scale beside it, not as a horizontal slider above the curve. Lay out the
// real panel and pin the scale's top-to-bottom order, the preamp label's
// baseline sharing the band labels' row, and the scale sitting left of the
// first band.
#[test]
fn equalizer_preamp_and_db_scale_sit_left_of_the_band_sliders() {
    let bands = [0.0; BAND_COUNT];
    let mut element = view_equalizer(false, 0.0, &bands, None);
    let capture = layout_capture::capture_text(&mut element, &layout_capture::headless_renderer());

    let plus_twelve = capture.find("+12 DB");
    let zero = capture.find("0 DB");
    let minus_twelve = capture.find("-12 DB");
    assert!(
        plus_twelve.y < zero.y && zero.y < minus_twelve.y,
        "the dB scale reads top to bottom: +12 {:?}, 0 {:?}, -12 {:?}",
        plus_twelve.y,
        zero.y,
        minus_twelve.y
    );
    assert!(
        minus_twelve.y - plus_twelve.y <= EQ_SLIDER_HEIGHT,
        "the dB scale fits one slider's height: +12 {:?}, -12 {:?}",
        plus_twelve.y,
        minus_twelve.y
    );

    let preamp = capture.find("Preamp");
    let first_frequency = capture.find(BAND_FREQUENCIES[0]);
    assert!(
        (preamp.y - first_frequency.y).abs() <= 0.5,
        "the preamp label sits in the band row: {preamp:?} vs {first_frequency:?}"
    );

    for scale in [plus_twelve, zero, minus_twelve] {
        assert!(
            scale.x + scale.width <= first_frequency.x,
            "the dB scale sits left of the first band: {scale:?} vs {first_frequency:?}"
        );
    }

    // The curve now shares the band row's left edge instead of starting over
    // the preamp and scale, so its first column must sit on the first band.
    let mut tree = Tree::new(element.as_widget());
    let renderer = layout_capture::headless_renderer();
    let node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &layout_capture::limits());
    let mut containers = CapturedContainers::default();
    element
        .as_widget_mut()
        .operate(&mut tree, Layout::new(&node), &renderer, &mut containers);
    let first_segment = containers
        .0
        .iter()
        .filter(|bounds| bounds.height == RESPONSE_CURVE_LINE_HEIGHT)
        .min_by(|left, right| left.x.total_cmp(&right.x))
        .copied()
        .expect("the curve draws a segment");
    assert!(
        (first_segment.x - first_frequency.x).abs() <= 0.5,
        "the curve starts at the first band: {first_segment:?} vs {first_frequency:?}"
    );
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
// row's glyph: it switches with playback so the rolled-up window still shows
// what pressing it will do.
#[test]
fn shade_play_pause_glyph_switches_with_playback_state() {
    assert_eq!(play_pause_glyph(false), "▶");
    assert_eq!(play_pause_glyph(true), "II");
}

// The main row and both compact strips must share one spec: same glyphs in the
// same order, same fixed widths, same messages. Pin the compact projection
// against `transport_glyphs` in both playback states.
#[test]
fn main_transport_and_shade_strip_share_glyphs_order_and_widths() {
    for is_playing in [false, true] {
        let full = transport_glyphs(is_playing);
        let shade = mini_transport_controls(is_playing);
        let full_spec: Vec<(&str, f32)> = full
            .iter()
            .map(|(glyph, _, width, _)| (*glyph, *width))
            .collect();
        let shade_spec: Vec<(&str, f32)> = shade
            .iter()
            .map(|(glyph, width, _)| (*glyph, *width))
            .collect();
        assert_eq!(full_spec, shade_spec);
        for ((_, _, _, full_message), (_, _, shade_message)) in full.iter().zip(shade.iter()) {
            assert_eq!(
                std::mem::discriminant(full_message),
                std::mem::discriminant(shade_message)
            );
        }
    }
}

// The four compact buttons must emit the same messages as their full
// transport-row twins, and each keeps a fixed width so the glyph swap cannot
// reflow the strip. Pin both, in row order.
#[test]
fn shade_controls_map_to_transport_messages_and_keep_fixed_widths() {
    let stopped = mini_transport_controls(false);
    let playing = mini_transport_controls(true);

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

    for mode in [
        TransportMode::Stopped,
        TransportMode::Paused,
        TransportMode::Playing,
    ] {
        let mut element =
            view_shade_strip(mode, 0.5, Cow::Borrowed("00:53"), Cow::Borrowed("3:30"));
        let mut tree = Tree::new(element.as_widget());
        let limits = Limits::new(Size::ZERO, Size::new(1000.0, 1000.0));
        let node = element
            .as_widget_mut()
            .layout(&mut tree, &renderer, &limits);
        assert_eq!(
            node.size().height,
            SHADE_STRIP_HEIGHT,
            "the strip must keep its fixed height in {mode:?} mode"
        );
    }
}

/// Lays a compact strip out with the headless tiny-skia renderer and records
/// every text fragment plus the bounds of the container tagged
/// [`MINI_VOLUME_ID`], so a test can see the time readout and the volume
/// slider's place in the real laid-out row.
fn captured_mini_strip(
    mut element: Element<'_, Message>,
) -> (Vec<String>, Option<Rectangle>, Size) {
    #[derive(Default)]
    struct CapturedContainer {
        slider: Option<Rectangle>,
    }
    impl Operation for CapturedContainer {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }
        fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
            if id == Some(&Id::new(MINI_VOLUME_ID)) {
                self.slider = Some(bounds);
            }
        }
    }

    let renderer = layout_capture::headless_renderer();
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

fn captured_shade_strip(elapsed: &str, total: &str) -> (Vec<String>, Option<Rectangle>, Size) {
    captured_mini_strip(view_shade_strip(
        TransportMode::Stopped,
        0.5,
        Cow::Borrowed(elapsed),
        Cow::Borrowed(total),
    ))
}

fn captured_library_strip(elapsed: &str, total: &str) -> (Vec<String>, Option<Rectangle>, Size) {
    captured_mini_strip(view_library_strip(
        TransportMode::Stopped,
        0.5,
        Cow::Borrowed(elapsed),
        Cow::Borrowed(total),
    ))
}

// The compact readout joins the elapsed (or remaining) value to the track's
// total, and keeps the single plain placeholder when no total is known.
#[test]
fn mini_transport_readout_appends_the_known_total() {
    for (elapsed, total, expected) in [
        ("00:53", "3:30", "00:53/3:30"),
        ("-02:37", "3:30", "-02:37/3:30"),
        ("00:53", "--:--", "00:53"),
        ("--:--", "--:--", "--:--"),
    ] {
        let readout = mini_transport_readout(Cow::Borrowed(elapsed), Cow::Borrowed(total));
        assert_eq!(readout, expected, "{elapsed:?} over {total:?}");
    }
}

// Both compact strips must wire the shared guard up, not just expose it: a
// real left press at the readout's bounds publishes exactly one
// `ToggleTimeDisplay` while playing or paused, and none while stopped.
#[test]
fn compact_readout_toggles_time_display_only_while_not_stopped() {
    let mut renderer = layout_capture::headless_renderer();
    for mode in [
        TransportMode::Playing,
        TransportMode::Paused,
        TransportMode::Stopped,
    ] {
        for strip in ["shade", "library"] {
            let mut element: iced::Element<'_, Message, iced::Theme, iced::Renderer> = match strip {
                "shade" => {
                    view_shade_strip(mode, 0.5, Cow::Borrowed("00:53"), Cow::Borrowed("3:30"))
                }
                _ => view_library_strip(mode, 0.5, Cow::Borrowed("00:53"), Cow::Borrowed("3:30")),
            };
            let capture = layout_capture::capture_text(&mut element, &renderer);
            let readout = capture.find("00:53/3:30");
            let mut user_interface = iced_runtime::user_interface::UserInterface::build(
                element,
                layout_capture::limits().max(),
                iced_runtime::user_interface::Cache::new(),
                &mut renderer,
            );
            let mut messages = Vec::new();
            let mut clipboard = iced::advanced::clipboard::Null;
            let _ = user_interface.update(
                &[iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
                    iced::mouse::Button::Left,
                ))],
                iced::mouse::Cursor::Available(iced::Point::new(
                    readout.center_x(),
                    readout.center_y(),
                )),
                &mut renderer,
                &mut clipboard,
                &mut messages,
            );
            let expected = mode != TransportMode::Stopped;
            let toggles = messages
                .iter()
                .filter(|message| matches!(message, Message::ToggleTimeDisplay))
                .count();
            assert_eq!(
                toggles,
                usize::from(expected),
                "a press on the {mode:?} {strip} readout must {} publish ToggleTimeDisplay",
                if expected { "still" } else { "not" }
            );
            if !expected {
                assert!(
                    messages.is_empty(),
                    "the stopped {strip} readout published {messages:?}"
                );
            }
        }
    }
}

// The strip's two non-button controls must actually reach the laid-out row:
// the time readout the caller gives it (elapsed or remaining) and the volume
// slider. The fill spacer pushes the slider to the right edge, so it does not
// sit immediately after the readout on the left.
#[test]
fn shade_strip_draws_the_time_readout_and_puts_the_volume_slider_at_the_right_edge() {
    for time in ["00:53", "-02:37"] {
        let (texts, slider, size) = captured_shade_strip(time, "3:30");
        let expected = format!("{time}/3:30");
        assert!(
            texts.iter().any(|text| text == &expected),
            "missing {expected} readout: {texts:?}"
        );
        let slider = slider.unwrap_or_else(|| panic!("missing volume slider: {texts:?}"));
        assert_eq!(slider.width, MINI_VOLUME_WIDTH);
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
    let (min, max, step, width, on_change) = mini_volume_spec();
    assert_eq!((min, max, step), (VOLUME_MIN, VOLUME_MAX, VOLUME_STEP));
    assert_eq!(width, MINI_VOLUME_WIDTH);
    assert_eq!(MINI_VOLUME_WIDTH, 80.0);
    assert!(matches!(
        on_change(0.5),
        Message::VolumeChange(value) if value == 0.5
    ));
}

// The library footer is a fixed-height strip, so adding it to the normal root
// column cannot let the browse content squeeze it or stretch it.
#[test]
fn view_library_strip_keeps_its_fixed_height() {
    let renderer = layout_capture::headless_renderer();

    for mode in [
        TransportMode::Stopped,
        TransportMode::Paused,
        TransportMode::Playing,
    ] {
        let mut element =
            view_library_strip(mode, 0.5, Cow::Borrowed("00:53"), Cow::Borrowed("3:30"));
        let mut tree = Tree::new(element.as_widget());
        let limits = Limits::new(Size::ZERO, Size::new(1000.0, 1000.0));
        let node = element
            .as_widget_mut()
            .layout(&mut tree, &renderer, &limits);
        assert_eq!(
            node.size().height,
            MINI_TRANSPORT_HEIGHT,
            "the library footer must keep its fixed height in {mode:?} mode"
        );
    }
}

// The footer's four glyphs, time readout, and right-edge volume slider must
// actually reach the laid-out row, in the same order and geometry as the
// shade strip's.
#[test]
fn library_strip_draws_the_four_controls_time_and_right_edge_volume() {
    for time in ["00:53", "-02:37"] {
        let (texts, slider, size) = captured_library_strip(time, "3:30");
        let expected = format!("{time}/3:30");
        let glyphs: Vec<&str> = texts
            .iter()
            .map(String::as_str)
            .filter(|text| ["◀◀", "▶", "■", "▶▶"].contains(text))
            .collect();
        assert_eq!(glyphs, vec!["◀◀", "▶", "■", "▶▶"]);
        assert!(
            texts.iter().any(|text| text == &expected),
            "missing {expected} readout: {texts:?}"
        );
        let slider = slider.unwrap_or_else(|| panic!("missing volume slider: {texts:?}"));
        assert_eq!(slider.width, MINI_VOLUME_WIDTH);
        assert!(
            (slider.x + slider.width - (size.width - 6.0)).abs() < 0.5,
            "slider must sit at the footer's right edge: {slider:?} in {size:?}"
        );
    }
}

// A section band must be exactly the title bar's height and carry only its
// centered label between the two filler tapes. The root-view tests check where
// the bands appear; this pins the band widget's own geometry and that no
// button or stray text rides along with the decorative label.
#[test]
fn title_band_centers_its_label_at_the_title_bar_height() {
    let mut element = view_title_band("NOSTALGIA EQUALIZER");
    let capture = layout_capture::capture_text(&mut element, &layout_capture::headless_renderer());

    assert_eq!(capture.size().height, TITLE_BAR_HEIGHT);
    let texts: Vec<&str> = capture.texts().collect();
    assert_eq!(texts, vec!["NOSTALGIA EQUALIZER"]);

    let label = capture.find("NOSTALGIA EQUALIZER");
    let label_center = label.x + label.width / 2.0;
    let band_center = capture.size().width / 2.0;
    assert!(
        (label_center - band_center).abs() < 0.5,
        "the section label must be centered in its band: {label:?} in {:?}",
        capture.size()
    );
}

// The app bar's drag region spans only the space between the A toggle and the
// shade button. Putting the tapes inside it must center NOSTALGIA in that
// middle span, not in the whole window, while the edge controls keep their
// pinned places.
#[test]
fn title_bar_centers_its_name_between_its_edge_controls() {
    let renderer = layout_capture::headless_renderer();
    let mut element = view_title_bar(false);
    let capture = layout_capture::capture_text(&mut element, &renderer);

    let mut tree = Tree::new(element.as_widget());
    let node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &layout_capture::limits());
    let mut containers = CapturedContainers::default();
    element
        .as_widget_mut()
        .operate(&mut tree, Layout::new(&node), &renderer, &mut containers);

    // The A toggle and the shade button each draw a container at their pinned
    // width. Read the A slot's right edge and the leftmost 22px button's left
    // edge from the laid-out tree rather than from their glyph bounds, whose
    // measured widths vary with the renderer.
    let a_right = containers
        .0
        .iter()
        .filter(|bounds| (bounds.width - CLUTTER_BUTTON_WIDTH).abs() < 0.5)
        .map(|bounds| bounds.x + bounds.width)
        .fold(f32::NEG_INFINITY, f32::max);
    let shade_left = containers
        .0
        .iter()
        .filter(|bounds| (bounds.width - TITLE_BAR_BUTTON_WIDTH).abs() < 0.5)
        .map(|bounds| bounds.x)
        .fold(f32::INFINITY, f32::min);
    let middle_center = (a_right + shade_left) / 2.0;

    let name = capture.find("NOSTALGIA");
    let name_center = name.x + name.width / 2.0;
    assert!(
        (name_center - middle_center).abs() < 0.5,
        "NOSTALGIA must center between the A and shade controls: {name:?}, \
         span {a_right:?}..{shade_left:?}"
    );
}
