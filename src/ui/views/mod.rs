//! Widget construction for the player's UI.
//!
//! Pure functions: each turns plain data — the current track and the loaded
//! songs, the playback state, or `&[Artist]` / `&[Album]` / `&[Song]` /
//! `&[Playlist]` — into an `Element` (a few label helpers return `&str` or
//! `Cow<str>` instead) and knows nothing about the player's state or update
//! loop. Keeping them free of the `WinampPlayer` struct means the view layer
//! can be reworked (or tested) independently of how the app is booted.
//!
//! Each panel lives in its own submodule — `about` (the transient About
//! overlay), `options` (the transient options menu), `browse` (the artist → album →
//! song drill-down, the flat lists, and Playlists, with their list scaffolding
//! in `browse::rows` and pure row wording and labels in `browse::rows::text`),
//! `title_bar`, `shade` (the rolled-up
//! transport strip), `mini_transport` (the compact row the shade strip and the
//! library footer share), `now_playing` (the LCD well, title row, indicator strip, status row, and
//! visualizer row, with its pure label, time, and marquee resolution in
//! `now_playing::text`), `seek_bar`, `transport`, and `equalizer`
//! — while the chrome helpers, slider specs, and track-length formatting the
//! panels share ([`spacer`], [`labeled_button`], [`fixed_width_button`],
//! [`chrome_slider`], and [`format_track_time`]) live here. The transient
//! overlays are `about`, `options`, `file_info`, `jump_time`, and `jump_file`.

use iced::{
    Color, Element, Length,
    mouse::ScrollDelta,
    widget::{Button, Slider, Space, Text},
};

use crate::ui::{Message, style};

mod about;
mod browse;
mod equalizer;
mod file_info;
mod jump_file;
mod jump_time;
mod mini_transport;
mod modal;
mod now_playing;
mod options;
mod seek_bar;
mod shade;
mod title_bar;
mod transport;

pub use about::view_about_overlay;
pub(crate) use browse::artist_album_count_label;
pub use browse::{
    BROWSE_LIST_ID, BROWSE_LIST_PADDING, BROWSE_PAGE_HEIGHT, BROWSE_ROW_HEIGHT, BROWSE_SEARCH_ID,
    can_go_back, can_retry_artists, view_albums, view_all_albums, view_all_albums_button,
    view_all_songs, view_all_songs_button, view_artists, view_back_button, view_browse_nav_buttons,
    view_genres, view_genres_button, view_open_button, view_playlists, view_playlists_button,
    view_recently_added, view_recently_added_button, view_recently_played,
    view_recently_played_button, view_retry_button, view_search_row, view_songs, view_years,
    view_years_button,
};
pub use equalizer::view_equalizer;
pub use file_info::view_file_info_overlay;
pub use jump_file::{
    JUMP_FILE_INPUT_ID, JUMP_FILE_LIST_ID, JUMP_FILE_LIST_PADDING, JUMP_FILE_PAGE_ROWS,
    JUMP_FILE_ROW_HEIGHT, JUMP_FILE_ROW_SPACING, view_jump_file_overlay,
};
#[cfg(test)]
pub(crate) use jump_file::{JUMP_FILE_LIST_HEIGHT, jump_file_row_label, jump_file_row_style};
pub(crate) use jump_file::{jump_file_highlighted, jump_file_matches};
pub use jump_time::{JUMP_TIME_INPUT_ID, view_jump_time_overlay};
pub use mini_transport::view_library_strip;
pub use now_playing::{
    IndicatorModes, KnownTrack, MixerDrag, TimeDisplay, dragged_marquee_offset, marquee_cycle_len,
    mixer_readout_text, needs_marquee, next_spectrum_bars, next_spectrum_peaks, now_playing_artist,
    now_playing_elapsed, now_playing_label, now_playing_number, now_playing_time,
    seek_readout_text, view_now_playing, visible_spectrum,
};
pub use options::{OptionsState, view_options_overlay};
pub use seek_bar::view_seek_bar;
pub use shade::view_shade_strip;
pub use title_bar::{view_title_band, view_title_bar};
pub use transport::view_transport_controls;

#[cfg(test)]
pub(super) use browse::BROWSE_VIEWS;
#[cfg(test)]
pub(super) use equalizer::{EQ_PREAMP_SLIDER_ID, eq_band_slider_id};
#[cfg(test)]
pub(crate) use mini_transport::{MINI_SEEK_ID, MINI_VOLUME_ID};
#[cfg(test)]
pub(super) use seek_bar::{SEEK_BAR_ID, SEEK_STRIP_ID, seek_bar_value};
pub(super) use shade::SHADED_HEIGHT;
#[cfg(test)]
pub(super) use title_bar::TITLE_BAR_HEIGHT;
#[cfg(test)]
pub(super) use transport::BALANCE_SLIDER_ID;

#[cfg(test)]
mod tests;

/// Formats `duration_ms` as zero-padded `m:ss`, with unbounded minutes so an hour or more reads as
/// `60:00`/`61:05` rather than rolling over. This is the shared core of [`format_track_time`],
/// whose zero case is the per-track `"--:--"` placeholder, and the browse song list's
/// total-playing-time footer, whose zero total is a real `"0:00"`, and the Jump to time box's
/// prefill. Shared by the Now Playing bar and the browse rows, so it lives beside the other
/// cross-panel helpers here rather than inside one panel.
pub(in crate::ui) fn format_minutes_seconds(duration_ms: u64) -> String {
    let total_seconds = duration_ms / 1000;
    format!("{}:{:02}", total_seconds / 60, total_seconds % 60)
}

/// A track's length readout for `duration_ms`: `"--:--"` when the source
/// supplied no duration (`0`), else the shared [`format_minutes_seconds`] form
/// (so a track under a minute reads `0:45` and one over an hour keeps counting
/// minutes rather than rolling into hours). Shared by the Now Playing bar and
/// the browse song rows, so it lives here rather than inside one panel. Pure so
/// the formatting is testable without an iced renderer.
pub fn format_track_time(duration_ms: u64) -> String {
    if duration_ms == 0 {
        return "--:--".to_string();
    }
    format_minutes_seconds(duration_ms)
}

/// A fixed-width horizontal gap between adjacent widgets.
///
/// The browse rows (a 10px gap between the title and the secondary label),
/// the transport row (a 20px gap between each control), the equalizer header
/// (an 8px gap between its on/off button and preset pick list), and the
/// compact transport row drawn by the shade strip and the library footer
/// (a 6px gap between its transport buttons) all construct the same `Space`,
/// so the `Space::new().width(Length::Fixed(..))` expression lives here once
/// instead of being repeated at every call site.
fn spacer(width: f32) -> Space {
    Space::new().width(Length::Fixed(width))
}

/// A button showing a single text label that emits `message` on press.
///
/// The browse Back, Open, Top, Sort, Reload, Clear, All Songs, All Albums,
/// Playlists, and Retry buttons call this directly; the equalizer panel's on/off
/// button reaches it through [`toggle_button`];
/// the transport row's seven buttons (Previous, Play, Pause, Stop, Next,
/// Repeat, Shuffle), the five in each compact strip (the shade strip and the
/// library footer), and the title bar's shade, minimize, and close buttons
/// reach it through [`fixed_width_button`]. They all build the same
/// `Button::new(Text::new(..)).on_press(..)` widget, so that expression lives
/// here once instead of being repeated at every call site. The label is
/// `'static` — a literal, or a `&'static str` such as
/// [`transport_glyphs`](transport::transport_glyphs) returns — so the
/// resulting button is `'static` like the view builders that
/// push it.
fn labeled_button(label: &'static str, message: Message) -> Button<'static, Message> {
    Button::new(Text::new(label))
        .on_press(message)
        .style(|_theme, status| style::chrome_button_style(status))
}

/// A [`labeled_button`] pinned to `width`, so changing its label cannot
/// resize it and reflow the widgets beside it in a row.
fn fixed_width_button(
    label: &'static str,
    width: f32,
    message: Message,
) -> Button<'static, Message> {
    labeled_button(label, message).width(Length::Fixed(width))
}

/// A [`labeled_button`] that sinks while `on` (see [`style::toggle_button_style`]), for a toggle
/// whose width follows its label: the equalizer's `EQ: On/Off`.
fn toggle_button(label: &'static str, message: Message, on: bool) -> Button<'static, Message> {
    labeled_button(label, message)
        .style(move |_theme, status| style::toggle_button_style(on, status))
}

/// A [`fixed_width_button`] that sinks while `on` (see [`style::toggle_button_style`]). This is the
/// one place the sunken-on rule is applied to a fixed-width toggle: the transport row's `EQ` and
/// `PL`, Repeat, Shuffle, and the equalizer's `AUTO` build through it (the title bar's `M` applies
/// the same style itself).
fn fixed_width_toggle_button(
    label: &'static str,
    width: f32,
    message: Message,
    on: bool,
) -> Button<'static, Message> {
    fixed_width_button(label, width, message)
        .style(move |_theme, status| style::toggle_button_style(on, status))
}

/// The volume slider's silence floor, the same lower bound
/// `state::clamp_volume` clamps to. Named here so the slider spec is a
/// testable seam; the spec test in the `tests` submodule ties it to the clamp
/// so the two cannot drift apart.
const VOLUME_MIN: f32 = 0.0;

/// The volume slider's full-volume ceiling, the same upper bound
/// `state::clamp_volume` clamps to (see [`VOLUME_MIN`]).
const VOLUME_MAX: f32 = 1.0;

/// The volume slider's drag granularity: one percent, so a drag spans the
/// whole `[VOLUME_MIN, VOLUME_MAX]` scale in fine steps. `pub(super)` so the
/// keyboard volume handlers in `handlers::mixer` nudge by the slider's own
/// step.
pub(super) const VOLUME_STEP: f32 = 0.01;

/// Maps a wheel roll to the slider direction it asks for: `1.0` raises,
/// `-1.0` lowers, and `0.0` is a no-op for a horizontal-only or zero roll.
///
/// iced passes winit's wheel deltas, whose positive `y` is the direction that
/// scrolls up. Only the sign matters, so one event becomes one slider step
/// regardless of whether the backend reports lines or pixels — just like one
/// arrow press. A `NaN` vertical delta compares false both ways, so it is the
/// no-op too.
pub(super) fn wheel_direction(delta: ScrollDelta) -> f32 {
    let y = match delta {
        ScrollDelta::Lines { y, .. } | ScrollDelta::Pixels { y, .. } => y,
    };
    if y > 0.0 {
        1.0
    } else if y < 0.0 {
        -1.0
    } else {
        0.0
    }
}

/// Maps a wheel roll over the player panels to the volume direction it asks
/// for. The browse list sits outside the area that publishes this, so its own
/// `Scrollable` keeps the wheel there; the balance, preamp, and band sliders
/// are their own exceptions (see the sibling helpers below).
pub(super) fn volume_wheel_message(delta: ScrollDelta) -> Message {
    Message::VolumeWheel(wheel_direction(delta))
}

/// Maps a wheel roll over the balance slider to the balance direction it asks
/// for, using the same sign mapping as [`volume_wheel_message`].
pub(super) fn balance_wheel_message(delta: ScrollDelta) -> Message {
    Message::BalanceWheel(wheel_direction(delta))
}

/// Maps a wheel roll over the equalizer's preamp slider to its gain direction.
pub(super) fn eq_preamp_wheel_message(delta: ScrollDelta) -> Message {
    Message::EqPreampWheel(wheel_direction(delta))
}

/// Maps a wheel roll over equalizer band `index` to that band's gain
/// direction.
pub(super) fn eq_band_wheel_message(index: usize, delta: ScrollDelta) -> Message {
    Message::EqBandWheel(index, wheel_direction(delta))
}

/// The left and right arrows' seek granularity: classic Winamp seeks five
/// seconds back or forward per press.
pub(super) const SEEK_STEP_SECS: i64 = 5;

/// The balance slider's hard-left floor, the same lower bound
/// `state::clamp_balance` clamps to. Named here so the slider spec is a
/// testable seam; the spec test in the `tests` submodule ties it to the clamp
/// so the two cannot drift apart.
const BALANCE_MIN: f32 = -1.0;

/// The balance slider's hard-right ceiling, the same upper bound
/// `state::clamp_balance` clamps to (see [`BALANCE_MIN`]).
const BALANCE_MAX: f32 = 1.0;

/// The balance slider's drag granularity: one percent of the scale per step,
/// matching the volume slider's step. `pub(super)` so the wheel handler in
/// `handlers::mixer` nudges by the slider's own step.
pub(super) const BALANCE_STEP: f32 = 0.01;

/// The equalizer sliders' drag granularity: one decibel, so every preamp and
/// band slider lands on a whole-dB gain within `GAIN_MIN_DB..=GAIN_MAX_DB`.
/// `pub(super)` so the wheel handlers in `handlers::mixer` nudge by the
/// sliders' own step.
pub(super) const EQ_STEP: f32 = 1.0;

/// Builds one horizontal chrome slider: it spans `min..=max`, starts at
/// `value`, steps by `step`, is `width` pixels wide, and paints both halves of
/// its rail `fill` with the shared [`style::level_slider_style`]. Dragging it
/// emits `on_change`.
///
/// The transport row's volume and balance sliders and the compact strips'
/// volume slider differ only in those values and their fill (callers pass a
/// level-dependent tint from [`style::volume_level_color`] or
/// [`style::balance_level_color`]), so the construction lives
/// here once. The seek bar and the equalizer's preamp build their own sliders
/// and keep the neutral chrome groove, while the equalizer's band sliders carry
/// the per-band gain tint from [`style::band_slider_style`].
fn chrome_slider(
    min: f32,
    max: f32,
    value: f32,
    step: f32,
    width: f32,
    fill: Color,
    on_change: fn(f32) -> Message,
) -> Element<'static, Message> {
    Slider::new(min..=max, value, on_change)
        .step(step)
        .width(Length::Fixed(width))
        .style(move |_theme, status| style::level_slider_style(fill, status))
        .on_release(Message::MixerReleased)
        .into()
}
