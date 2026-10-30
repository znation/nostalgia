use super::super::{CurrentView, Message, OptionsChoice, style, theme};
use super::browse::rows::text::{
    album_count_label, album_list_summary, album_row, artist_list_summary, artist_row,
    browse_placeholder, empty_list_label, list_count_summary, playlist_row, row_prefix,
    song_list_summary, song_row,
};
use super::browse::rows::{browse_row_style, row_build_count, scrollable_list};
use super::browse::{
    BROWSE_SEARCH_ID, BROWSE_VIEWS, view_clear_search_button, view_reload_button,
    view_save_list_button, view_search_box, view_search_row, view_sort_button, view_top_button,
};
use super::equalizer::{
    AUTO_BUTTON_WIDTH, CUSTOM_PRESET_LABEL, EQ_SLIDER_HEIGHT, RESPONSE_CURVE_COLUMNS,
    RESPONSE_CURVE_GUIDE_THICKNESS, RESPONSE_CURVE_HEIGHT, RESPONSE_CURVE_LINE_HEIGHT, auto_label,
    auto_toggle, eq_enabled_label, eq_toggle, preamp_slider_style, preset_choices,
    preset_pick_list_label, response_curve_offsets, response_curve_row,
};
use super::mini_transport::{
    MINI_SEEK_ID, MINI_TRANSPORT_HEIGHT, MINI_VOLUME_ID, MINI_VOLUME_WIDTH, compact_readout_color,
    mini_transport_controls, mini_transport_readout, mini_volume_spec, view_library_strip,
};
use super::now_playing::indicator::{
    IndicatorModes, LCD_INDICATOR_LABELS, indicator_label_color, lcd_indicator_message,
    lcd_indicator_strip,
};
use super::now_playing::status::{CHANNEL_LABELS, channel_label_color, now_playing_status_row};
use super::now_playing::text::{format_lcd_elapsed, format_lcd_remaining};
use super::now_playing::visualizer::{
    next_spectrum_bars, next_spectrum_peaks, now_playing_visualizer_row, visible_spectrum,
    visualizer_bar_heights, visualizer_bar_segments, visualizer_peak_marker_bottom,
};
use super::now_playing::{
    KnownTrack, MARQUEE_WIDTH, TimeDisplay, channel_mode_label, display_len,
    elapsed_readout_clickable, elapsed_readout_color, format_playback_prefix, marquee_text,
    needs_marquee, now_playing_artist, now_playing_content, now_playing_elapsed, now_playing_label,
    now_playing_number, now_playing_row, now_playing_time, transport_glyph, transport_glyph_color,
};
use super::seek_bar::seek_wheel_message;
use super::shade::{SHADE_VOLUME_RIGHT_INSET, SHADED_HEIGHT, view_shade_strip};
use super::title_bar::{TITLE_BAR_BUTTON_WIDTH, TITLE_BAR_LEFT_BUTTON_WIDTH};
use super::transport::{
    EQ_PANEL_BUTTON_WIDTH, LIBRARY_PANEL_BUTTON_WIDTH, eq_panel_toggle, library_panel_toggle,
    repeat_toggle, shuffle_toggle, transport_buttons, transport_glyphs,
};
use super::{
    BALANCE_MAX, BALANCE_MIN, BALANCE_STEP, EQ_STEP, PANEL_TEXT_SIZE, TITLE_BAR_HEIGHT, VOLUME_MAX,
    VOLUME_MIN, VOLUME_STEP, balance_wheel_message, can_go_back, can_retry_artists,
    eq_band_wheel_message, eq_preamp_wheel_message, fixed_width_button, format_track_time,
    seek_bar_value, spacer, toggle_button, view_albums, view_all_albums, view_all_albums_button,
    view_all_songs, view_all_songs_button, view_artists, view_back_button, view_browse_nav_buttons,
    view_equalizer, view_now_playing, view_open_button, view_playlists, view_playlists_button,
    view_retry_button, view_seek_bar, view_songs, view_title_band, view_title_bar,
    view_transport_controls, volume_wheel_message,
};
use crate::audio::{PlaybackFormat, Spectrum};
use crate::equalizer::{
    BAND_COUNT, BAND_FREQUENCIES, CustomPreset, GAIN_MAX_DB, GAIN_MIN_DB, PRESETS, PresetChoice,
    clamp_gain,
};
use crate::library::{Album, Artist, Playlist, Song};
use crate::sample_library::sample_library;
use crate::state::{TransportMode, clamp_balance, clamp_volume};
use crate::test_support::{
    recording_spectrum, sample_album, sample_artist, sample_playlist, sample_song,
};
use crate::ui::layout_capture;
use iced::advanced::widget::{Id, Tree, operation::Operation};
use iced::{Color, Element, Length, Rectangle, Size};
use std::borrow::Cow;
use std::collections::HashMap;
use std::time::Duration;

/// The bounds of every container a built widget draws, collected through a
/// widget operation. The visualizer's bars are borderless containers with no
/// text, so the bar segments are what a test groups by their x position to pin
/// the bar count and each bar's height.
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
        None,
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
                    album: String::new(),
                    year: None,
                    genre: String::new(),
                    track_number: None,
                    disc_number: None,
                    composer: String::new(),
                    content_rating: None,
                    number: String::new(),
                    time: "--:--".to_string(),
                    duration_ms: 0,
                },
            )
        })
        .collect()
}

/// Asserts that `clamp` stores exactly `min..=max`: both endpoints survive
/// unchanged and a step beyond each is clamped back. This pins a slider's
/// range equal to its shared-state range without reading the iced widget.
fn assert_slider_range_matches_clamp(clamp: fn(f32) -> f32, min: f32, max: f32, step: f32) {
    assert_eq!(clamp(min), min);
    assert_eq!(clamp(max), max);
    assert_eq!(clamp(min - step), min);
    assert_eq!(clamp(max + step), max);
}

/// Whether any pixel in `region` (RGBA bytes) is `color` at 8-bit channels.
fn region_has_color(region: &[u8], color: Color) -> bool {
    let [r, g, b, a] = color.into_rgba8();
    region.chunks_exact(4).any(|pixel| pixel == [r, g, b, a])
}

// The seven transient panels re-based their 18 px texts onto the 8-px grid the
// vendored pixel faces draw crisp at (the pixel-font split's part 4/4); the
// shared constant keeps the claim in one pin instead of ten literals.
#[test]
fn panel_text_size_is_on_the_pixel_grid() {
    assert_eq!(PANEL_TEXT_SIZE, 16.0);
    assert_eq!(
        PANEL_TEXT_SIZE % 8.0,
        0.0,
        "the panels must draw at an 8-px grid size"
    );
}

mod browse;
mod chrome;
mod equalizer;
mod jump_file;
mod lcd_display;
mod nav_buttons;
mod now_playing;
mod seek_wheel;
mod shade;
mod transport;
