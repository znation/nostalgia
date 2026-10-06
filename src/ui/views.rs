//! Widget construction for the player's UI.
//!
//! Pure functions: each turns plain data — the current track and the loaded
//! songs, the playback state, or `&[Artist]` / `&[Album]` / `&[Song]` — into
//! an `Element` (a couple of label helpers return a `String` instead) and
//! knows nothing about the player's state or update loop. Keeping them free
//! of the `WinampPlayer` struct means the view layer can be reworked (or
//! tested) independently of how the app is booted.

use std::borrow::Cow;
use std::collections::HashMap;

use iced::{
    Background, Color, Element, Length,
    widget::{Button, Column, Row, Scrollable, Slider, Space, Text, VerticalSlider},
};

use crate::equalizer::{BAND_COUNT, BAND_FREQUENCIES, GAIN_MAX_DB, GAIN_MIN_DB};
use crate::library::{Album, Artist, Song};

use super::{CurrentView, Message};

/// A fixed-width horizontal gap between adjacent widgets.
///
/// The browse rows (a 10px gap between the title and the secondary label)
/// and the transport row (a 20px gap between each control) all construct
/// the same `Space`, so the `Space::new().width(Length::Fixed(..))`
/// expression lives here once instead of being repeated at every call site.
fn spacer(width: f32) -> Space {
    Space::new().width(Length::Fixed(width))
}

/// The highlight colour behind the currently playing row in a Songs list,
/// echoing Winamp's playlist selection bar (a saturated blue).
const PLAYING_ROW_HIGHLIGHT: Color = Color::from_rgb(0.25, 0.5, 1.0);

/// A button showing a single text label that emits `message` on press.
///
/// The transport row's five buttons (Play/Pause, Stop, Previous, Next,
/// Repeat) and the browse Back button all build the same
/// `Button::new(Text::new(..)).on_press(..)` widget, so that expression
/// lives here once instead of being repeated at every call site. The label
/// is `'static` — a literal, or a `&'static str` such as [`play_pause_label`]
/// returns — so the resulting button is `'static` like the view builders
/// that push it.
fn labeled_button(label: &'static str, message: Message) -> Button<'static, Message> {
    Button::new(Text::new(label)).on_press(message)
}

/// Builds a scrollable list where each item is a button showing a title
/// followed by a secondary label, emitting the given message on press.
/// Shared by the artists, albums, and songs views. The last tuple element
/// marks the currently playing row: such a row gets a `▶` prefix and a
/// highlighted background so the list reads as a playlist. Only `song_row`
/// ever sets the flag to true.
///
/// The rows borrow their titles from the list the caller passes in rather
/// than owning clones: this builder runs on every view refresh, so the
/// borrowed title avoids a `String` allocation per row per frame. Only the
/// one marked row needs an owned title, for its `▶` prefix.
fn scrollable_list<'a>(
    items: impl IntoIterator<Item = (&'a str, &'static str, Message, bool)>,
) -> Element<'a, Message> {
    let mut column = Column::new().padding(20);

    for (title, label, message, is_current) in items {
        let title: Cow<'a, str> = if is_current {
            Cow::Owned(format!("▶ {title}"))
        } else {
            Cow::Borrowed(title)
        };
        let button = Button::new(
            Row::new()
                .push(Text::new(title).size(18))
                .push(spacer(10.0))
                .push(Text::new(label).size(14)),
        )
        .on_press(message);
        // The current row keeps the theme's hover/pressed/text styling and
        // only swaps the background for the highlight colour.
        column = column.push(if is_current {
            button.style(|theme, status| {
                let mut style = iced::widget::button::background(theme, status);
                style.background = Some(Background::Color(PLAYING_ROW_HIGHLIGHT));
                style
            })
        } else {
            button
        });
    }

    Scrollable::new(column)
        .width(Length::Fill)
        .height(Length::FillPortion(3))
        .into()
}

/// The browse row an artist becomes: its name, the "View Albums" hint, the
/// message emitted when it is pressed, and the current-track flag. Kept out
/// of `view_artists` so the title/label/selection contract is testable
/// without an iced renderer. Artists are never the currently playing row, so
/// the flag is always false.
fn artist_row(artist: &Artist) -> (&str, &'static str, Message, bool) {
    (
        artist.name.as_str(),
        "View Albums",
        Message::ArtistSelected(artist.id.clone()),
        false,
    )
}

/// The browse row an album becomes: its title, the "View Songs" hint, the
/// message emitted when it is pressed, and the current-track flag. Albums
/// are never the currently playing row, so the flag is always false.
fn album_row(album: &Album) -> (&str, &'static str, Message, bool) {
    (
        album.title.as_str(),
        "View Songs",
        Message::AlbumSelected(album.id.clone()),
        false,
    )
}

/// The browse row a song becomes: its title, the "Play" hint, the message
/// emitted when it is pressed, and whether it is the currently playing track
/// (true only when `current_track` names this song). The flag feeds
/// `scrollable_list`'s `▶` + highlight marker, so the album list reads as a
/// playlist.
fn song_row<'a>(
    song: &'a Song,
    current_track: Option<&str>,
) -> (&'a str, &'static str, Message, bool) {
    (
        song.title.as_str(),
        "Play",
        Message::TrackSelected(song.id.clone()),
        Some(song.id.as_str()) == current_track,
    )
}

pub fn view_artists(artists: &[Artist]) -> Element<'_, Message> {
    scrollable_list(artists.iter().map(artist_row))
}

pub fn view_albums(albums: &[Album]) -> Element<'_, Message> {
    scrollable_list(albums.iter().map(album_row))
}

pub fn view_songs<'a>(songs: &'a [Song], current_track: Option<&str>) -> Element<'a, Message> {
    scrollable_list(songs.iter().map(|song| song_row(song, current_track)))
}

/// Whether the browse view has a level above it to return to. The Albums and
/// Songs views do — the Back button is shown above their lists — while the
/// top-level Artists list has nothing to go back to.
pub fn can_go_back(view: &CurrentView) -> bool {
    !matches!(view, CurrentView::Artists)
}

/// The browse view's Back button, stepping the hierarchy one level up (Songs
/// → Albums → Artists). Rendered only where [`can_go_back`] is true; the
/// update loop turns the pressed message into the view change.
pub fn view_back_button() -> Element<'static, Message> {
    labeled_button("Back", Message::Back).into()
}

/// The Now Playing bar label: the title of `current_track` when `titles`
/// knows it; the raw id when it doesn't; "Nothing" when stopped. `titles` is
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
/// track's resolved title (or "Nothing" when stopped). Takes the already
/// resolved label (from [`now_playing_label`]) so this per-frame widget build
/// does no song lookup itself. The label is a [`Cow`]: the borrowed case keeps
/// the title owned by the caller's index (no allocation), while the owned
/// fallback is moved into the widget, so the built element never borrows a
/// temporary.
pub fn view_now_playing(label: Cow<'_, str>) -> Element<'_, Message> {
    Row::new()
        .push(Text::new("Now Playing: ").size(20))
        .push(Text::new(label).size(20))
        .into()
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

/// The equalizer on/off button's label: "EQ: On" while the equalizer is
/// engaged, "EQ: Off" when it is off. Pure so the label logic is testable
/// without an iced `Element`, like [`repeat_label`].
fn eq_enabled_label(enabled: bool) -> &'static str {
    if enabled { "EQ: On" } else { "EQ: Off" }
}

/// The transport row: the Play/Pause, Stop, Previous, Next, and Repeat
/// buttons and the volume slider. `volume` is the slider's current value;
/// dragging it emits `Message::VolumeChange`. `repeat` is the shared Repeat
/// flag, shown on the Repeat button and toggled by pressing it. The Stop
/// label is static — Stop is always pressable, even when already stopped, as
/// in Winamp — so no `play_pause_label`-style helper is needed.
pub fn view_transport_controls(
    is_playing: bool,
    volume: f32,
    repeat: bool,
) -> Element<'static, Message> {
    Row::new()
        .push(labeled_button(
            play_pause_label(is_playing),
            Message::PlayPause,
        ))
        .push(spacer(20.0))
        .push(labeled_button("Stop", Message::Stop))
        .push(spacer(20.0))
        .push(labeled_button("Previous", Message::PreviousTrack))
        .push(spacer(20.0))
        .push(labeled_button("Next", Message::NextTrack))
        .push(spacer(20.0))
        .push(labeled_button(repeat_label(repeat), Message::ToggleRepeat))
        .push(spacer(20.0))
        .push(
            Slider::new(0.0..=1.0, volume, Message::VolumeChange)
                .step(0.01)
                .width(Length::Fixed(100.0)),
        )
        .into()
}

/// The equalizer panel: an on/off button, a preamp slider, and a row of the
/// [`BAND_COUNT`] vertical band sliders labelled from [`BAND_FREQUENCIES`].
///
/// `enabled` is the shared EQ on/off flag shown on the button and toggled by
/// pressing it; `preamp` and `bands` are the stored gains the sliders start
/// from. Every slider spans `GAIN_MIN_DB..=GAIN_MAX_DB` in 1 dB steps and
/// emits its own change message, so dragging one routes a clamped gain back
/// into shared state. The band row is built by index so each slider's closure
/// captures its own band number — the vertical twin of the volume slider in
/// [`view_transport_controls`].
pub fn view_equalizer(
    enabled: bool,
    preamp: f32,
    bands: &[f32; BAND_COUNT],
) -> Element<'static, Message> {
    let mut band_row = Row::new();
    for (index, frequency) in BAND_FREQUENCIES.iter().enumerate() {
        band_row = band_row.push(
            Column::new()
                .push(
                    VerticalSlider::new(GAIN_MIN_DB..=GAIN_MAX_DB, bands[index], move |gain| {
                        Message::EqBandChange(index, gain)
                    })
                    .step(1.0)
                    .height(Length::Fixed(100.0)),
                )
                .push(Text::new(*frequency).size(12)),
        );
    }

    Column::new()
        .push(labeled_button(
            eq_enabled_label(enabled),
            Message::ToggleEqualizer,
        ))
        .push(
            Row::new().push(Text::new("Preamp")).push(
                Slider::new(GAIN_MIN_DB..=GAIN_MAX_DB, preamp, Message::EqPreampChange)
                    .step(1.0)
                    .width(Length::Fixed(150.0)),
            ),
        )
        .push(band_row)
        .into()
}

#[cfg(test)]
mod tests {
    use super::{
        CurrentView, Message, album_row, artist_row, can_go_back, eq_enabled_label,
        now_playing_label, play_pause_label, repeat_label, song_row, view_albums, view_artists,
        view_back_button, view_equalizer, view_now_playing, view_songs, view_transport_controls,
    };
    use crate::equalizer::{BAND_COUNT, GAIN_MAX_DB, GAIN_MIN_DB};
    use crate::library::{sample_album, sample_artist, sample_song};
    use crate::sample_library::sample_library;
    use std::collections::HashMap;

    // Each row maps one library entry to the (title, secondary label, press
    // message, current-track flag) tuple that `scrollable_list` renders as a
    // button. Wrong label text or a swapped selection message would silently
    // break the browse UI, so the mapping is the contract these tests pin
    // down — the flag is pinned too, since it drives the `▶`/highlight
    // marker.

    #[test]
    fn artist_row_uses_name_and_selects_the_artist() {
        let artist = sample_artist();
        let (title, label, message, is_current) = artist_row(&artist);
        assert_eq!(title, "The Sample Band");
        assert_eq!(label, "View Albums");
        assert!(matches!(message, Message::ArtistSelected(id) if id == "artist-1"));
        assert!(!is_current);
    }

    #[test]
    fn album_row_uses_title_and_selects_the_album() {
        let album = sample_album();
        let (title, label, message, is_current) = album_row(&album);
        assert_eq!(title, "First Record");
        assert_eq!(label, "View Songs");
        assert!(matches!(message, Message::AlbumSelected(id) if id == "album-1"));
        assert!(!is_current);
    }

    #[test]
    fn song_row_uses_title_and_selects_the_song() {
        let song = sample_song();
        let (title, label, message, is_current) = song_row(&song, None);
        assert_eq!(title, "Opening");
        assert_eq!(label, "Play");
        assert!(matches!(message, Message::TrackSelected(id) if id == "song-1"));
        assert!(!is_current);
    }

    #[test]
    fn song_row_marks_the_current_track() {
        // The flag is true only when `current_track` names exactly this
        // song; a different id — or no track at all — leaves the row
        // unmarked, so the `▶`/highlight marker follows the playback state.
        let (_, _, _, is_current) = song_row(&sample_song(), Some("song-1"));
        assert!(is_current);

        let (_, _, _, is_current) = song_row(&sample_song(), Some("other-song"));
        assert!(!is_current);

        let (_, _, _, is_current) = song_row(&sample_song(), None);
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

    // The Now Playing bar and transport controls are built in this module
    // (grouped with the other widget builders), so their pure label logic is
    // pinned here alongside the browse-row mappings.

    /// Builds the id→title index the player's `store_songs` maintains, from
    /// `(id, title)` pairs — the shape `now_playing_label` resolves against.
    fn known_titles(entries: &[(&str, &str)]) -> HashMap<String, String> {
        entries
            .iter()
            .map(|(id, title)| (id.to_string(), title.to_string()))
            .collect()
    }

    #[test]
    fn now_playing_label_shows_nothing_when_stopped() {
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
        let _artists = view_artists(&library.artists);
        let _albums = view_albums(&library.albums_by_artist["artist-1"]);
        let _songs = view_songs(&library.songs_by_album["album-1"], None);
        let _songs_marked = view_songs(&library.songs_by_album["album-1"], Some("song-1"));
    }

    #[test]
    fn browse_views_construct_over_an_empty_list() {
        // Every browse view renders its pre-load state — an empty buffer —
        // before the first fetch lands, so `scrollable_list` must build a
        // scrollable over zero rows, with or without a current track set.
        let _artists = view_artists(&[]);
        let _albums = view_albums(&[]);
        let _songs = view_songs(&[], None);
        let _songs_marked = view_songs(&[], Some("song-1"));
    }

    #[test]
    fn now_playing_bar_and_back_button_construct() {
        let _bar = view_now_playing("Opening".into());
        let _back = view_back_button();
    }

    #[test]
    fn transport_controls_construct_for_both_play_states_volume_endpoints_and_repeat_states() {
        // `view()` passes the shared state's `is_playing`, clamped `volume`,
        // and `repeat` straight through, so build the transport row for every
        // value the update arm can store, in both play and repeat states.
        for volume in [0.0, 0.5, 1.0] {
            for is_playing in [false, true] {
                for repeat in [false, true] {
                    let _controls = view_transport_controls(is_playing, volume, repeat);
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
                let bands = [gain; BAND_COUNT];
                let _panel = view_equalizer(enabled, gain, &bands);
            }
        }
    }
}
