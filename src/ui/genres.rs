//! The Genres browse list's rows: the library's songs grouped by primary
//! genre. Pure, so the grouping and its ordering are testable without a
//! player.

use super::song_groups::{SongGroup, group_songs};
use crate::library::Song;

/// One row of the Genres list: a genre's name and the songs filed under it.
pub(in crate::ui) type Genre = SongGroup;

/// Groups `songs` by their trimmed genre, comparing names case-insensitively
/// and naming each group after the first spelling seen. Groups are ordered by
/// lowercase name, with the `Unknown` group for blank genres always last, and
/// songs keep their input order inside a group.
pub(in crate::ui) fn group_by_genre(songs: Vec<Song>) -> Vec<Genre> {
    group_songs(songs, |song| {
        let genre = song.genre.trim();
        (!genre.is_empty()).then(|| (genre.to_lowercase(), genre.to_string()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::sample_song;

    fn song(id: &str, genre: &str) -> Song {
        Song {
            id: id.to_string(),
            genre: genre.to_string(),
            ..sample_song()
        }
    }

    fn ids(genre: &Genre) -> Vec<&str> {
        genre.songs.iter().map(|song| song.id.as_str()).collect()
    }

    #[test]
    fn groups_are_ordered_by_name_with_songs_in_library_order() {
        let genres = group_by_genre(vec![
            song("1", "Rock"),
            song("2", "Jazz"),
            song("3", "Rock"),
        ]);

        let names: Vec<&str> = genres.iter().map(|genre| genre.name.as_str()).collect();
        assert_eq!(names, ["Jazz", "Rock"]);
        assert_eq!(ids(&genres[1]), ["1", "3"]);
    }

    #[test]
    fn spellings_merge_case_insensitively_under_the_first_one_seen() {
        let genres = group_by_genre(vec![song("1", "hip-hop"), song("2", " Hip-Hop ")]);

        assert_eq!(genres.len(), 1);
        assert_eq!(genres[0].name, "hip-hop");
        assert_eq!(ids(&genres[0]), ["1", "2"]);
    }

    #[test]
    fn blank_genres_form_a_last_unknown_group() {
        let genres = group_by_genre(vec![song("1", ""), song("2", "Zydeco"), song("3", "  ")]);

        let names: Vec<&str> = genres.iter().map(|genre| genre.name.as_str()).collect();
        assert_eq!(names, ["Zydeco", "Unknown"]);
        assert_eq!(ids(&genres[1]), ["1", "3"]);
    }
}
