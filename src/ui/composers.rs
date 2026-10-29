//! The Composers browse list's rows: the library's songs grouped by
//! composer. Pure, so the grouping and its ordering are testable without a
//! player.

use super::song_groups::{SongGroup, group_songs};
use crate::library::Song;

/// One row of the Composers list: a composer's name and the songs they wrote.
pub(in crate::ui) type Composer = SongGroup;

/// Groups `songs` by their trimmed composer, comparing names
/// case-insensitively and naming each group after the first spelling seen.
/// Groups are ordered by lowercase name, with the `Unknown` group for blank
/// composers always last, and songs keep their input order inside a group.
pub(in crate::ui) fn group_by_composer(songs: Vec<Song>) -> Vec<Composer> {
    group_songs(songs, |song| {
        let composer = song.composer.trim();
        (!composer.is_empty()).then(|| (composer.to_lowercase(), composer.to_string()))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::sample_song;

    fn song(id: &str, composer: &str) -> Song {
        Song {
            id: id.to_string(),
            composer: composer.to_string(),
            ..sample_song()
        }
    }

    fn ids(composer: &Composer) -> Vec<&str> {
        composer.songs.iter().map(|song| song.id.as_str()).collect()
    }

    fn names(composers: &[Composer]) -> Vec<&str> {
        composers.iter().map(|c| c.name.as_str()).collect()
    }

    #[test]
    fn names_compare_case_insensitively_and_trim() {
        let composers = group_by_composer(vec![song("1", "Jane Doe"), song("2", " jane doe ")]);

        assert_eq!(names(&composers), ["Jane Doe"]);
        assert_eq!(ids(&composers[0]), ["1", "2"]);
    }

    #[test]
    fn blank_composers_form_a_last_unknown_group() {
        let composers = group_by_composer(vec![song("1", "  "), song("2", "Zed"), song("3", "")]);

        assert_eq!(names(&composers), ["Zed", "Unknown"]);
        assert_eq!(ids(&composers[1]), ["1", "3"]);
    }

    #[test]
    fn groups_are_ordered_by_name_not_first_appearance() {
        let composers =
            group_by_composer(vec![song("1", "Zed"), song("2", "amy"), song("3", "Bob")]);

        assert_eq!(names(&composers), ["amy", "Bob", "Zed"]);
    }
}
