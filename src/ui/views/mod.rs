//! Widget construction for the player's UI.
//!
//! Pure functions: each turns plain data — the current track and the loaded
//! songs, the playback state, or `&[Artist]` / `&[Album]` / `&[Song]` /
//! `&[Playlist]` — into an `Element` (a few label helpers return `&str` or
//! `Cow<str>` instead) and knows nothing about the player's state or update
//! loop. Keeping them free of the `WinampPlayer` struct means the view layer
//! can be reworked (or tested) independently of how the app is booted.
//!
//! Each panel lives in its own submodule — `browse` (the artist → album →
//! song drill-down, the flat lists, and Playlists, with their rows and list
//! scaffolding in `browse::rows`), `title_bar`, `shade` (the rolled-up
//! transport strip), `now_playing` (the LCD widgets, with its pure label, time,
//! and marquee resolution in `now_playing::text`), `seek_bar`, `transport`, and `equalizer`
//! — while the chrome helpers, slider specs, and track-length formatting the
//! panels share ([`spacer`], [`labeled_button`], [`fixed_width_button`],
//! [`chrome_slider`], and [`format_track_time`]) live here.

use iced::{
    Color, Element, Length,
    widget::{Button, Slider, Space, Text},
};

use crate::ui::{Message, style};

mod browse;
mod equalizer;
mod now_playing;
mod seek_bar;
mod shade;
mod title_bar;
mod transport;

pub use browse::{
    BROWSE_LIST_ID, BROWSE_LIST_PADDING, BROWSE_ROW_HEIGHT, can_go_back, can_retry_artists,
    view_albums, view_all_albums, view_all_albums_button, view_all_songs, view_all_songs_button,
    view_artists, view_back_button, view_browse_nav_buttons, view_playlists, view_playlists_button,
    view_retry_button, view_search_box, view_songs,
};
pub use equalizer::view_equalizer;
pub use now_playing::{
    KnownTrack, TimeDisplay, needs_marquee, now_playing_artist, now_playing_elapsed,
    now_playing_label, now_playing_number, now_playing_time, view_now_playing,
};
pub use seek_bar::view_seek_bar;
pub use shade::view_shade_strip;
pub use title_bar::view_title_bar;
pub use transport::view_transport_controls;

#[cfg(test)]
pub(super) use browse::BROWSE_VIEWS;
#[cfg(test)]
pub(super) use seek_bar::seek_bar_value;
pub(super) use shade::SHADED_HEIGHT;
#[cfg(test)]
pub(super) use title_bar::TITLE_BAR_HEIGHT;

#[cfg(test)]
mod tests;

/// Formats `duration_ms` as zero-padded `m:ss`, with unbounded minutes so an
/// hour or more reads as `60:00`/`61:05` rather than rolling over. This is the
/// shared core of [`format_track_time`], whose zero case is the per-track
/// `"--:--"` placeholder, and the browse song list's total-playing-time
/// footer, whose zero total is a real `"0:00"`. Shared by the Now Playing bar
/// and the browse rows, so it lives beside the other cross-panel helpers here
/// rather than inside one panel.
fn format_minutes_seconds(duration_ms: u64) -> String {
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
/// (an 8px gap between its on/off button and preset pick list), and the shade
/// strip (a 6px gap between its transport buttons) all construct the same
/// `Space`, so the `Space::new().width(Length::Fixed(..))` expression lives
/// here once instead of being repeated at every call site.
fn spacer(width: f32) -> Space {
    Space::new().width(Length::Fixed(width))
}

/// A button showing a single text label that emits `message` on press.
///
/// The browse Back, Top, Sort, All Songs, All Albums, Playlists, and Retry
/// buttons and the equalizer panel's on/off button call this directly; the
/// transport row's six buttons (Play/Pause, Stop, Previous, Next, Repeat,
/// Shuffle), the shade strip's four (Previous, Play/Pause, Stop, Next), and
/// the title bar's shade, minimize, and close buttons reach it through
/// [`fixed_width_button`]. They all build the same
/// `Button::new(Text::new(..)).on_press(..)` widget, so that expression lives
/// here once instead of being repeated at every call site. The label is
/// `'static` — a literal, or a `&'static str` such as
/// [`play_pause_glyph`](transport::play_pause_glyph) returns — so the
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
/// matching the volume slider's step.
const BALANCE_STEP: f32 = 0.01;

/// The equalizer sliders' drag granularity: one decibel, so every preamp and
/// band slider lands on a whole-dB gain within `GAIN_MIN_DB..=GAIN_MAX_DB`.
const EQ_STEP: f32 = 1.0;

/// Builds one horizontal chrome slider: it spans `min..=max`, starts at
/// `value`, steps by `step`, is `width` pixels wide, and paints both halves of
/// its rail `fill` with the shared [`style::level_slider_style`]. Dragging it
/// emits `on_change`.
///
/// The transport row's volume and balance sliders and the shade strip's volume
/// slider differ only in those values and their fill, so the construction lives
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
        .into()
}
