use super::super::{CurrentView, Message, style, theme};
use super::browse::{
    BROWSE_VIEWS, album_row, artist_row, browse_placeholder, current_row_style, empty_list_label,
    song_row,
};
use super::equalizer::eq_enabled_label;
use super::now_playing::{KnownTrack, format_track_time, now_playing_artist, now_playing_label};
use super::transport::{play_pause_label, repeat_label, shuffle_label, transport_buttons};
use super::{
    BALANCE_MAX, BALANCE_MIN, BALANCE_STEP, EQ_STEP, VOLUME_MAX, VOLUME_MIN, VOLUME_STEP,
    can_go_back, can_retry_artists, view_albums, view_all_songs, view_all_songs_button,
    view_artists, view_back_button, view_equalizer, view_now_playing, view_retry_button,
    view_search_box, view_songs, view_transport_controls,
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
    let (title, label, message, is_current) = song_row(0, 0, &song, None, None);
    assert_eq!(title, "Opening");
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
fn song_row_prefers_the_lists_cached_length_label() {
    // The loaded song list caches each row's formatted length, and the row
    // builder must use it rather than formatting the duration again. A
    // cached label that differs from the duration's formatting proves the
    // row borrows the cache; the `None` tests above pin the fallback.
    let song = sample_song();
    let (_, label, _, _) = song_row(0, 0, &song, Some("1:23"), None);
    assert_eq!(label, "1:23");
}

#[test]
fn song_row_shows_an_unknown_duration_as_blank_time() {
    // A song whose source supplied no duration (`0`) renders the same
    // `--:--` placeholder as the Now Playing bar, not `0:00`.
    let mut song = sample_song();
    song.duration_ms = 0;
    let (_, label, _, _) = song_row(0, 0, &song, None, None);
    assert_eq!(label, "--:--");
}

#[test]
fn song_row_marks_the_current_track() {
    // The flag is true only when `current_track` names exactly this
    // song; a different id — or no track at all — leaves the row
    // unmarked, so the `▶`/highlight marker follows the playback state.
    let (_, _, _, is_current) = song_row(0, 0, &sample_song(), None, Some("song-1"));
    assert!(is_current);

    let (_, _, _, is_current) = song_row(0, 0, &sample_song(), None, Some("other-song"));
    assert!(!is_current);

    let (_, _, _, is_current) = song_row(0, 0, &sample_song(), None, None);
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

/// Builds the id→track index the player maintains for the Now Playing
/// bar, from `(id, artist, title)` triples — the shape `now_playing_label`
/// and `now_playing_artist` resolve against. Durations are `0` here; the
/// length formatting is pinned separately through `format_track_time`.
fn known_tracks(entries: &[(&str, &str, &str)]) -> HashMap<String, KnownTrack> {
    entries
        .iter()
        .map(|(id, artist, title)| {
            (
                id.to_string(),
                KnownTrack {
                    title: title.to_string(),
                    artist: artist.to_string(),
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
fn format_track_time_shows_placeholders_when_the_source_supplied_no_duration() {
    assert_eq!(format_track_time(0), "--:--");
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
    let _songs = view_songs(songs, &[], epoch, loading, None, None, false);
    let _songs_marked = view_songs(songs, &[], epoch, loading, None, Some("song-1"), false);
    let _songs_searched = view_songs(songs, &[], epoch, loading, None, None, true);
    let _all_songs = view_all_songs(songs, &[], epoch, loading, None, None);
    let _all_songs_marked = view_all_songs(songs, &[], epoch, loading, None, Some("song-1"));
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
        "Opening".into(),
        "The Sample Band".into(),
        "3:30".to_string(),
    );
    let _bar = view_now_playing("Opening".into(), "".into(), "3:30".to_string());
    let _back = view_back_button();
    let _retry = view_retry_button();
    let _all_songs = view_all_songs_button();
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
