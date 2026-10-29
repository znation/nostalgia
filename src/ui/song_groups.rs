//! The grouped browse lists' shared rows and footer: songs bucketed under a
//! key, as the Genres, Years, and Composers lists show them. Pure, so the grouping and its ordering
//! are testable without a player.

use crate::library::Song;
use crate::ui::views::format_minutes_seconds;

/// The name a song with no key value is grouped under.
const UNKNOWN: &str = "Unknown";

/// One row of a grouped list: the group's name and the songs filed under it.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::ui) struct SongGroup {
    pub(in crate::ui) name: String,
    pub(in crate::ui) songs: Vec<Song>,
}

/// Groups `songs` by `key`, which returns `None` for a song with no value
/// (it goes in the `Unknown` group) or `Some((sort_key, display_name))`.
/// Songs with equal sort keys share a group named after the first display
/// name seen. Groups are ordered by sort key, with the `Unknown` group (added
/// only when non-empty) always last, and songs keep their input order inside
/// a group.
pub(in crate::ui) fn group_songs<K: Ord>(
    songs: Vec<Song>,
    key: impl Fn(&Song) -> Option<(K, String)>,
) -> Vec<SongGroup> {
    let mut keyed: Vec<(K, SongGroup)> = Vec::new();
    let mut unknown = Vec::new();
    for song in songs {
        let Some((sort_key, name)) = key(&song) else {
            unknown.push(song);
            continue;
        };
        match keyed.iter_mut().find(|(existing, _)| *existing == sort_key) {
            Some((_, group)) => group.songs.push(song),
            None => keyed.push((
                sort_key,
                SongGroup {
                    name,
                    songs: vec![song],
                },
            )),
        }
    }
    keyed.sort_by(|(left, _), (right, _)| left.cmp(right));
    let mut groups: Vec<SongGroup> = keyed.into_iter().map(|(_, group)| group).collect();
    if !unknown.is_empty() {
        groups.push(SongGroup {
            name: UNKNOWN.to_string(),
            songs: unknown,
        });
    }
    groups
}

/// The grouping key for a free-text field such as genre or composer: `None`
/// when `text` is blank or spells `unknown` in any case (both join the
/// sentinel [`UNKNOWN`] group), otherwise its lowercase form (the sort key)
/// paired with its trimmed spelling (the display name). Treating the spelled
/// word as missing keeps one `Unknown` row: `trimmed_text_key("Unknown")`
/// would otherwise sort a group named `Unknown` among the named rows, next to
/// or apart from the sentinel.
pub(in crate::ui) fn trimmed_text_key(text: &str) -> Option<(String, String)> {
    let text = text.trim();
    (!text.is_empty() && !text.eq_ignore_ascii_case("unknown"))
        .then(|| (text.to_lowercase(), text.to_string()))
}

/// The total playing time of `songs`: the `u64::saturating_add` sum of their
/// `duration_ms` formatted by `format_minutes_seconds`, with a trailing `+`
/// when any song has no known length (`duration_ms == 0`). No songs read `0:00`.
fn songs_total<'a>(songs: impl IntoIterator<Item = &'a Song>) -> String {
    let (total_ms, unknown) = songs
        .into_iter()
        .fold((0u64, false), |(total, unknown), song| {
            (
                total.saturating_add(song.duration_ms),
                unknown || song.duration_ms == 0,
            )
        });
    let mark = if unknown { "+" } else { "" };
    format!("{}{mark}", format_minutes_seconds(total_ms))
}

/// The secondary label a grouped row shows: `"<count> song(s) - <total>"`,
/// like the song-list footer (`1 song - 3:30`, `2 songs - 7:00`). The total
/// is [`songs_total`] of the group's songs.
pub(in crate::ui) fn group_row_label(group: &SongGroup) -> String {
    let noun = if group.songs.len() == 1 {
        "song"
    } else {
        "songs"
    };
    format!(
        "{} {noun} - {}",
        group.songs.len(),
        songs_total(&group.songs)
    )
}

/// The footer a grouped list shows: `"<count> <noun> - <total>"`, like the
/// album-list footer (`2 genres - 7:10`). `noun` is `singular` exactly when
/// there is one group; the total is [`songs_total`] of every song in every
/// group, so it carries the same trailing `+` for unknown lengths. When
/// `selected` names a group the footer reads `"<count> <noun> -
/// <selected>/<total>"`, where `<selected>` is [`songs_total`] of that group's
/// songs; an index past the list gives the plain form.
pub(in crate::ui) fn group_list_summary(
    groups: &[SongGroup],
    selected: Option<usize>,
    singular: &str,
    plural: &str,
) -> String {
    let noun = if groups.len() == 1 { singular } else { plural };
    let total = songs_total(groups.iter().flat_map(|group| &group.songs));
    match selected.and_then(|index| groups.get(index)) {
        Some(group) => format!(
            "{} {noun} - {}/{total}",
            groups.len(),
            songs_total(&group.songs)
        ),
        None => format!("{} {noun} - {total}", groups.len()),
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

    fn timed(id: &str, genre: &str, duration_ms: u64) -> Song {
        Song {
            duration_ms,
            ..song(id, genre)
        }
    }

    fn by_genre(songs: Vec<Song>) -> Vec<SongGroup> {
        group_songs(songs, |song| {
            let genre = song.genre.as_str();
            (!genre.is_empty()).then(|| (genre.to_string(), genre.to_string()))
        })
    }

    fn names(groups: &[SongGroup]) -> Vec<&str> {
        groups.iter().map(|group| group.name.as_str()).collect()
    }

    #[test]
    fn groups_are_ordered_by_sort_key_not_first_appearance() {
        let groups = by_genre(vec![song("1", "b"), song("2", "a"), song("3", "b")]);

        assert_eq!(names(&groups), ["a", "b"]);
        assert_eq!(groups[1].songs.len(), 2);
    }

    #[test]
    fn equal_sort_keys_share_a_group_named_after_the_first_display_name() {
        let groups = group_songs(vec![song("1", "Pop"), song("2", "POP")], |song| {
            Some((song.genre.to_lowercase(), song.genre.clone()))
        });

        assert_eq!(names(&groups), ["Pop"]);
        assert_eq!(groups[0].songs.len(), 2);
    }

    #[test]
    fn unknown_spelled_text_is_keyless_like_blank_text() {
        assert_eq!(trimmed_text_key("Unknown"), None);
        assert_eq!(trimmed_text_key("  UNKNOWN "), None);
        assert_eq!(trimmed_text_key(""), None);
        assert_eq!(
            trimmed_text_key(" Pop "),
            Some(("pop".to_string(), "Pop".to_string()))
        );
    }

    #[test]
    fn keyless_songs_form_a_last_unknown_group_only_when_present() {
        let groups = by_genre(vec![song("1", ""), song("2", "z")]);
        assert_eq!(names(&groups), ["z", "Unknown"]);
        assert_eq!(groups[1].songs[0].id, "1");

        assert_eq!(names(&by_genre(vec![song("1", "z")])), ["z"]);
    }

    #[test]
    fn row_label_pluralizes_and_totals_the_playing_time() {
        let groups = by_genre(vec![song("1", "a"), song("2", "b"), song("3", "b")]);

        assert_eq!(group_row_label(&groups[0]), "1 song - 3:30");
        assert_eq!(group_row_label(&groups[1]), "2 songs - 7:00");
    }

    #[test]
    fn row_label_marks_a_total_with_an_unknown_length_with_a_plus() {
        let unknown = Song {
            duration_ms: 0,
            ..song("1", "a")
        };
        let known = song("2", "a");
        let group = |songs| SongGroup {
            name: "a".to_string(),
            songs,
        };

        assert_eq!(
            group_row_label(&group(vec![unknown.clone()])),
            "1 song - 0:00+"
        );
        assert_eq!(
            group_row_label(&group(vec![unknown, known])),
            "2 songs - 3:30+"
        );
    }

    #[test]
    fn list_summary_counts_groups_and_totals_every_song() {
        let two = by_genre(vec![
            timed("1", "a", 180_000),
            timed("2", "a", 120_000),
            timed("3", "b", 330_000),
        ]);
        assert_eq!(
            group_list_summary(&two, None, "genre", "genres"),
            "2 genres - 10:30"
        );

        let one = by_genre(vec![timed("1", "a", 210_000)]);
        assert_eq!(
            group_list_summary(&one, None, "genre", "genres"),
            "1 genre - 3:30"
        );

        let unknown = by_genre(vec![timed("1", "a", 210_000), timed("2", "b", 0)]);
        assert_eq!(
            group_list_summary(&unknown, None, "genre", "genres"),
            "2 genres - 3:30+"
        );

        let huge = by_genre(vec![timed("1", "a", u64::MAX), timed("2", "b", 5)]);
        assert!(group_list_summary(&huge, None, "genre", "genres").starts_with("2 genres - "));
    }

    #[test]
    fn list_summary_puts_the_selected_group_time_before_the_total() {
        let groups = by_genre(vec![
            timed("1", "a", 180_000),
            timed("2", "a", 120_000),
            timed("3", "b", 330_000),
        ]);
        assert_eq!(
            group_list_summary(&groups, Some(1), "genre", "genres"),
            "2 genres - 5:30/10:30"
        );
    }

    #[test]
    fn list_summary_marks_an_unknown_selected_group_time() {
        let groups = by_genre(vec![timed("1", "a", 210_000), timed("2", "b", 0)]);
        assert_eq!(
            group_list_summary(&groups, Some(1), "genre", "genres"),
            "2 genres - 0:00+/3:30+"
        );
    }

    #[test]
    fn list_summary_ignores_a_selection_past_the_list() {
        let groups = by_genre(vec![timed("1", "a", 210_000)]);
        assert_eq!(
            group_list_summary(&groups, Some(5), "genre", "genres"),
            "1 genre - 3:30"
        );
    }
}
