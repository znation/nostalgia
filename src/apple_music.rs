use reqwest::{Client, Error};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppleMusicToken {
    access_token: String,
    expires_in: u64,
    refresh_token: String,
}

/// An artist in the user's library.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Artist {
    pub id: String,
    pub name: String,
}

/// An album by an [`Artist`], linked to it by [`Album::artist_id`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Album {
    pub id: String,
    pub title: String,
    pub artist_id: String,
}

/// A song on an [`Album`], linked to it by [`Song::album_id`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Song {
    pub id: String,
    pub title: String,
    pub album_id: String,
}

/// The music-library service. Until the real Apple Music API lands, every
/// browse query is answered from an in-memory [`sample_library`], so the UI
/// and its tests share the same stub data (consistent with the existing
/// `get_library` stub).
#[derive(Clone)]
pub struct AppleMusicService {
    client: Client,
    token: Option<AppleMusicToken>,
    state: Arc<Mutex<AppState>>,
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
        Self {
            client: Client::new(),
            token: None,
            state,
        }
    }

    async fn authenticate(&mut self, _username: &str, _password: &str) -> Result<(), Error> {
        // In a real implementation, this would:
        // 1. Exchange credentials for an access token
        // 2. Store the token for future requests

        println!("Authenticating with Apple Music...");
        Ok(())
    }

    async fn get_library(&self) -> Result<Vec<String>, Error> {
        // In a real implementation, this would:
        // 1. Make API call to get user's library
        // 2. Return list of tracks

        println!("Fetching Apple Music library...");
        Ok(vec![
            "Track 1".to_string(),
            "Track 2".to_string(),
            "Track 3".to_string(),
        ])
    }

    pub async fn play_track(&self, track_id: &str) -> Result<(), Error> {
        // In a real implementation, this would:
        // 1. Make API call to start playback
        // 2. Update the state with current track

        let mut state = self.state.lock().await;
        state.current_track = Some(track_id.to_string());
        state.is_playing = true;

        println!("Playing track: {}", track_id);
        Ok(())
    }

    async fn pause(&self) -> Result<(), Error> {
        // In a real implementation, this would:
        // 1. Make API call to pause playback
        // 2. Update the state

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

    /// All favorite artists (every artist in the sample library).
    pub async fn get_favorite_artists(&self) -> Result<Vec<Artist>, Error> {
        Ok(sample_library().artists)
    }

    /// Albums by the given artist; unknown artists yield an empty list.
    pub async fn get_albums_by_artist(&self, artist_id: &str) -> Result<Vec<Album>, Error> {
        Ok(sample_library()
            .albums
            .into_iter()
            .filter(|album| album.artist_id == artist_id)
            .collect())
    }

    /// Songs on the given album; unknown albums yield an empty list.
    pub async fn get_songs_from_album(&self, album_id: &str) -> Result<Vec<Song>, Error> {
        Ok(sample_library()
            .songs
            .into_iter()
            .filter(|song| song.album_id == album_id)
            .collect())
    }
}

/// The in-memory stand-in for a real Apple Music library, kept in one place
/// so the browse flow and its tests agree on the data.
struct SampleLibrary {
    artists: Vec<Artist>,
    albums: Vec<Album>,
    songs: Vec<Song>,
}

/// Builds the sample library: three artists, each with one or two albums,
/// each album with a couple of songs.
fn sample_library() -> SampleLibrary {
    SampleLibrary {
        artists: vec![
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
        ],
        albums: vec![
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
        ],
        songs: vec![
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
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_service() -> AppleMusicService {
        AppleMusicService::new(Arc::new(Mutex::new(AppState::default())))
    }

    #[test]
    fn sample_library_is_non_empty() {
        let library = sample_library();
        assert!(!library.artists.is_empty());
        assert!(!library.albums.is_empty());
        assert!(!library.songs.is_empty());
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
    async fn get_albums_by_artist_unknown_id_is_empty() {
        let albums = test_service()
            .get_albums_by_artist("no-such-artist")
            .await
            .unwrap();
        assert!(albums.is_empty());
    }

    #[tokio::test]
    async fn get_songs_from_album_returns_only_matching_songs() {
        let service = test_service();
        let songs = service.get_songs_from_album("album-1").await.unwrap();
        assert!(!songs.is_empty());
        assert!(songs.iter().all(|song| song.album_id == "album-1"));
    }

    #[tokio::test]
    async fn get_songs_from_album_unknown_id_is_empty() {
        let songs = test_service()
            .get_songs_from_album("no-such-album")
            .await
            .unwrap();
        assert!(songs.is_empty());
    }
}
