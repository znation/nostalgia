//! Sample-library fallbacks for the browse and search queries.

use std::collections::HashMap;

use crate::library::{Album, Playlist, Song};
use crate::sample_library::sample_library;

/// The group stored under `id` in `index`, or an empty list when the id is
/// unknown. The read-side twin of `index_by`: the browse queries both look
/// up their matches in the prebuilt tables, so this get-then-clone-then-
/// default chain lives here once instead of in each query method.
pub(super) fn lookup<T: Clone>(index: &HashMap<String, Vec<T>>, id: &str) -> Vec<T> {
    index.get(id).cloned().unwrap_or_default()
}

/// Every sample-library album in library order: the ordered
/// `artists` → `albums_by_artist` walk. Both the flat album list and
/// [`sample_songs_in_library_order`] start from this walk, so the albums the
/// All Albums view shows and the album order behind the flat song list cannot
/// drift apart. The library indexes albums by artist in a `HashMap`, whose
/// iteration order is unspecified, so walking it directly would shuffle the
/// results; this chain keeps them stable and in the same order the Albums
/// view shows a browsed artist's albums. Pure, so the order is testable
/// without a service.
fn sample_albums_in_library_order() -> impl Iterator<Item = &'static Album> {
    let library = sample_library();
    library.artists.iter().flat_map(|artist| {
        library
            .albums_by_artist
            .get(&artist.id)
            .into_iter()
            .flatten()
    })
}

/// Every sample-library album, in library order.
pub(super) fn sample_all_albums() -> Vec<Album> {
    sample_albums_in_library_order().cloned().collect()
}

/// Every sample-library album, last first. The sample library carries no added dates, so the
/// reversed library order stands in for newest first.
pub(super) fn sample_recently_added_albums() -> Vec<Album> {
    let mut albums = sample_all_albums();
    albums.reverse();
    albums
}

/// Every sample-library song, last first. The sample library keeps no play history, so the
/// reversed library order stands in for most recently played first.
pub(super) fn sample_recently_played_songs() -> Vec<Song> {
    let mut songs = sample_all_songs();
    songs.reverse();
    songs
}

/// Every sample-library playlist, in library order.
pub(super) fn sample_all_playlists() -> Vec<Playlist> {
    sample_library().playlists.clone()
}

/// Every sample-library song in library order: the ordered
/// `artists` → `albums_by_artist` → `songs_by_album` walk, so
/// [`sample_all_songs`] and [`sample_songs_matching`] share one order and
/// cannot drift apart. The album ordering comes from
/// [`sample_albums_in_library_order`], the same walk the flat All Albums view
/// uses. The library indexes songs by album in a `HashMap`, whose iteration
/// order is unspecified, so walking it directly would shuffle the results;
/// this chain keeps them stable and in the same order the Songs view shows a
/// browsed album. Pure, so the order is testable without a service.
fn sample_songs_in_library_order() -> impl Iterator<Item = &'static Song> {
    let library = sample_library();
    sample_albums_in_library_order()
        .flat_map(|album| library.songs_by_album.get(&album.id).into_iter().flatten())
}

/// Every sample-library song, in library order.
pub(super) fn sample_all_songs() -> Vec<Song> {
    sample_songs_in_library_order().cloned().collect()
}

/// Every sample-library album title, keyed by album id and case-folded, for
/// the search's album-title match. Walks [`sample_albums_in_library_order`],
/// so a song's album is looked up by the id the browse lists already carry.
fn sample_album_titles_by_id() -> HashMap<&'static str, String> {
    sample_albums_in_library_order()
        .map(|album| (album.id.as_str(), album.title.to_lowercase()))
        .collect()
}

/// Every sample-library song for which each whitespace-separated word of
/// `query` is contained case-insensitively in its title, artist, album title,
/// genre, composer, or release year, in library order. Each word may match a
/// different field, in any order.
///
/// `AppleMusicService::search_songs` rejects a blank query before this runs,
/// so the no-words case is unreachable there.
///
/// Walks [`sample_songs_in_library_order`], so the matches keep the same
/// stable order the Songs view shows a browsed album. Pure, so the filter is
/// testable without a service.
pub(super) fn sample_songs_matching(query: &str) -> Vec<Song> {
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    let album_titles = sample_album_titles_by_id();
    sample_songs_in_library_order()
        .filter(|song| {
            words
                .iter()
                .all(|word| song_matches_word(song, &album_titles, word))
        })
        .cloned()
        .collect()
}

/// Whether the case-folded `word` appears in the song's title, artist, genre,
/// composer, release year, or album title (looked up in `album_titles`).
fn song_matches_word(song: &Song, album_titles: &HashMap<&str, String>, word: &str) -> bool {
    song.title.to_lowercase().contains(word)
        || song.artist.to_lowercase().contains(word)
        || song.genre.to_lowercase().contains(word)
        || song.composer.to_lowercase().contains(word)
        || song
            .year
            .is_some_and(|year| year.to_string().contains(word))
        || album_titles
            .get(song.album_id.as_str())
            .is_some_and(|title| title.contains(word))
}
