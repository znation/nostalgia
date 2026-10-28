//! Sample model values, stepping fixtures, sessions, and the `"Rock"` preset shared by the crate's tests.

use crate::equalizer::{PRESETS, Preset};
use crate::library::{Album, Artist, Playlist, Song};
use crate::music_kit_auth::MusicKitSession;

/// The preview asset URL the transport and audio tests load and assert on, and
/// the browse tests map a resource to. It lives here once so a change to the
/// placeholder reaches every suite that names it instead of drifting between
/// per-file copies.
pub(crate) const PREVIEW_URL: &str = "https://example.test/preview.m4a";

/// The three songs on `album-1` that the Previous/Next stepping tests step
/// through, shared by the `ui::transport` and `ui` test suites. Both suites
/// pin the same stepping behavior over the same three-song album; building
/// this list independently in each suite would let a rename or reorder in one
/// fixture silently diverge from the other, so it lives here once, next to the
/// other shared fixtures, and is compiled only for tests.
pub(crate) fn stepping_songs() -> Vec<Song> {
    vec![
        Song {
            id: "song-1".to_string(),
            title: "One".to_string(),
            artist: "The Sample Band".to_string(),
            album: "First Record".to_string(),
            year: None,
            genre: String::new(),
            track_number: None,
            composer: String::new(),
            album_id: "album-1".to_string(),
            duration_ms: 210_000,
            preview_url: None,
        },
        Song {
            id: "song-2".to_string(),
            title: "Two".to_string(),
            artist: "The Sample Band".to_string(),
            album: "First Record".to_string(),
            year: None,
            genre: String::new(),
            track_number: None,
            composer: String::new(),
            album_id: "album-1".to_string(),
            duration_ms: 125_000,
            preview_url: None,
        },
        Song {
            id: "song-3".to_string(),
            title: "Three".to_string(),
            artist: "The Sample Band".to_string(),
            album: "First Record".to_string(),
            year: None,
            genre: String::new(),
            track_number: None,
            composer: String::new(),
            album_id: "album-1".to_string(),
            duration_ms: 95_000,
            preview_url: None,
        },
    ]
}

/// The one-song album the single-song stepping tests step through, shared by
/// the two `ui::transport` tests that pin the single-song edge cases (Next and
/// Previous on a one-element list). Both need this same list; building it
/// independently in each suite would let a rename in one fixture silently
/// diverge from the other, so it lives here once, next to [`stepping_songs`],
/// and is compiled only for tests.
pub(crate) fn single_song_album() -> Vec<Song> {
    vec![Song {
        id: "song-1".to_string(),
        title: "Only".to_string(),
        artist: "The Sample Band".to_string(),
        album: "First Record".to_string(),
        year: None,
        genre: String::new(),
        track_number: None,
        composer: String::new(),
        album_id: "album-1".to_string(),
        duration_ms: 180_000,
        preview_url: None,
    }]
}

/// The one-song album the browse-away tests land on after leaving `album-1`:
/// album-2's "song-4" / "B-side".
/// `songs_loaded_replaces_the_previous_albums_songs` and
/// `now_playing_label_keeps_the_track_name_after_browsing_to_another_album`
/// need this list, as do
/// `album_selected_clears_the_previous_albums_songs`,
/// `selection_messages_with_a_stale_epoch_or_index_do_nothing`,
/// `track_selected_stale_press_leaves_known_tracks_untouched`, and the three
/// bar tests through `play_song_1_then_browse_to_album_2`; a retitle or id
/// change in one copy would silently diverge from the others, so it lives
/// here once, next to [`single_song_album`], and is compiled only for tests.
pub(crate) fn second_album_songs() -> Vec<Song> {
    vec![Song {
        id: "song-4".to_string(),
        title: "B-side".to_string(),
        artist: "The Sample Band".to_string(),
        album: "Second Record".to_string(),
        year: None,
        genre: String::new(),
        track_number: None,
        composer: String::new(),
        album_id: "album-2".to_string(),
        duration_ms: 95_000,
        preview_url: None,
    }]
}

/// A single representative artist, album, and song, shared by the `library`,
/// `apple_music` (including its `rest` client), `ui::views`, and `ui` test
/// suites. Each suite needs these same objects; building them independently
/// would let a retitle or id change in one fixture silently diverge from the
/// others, so they live here once, next to [`stepping_songs`], and each test
/// only names which one it wants. The browse-mapping tests reuse these,
/// setting the artist or album id from the query where the browse supplies
/// one.
pub(crate) fn sample_artist() -> Artist {
    Artist {
        id: "artist-1".to_string(),
        name: "The Sample Band".to_string(),
        album_count: 0,
    }
}

/// The representative album fixture; [`sample_song`] belongs to it.
pub(crate) fn sample_album() -> Album {
    Album {
        id: "album-1".to_string(),
        title: "First Record".to_string(),
        artist_id: "artist-1".to_string(),
        artist: "The Sample Band".to_string(),
        duration_ms: 0,
    }
}

/// The representative playlist fixture.
pub(crate) fn sample_playlist() -> Playlist {
    Playlist {
        id: "playlist-1".to_string(),
        name: "Morning Drive".to_string(),
    }
}

/// The representative song fixture; it belongs to [`sample_album`] by
/// [`sample_artist`].
pub(crate) fn sample_song() -> Song {
    Song {
        id: "song-1".to_string(),
        title: "Opening".to_string(),
        artist: "The Sample Band".to_string(),
        album: "First Record".to_string(),
        year: None,
        genre: String::new(),
        track_number: None,
        composer: String::new(),
        album_id: "album-1".to_string(),
        duration_ms: 210_000,
        preview_url: None,
    }
}

/// The session the browser sign-in flow returns in the tests, with
/// recognizable tokens so a test can assert the service stored exactly what
/// the flow produced. The `apple_music` sign-in tests and the `ui` startup
/// test all build this same session, so the token pair lives here once.
pub(crate) fn sign_in_session() -> MusicKitSession {
    MusicKitSession {
        developer_token: "dev-token".to_string(),
        user_token: "user-token".to_string(),
    }
}

/// The session a signed-in service stores, with recognizable tokens so a test
/// can assert the transport saw exactly these credentials. The `apple_music`
/// service tests and the `apple_music::rest` client tests both build it, so
/// the token pair lives here once.
pub(crate) fn rest_session() -> MusicKitSession {
    MusicKitSession {
        developer_token: "developer-token".to_string(),
        user_token: "user-token".to_string(),
    }
}

/// The classic "Rock" preset from [`PRESETS`], shared by the `equalizer`,
/// `state`, and `ui` test suites. Each suite needs the same
/// `.find(|preset| preset.name == "Rock").expect(...)` lookup; without this
/// helper a rename in the table would fail at each call site, each repeating
/// the panic message, so the lookup lives here once and each test only names
/// the curve it wants.
pub(crate) fn rock_preset() -> Preset {
    *PRESETS
        .iter()
        .find(|preset| preset.name == "Rock")
        .expect("the preset table must contain Rock")
}
