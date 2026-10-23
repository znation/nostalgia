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
//! song drill-down, the flat lists, and Playlists, with their rows),
//! `title_bar`, `shade` (the rolled-up transport strip), `now_playing`,
//! `seek_bar`, `transport`, and `equalizer` — while the chrome helpers and
//! slider specs the panels share ([`spacer`], [`labeled_button`],
//! [`fixed_width_button`], and [`chrome_slider`]) live here.

use iced::{
    Element, Length,
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
    BROWSE_LIST_ID, can_go_back, can_retry_artists, view_albums, view_all_albums,
    view_all_albums_button, view_all_songs, view_all_songs_button, view_artists, view_back_button,
    view_browse_nav_buttons, view_playlists, view_playlists_button, view_retry_button,
    view_search_box, view_songs,
};
pub use equalizer::view_equalizer;
pub use now_playing::{
    KnownTrack, TimeDisplay, format_track_time, needs_marquee, now_playing_artist,
    now_playing_elapsed, now_playing_label, now_playing_time, view_now_playing,
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

/// A fixed-width horizontal gap between adjacent widgets.
///
/// The browse rows (a 10px gap between the title and the secondary label),
/// the transport row (a 20px gap between each control), and the equalizer
/// header (an 8px gap between its on/off button and preset pick list) all
/// construct the same `Space`, so the `Space::new().width(Length::Fixed(..))`
/// expression lives here once instead of being repeated at every call site.
fn spacer(width: f32) -> Space {
    Space::new().width(Length::Fixed(width))
}

/// A button showing a single text label that emits `message` on press.
///
/// The browse Back, Top, Sort, All Songs, All Albums, Playlists, and Retry buttons and
/// the equalizer panel's on/off button call this directly; the transport row's six
/// buttons (Play/Pause, Stop, Previous, Next, Repeat, Shuffle) and the title
/// bar's shade, minimize, and close buttons reach it through
/// [`fixed_width_button`]. They all build the same
/// `Button::new(Text::new(..)).on_press(..)` widget, so that expression lives
/// here once instead of being repeated at every call site. The label is
/// `'static` — a literal, or a `&'static str` such as
/// [`play_pause_label`](transport::play_pause_label) returns — so the
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
/// keyboard volume arms in `ui::mod` nudge by the slider's own step.
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
/// `value`, steps by `step`, is `width` pixels wide, and is restyled with the
/// shared [`style::chrome_slider_style`]. Dragging it emits `on_change`.
///
/// The transport row's volume and balance sliders and the equalizer's preamp
/// slider differ only in those values, so the construction lives here once.
/// The equalizer's band sliders are the vertical twin and build a
/// `VerticalSlider`, so they do not call this.
fn chrome_slider(
    min: f32,
    max: f32,
    value: f32,
    step: f32,
    width: f32,
    on_change: fn(f32) -> Message,
) -> Element<'static, Message> {
    Slider::new(min..=max, value, on_change)
        .step(step)
        .width(Length::Fixed(width))
        .style(|_theme, status| style::chrome_slider_style(status))
        .into()
}
