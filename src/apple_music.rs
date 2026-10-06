//! The seam to the Apple Music API.
//!
//! This is the narrow integration point a real Apple Music backend will
//! replace. Until it lands, `AppleMusicService` answers every browse query
//! from the shared in-memory sample library
//! ([`crate::sample_library::sample_library`]) and stubs playback as
//! shared-state transitions: `play_track` records the selected track and
//! marks it playing, `pause` clears the flag. The still-unimplemented stubs
//! (`next_track`, `previous_track`, and the `token` field with its
//! `AppleMusicToken` type) stay so a real implementation has a surface to
//! land on.

use reqwest::Error;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::library::{Album, Artist, Song};
use crate::sample_library::sample_library;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppleMusicToken {
    access_token: String,
    expires_in: u64,
    refresh_token: String,
}

/// The music-library service. Until the real Apple Music API lands, every
/// browse query is answered from the shared [`crate::sample_library::sample_library`],
/// so the UI and its tests agree on the same stub data.
#[derive(Clone)]
pub struct AppleMusicService {
    // Unused until a real Apple Music API sets it during authentication.
    #[allow(dead_code)]
    token: Option<AppleMusicToken>,
    state: Arc<Mutex<AppState>>,
}

/// Transport stubs kept as the seam a real Apple Music implementation will
/// fill: pause, next, and previous are not yet wired to the UI (the
/// transport.rs stepping helpers drive those buttons), so `dead_code` is
/// allowed on exactly this block and the `token` field — a *newly* dead
/// field or method elsewhere still triggers the warning the clean loop
/// relies on to find removable code.
#[allow(dead_code)]
impl AppleMusicService {
    async fn pause(&self) -> Result<(), Error> {
        // As with `play_track`, the stub only owns the shared-state
        // transition — clear the playing flag. A real implementation
        // would add the API call that pauses audio.

        let mut state = self.state.lock().await;
        state.is_playing = false;

        println!("Paused playback");
        Ok(())
    }

    async fn next_track(&self) -> Result<(), Error> {
        // In a real implementation, this would:
        // 1. Get current track position
        // 2. Play next track in library

        println!("Playing next track");
        Ok(())
    }

    async fn previous_track(&self) -> Result<(), Error> {
        // In a real implementation, this would:
        // 1. Get current track position
        // 2. Play previous track in library

        println!("Playing previous track");
        Ok(())
    }
}

/// Initializes the service.
///
/// In a real implementation, this would:
/// 1. Authenticate with Apple Music
/// 2. Get user's library
/// 3. Set up event listeners
///
/// The stub only records that the service is ready; `_state` is kept as the
/// handle a real implementation will use.
pub fn init_service(_state: Arc<Mutex<AppState>>) {
    println!("Apple Music service initialized");
}

impl AppleMusicService {
    pub fn new(state: Arc<Mutex<AppState>>) -> Self {
        Self { token: None, state }
    }

    pub async fn play_track(&self, track_id: &str) -> Result<(), Error> {
        // No real playback yet — the stub's job is the shared-state
        // transition: record the selected track and mark it playing.
        // A real implementation would add the API call that starts audio.

        let mut state = self.state.lock().await;
        state.current_track = Some(track_id.to_string());
        state.is_playing = true;

        println!("Playing track: {}", track_id);
        Ok(())
    }

    /// All favorite artists (every artist in the sample library).
    pub async fn get_favorite_artists(&self) -> Result<Vec<Artist>, Error> {
        Ok(sample_library().artists.clone())
    }

    /// Albums by the given artist; unknown artists yield an empty list.
    pub async fn get_albums_by_artist(&self, artist_id: &str) -> Result<Vec<Album>, Error> {
        Ok(lookup(&sample_library().albums_by_artist, artist_id))
    }

    /// Songs on the given album; unknown albums yield an empty list.
    pub async fn get_songs_from_album(&self, album_id: &str) -> Result<Vec<Song>, Error> {
        Ok(lookup(&sample_library().songs_by_album, album_id))
    }
}

/// The group stored under `id` in `index`, or an empty list when the id is
/// unknown. The read-side twin of `index_by`: the browse queries both look
/// up their matches in the prebuilt tables, so this get-then-clone-then-
/// default chain lives here once instead of in each query method.
fn lookup<T: Clone>(index: &HashMap<String, Vec<T>>, id: &str) -> Vec<T> {
    index.get(id).cloned().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_service() -> AppleMusicService {
        AppleMusicService::new(Arc::new(Mutex::new(AppState::default())))
    }

    #[tokio::test]
    async fn get_favorite_artists_returns_all_artists() {
        let artists = test_service().get_favorite_artists().await.unwrap();
        assert!(!artists.is_empty());
    }

    #[tokio::test]
    async fn get_albums_by_artist_returns_only_matching_albums() {
        let service = test_service();
        let albums = service.get_albums_by_artist("artist-1").await.unwrap();
        assert!(!albums.is_empty());
        assert!(albums.iter().all(|album| album.artist_id == "artist-1"));
        assert!(albums.iter().all(|album| album.artist_id != "artist-2"));
    }

    #[tokio::test]
    async fn get_albums_by_artist_returns_all_matching_albums_in_library_order() {
        // The lookup table must keep every album of a multi-album artist and
        // preserve the library's original order: a regression that dropped
        // duplicates or reversed a group would still pass the all()-style
        // checks above, so pin the exact group here.
        let albums = test_service()
            .get_albums_by_artist("artist-1")
            .await
            .unwrap();
        let ids: Vec<&str> = albums.iter().map(|album| album.id.as_str()).collect();
        assert_eq!(ids, vec!["album-1", "album-2"]);

        // An existing artist with no albums yields an empty list, distinct
        // from an unknown id (same lookup path, but worth pinning the sample).
        assert!(
            test_service()
                .get_albums_by_artist("artist-3")
                .await
                .unwrap()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn get_albums_by_artist_unknown_id_is_empty() {
        let albums = test_service()
            .get_albums_by_artist("no-such-artist")
            .await
            .unwrap();
        assert!(albums.is_empty());
    }

    #[tokio::test]
    async fn get_albums_by_artist_returns_single_album_artists_album() {
        // The empty (artist-3) and multi-album (artist-1) groups are pinned
        // above; artist-2's single-album group is the third branch of the
        // `index_by` grouping — a regression that dropped a one-element group
        // (e.g. only pushing after a second element arrives) would clear the
        // other two tests and silently lose this artist's album.
        let albums = test_service()
            .get_albums_by_artist("artist-2")
            .await
            .unwrap();
        let ids: Vec<&str> = albums.iter().map(|album| album.id.as_str()).collect();
        assert_eq!(ids, vec!["album-3"]);
    }

    #[tokio::test]
    async fn get_songs_from_album_returns_only_matching_songs() {
        let service = test_service();
        let songs = service.get_songs_from_album("album-1").await.unwrap();
        assert!(!songs.is_empty());
        assert!(songs.iter().all(|song| song.album_id == "album-1"));
    }

    #[tokio::test]
    async fn get_songs_from_album_returns_all_songs_in_library_order() {
        // As with albums: a multi-song album must come back whole and in the
        // sample library's order, not a subset or a reversed group.
        let songs = test_service()
            .get_songs_from_album("album-1")
            .await
            .unwrap();
        let ids: Vec<&str> = songs.iter().map(|song| song.id.as_str()).collect();
        assert_eq!(ids, vec!["song-1", "song-2", "song-3"]);
    }

    #[tokio::test]
    async fn get_songs_from_album_unknown_id_is_empty() {
        let songs = test_service()
            .get_songs_from_album("no-such-album")
            .await
            .unwrap();
        assert!(songs.is_empty());
    }

    #[tokio::test]
    async fn get_songs_from_album_returns_single_song_albums_song() {
        // album-1 (multi-song) and the unknown-id case are pinned above;
        // album-2's single-song group is the third branch — if the grouping
        // dropped one-element groups, this song would silently vanish.
        let songs = test_service()
            .get_songs_from_album("album-2")
            .await
            .unwrap();
        let ids: Vec<&str> = songs.iter().map(|song| song.id.as_str()).collect();
        assert_eq!(ids, vec!["song-4"]);
    }

    #[tokio::test]
    async fn play_track_sets_current_track_and_starts_playing() {
        let service = test_service();
        let state = service.state.clone();

        service.play_track("song-1").await.unwrap();

        let state = state.lock().await;
        assert_eq!(state.current_track.as_deref(), Some("song-1"));
        assert!(state.is_playing);
    }

    #[tokio::test]
    async fn pause_stops_playing_but_keeps_current_track() {
        let service = test_service();
        let state = service.state.clone();

        service.play_track("song-1").await.unwrap();
        service.pause().await.unwrap();

        let state = state.lock().await;
        assert!(!state.is_playing);
        assert_eq!(state.current_track.as_deref(), Some("song-1"));
    }
}
