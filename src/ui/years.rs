//! The Years browse list's rows: the library's songs grouped by release
//! year. Pure, so the grouping and its ordering are testable without a
//! player.

use super::song_groups::{SongGroup, group_songs};
use crate::library::Song;

/// One row of the Years list: a year's name and the songs released in it.
pub(in crate::ui) type Year = SongGroup;

/// Groups `songs` by release year, oldest first, with the `Unknown` group for
/// songs without a year always last. Songs keep their input order inside a
/// group.
pub(in crate::ui) fn group_by_year(songs: Vec<Song>) -> Vec<Year> {
    group_songs(songs, |song| song.year.map(|year| (year, year.to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::sample_song;

    fn song(id: &str, year: Option<u16>) -> Song {
        Song {
            id: id.to_string(),
            year,
            ..sample_song()
        }
    }

    fn ids(year: &Year) -> Vec<&str> {
        year.songs.iter().map(|song| song.id.as_str()).collect()
    }

    #[test]
    fn groups_are_ordered_oldest_first_with_songs_in_library_order() {
        let years = group_by_year(vec![
            song("1", Some(2005)),
            song("2", Some(2001)),
            song("3", Some(2005)),
        ]);

        let names: Vec<&str> = years.iter().map(|year| year.name.as_str()).collect();
        assert_eq!(names, ["2001", "2005"]);
        assert_eq!(ids(&years[1]), ["1", "3"]);
    }

    #[test]
    fn missing_years_form_a_last_unknown_group() {
        let years = group_by_year(vec![
            song("1", None),
            song("2", Some(1999)),
            song("3", None),
        ]);

        let names: Vec<&str> = years.iter().map(|year| year.name.as_str()).collect();
        assert_eq!(names, ["1999", "Unknown"]);
        assert_eq!(ids(&years[1]), ["1", "3"]);
    }
}
