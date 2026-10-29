//! The grouped browse lists' shared rows: songs bucketed under a key, as the
//! Genres and Years lists show them. Pure, so the grouping and its ordering
//! are testable without a player.

use crate::library::Song;

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
/// when `text` is blank, otherwise its lowercase form (the sort key) paired
/// with its trimmed spelling (the display name).
pub(in crate::ui) fn trimmed_text_key(text: &str) -> Option<(String, String)> {
    let text = text.trim();
    (!text.is_empty()).then(|| (text.to_lowercase(), text.to_string()))
}

/// The secondary label a grouped row shows: `1 song` or `N songs`.
pub(in crate::ui) fn song_count_label(group: &SongGroup) -> String {
    match group.songs.len() {
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
    fn keyless_songs_form_a_last_unknown_group_only_when_present() {
        let groups = by_genre(vec![song("1", ""), song("2", "z")]);
        assert_eq!(names(&groups), ["z", "Unknown"]);
        assert_eq!(groups[1].songs[0].id, "1");

        assert_eq!(names(&by_genre(vec![song("1", "z")])), ["z"]);
    }

    #[test]
    fn count_label_pluralizes() {
        let groups = by_genre(vec![song("1", "a"), song("2", "b"), song("3", "b")]);

        assert_eq!(song_count_label(&groups[0]), "1 song");
        assert_eq!(song_count_label(&groups[1]), "2 songs");
    }
}
