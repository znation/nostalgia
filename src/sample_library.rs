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

/// Returns the shared sample library, building it at most once.
///
/// Every browse query (favorite artists, albums by artist, songs from album)
/// reads this on navigation, so it is cached in a [`OnceLock`] instead of
/// being reconstructed per call — building the library allocates every
/// artist, album, and song `String`, and the data never changes.
static SAMPLE_LIBRARY: OnceLock<SampleLibrary> = OnceLock::new();

pub fn sample_library() -> &'static SampleLibrary {
    SAMPLE_LIBRARY.get_or_init(SampleLibrary::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_library_is_non_empty() {
        let library = sample_library();
        assert!(!library.artists.is_empty());
        assert!(!library.albums_by_artist.is_empty());
        assert!(!library.songs_by_album.is_empty());
    }
}
