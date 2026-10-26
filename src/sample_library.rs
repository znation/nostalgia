//! The in-memory stand-in for a real Apple Music library.
//!
//! The Apple Music service (`crate::apple_music`) answers a library query from
//! this shared library when no `MusicKit` session is stored — signed out, or
//! before the startup sign-in lands — and from the REST client otherwise. It
//! also gives the UI and its tests one shared library to agree on, so a test
//! needs no network. The module stays as the signed-out fallback rather than
//! being replaced by the REST client.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::library::{Album, Artist, Playlist, Song};

/// The in-memory stand-in for a real Apple Music library, kept in one place
/// so the browse flow and its tests agree on the data.
pub struct SampleLibrary {
    /// Every artist in the library, in the order the Artists view renders
    /// them.
    pub artists: Vec<Artist>,
    /// Albums of each artist, keyed by [`Album::artist_id`] — built once here
    /// so a browse query is an O(matches) lookup instead of rescanning the
    /// whole album list on every navigation.
    pub albums_by_artist: HashMap<String, Vec<Album>>,
    /// Songs of each album, keyed by [`Song::album_id`], as above.
    pub songs_by_album: HashMap<String, Vec<Song>>,
    /// Every playlist in the library, in the order the Playlists view renders
    /// them.
    pub playlists: Vec<Playlist>,
    /// Songs of each playlist, keyed by [`Playlist::id`], in the playlist's
    /// named order rather than library order.
    pub songs_by_playlist: HashMap<String, Vec<Song>>,
}

/// Groups `items` by the key each element yields, keeping each group in its
/// original order — the album-per-artist and song-per-album lookup tables,
/// built once at library construction so a browse query looks up its matches
/// rather than rescanning the whole list per query.
fn index_by<T: Clone>(items: &[T], key: impl Fn(&T) -> &str) -> HashMap<String, Vec<T>> {
    let mut index: HashMap<String, Vec<T>> = HashMap::new();
    for item in items {
        index
            .entry(key(item).to_string())
            .or_default()
            .push(item.clone());
    }
    index
}

/// The songs named by `ids`, in that order, cloned from `songs`. A playlist
/// keeps its own named order, which need not match the album order the songs
/// were built in, so its group cannot come from [`index_by`]. An id with no
/// matching song is skipped; the content test pins every group, so a missing
/// song cannot pass silently.
fn songs_in_named_order(songs: &[Song], ids: &[&str]) -> Vec<Song> {
    ids.iter()
        .filter_map(|id| songs.iter().find(|song| song.id == *id).cloned())
        .collect()
}

impl SampleLibrary {
    /// Builds the sample library: three artists — two with one or two albums
    /// each, one with none — one to three songs per album, and three playlists
    /// drawing songs from those albums in their own order.
    fn new() -> Self {
        let mut albums = vec![
            Album {
                id: "album-1".to_string(),
                title: "First Record".to_string(),
                artist_id: "artist-1".to_string(),
                artist: "The Sample Band".to_string(),
                duration_ms: 0,
            },
            Album {
                id: "album-2".to_string(),
                title: "Second Record".to_string(),
                artist_id: "artist-1".to_string(),
                artist: "The Sample Band".to_string(),
                duration_ms: 0,
            },
            Album {
                id: "album-3".to_string(),
                title: "Debut".to_string(),
                artist_id: "artist-2".to_string(),
                artist: "Echo Chamber".to_string(),
                duration_ms: 0,
            },
        ];
        let album_count = |artist_id: &str| {
            albums
                .iter()
                .filter(|album| album.artist_id == artist_id)
                .count()
        };
        let artists = vec![
            Artist {
                id: "artist-1".to_string(),
                name: "The Sample Band".to_string(),
                album_count: album_count("artist-1"),
            },
            Artist {
                id: "artist-2".to_string(),
                name: "Echo Chamber".to_string(),
                album_count: album_count("artist-2"),
            },
            Artist {
                id: "artist-3".to_string(),
                name: "Mono Tones".to_string(),
                album_count: album_count("artist-3"),
            },
        ];
        let songs = vec![
            Song {
                id: "song-1".to_string(),
                title: "Opening".to_string(),
                artist: "The Sample Band".to_string(),
                album: "First Record".to_string(),
                year: Some(2001),
                genre: "Rock".to_string(),
                track_number: Some(1),
                album_id: "album-1".to_string(),
                duration_ms: 210_000,
                preview_url: None,
            },
            Song {
                id: "song-2".to_string(),
                title: "Middle".to_string(),
                artist: "The Sample Band".to_string(),
                album: "First Record".to_string(),
                year: Some(2001),
                genre: "Rock".to_string(),
                track_number: Some(2),
                album_id: "album-1".to_string(),
                duration_ms: 240_000,
                preview_url: None,
            },
            Song {
                id: "song-3".to_string(),
                title: "Ending".to_string(),
                artist: "The Sample Band".to_string(),
                album: "First Record".to_string(),
                year: Some(2001),
                genre: "Rock".to_string(),
                track_number: Some(3),
                album_id: "album-1".to_string(),
                duration_ms: 180_000,
                preview_url: None,
            },
            Song {
                id: "song-4".to_string(),
                title: "B-side".to_string(),
                artist: "The Sample Band".to_string(),
                album: "Second Record".to_string(),
                year: Some(2003),
                genre: "Alternative".to_string(),
                track_number: Some(1),
                album_id: "album-2".to_string(),
                duration_ms: 95_000,
                preview_url: None,
            },
            Song {
                id: "song-5".to_string(),
                title: "Headliner".to_string(),
                artist: "Echo Chamber".to_string(),
                album: "Debut".to_string(),
                year: Some(2005),
                genre: "Electronic".to_string(),
                track_number: Some(1),
                album_id: "album-3".to_string(),
                duration_ms: 200_000,
                preview_url: None,
            },
        ];
        let playlists = vec![
            Playlist {
                id: "playlist-1".to_string(),
                name: "Morning Drive".to_string(),
            },
            Playlist {
                id: "playlist-2".to_string(),
                name: "Focus".to_string(),
            },
            Playlist {
                id: "playlist-3".to_string(),
                name: "Sample Favorites".to_string(),
            },
        ];
        let songs_by_playlist = HashMap::from([
            (
                "playlist-1".to_string(),
                songs_in_named_order(&songs, &["song-1", "song-4"]),
            ),
            (
                "playlist-2".to_string(),
                songs_in_named_order(&songs, &["song-2", "song-5"]),
            ),
            (
                "playlist-3".to_string(),
                songs_in_named_order(&songs, &["song-1", "song-2", "song-3"]),
            ),
        ]);
        // Each album's running time is the sum of the sample songs the
        // library reports for it, derived here rather than hardcoded so the
        // row's total follows the songs. `saturating_add` keeps a pathological
        // total from wrapping instead of wrapping to a nonsense short time.
        let songs_by_album = index_by(&songs, |song| &song.album_id);
        for album in &mut albums {
            if let Some(songs) = songs_by_album.get(&album.id) {
                album.duration_ms = songs
                    .iter()
                    .fold(0, |total, song| total.saturating_add(song.duration_ms));
            }
        }
        SampleLibrary {
            artists,
            albums_by_artist: index_by(&albums, |album| &album.artist_id),
            songs_by_album,
            playlists,
            songs_by_playlist,
        }
    }
}

/// The process-wide cache behind [`sample_library`].
static SAMPLE_LIBRARY: OnceLock<SampleLibrary> = OnceLock::new();

/// Returns the shared sample library, building it at most once.
///
/// Every browse query (favorite artists, albums by artist, songs from album,
/// every song, every album, every playlist, and songs from a playlist) reads
/// this on navigation, so it is cached in a [`OnceLock`] instead of being
/// reconstructed per call — building the library allocates every artist,
/// album, song, and playlist `String`, and the data never changes.
pub fn sample_library() -> &'static SampleLibrary {
    SAMPLE_LIBRARY.get_or_init(SampleLibrary::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Asserts that `items` yield exactly the expected `(id, label)` pairs, in
    /// order. The content-contract tests each pin one browse list's (id, label)
    /// pairs — the top-level artists and every album- and song-group in the
    /// indexed library — and would otherwise repeat the same map-to-pairs-then-
    /// compare chain, so it lives here once and each test only names its items
    /// and expected pairs.
    fn assert_id_label_pairs<T>(
        items: &[T],
        id: impl Fn(&T) -> &str,
        label: impl Fn(&T) -> &str,
        expected: &[(&str, &str)],
    ) {
        let pairs: Vec<(&str, &str)> = items.iter().map(|item| (id(item), label(item))).collect();
        assert_eq!(pairs, expected);
    }

    #[test]
    fn sample_library_is_non_empty() {
        let library = sample_library();
        assert!(!library.artists.is_empty());
        assert!(!library.albums_by_artist.is_empty());
        assert!(!library.songs_by_album.is_empty());
        assert!(!library.playlists.is_empty());
        assert!(!library.songs_by_playlist.is_empty());
    }

    // The browse views render these exact names, so the library's content is
    // the data contract the UI depends on. The service tests reach the same
    // data but assert ids only, so a renamed or reordered artist in this
    // source of truth would pass every existing test while showing wrong
    // names on screen. Pin the ids and names together, in order.
    #[test]
    fn sample_library_exposes_the_expected_artists_in_order() {
        assert_id_label_pairs(
            &sample_library().artists,
            |artist| artist.id.as_str(),
            |artist| artist.name.as_str(),
            &[
                ("artist-1", "The Sample Band"),
                ("artist-2", "Echo Chamber"),
                ("artist-3", "Mono Tones"),
            ],
        );
    }

    #[test]
    fn sample_library_artists_carry_their_album_counts() {
        let counts: Vec<usize> = sample_library()
            .artists
            .iter()
            .map(|artist| artist.album_count)
            .collect();
        assert_eq!(counts, vec![2, 1, 0]);
    }

    // `index_by` inserts only groups that exist, so an artist without albums
    // (artist-3) gets no key — never an empty group. The key set is exactly
    // the two artists that have albums, and each group is complete and in
    // library order; the ids and titles are the content the albums view
    // renders.
    #[test]
    fn albums_are_indexed_by_artist_in_library_order_with_no_empty_groups() {
        let library = sample_library();

        assert_eq!(library.albums_by_artist.len(), 2);
        assert!(!library.albums_by_artist.contains_key("artist-3"));

        assert_id_label_pairs(
            &library.albums_by_artist["artist-1"],
            |album| album.id.as_str(),
            |album| album.title.as_str(),
            &[("album-1", "First Record"), ("album-2", "Second Record")],
        );

        assert_id_label_pairs(
            &library.albums_by_artist["artist-2"],
            |album| album.id.as_str(),
            |album| album.title.as_str(),
            &[("album-3", "Debut")],
        );
    }

    // The album rows render each album's running time from `Album::duration_ms`,
    // and these totals are user-visible when no Apple Music session is stored.
    // Pin the three derived totals in artist order so a changed song duration
    // or album grouping cannot silently make a row show a wrong time. The
    // expected values are the sums of the sample songs' durations above:
    // 210+240+180 seconds, 95 seconds, and 200 seconds.
    #[test]
    fn albums_carry_their_songs_total_durations() {
        let library = sample_library();

        assert_eq!(library.albums_by_artist["artist-1"][0].duration_ms, 630_000);
        assert_eq!(library.albums_by_artist["artist-1"][1].duration_ms, 95_000);
        assert_eq!(library.albums_by_artist["artist-2"][0].duration_ms, 200_000);
    }

    // As with albums: every album with songs gets exactly one group keyed by
    // its id, in library order, and the group holds the titles the songs
    // view renders.
    #[test]
    fn songs_are_indexed_by_album_in_library_order_with_no_empty_groups() {
        let library = sample_library();

        assert_eq!(library.songs_by_album.len(), 3);

        assert_id_label_pairs(
            &library.songs_by_album["album-1"],
            |song| song.id.as_str(),
            |song| song.title.as_str(),
            &[
                ("song-1", "Opening"),
                ("song-2", "Middle"),
                ("song-3", "Ending"),
            ],
        );

        assert_id_label_pairs(
            &library.songs_by_album["album-2"],
            |song| song.id.as_str(),
            |song| song.title.as_str(),
            &[("song-4", "B-side")],
        );

        assert_id_label_pairs(
            &library.songs_by_album["album-3"],
            |song| song.id.as_str(),
            |song| song.title.as_str(),
            &[("song-5", "Headliner")],
        );
    }

    // Playlists render in their own order, and each playlist's songs keep the
    // order the playlist names, which deliberately differs from album order
    // (playlist-1 interleaves album-1 and album-2 songs). Pin every group's ids
    // and titles so a reordered or dropped group cannot pass.
    #[test]
    fn playlists_are_indexed_with_their_songs_in_named_order() {
        let library = sample_library();

        assert_id_label_pairs(
            &library.playlists,
            |playlist| playlist.id.as_str(),
            |playlist| playlist.name.as_str(),
            &[
                ("playlist-1", "Morning Drive"),
                ("playlist-2", "Focus"),
                ("playlist-3", "Sample Favorites"),
            ],
        );

        assert_eq!(library.songs_by_playlist.len(), 3);
        assert!(
            library
                .songs_by_playlist
                .values()
                .all(|songs| !songs.is_empty())
        );

        assert_id_label_pairs(
            &library.songs_by_playlist["playlist-1"],
            |song| song.id.as_str(),
            |song| song.title.as_str(),
            &[("song-1", "Opening"), ("song-4", "B-side")],
        );

        assert_id_label_pairs(
            &library.songs_by_playlist["playlist-2"],
            |song| song.id.as_str(),
            |song| song.title.as_str(),
            &[("song-2", "Middle"), ("song-5", "Headliner")],
        );

        assert_id_label_pairs(
            &library.songs_by_playlist["playlist-3"],
            |song| song.id.as_str(),
            |song| song.title.as_str(),
            &[
                ("song-1", "Opening"),
                ("song-2", "Middle"),
                ("song-3", "Ending"),
            ],
        );
    }

    /// Every song in `library`, in library order: all of `album-1`'s songs,
    /// then `album-2`'s, then `album-3`'s. The two content tests below walk the
    /// whole library in this order to pin one song field at a time, and both
    /// would otherwise repeat the same album-id list and `flat_map` chain; it
    /// lives here once so adding or reordering an album touches one place.
    fn songs_in_library_order(library: &SampleLibrary) -> impl Iterator<Item = &Song> {
        ["album-1", "album-2", "album-3"]
            .iter()
            .flat_map(|album_id| &library.songs_by_album[*album_id])
    }

    // The Now Playing bar formats a song's length from `Song::duration_ms`
    // (see `ui::views::format_track_time`), and `sample_library` is what the
    // app browses when no Apple Music session is stored, so these values are
    // user-visible. The index tests above pin each song's id and title but
    // never its duration, so a wrong or zero duration here would show a wrong
    // (or `--:--`) time in the Now Playing bar while every existing test
    // stayed green. Pin every song's duration, in library order.
    #[test]
    fn songs_carry_the_documented_durations() {
        let library = sample_library();
        let durations: Vec<(&str, u64)> = songs_in_library_order(library)
            .map(|song| (song.id.as_str(), song.duration_ms))
            .collect();

        assert_eq!(
            durations,
            vec![
                ("song-1", 210_000u64),
                ("song-2", 240_000),
                ("song-3", 180_000),
                ("song-4", 95_000),
                ("song-5", 200_000),
            ]
        );
    }

    // The Now Playing bar renders `<artist> - <title>` from `Song::artist`,
    // and `sample_library` is what the app browses when no Apple Music
    // session is stored, so these values are user-visible. As with the
    // duration test above, the index tests pin each song's id and title but
    // never its artist, so a wrong or empty artist here would make the bar
    // read a wrong name (or drop the artist) while every existing test stayed
    // green. Pin every song's artist, in library order, matching its album's
    // `artist_id`.
    #[test]
    fn songs_carry_the_documented_artists() {
        let library = sample_library();
        let artists: Vec<(&str, &str)> = songs_in_library_order(library)
            .map(|song| (song.id.as_str(), song.artist.as_str()))
            .collect();

        assert_eq!(
            artists,
            vec![
                ("song-1", "The Sample Band"),
                ("song-2", "The Sample Band"),
                ("song-3", "The Sample Band"),
                ("song-4", "The Sample Band"),
                ("song-5", "Echo Chamber"),
            ]
        );
    }

    // The File info panel draws a `Year:` line from `Song::year`, and
    // `sample_library` is what the app browses when no Apple Music session is
    // stored, so these values are user-visible. As with the duration and
    // artist tests above, the index tests pin each song's id and title but
    // never its year, so a missing or wrong year here would drop the line (or
    // show a wrong one) while every existing test stayed green. Pin every
    // song's year, in library order.
    #[test]
    fn songs_carry_the_documented_years() {
        let library = sample_library();
        let years: Vec<(&str, Option<u16>)> = songs_in_library_order(library)
            .map(|song| (song.id.as_str(), song.year))
            .collect();

        assert_eq!(
            years,
            vec![
                ("song-1", Some(2001u16)),
                ("song-2", Some(2001)),
                ("song-3", Some(2001)),
                ("song-4", Some(2003)),
                ("song-5", Some(2005)),
            ]
        );
    }

    // The File info panel draws a `Genre:` line from `Song::genre`, and
    // `sample_library` is what the app browses when no Apple Music session is
    // stored, so these values are user-visible. The index tests pin each song's
    // id and title but never its genre, so a missing or wrong genre here would
    // drop the line (or show a wrong one) while every existing test stayed
    // green. Pin every song's genre, in library order.
    #[test]
    fn songs_carry_the_documented_genres() {
        let library = sample_library();
        let genres: Vec<(&str, &str)> = songs_in_library_order(library)
            .map(|song| (song.id.as_str(), song.genre.as_str()))
            .collect();

        assert_eq!(
            genres,
            vec![
                ("song-1", "Rock"),
                ("song-2", "Rock"),
                ("song-3", "Rock"),
                ("song-4", "Alternative"),
                ("song-5", "Electronic"),
            ]
        );
    }

    // The File info panel draws a `Track #:` line from `Song::track_number`,
    // and `sample_library` is what the app browses when no Apple Music session
    // is stored, so these values are user-visible. The index tests pin each
    // song's id and title but never its track number, so a missing or wrong
    // number here would drop the line (or show a wrong one) while every
    // existing test stayed green. Pin every song's number, in library order.
    #[test]
    fn songs_carry_the_documented_track_numbers() {
        let library = sample_library();
        let numbers: Vec<(&str, Option<u32>)> = songs_in_library_order(library)
            .map(|song| (song.id.as_str(), song.track_number))
            .collect();

        assert_eq!(
            numbers,
            vec![
                ("song-1", Some(1u32)),
                ("song-2", Some(2)),
                ("song-3", Some(3)),
                ("song-4", Some(1)),
                ("song-5", Some(1)),
            ]
        );
    }

    // `sample_library` is cached in a `OnceLock` so every browse query reads
    // the same prebuilt instance instead of rebuilding the library (and its
    // `String` allocations) per call. Pointer identity across calls is the
    // observable guarantee of that caching.
    #[test]
    fn sample_library_is_cached_as_a_singleton() {
        assert!(std::ptr::eq(sample_library(), sample_library()));
    }
}
