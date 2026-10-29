//! The Years browse list's rows: the library's songs grouped by release
//! year. Pure, so the grouping and its ordering are testable without a
//! player.

use super::genres::songs_label;
use crate::library::Song;

/// The name a song with no release year is grouped under.
const UNKNOWN_YEAR: &str = "Unknown";

/// One row of the Years list: a year's name and the songs released in it.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::ui) struct Year {
    pub(in crate::ui) name: String,
    pub(in crate::ui) songs: Vec<Song>,
}

/// Groups `songs` by release year, oldest first, with the `Unknown` group for
/// songs without a year always last. Songs keep their input order inside a
/// group.
pub(in crate::ui) fn group_by_year(songs: Vec<Song>) -> Vec<Year> {
    let mut dated: Vec<(u16, Year)> = Vec::new();
    let mut unknown = Vec::new();
    for song in songs {
        let Some(year) = song.year else {
            unknown.push(song);
            continue;
        };
        match dated.iter_mut().find(|(existing, _)| *existing == year) {
            Some((_, group)) => group.songs.push(song),
            None => dated.push((
                year,
                Year {
                    name: year.to_string(),
                    songs: vec![song],
                },
            )),
        }
    }
    dated.sort_by_key(|(year, _)| *year);
    let mut years: Vec<Year> = dated.into_iter().map(|(_, year)| year).collect();
    if !unknown.is_empty() {
        years.push(Year {
            name: UNKNOWN_YEAR.to_string(),
            songs: unknown,
        });
    }
    years
}

/// The secondary label a Years row shows: `1 song` or `N songs`.
pub(in crate::ui) fn song_count_label(year: &Year) -> String {
    songs_label(year.songs.len())
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

    #[test]
    fn count_label_pluralizes() {
        let years = group_by_year(vec![
            song("1", Some(2000)),
            song("2", Some(2001)),
            song("3", Some(2001)),
        ]);

        assert_eq!(song_count_label(&years[0]), "1 song");
        assert_eq!(song_count_label(&years[1]), "2 songs");
    }
}
