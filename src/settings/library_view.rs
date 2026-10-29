//! The top-level browse list the library reopens on at the next launch.

/// A top-level browse list: the lists the Artists view links to, plus Artists
/// itself. Drilled-in levels (an album, a playlist, search results) are not
/// here, because their ids can go stale between launches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LibraryView {
    /// The artists list, where the library opens when nothing was saved.
    #[default]
    Artists,
    /// The flat All Songs list.
    AllSongs,
    /// The flat All Albums list.
    AllAlbums,
    /// The Recently Added list.
    RecentlyAdded,
    /// The Recently Played list.
    RecentlyPlayed,
    /// The Playlists list.
    Playlists,
    /// The Genres list.
    Genres,
    /// The Years list.
    Years,
}

impl LibraryView {
    /// The word the settings file stores for this list.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::Artists => "artists",
            Self::AllSongs => "all_songs",
            Self::AllAlbums => "all_albums",
            Self::RecentlyAdded => "recently_added",
            Self::RecentlyPlayed => "recently_played",
            Self::Playlists => "playlists",
            Self::Genres => "genres",
            Self::Years => "years",
        }
    }

    /// The list a stored word names, or `None` for an unrecognized word; the
    /// exact inverse of [`Self::key`].
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        [
            Self::Artists,
            Self::AllSongs,
            Self::AllAlbums,
            Self::RecentlyAdded,
            Self::RecentlyPlayed,
            Self::Playlists,
            Self::Genres,
            Self::Years,
        ]
        .into_iter()
        .find(|view| view.key() == key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_round_trip_for_every_variant() {
        for view in [
            LibraryView::Artists,
            LibraryView::AllSongs,
            LibraryView::AllAlbums,
            LibraryView::RecentlyAdded,
            LibraryView::RecentlyPlayed,
            LibraryView::Playlists,
            LibraryView::Genres,
            LibraryView::Years,
        ] {
            assert_eq!(LibraryView::from_key(view.key()), Some(view));
        }
    }

    #[test]
    fn unknown_keys_are_none() {
        assert_eq!(LibraryView::from_key("albums"), None);
        assert_eq!(LibraryView::from_key(""), None);
    }
}
