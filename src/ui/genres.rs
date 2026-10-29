//! The Genres browse list's rows: the library's songs grouped by primary
//! genre. Pure, so the grouping and its ordering are testable without a
//! player.

use crate::library::Song;

/// The name a song with a blank genre is grouped under.
const UNKNOWN_GENRE: &str = "Unknown";

/// One row of the Genres list: a genre's name and the songs filed under it.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::ui) struct Genre {
    pub(in crate::ui) name: String,
    pub(in crate::ui) songs: Vec<Song>,
}

/// Groups `songs` by their trimmed genre, comparing names case-insensitively
/// and naming each group after the first spelling seen. Groups are ordered by
/// lowercase name, with the `Unknown` group for blank genres always last, and
/// songs keep their input order inside a group.
pub(in crate::ui) fn group_by_genre(songs: Vec<Song>) -> Vec<Genre> {
    let mut named: Vec<(String, Genre)> = Vec::new();
    let mut unknown = Vec::new();
    for song in songs {
        let genre = song.genre.trim();
        if genre.is_empty() {
            unknown.push(song);
            continue;
        }
        let key = genre.to_lowercase();
        match named.iter_mut().find(|(existing, _)| *existing == key) {
            Some((_, group)) => group.songs.push(song),
            None => named.push((
                key,
                Genre {
                    name: genre.to_string(),
                    songs: vec![song],
                },
            )),
        }
    }
    named.sort_by(|(left, _), (right, _)| left.cmp(right));
    let mut genres: Vec<Genre> = named.into_iter().map(|(_, genre)| genre).collect();
    if !unknown.is_empty() {
        genres.push(Genre {
            name: UNKNOWN_GENRE.to_string(),
            songs: unknown,
        });
    }
    genres
}

/// The secondary label a Genres row shows: `1 song` or `N songs`.
pub(in crate::ui) fn song_count_label(genre: &Genre) -> String {
    songs_label(genre.songs.len())
}

/// `1 song` or `N songs`, shared by the lists that count a group's songs.
pub(in crate::ui) fn songs_label(count: usize) -> String {
    match count {
        1 => "1 song".to_string(),
        count => format!("{count} songs"),
    }
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

    #[test]
    fn count_label_pluralizes() {
        let genres = group_by_genre(vec![song("1", "A"), song("2", "B"), song("3", "B")]);

        assert_eq!(song_count_label(&genres[0]), "1 song");
        assert_eq!(song_count_label(&genres[1]), "2 songs");
    }
}
