//! Browse and search queries for [`AppleMusicService`].

use std::sync::Arc;

use super::rest;
use super::sample::{
    lookup, sample_all_albums, sample_all_playlists, sample_all_songs,
    sample_recently_added_albums, sample_recently_played_songs, sample_songs_matching,
};
use super::threads::off_thread;
use super::validation::{IdKind, ensure_id_is_valid};
use super::{AppleMusicError, AppleMusicService};
use crate::library::{Album, Artist, Playlist, Song, album_counts_by_artist};
use crate::music_kit_auth::MusicKitSession;
use crate::sample_library::sample_library;

/// Reads the whole-library song durations and fills each of `albums` from
/// them, returning the albums. The durations map is keyed by the exact
/// `(album title, artist display name)` pair the whole-library songs read
/// groups by; a miss — no song reported that album — sets `0`, which the row
/// renders as the `--:--` placeholder. The lookup clones the key's two parts
/// because the map owns its strings while `albums` borrows its own.
fn with_album_durations(
    rest: &rest::RestLibrary,
    session: &MusicKitSession,
    mut albums: Vec<Album>,
) -> Result<Vec<Album>, AppleMusicError> {
    let durations = rest.get_album_durations(session)?;
    for album in &mut albums {
        album.duration_ms = durations
            .get(&(album.title.clone(), album.artist.clone()))
            .copied()
            .unwrap_or(0);
    }
    Ok(albums)
}

impl AppleMusicService {
    /// All favorite artists: the signed-in user's library artists when a
    /// session is stored, and every sample-library artist otherwise.
    ///
    /// A signed-in load also reads the whole albums collection (one extra
    /// paginated request) and counts its albums by exact [`Album::artist`]
    /// display name, because `get_all_albums` deliberately leaves
    /// [`Album::artist_id`] empty. An artist whose name matches no album gets
    /// a count of `0`. Either request failing surfaces as this query's
    /// [`AppleMusicError`], which the UI's existing Retry path handles.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when either signed-in request fails; with
    /// no session the sample-library lookup never fails.
    pub async fn get_favorite_artists(&self) -> Result<Vec<Artist>, AppleMusicError> {
        self.browse(
            |rest, session| {
                let mut artists = rest.get_favorite_artists(session)?;
                let albums = rest.get_all_albums(session)?;
                let counts = album_counts_by_artist(&albums);
                for artist in &mut artists {
                    artist.album_count = counts.get(artist.name.as_str()).copied().unwrap_or(0);
                }
                Ok(artists)
            },
            || sample_library().artists.clone(),
        )
        .await
    }

    /// Albums by the given artist: the signed-in user's library albums when a
    /// session is stored, and the sample library's matching albums otherwise
    /// (unknown artists yield an empty list).
    ///
    /// A blank `artist_id` — empty or only whitespace — or one carrying a
    /// terminal control character is rejected with an [`AppleMusicError`]
    /// instead of looked up: no artist has a blank id, so a blank id is a
    /// caller bug, and returning the empty list would report it as the
    /// ordinary "no albums" case. The same id guard
    /// [`AppleMusicService::play_track`] applies to its track id.
    ///
    /// A signed-in load also reads the whole-library songs collection (one
    /// extra paginated request) and fills each album's [`Album::duration_ms`]
    /// from the songs' summed durations; a failed songs request fails the
    /// whole album load through the existing Retry path. The signed-out sample
    /// library's albums already carry their durations.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when `artist_id` is blank (empty or only
    /// whitespace), carries a control character, or either signed-in request
    /// fails.
    pub async fn get_albums_by_artist(
        &self,
        artist_id: &str,
    ) -> Result<Vec<Album>, AppleMusicError> {
        ensure_id_is_valid(artist_id, IdKind::Artist)?;
        let id = artist_id.to_string();
        self.browse(
            move |rest, session| {
                let albums = rest.get_albums_by_artist(session, &id)?;
                with_album_durations(rest, session, albums)
            },
            || lookup(&sample_library().albums_by_artist, artist_id),
        )
        .await
    }

    /// Songs on the given album: the signed-in user's library songs when a
    /// session is stored, and the sample library's matching songs otherwise
    /// (unknown albums yield an empty list).
    ///
    /// A blank `album_id` — empty or only whitespace — or one carrying a
    /// terminal control character is rejected with an [`AppleMusicError`], the
    /// album-query twin of [`AppleMusicService::get_albums_by_artist`].
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when `album_id` is blank (empty or only
    /// whitespace), carries a control character, or the signed-in request
    /// fails.
    pub async fn get_songs_from_album(&self, album_id: &str) -> Result<Vec<Song>, AppleMusicError> {
        ensure_id_is_valid(album_id, IdKind::Album)?;
        let id = album_id.to_string();
        self.browse(
            move |rest, session| rest.get_songs_from_album(session, &id),
            || lookup(&sample_library().songs_by_album, album_id),
        )
        .await
    }

    /// Every playlist in the library: the signed-in user's whole Apple Music
    /// library when a session is stored, and every sample-library playlist
    /// otherwise.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the signed-in request fails; with
    /// no session the sample-library answer never fails.
    pub async fn get_playlists(&self) -> Result<Vec<Playlist>, AppleMusicError> {
        self.browse(
            |rest, session| rest.get_all_playlists(session),
            sample_all_playlists,
        )
        .await
    }

    /// Songs in the given playlist, in the playlist's named order: the
    /// signed-in user's Apple Music library when a session is stored, and the
    /// sample library otherwise (unknown playlists yield an empty list).
    ///
    /// A blank `playlist_id` — empty or only whitespace — or one carrying a
    /// control character is rejected with an [`AppleMusicError`], the
    /// playlist-query twin of
    /// [`AppleMusicService::get_songs_from_album`].
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when `playlist_id` is blank (empty or
    /// only whitespace) or carries a control character, or when the
    /// signed-in request fails; the sample-library lookup otherwise never
    /// fails.
    pub async fn get_songs_from_playlist(
        &self,
        playlist_id: &str,
    ) -> Result<Vec<Song>, AppleMusicError> {
        ensure_id_is_valid(playlist_id, IdKind::Playlist)?;
        let id = playlist_id.to_string();
        self.browse(
            move |rest, session| rest.get_songs_from_playlist(session, &id),
            || lookup(&sample_library().songs_by_playlist, playlist_id),
        )
        .await
    }

    /// Every song in the library: the signed-in user's whole Apple Music
    /// library when a session is stored, and every sample-library song
    /// otherwise.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the signed-in request fails; with
    /// no session the sample-library lookup never fails.
    pub async fn get_all_songs(&self) -> Result<Vec<Song>, AppleMusicError> {
        self.browse(
            |rest, session| rest.get_all_songs(session),
            sample_all_songs,
        )
        .await
    }

    /// Every album in the library: the signed-in user's whole Apple Music
    /// library when a session is stored, and every sample-library album
    /// otherwise.
    ///
    /// A signed-in load reads the albums collection first and the whole-library
    /// songs collection second (one extra paginated request), then fills each
    /// album's [`Album::duration_ms`] from the songs' summed durations; a
    /// failed songs request fails the whole album load through the existing
    /// Retry path. The signed-out sample library's albums already carry their
    /// durations.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when either signed-in request fails; with
    /// no session the sample-library lookup never fails.
    pub async fn get_all_albums(&self) -> Result<Vec<Album>, AppleMusicError> {
        self.browse(
            |rest, session| {
                let albums = rest.get_all_albums(session)?;
                with_album_durations(rest, session, albums)
            },
            sample_all_albums,
        )
        .await
    }

    /// The library's recently added albums, newest first: the signed-in user's
    /// `recently-added` library albums when a session is stored, and the
    /// sample library's albums in reverse order otherwise.
    ///
    /// Like [`AppleMusicService::get_all_albums`], a signed-in load also reads the
    /// whole-library songs collection (one extra paginated request) to fill each
    /// album's [`Album::duration_ms`]; a failed songs request fails the whole load
    /// through the existing Retry path.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when either signed-in request fails; with
    /// no session the sample-library lookup never fails.
    pub async fn get_recently_added_albums(&self) -> Result<Vec<Album>, AppleMusicError> {
        self.browse(
            |rest, session| {
                let albums = rest.get_recently_added_albums(session)?;
                with_album_durations(rest, session, albums)
            },
            sample_recently_added_albums,
        )
        .await
    }

    /// The songs the user played most recently: the signed-in user's Apple Music play history
    /// when a session is stored, and the sample library's songs in reverse order otherwise.
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when the signed-in request fails; with
    /// no session the sample-library lookup never fails.
    pub async fn get_recently_played_songs(&self) -> Result<Vec<Song>, AppleMusicError> {
        self.browse(
            |rest, session| rest.get_recently_played_songs(session),
            sample_recently_played_songs,
        )
        .await
    }

    /// Songs in the library matching `query`: every song whose title, artist,
    /// album title, genre, or composer contains `query` case-insensitively, in library order.
    ///
    /// A signed-in service answers from the signed-in user's Apple Music
    /// library through [`rest::RestLibrary::search_library`]; a signed-out one
    /// filters the in-memory sample library. A blank `query` — empty or only
    /// whitespace — is rejected with an [`AppleMusicError`] instead of answered
    /// with the whole library: a blank search has nothing to match, and
    /// returning every song would report the caller's empty input as an
    /// ordinary "everything matches" result. The query guard is the browse
    /// queries' id guard's twin (see [`ensure_id_is_valid`]).
    ///
    /// # Errors
    ///
    /// Returns an [`AppleMusicError`] when `query` is blank (empty or only
    /// whitespace), or when the signed-in search request fails.
    pub async fn search_songs(&self, query: &str) -> Result<Vec<Song>, AppleMusicError> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return Err(AppleMusicError::new(format!(
                "search query must not be blank (got {query:?})"
            )));
        }
        let term = trimmed.to_string();
        self.browse(
            move |rest, session| rest.search_library(session, &term),
            || sample_songs_matching(trimmed),
        )
        .await
    }

    /// Answers a browse query from the signed-in REST library when a session
    /// is stored, and from `sample` otherwise.
    ///
    /// `call` runs on a worker thread through `off_thread`, so the REST
    /// client's blocking HTTP request never stalls iced's executor; `sample`
    /// runs on the caller's thread and only when no session is stored, so the
    /// offline path never touches the REST client.
    async fn browse<T: Send + 'static>(
        &self,
        call: impl FnOnce(&rest::RestLibrary, &MusicKitSession) -> Result<Vec<T>, AppleMusicError>
        + Send
        + 'static,
        sample: impl FnOnce() -> Vec<T>,
    ) -> Result<Vec<T>, AppleMusicError> {
        match self.session() {
            Some(session) => {
                let rest = Arc::clone(&self.rest);
                off_thread(move || call(&rest, &session)).await
            }
            None => Ok(sample()),
        }
    }
}
