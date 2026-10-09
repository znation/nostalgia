//! Widget construction for the player's UI.
//!
//! Pure functions: each turns plain data — the current track and the loaded
//! songs, the playback state, or `&[Artist]` / `&[Album]` / `&[Song]` — into
//! an `Element` (a few label helpers return `&str` or `Cow<str>` instead) and
//! knows nothing about the player's state or update loop. Keeping them free
//! of the `WinampPlayer` struct means the view layer can be reworked (or
//! tested) independently of how the app is booted.

use std::borrow::Cow;
use std::collections::HashMap;

use iced::{
    Background, Element, Length,
    widget::{
        Button, Column, Container, MouseArea, PickList, Row, Scrollable, Slider, Space, Text,
        VerticalSlider, button,
    },
};

use crate::equalizer::{BAND_COUNT, BAND_FREQUENCIES, GAIN_MAX_DB, GAIN_MIN_DB, PRESETS, Preset};
use crate::library::{Album, Artist, Song};

use super::{CurrentView, Message, bevel, style, theme};

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
/// The browse Back and Retry buttons and the equalizer panel's on/off button
/// call this directly; the transport row's six buttons (Play/Pause, Stop,
/// Previous, Next, Repeat, Shuffle) and the title bar's shade, minimize, and close
/// buttons reach it through [`fixed_width_button`]. They all build the same
/// `Button::new(Text::new(..)).on_press(..)` widget, so that expression lives
/// here once instead of being repeated at every call site. The label is
/// `'static` — a literal, or a `&'static str` such as [`play_pause_label`]
/// returns — so the resulting button is `'static` like the view builders that
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

/// The placeholder a browse view shows when its list has no rows: one wording
/// per browse level, so an empty Artists, Albums, or Songs panel names the
/// list rather than showing a blank panel. Pure so the per-level wording is
/// testable without an iced renderer, like the button label helpers below.
fn empty_list_label(view: &CurrentView) -> &'static str {
    match view {
        CurrentView::Artists => "No artists",
        CurrentView::Albums => "No albums",
        CurrentView::Songs => "No songs",
    }
}

/// The placeholder a browse view shows when its list has no rows, choosing
/// between a fetch failure, "still loading", and "loaded, but empty".
///
/// `error` is the list's most recent fetch-failure report (see
/// `browse::BrowseList::fail`/`browse::BrowseList::clear`): when present it wins, so a
/// backend failure is shown rather than misread as an empty library.
/// Otherwise `loading` is the list's load state: true means the reply the
/// navigation just scheduled is still in flight, so the panel reads as
/// loading rather than as an empty library; false means a reply landed —
/// possibly an empty one — so [`empty_list_label`] names the list. Pure, like
/// [`empty_list_label`], so every wording is testable without an iced
/// renderer.
fn browse_placeholder<'a>(view: &CurrentView, loading: bool, error: Option<&'a str>) -> &'a str {
    if let Some(error) = error {
        error
    } else if loading {
        "Loading…"
    } else {
        empty_list_label(view)
    }
}

/// The style for a browse row that is currently playing: the flat
/// [`style::playlist_row_style`] with its background forced to
/// [`theme::PLAYING_ROW_HIGHLIGHT`], so the playing row reads as a
/// selection bar in every hover/press state instead of following the row's
/// face ladder under the cursor. Pure, so the highlight — which iced's
/// `Element` API gives no way to read back from the built `Button`, and whose
/// closure runs only at render time — is testable.
fn current_row_style(status: button::Status) -> button::Style {
    let mut row = style::playlist_row_style(status);
    row.background = Some(Background::Color(theme::PLAYING_ROW_HIGHLIGHT));
    row
}

/// Builds a scrollable list where each item is a button showing a title
/// followed by a secondary label, emitting the given message on press.
/// Shared by the artists, albums, and songs views. The last tuple element
/// marks the currently playing row: such a row gets a `▶` prefix and a
/// highlighted background so the list reads as a playlist. Only `song_row`
/// ever sets the flag to true.
///
/// When `items` yields nothing, the list renders `empty_label` in place of a
/// blank panel. The caller picks the wording via [`browse_placeholder`],
/// which distinguishes a fetch failure, a buffer no reply has populated yet
/// (still loading), and one that loaded empty and names the list.
///
/// The rows borrow their titles from the list the caller passes in rather
/// than owning clones: this builder runs on every view refresh, so the
/// borrowed title avoids a `String` allocation per row per frame. The
/// playing row's `▶` marker is pushed as its own static label beside the
/// borrowed title rather than formatted into an owned one, so the marker
/// adds no `String` allocation per frame either.
fn scrollable_list<'a>(
    items: impl IntoIterator<Item = (&'a str, &'static str, Message, bool)>,
    empty_label: &'a str,
) -> Element<'a, Message> {
    let mut column = Column::new().padding(20);
    let mut is_empty = true;

    for (title, label, message, is_current) in items {
        is_empty = false;
        let row = if is_current {
            Row::new().push(Text::new("▶ ").size(18))
        } else {
            Row::new()
        };
        let button = Button::new(
            row.push(Text::new(title).size(18))
                .push(spacer(10.0))
                .push(Text::new(label).size(14)),
        )
        .on_press(message)
        .style(|_theme, status| style::playlist_row_style(status));
        // The current row keeps the playlist row's text and hover/press face
        // and only swaps the background for the highlight colour.
        column = column.push(if is_current {
            button.style(|_theme, status| current_row_style(status))
        } else {
            button
        });
    }

    if is_empty {
        column = column.push(Text::new(empty_label).size(18));
    }

    Scrollable::new(column)
        .width(Length::Fill)
        .height(Length::FillPortion(3))
        .style(|_theme, _status| style::playlist_scrollable_style())
        .into()
}

/// The browse row an artist becomes: its name, the "View Albums" hint, the
/// message emitted when it is pressed, and the current-track flag. `epoch` is
/// the loaded-list epoch and `index` the artist's position in that list; both
/// become the selection message's payload, so building a row never clones the
/// artist's id — the update loop resolves the index back to an id only when
/// the row is actually pressed, and rejects the press if the list has since
/// been replaced. Kept out of `view_artists` so the title/label/selection
/// contract is testable without an iced renderer. Artists are never the
/// currently playing row, so the flag is always false.
fn artist_row(epoch: u64, index: usize, artist: &Artist) -> (&str, &'static str, Message, bool) {
    (
        artist.name.as_str(),
        "View Albums",
        Message::ArtistSelected { epoch, index },
        false,
    )
}

/// The browse row an album becomes: its title, the "View Songs" hint, the
/// message emitted when it is pressed, and the current-track flag. `epoch`
/// and `index` are the loaded-list epoch and the album's position in that
/// list, carried so building a row never clones the album's id (see
/// [`artist_row`]). Albums are never the currently playing row, so the flag
/// is always false.
fn album_row(epoch: u64, index: usize, album: &Album) -> (&str, &'static str, Message, bool) {
    (
        album.title.as_str(),
        "View Songs",
        Message::AlbumSelected { epoch, index },
        false,
    )
}

/// The browse row a song becomes: its title, the "Play" hint, the message
/// emitted when it is pressed, and whether it is the currently playing track
/// (true only when `current_track` names this song). `epoch` and `index` are
/// the loaded-list epoch and the song's position in that list, carried so
/// building a row never clones the song's id (see [`artist_row`]). The flag
/// feeds `scrollable_list`'s `▶` + highlight marker, so the song list reads
/// as a playlist.
fn song_row<'a>(
    epoch: u64,
    index: usize,
    song: &'a Song,
    current_track: Option<&str>,
) -> (&'a str, &'static str, Message, bool) {
    (
        song.title.as_str(),
        "Play",
        Message::TrackSelected { epoch, index },
        Some(song.id.as_str()) == current_track,
    )
}

/// Builds a browse list from its rows: the shared body of [`view_artists`],
/// [`view_albums`], and [`view_songs`]. `rows` yields each item's
/// `(title, label, message, current-track)` tuple (see [`scrollable_list`]),
/// and `view` selects the empty-list wording via [`browse_placeholder`]; a
/// fetch failure wins.
fn browse_view<'a>(
    view: CurrentView,
    rows: impl IntoIterator<Item = (&'a str, &'static str, Message, bool)>,
    loading: bool,
    error: Option<&'a str>,
) -> Element<'a, Message> {
    scrollable_list(rows, browse_placeholder(&view, loading, error))
}

/// The Artists browse view: one row per artist, in the order given, each
/// emitting [`Message::ArtistSelected`] with the artist's index in the list
/// and the list's `epoch`. `loading` and `error` select the placeholder
/// wording (see [`browse_placeholder`]); a fetch failure wins. Built by
/// [`browse_view`] from the [`artist_row`] mapping.
pub fn view_artists<'a>(
    artists: &'a [Artist],
    epoch: u64,
    loading: bool,
    error: Option<&'a str>,
) -> Element<'a, Message> {
    browse_view(
        CurrentView::Artists,
        artists
            .iter()
            .enumerate()
            .map(|(index, artist)| artist_row(epoch, index, artist)),
        loading,
        error,
    )
}

/// The Albums browse view: one row per album, in the order given, each
/// emitting [`Message::AlbumSelected`] with the album's index in the list and
/// the list's `epoch`. `loading` and `error` select the placeholder wording
/// (see [`browse_placeholder`]); a fetch failure wins. Built by
/// [`browse_view`] from the [`album_row`] mapping.
pub fn view_albums<'a>(
    albums: &'a [Album],
    epoch: u64,
    loading: bool,
    error: Option<&'a str>,
) -> Element<'a, Message> {
    browse_view(
        CurrentView::Albums,
        albums
            .iter()
            .enumerate()
            .map(|(index, album)| album_row(epoch, index, album)),
        loading,
        error,
    )
}

/// The Songs browse view: one row per song, in the order given, each emitting
/// [`Message::TrackSelected`] with the song's index in the list and the
/// list's `epoch`; the row whose id is `current_track` is marked as playing.
/// `loading` and `error` select the placeholder wording (see
/// [`browse_placeholder`]); a fetch failure wins. Built by [`browse_view`]
/// from the [`song_row`] mapping.
pub fn view_songs<'a>(
    songs: &'a [Song],
    epoch: u64,
    loading: bool,
    error: Option<&'a str>,
    current_track: Option<&str>,
) -> Element<'a, Message> {
    browse_view(
        CurrentView::Songs,
        songs
            .iter()
            .enumerate()
            .map(|(index, song)| song_row(epoch, index, song, current_track)),
        loading,
        error,
    )
}

/// Whether the browse view has a level above it to return to. The Albums and
/// Songs views do — the Back button is shown above their lists — while the
/// top-level Artists list has nothing to go back to.
///
/// `#[must_use]`: a discarded result is a logic bug — the caller that decides
/// whether to show the Back button would silently never show it.
#[must_use]
pub fn can_go_back(view: &CurrentView) -> bool {
    !matches!(view, CurrentView::Artists)
}

/// Whether the Artists view should offer its Retry button. Only the
/// top-level artists fetch is unrecoverable by navigation, so Retry belongs
/// to the Artists view and only while that fetch has failed; the Albums and
/// Songs levels recover by navigating back into them. The update loop's
/// `LoadArtists` arm clears the failure and re-issues the fetch.
///
/// `#[must_use]`: a discarded result is a logic bug — the caller that decides
/// whether to show the Retry button would silently never show it.
#[must_use]
pub fn can_retry_artists(view: &CurrentView, error: Option<&str>) -> bool {
    matches!(view, CurrentView::Artists) && error.is_some()
}

/// The browse view's Back button, stepping the hierarchy one level up (Songs
/// → Albums → Artists). Rendered only where [`can_go_back`] is true; the
/// update loop turns the pressed message into the view change.
pub fn view_back_button() -> Element<'static, Message> {
    labeled_button("Back", Message::Back).into()
}

/// The Artists view's Retry button, re-running the top-level artists fetch
/// after it failed. Rendered only where [`can_retry_artists`] is true; the
/// update loop's `LoadArtists` arm clears the failure and re-issues the
/// fetch, so the button turns the otherwise unrecoverable failed startup
/// fetch — no navigation re-issues it — into a retry.
pub fn view_retry_button() -> Element<'static, Message> {
    labeled_button("Retry", Message::LoadArtists).into()
}

/// The custom title bar's app name.
const TITLE_BAR_TEXT: &str = "NOSTALGIA";

/// The custom title bar's height in px: short, like the classic Winamp title
/// bar, rather than the OS frame's.
pub(super) const TITLE_BAR_HEIGHT: f32 = 24.0;

/// Each title-bar button's pinned width, so its glyph cannot resize it and
/// reflow the drag region beside it.
const TITLE_BAR_BUTTON_WIDTH: f32 = 22.0;

/// The always-on-top clutter toggle's pinned width. It is narrower than the
/// window buttons: the A slot is a small square in the classic skin, and it
/// sits at the left edge of the bar rather than beside the window buttons.
const CLUTTER_BUTTON_WIDTH: f32 = 18.0;

/// The custom Winamp title bar: the app name on a raised
/// [`theme::TITLE_BLUE`] band, draggable to move the window, with the clutter
/// bar's always-on-top toggle at the left and shade, minimize, and close
/// buttons on the right. Double-clicking the band toggles the classic roll-up
/// (shade) mode via [`Message::ToggleWindowShade`].
///
/// The whole band except the buttons is a [`MouseArea`], so a press anywhere
/// on it emits [`Message::WindowDragged`] and the update loop begins the OS
/// window drag, while a double-click emits [`Message::ToggleWindowShade`].
/// The A toggle is sunken while `always_on_top` holds and raised otherwise;
/// the shade, minimize, and close buttons are [`fixed_width_button`]s — the
/// same raised chrome as the transport row — pinned so their glyphs cannot
/// resize them. The bar paints [`style::title_bar_style`] and wraps it in
/// [`bevel::raised_panel`], so it carries the base skin's raised bevel.
pub fn view_title_bar(always_on_top: bool) -> Element<'static, Message> {
    let drag_region = MouseArea::new(
        Container::new(Text::new(TITLE_BAR_TEXT).size(14).color(theme::TEXT))
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(iced::alignment::Vertical::Center)
            .padding([0, 6]),
    )
    .on_press(Message::WindowDragged)
    .on_double_click(Message::ToggleWindowShade);

    let clutter_toggle = Button::new(Text::new("A"))
        .on_press(Message::ToggleAlwaysOnTop)
        .width(Length::Fixed(CLUTTER_BUTTON_WIDTH))
        .style(move |_theme, status| {
            if always_on_top {
                style::chrome_button_style(button::Status::Pressed)
            } else {
                style::chrome_button_style(status)
            }
        });

    bevel::raised_panel(
        Container::new(
            Row::new()
                .push(clutter_toggle)
                .push(drag_region)
                .push(fixed_width_button(
                    "▭",
                    TITLE_BAR_BUTTON_WIDTH,
                    Message::ToggleWindowShade,
                ))
                .push(fixed_width_button(
                    "–",
                    TITLE_BAR_BUTTON_WIDTH,
                    Message::MinimizeWindow,
                ))
                .push(fixed_width_button(
                    "✕",
                    TITLE_BAR_BUTTON_WIDTH,
                    Message::CloseWindow,
                )),
        )
        .width(Length::Fill)
        .height(Length::Fixed(TITLE_BAR_HEIGHT))
        .style(|_theme| style::title_bar_style()),
    )
}

/// The Now Playing bar label: the title of `current_track` when `titles`
/// knows it; the raw id when it doesn't; "Nothing" when no track is current
/// (Stop keeps the interrupted track's title). `titles` is
/// the player's known id→title index, so this per-frame resolution is a single
/// map get rather than a scan of every song the player has ever loaded. The
/// result borrows the title from `titles` (or the `"Nothing"` literal) in the
/// common cases, so a per-frame refresh allocates only for the rare unknown-id
/// fallback instead of cloning the title on every view. Pure data → `Cow` so
/// the title resolution is testable without an iced renderer.
pub fn now_playing_label<'a>(
    titles: &'a HashMap<String, String>,
    current_track: Option<&str>,
) -> Cow<'a, str> {
    match current_track {
        Some(id) => titles
            .get(id)
            .map(|title| Cow::Borrowed(title.as_str()))
            .unwrap_or_else(|| Cow::Owned(id.to_string())),
        None => Cow::Borrowed("Nothing"),
    }
}

/// The Now Playing bar: the "Now Playing:" caption followed by the current
/// track's resolved title (or "Nothing" when nothing is current). Takes the
/// already resolved label (from [`now_playing_label`]) so this per-frame
/// widget build does no song lookup itself. The label is a [`Cow`]: the
/// borrowed case keeps the title owned by the caller's index (no allocation),
/// while the owned fallback is moved into the widget, so the built element
/// never borrows a temporary.
pub fn view_now_playing(label: Cow<'_, str>) -> Element<'_, Message> {
    bevel::lcd_well(
        Row::new()
            .push(Text::new("Now Playing: ").size(20))
            .push(Text::new(label).size(20).color(theme::LCD_GREEN)),
    )
}

/// The Play/Pause button's label: "Pause" while playing, "Play" when
/// stopped. Pure so the label logic is testable without an iced `Element`.
fn play_pause_label(is_playing: bool) -> &'static str {
    if is_playing { "Pause" } else { "Play" }
}

/// The Repeat button's label: "Repeat: On" while Repeat is on, "Repeat:
/// Off" when it is off. Pure so the label logic is testable without an
/// iced `Element`, like [`play_pause_label`].
fn repeat_label(repeat: bool) -> &'static str {
    if repeat { "Repeat: On" } else { "Repeat: Off" }
}

/// The Shuffle button's label: "Shuffle: On" while Shuffle is on, "Shuffle:
/// Off" when it is off. Pure so the label logic is testable without an iced
/// `Element`, like [`repeat_label`].
fn shuffle_label(shuffle: bool) -> &'static str {
    if shuffle {
        "Shuffle: On"
    } else {
        "Shuffle: Off"
    }
}

/// The equalizer on/off button's label: "EQ: On" while the equalizer is
/// engaged, "EQ: Off" when it is off. Pure so the label logic is testable
/// without an iced `Element`, like [`repeat_label`].
fn eq_enabled_label(enabled: bool) -> &'static str {
    if enabled { "EQ: On" } else { "EQ: Off" }
}

/// The volume slider's silence floor, the same lower bound
/// `state::clamp_volume` clamps to. Named here so the slider spec is a
/// testable seam; the test below ties it to the clamp so the two cannot
/// drift apart.
const VOLUME_MIN: f32 = 0.0;

/// The volume slider's full-volume ceiling, the same upper bound
/// `state::clamp_volume` clamps to (see [`VOLUME_MIN`]).
const VOLUME_MAX: f32 = 1.0;

/// The volume slider's drag granularity: one percent, so a drag spans the
/// whole `[VOLUME_MIN, VOLUME_MAX]` scale in fine steps. `pub(super)` so the
/// keyboard volume arms in `ui::mod` nudge by the slider's own step.
pub(super) const VOLUME_STEP: f32 = 0.01;

/// The balance slider's hard-left floor, the same lower bound
/// `state::clamp_balance` clamps to. Named here so the slider spec is a
/// testable seam; the test below ties it to the clamp so the two cannot
/// drift apart.
const BALANCE_MIN: f32 = -1.0;

/// The balance slider's hard-right ceiling, the same upper bound
/// `state::clamp_balance` clamps to (see [`BALANCE_MIN`]).
const BALANCE_MAX: f32 = 1.0;

/// The balance slider's drag granularity: the same step as the volume
/// slider, so a drag spans the `[BALANCE_MIN, BALANCE_MAX]` scale in fine
/// steps.
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

/// The transport row's six chrome buttons, in row order: Play/Pause, Stop,
/// Previous, Next, Repeat, and Shuffle. `is_playing`, `repeat`, and `shuffle`
/// resolve the three labels that change with state; the other three are static
/// literals.
///
/// Each button is pinned to the fixed face width of its widest label, so a
/// label swap — Play for Pause, Repeat: Off for Repeat: On, or Shuffle: Off
/// for Shuffle: On — cannot resize the button and reflow every widget after it
/// in the row. Returned as `Element`s so the width each button requests is
/// observable in a test.
fn transport_buttons(
    is_playing: bool,
    repeat: bool,
    shuffle: bool,
) -> [Element<'static, Message>; 6] {
    // The widths, in pixels, of each button's widest label: Play/Pause must
    // fit "Pause", Repeat must fit "Repeat: Off", and Shuffle must fit
    // "Shuffle: Off". They are the face widths measured from the running
    // window (see the qa reproduction), so at rest each button keeps the
    // width its label already had.
    const PLAY_PAUSE_WIDTH: f32 = 65.0;
    const STOP_WIDTH: f32 = 54.0;
    const PREVIOUS_WIDTH: f32 = 84.0;
    const NEXT_WIDTH: f32 = 54.0;
    const REPEAT_WIDTH: f32 = 104.0;
    const SHUFFLE_WIDTH: f32 = 112.0;

    [
        fixed_width_button(
            play_pause_label(is_playing),
            PLAY_PAUSE_WIDTH,
            Message::PlayPause,
        )
        .into(),
        fixed_width_button("Stop", STOP_WIDTH, Message::Stop).into(),
        fixed_width_button("Previous", PREVIOUS_WIDTH, Message::PreviousTrack).into(),
        fixed_width_button("Next", NEXT_WIDTH, Message::NextTrack).into(),
        fixed_width_button(repeat_label(repeat), REPEAT_WIDTH, Message::ToggleRepeat).into(),
        fixed_width_button(
            shuffle_label(shuffle),
            SHUFFLE_WIDTH,
            Message::ToggleShuffle,
        )
        .into(),
    ]
}

/// The transport row: the Play/Pause, Stop, Previous, Next, Repeat, and
/// Shuffle buttons, the volume slider, and the balance slider. `volume` and
/// `balance` are the sliders' current values; dragging them emits
/// `Message::VolumeChange` and `Message::BalanceChange` respectively.
/// `repeat` is the shared Repeat flag and `shuffle` the shared Shuffle flag,
/// each shown on its button and toggled by pressing it. The Stop label is
/// static — Stop is always pressable, even when already stopped, as in Winamp
/// — so no `play_pause_label`-style helper is needed.
pub fn view_transport_controls(
    is_playing: bool,
    volume: f32,
    balance: f32,
    repeat: bool,
    shuffle: bool,
) -> Element<'static, Message> {
    let mut row = Row::new();
    for button in transport_buttons(is_playing, repeat, shuffle) {
        row = row.push(button).push(spacer(20.0));
    }
    row.push(chrome_slider(
        VOLUME_MIN,
        VOLUME_MAX,
        volume,
        VOLUME_STEP,
        100.0,
        Message::VolumeChange,
    ))
    .push(chrome_slider(
        BALANCE_MIN,
        BALANCE_MAX,
        balance,
        BALANCE_STEP,
        100.0,
        Message::BalanceChange,
    ))
    .into()
}

/// The equalizer panel: an on/off button, a preset pick list, a preamp slider,
/// and a row of the [`BAND_COUNT`] vertical band sliders labelled from
/// [`BAND_FREQUENCIES`].
///
/// `enabled` is the shared EQ on/off flag shown on the button and toggled by
/// pressing it; `preset` is the applied curve the pick list shows (or `None`,
/// drawn as the "(none)" custom-curve placeholder), and picking one emits
/// [`Message::EqPresetSelected`]; `preamp` and `bands` are the stored gains the
/// sliders start from. Every slider spans `GAIN_MIN_DB..=GAIN_MAX_DB` in 1 dB steps and
/// emits its own change message, so dragging one routes a clamped gain back
/// into shared state. The band row is built by index so each slider's closure
/// captures its own band number — the vertical twin of the volume slider in
/// [`view_transport_controls`].
pub fn view_equalizer(
    enabled: bool,
    preamp: f32,
    bands: &[f32; BAND_COUNT],
    preset: Option<Preset>,
) -> Element<'static, Message> {
    let mut band_row = Row::new();
    for (index, frequency) in BAND_FREQUENCIES.iter().enumerate() {
        band_row = band_row.push(
            Column::new()
                .push(
                    VerticalSlider::new(GAIN_MIN_DB..=GAIN_MAX_DB, bands[index], move |gain| {
                        Message::EqBandChange(index, gain)
                    })
                    .step(EQ_STEP)
                    .height(Length::Fixed(100.0))
                    .style(|_theme, status| style::chrome_slider_style(status)),
                )
                .push(Text::new(*frequency).size(12)),
        );
    }

    bevel::raised_panel(
        Column::new()
            .push(
                Row::new()
                    .push(labeled_button(
                        eq_enabled_label(enabled),
                        Message::ToggleEqualizer,
                    ))
                    .push(spacer(8.0))
                    .push(
                        // `PickList` stores its options by value, so passing the
                        // `[Preset; 19]` array copies ~1.2 KB into the widget on
                        // every rebuild; passing the slice borrows the static
                        // array instead. The pick list only ever reads the options
                        // through `Borrow<[Preset]>`, so the slice is equivalent.
                        PickList::new(PRESETS.as_slice(), preset, Message::EqPresetSelected)
                            .placeholder("(none)")
                            .text_size(12)
                            .style(|_theme, status| style::chrome_pick_list_style(status))
                            .menu_style(|_theme| style::preset_menu_style()),
                    ),
            )
            .push(Row::new().push(Text::new("Preamp")).push(chrome_slider(
                GAIN_MIN_DB,
                GAIN_MAX_DB,
                preamp,
                EQ_STEP,
                150.0,
                Message::EqPreampChange,
            )))
            .push(band_row),
    )
}

/// Every browse view the UI can assemble, in hierarchy order. The per-view
/// tests and the full-input test in `ui::tests` all drive each level, so
/// naming the list once means a new view is added here rather than to each
/// loop's literal.
#[cfg(test)]
pub(super) const BROWSE_VIEWS: [CurrentView; 3] = [
    CurrentView::Artists,
    CurrentView::Albums,
    CurrentView::Songs,
];

#[cfg(test)]
mod tests {
    use super::{
        BALANCE_MAX, BALANCE_MIN, BALANCE_STEP, BROWSE_VIEWS, CurrentView, EQ_STEP, Message,
        VOLUME_MAX, VOLUME_MIN, VOLUME_STEP, album_row, artist_row, browse_placeholder,
        can_go_back, can_retry_artists, current_row_style, empty_list_label, eq_enabled_label,
        now_playing_label, play_pause_label, repeat_label, shuffle_label, song_row, style, theme,
        transport_buttons, view_albums, view_artists, view_back_button, view_equalizer,
        view_now_playing, view_retry_button, view_songs, view_transport_controls,
    };
    use crate::equalizer::{BAND_COUNT, GAIN_MAX_DB, GAIN_MIN_DB, PRESETS, clamp_gain};
    use crate::library::{Album, Artist, Song};
    use crate::sample_library::sample_library;
    use crate::state::{clamp_balance, clamp_volume};
    use crate::test_support::{sample_album, sample_artist, sample_song};
    use iced::Length;
    use std::borrow::Cow;
    use std::collections::HashMap;

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
        assert_eq!(title, "First Record");
        assert_eq!(label, "View Songs");
        assert!(matches!(
            message,
            Message::AlbumSelected { epoch: 0, index: 0 }
        ));
        assert!(!is_current);
    }

    #[test]
    fn song_row_uses_title_and_selects_the_song_by_index() {
        // As with the other rows: the message carries the row's position and
        // the list's epoch, and
        // `track_selected_starts_playback_of_the_selected_track` in
        // `ui::tests` pins the index-to-id mapping.
        let song = sample_song();
        let (title, label, message, is_current) = song_row(0, 0, &song, None);
        assert_eq!(title, "Opening");
        assert_eq!(label, "Play");
        assert!(matches!(
            message,
            Message::TrackSelected { epoch: 0, index: 0 }
        ));
        assert!(!is_current);
    }

    #[test]
    fn song_row_marks_the_current_track() {
        // The flag is true only when `current_track` names exactly this
        // song; a different id — or no track at all — leaves the row
        // unmarked, so the `▶`/highlight marker follows the playback state.
        let (_, _, _, is_current) = song_row(0, 0, &sample_song(), Some("song-1"));
        assert!(is_current);

        let (_, _, _, is_current) = song_row(0, 0, &sample_song(), Some("other-song"));
        assert!(!is_current);

        let (_, _, _, is_current) = song_row(0, 0, &sample_song(), None);
        assert!(!is_current);
    }

    // The Back button is a browse-navigation control: it must exist only
    // where there is a level above to return to, so the visibility predicate
    // is pinned alongside the other pure browse helpers.
    #[test]
    fn back_button_is_available_only_below_the_artist_list() {
        assert!(!can_go_back(&CurrentView::Artists));
        assert!(can_go_back(&CurrentView::Albums));
        assert!(can_go_back(&CurrentView::Songs));
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

    /// Builds the id→title index the player maintains for the Now Playing
    /// bar, from `(id, title)` pairs — the shape `now_playing_label`
    /// resolves against.
    fn known_titles(entries: &[(&str, &str)]) -> HashMap<String, String> {
        entries
            .iter()
            .map(|(id, title)| (id.to_string(), title.to_string()))
            .collect()
    }

    #[test]
    fn now_playing_label_shows_nothing_with_no_current_track() {
        assert_eq!(now_playing_label(&known_titles(&[]), None), "Nothing");
    }

    #[test]
    fn now_playing_label_resolves_known_track_to_title() {
        let titles = known_titles(&[("song-1", "Opening")]);
        assert_eq!(now_playing_label(&titles, Some("song-1")), "Opening");
    }

    #[test]
    fn now_playing_label_falls_back_to_id_when_track_not_in_titles() {
        let titles = known_titles(&[("song-1", "Opening")]);
        assert_eq!(
            now_playing_label(&titles, Some("no-such-song")),
            "no-such-song"
        );
    }

    #[test]
    fn now_playing_label_falls_back_to_id_when_titles_is_empty() {
        assert_eq!(
            now_playing_label(&known_titles(&[]), Some("song-1")),
            "song-1"
        );
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
        let titles = known_titles(&[("song-1", "Opening")]);

        let label = now_playing_label(&titles, Some("song-1"));
        assert!(matches!(&label, Cow::Borrowed(_)));
        assert_eq!(label.as_ptr(), titles["song-1"].as_ptr());

        let label = now_playing_label(&titles, None);
        assert!(matches!(&label, Cow::Borrowed(_)));

        // The one allocating case stays owned: the unknown id is copied into
        // the fallback label.
        let label = now_playing_label(&titles, Some("no-such-song"));
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
            1,
            false,
        );
    }

    #[test]
    fn browse_views_construct_over_an_empty_list() {
        // Every browse view renders its pre-load state — an empty buffer —
        // before the first fetch lands, so `scrollable_list` must build a
        // scrollable over zero rows, with or without a current track set.
        construct_every_browse_view(&[], &[], &[], 0, true);
    }

    /// Builds every browse view over `artists`, `albums`, and `songs`, each
    /// with the given `epoch`, `loading`, and no error: the Artists, Albums,
    /// and Songs views, the last once with no current track and once with
    /// `song-1` marked as playing. iced `Element`s expose no tree
    /// introspection, so the only observable contract is that each builder
    /// constructs without panicking; the two tests above call this over the
    /// loaded sample library and over the pre-load empty buffers.
    fn construct_every_browse_view(
        artists: &[Artist],
        albums: &[Album],
        songs: &[Song],
        epoch: u64,
        loading: bool,
    ) {
        let _artists = view_artists(artists, epoch, loading, None);
        let _albums = view_albums(albums, epoch, loading, None);
        let _songs = view_songs(songs, epoch, loading, None, None);
        let _songs_marked = view_songs(songs, epoch, loading, None, Some("song-1"));
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
            assert_eq!(browse_placeholder(&view, true, None), "Loading…");
            assert_eq!(
                browse_placeholder(&view, false, None),
                empty_list_label(&view)
            );
        }
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
                assert_eq!(browse_placeholder(&view, loading, Some(report)), report);
            }
        }
    }

    #[test]
    fn now_playing_bar_and_browse_action_buttons_construct() {
        let _bar = view_now_playing("Opening".into());
        let _back = view_back_button();
        let _retry = view_retry_button();
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
                            let _controls = view_transport_controls(
                                is_playing, volume, balance, repeat, shuffle,
                            );
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
}
