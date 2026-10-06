//! The in-memory stand-in for a real Apple Music library.
//!
//! The Apple Music service (`crate::apple_music`) answers every browse query
//! from this shared stub until the real API lands, so the UI and its tests
//! agree on the same data. This is the stub behind the service seam: when a
//! real Apple Music library arrives, this whole module is deleted and the
//! service's browse methods get real implementations.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::library::{Album, Artist, Song};

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

impl SampleLibrary {
    /// Builds the sample library: three artists — two with one or two albums
    /// each, one with none — and one to three songs per album.
    fn new() -> Self {
        let artists = vec![
            Artist {
                id: "artist-1".to_string(),
                name: "The Sample Band".to_string(),
            },
            Artist {
                id: "artist-2".to_string(),
                name: "Echo Chamber".to_string(),
            },
            Artist {
                id: "artist-3".to_string(),
                name: "Mono Tones".to_string(),
            },
        ];
        let albums = vec![
            Album {
                id: "album-1".to_string(),
                title: "First Record".to_string(),
                artist_id: "artist-1".to_string(),
            },
            Album {
                id: "album-2".to_string(),
                title: "Second Record".to_string(),
                artist_id: "artist-1".to_string(),
            },
            Album {
                id: "album-3".to_string(),
                title: "Debut".to_string(),
                artist_id: "artist-2".to_string(),
            },
        ];
        let songs = vec![
            Song {
                id: "song-1".to_string(),
                title: "Opening".to_string(),
                album_id: "album-1".to_string(),
            },
            Song {
                id: "song-2".to_string(),
                title: "Middle".to_string(),
                album_id: "album-1".to_string(),
            },
            Song {
                id: "song-3".to_string(),
                title: "Ending".to_string(),
                album_id: "album-1".to_string(),
            },
            Song {
                id: "song-4".to_string(),
                title: "B-side".to_string(),
                album_id: "album-2".to_string(),
            },
            Song {
                id: "song-5".to_string(),
                title: "Headliner".to_string(),
                album_id: "album-3".to_string(),
            },
        ];
        SampleLibrary {
            artists,
            albums_by_artist: index_by(&albums, |album| &album.artist_id),
            songs_by_album: index_by(&songs, |song| &song.album_id),
        }
    }
}

/// The process-wide cache behind [`sample_library`].
static SAMPLE_LIBRARY: OnceLock<SampleLibrary> = OnceLock::new();

/// Returns the shared sample library, building it at most once.
///
/// Every browse query (favorite artists, albums by artist, songs from album)
/// reads this on navigation, so it is cached in a [`OnceLock`] instead of
/// being reconstructed per call — building the library allocates every
/// artist, album, and song `String`, and the data never changes.
pub fn sample_library() -> &'static SampleLibrary {
    SAMPLE_LIBRARY.get_or_init(SampleLibrary::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Asserts that `items` yield exactly the expected `(id, label)` pairs, in
    /// order. The content-contract tests each pin one browse list's (id, label)
    /// pairs — the top-level artists and every album- and song-group in the
    /// indexed library — and all used to repeat the same map-to-pairs-then-
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

    // `sample_library` is cached in a `OnceLock` so every browse query reads
    // the same prebuilt instance instead of rebuilding the library (and its
    // `String` allocations) per call. Pointer identity across calls is the
    // observable guarantee of that caching.
    #[test]
    fn sample_library_is_cached_as_a_singleton() {
        assert!(std::ptr::eq(sample_library(), sample_library()));
    }
}
