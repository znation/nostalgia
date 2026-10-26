//! The transient panels' jump handlers: Ctrl+J opens Jump to time, plain J
//! opens Jump to File, each keystroke stores the typed text, and Enter or
//! `Jump` seeks or plays the first match.

use std::time::Duration;

use iced::Task;

use super::playback::preview::seek_preview_to;
use super::window::toggle_window_shade;
use crate::ui::views;
use crate::ui::{CurrentView, Message, WinampPlayer};

/// Parses a typed jump time: plain ASCII-digit seconds (`95`) or `M:SS` with
/// one or more minute digits and one or two second digits below 60 (`1:35`,
/// `1:5`). Surrounding whitespace is ignored; anything else, including a value
/// that overflows `u64` seconds, gives `None`.
pub(in crate::ui) fn parse_jump_time(text: &str) -> Option<Duration> {
    let text = text.trim();
    let all_digits = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
    let seconds = match text.split_once(':') {
        None if all_digits(text) => text.parse::<u64>().ok()?,
        Some((minutes, seconds))
            if all_digits(minutes) && all_digits(seconds) && seconds.len() <= 2 =>
        {
            let seconds: u64 = seconds.parse().ok()?;
            if seconds >= 60 {
                return None;
            }
            minutes
                .parse::<u64>()
                .ok()?
                .checked_mul(60)?
                .checked_add(seconds)?
        }
        _ => return None,
    };
    Some(Duration::from_secs(seconds))
}

/// The cached row titles and epoch of the showing song list: the browsed Songs
/// buffer, or the flat All Songs buffer, and `None` on every other view. The
/// Jump to File panel names rows from this list and plays them through
/// `TrackSelected`, so the titles shown and the epoch/index dispatched agree
/// with the list the view drew.
fn song_list(player: &WinampPlayer) -> Option<(&[String], u64)> {
    match player.current_view {
        CurrentView::Songs => Some((&player.songs.titles, player.songs.epoch)),
        CurrentView::AllSongs => Some((&player.all_songs.titles, player.all_songs.epoch)),
        _ => None,
    }
}

/// Ctrl+J: flip the transient Jump to time panel. Opening needs a loaded
/// preview with a known length; without one the flag stays as it was. Opening
/// clears the query, closes About, File info, and Jump to File, focuses the
/// time box, and while shaded first restores the window as `toggle_about`
/// does. Closing only clears the flag. The flag is layout-only and not
/// persisted.
pub(in crate::ui) fn toggle_jump_time(player: &mut WinampPlayer) -> Task<Message> {
    if player.jump_time_visible {
        player.jump_time_visible = false;
        return Task::none();
    }
    if player.playback_duration.is_none() {
        return Task::none();
    }
    player.jump_time_visible = true;
    player.jump_time_query.clear();
    player.about_visible = false;
    player.file_info_visible = false;
    player.jump_file_visible = false;
    let focus = iced::widget::operation::focus(views::JUMP_TIME_INPUT_ID);
    if player.shaded {
        Task::batch([toggle_window_shade(player), focus])
    } else {
        focus
    }
}

/// A keystroke in the time box: store the typed text.
pub(in crate::ui) fn jump_time_changed(player: &mut WinampPlayer, text: String) -> Task<Message> {
    player.jump_time_query = text;
    Task::none()
}

/// Enter or `Jump`: with the panel open, a parsable query, and a known preview
/// length, seek to the typed time (clamped to the length) and close the panel.
/// Otherwise nothing changes, so a typo leaves the panel open.
pub(in crate::ui) fn jump_time_submitted(player: &mut WinampPlayer) -> Task<Message> {
    if !player.jump_time_visible {
        return Task::none();
    }
    let (Some(target), Some(length)) = (
        parse_jump_time(&player.jump_time_query),
        player.playback_duration,
    ) else {
        return Task::none();
    };
    seek_preview_to(player, target.min(length));
    player.jump_time_visible = false;
    Task::none()
}

/// Plain J: flip the transient Jump to File panel. Opening needs a showing song
/// list ([`song_list`]); elsewhere the flag stays as it was. Opening clears the
/// filter, closes About, File info, and Jump to time, focuses the filter box,
/// and while shaded first restores the window as `toggle_about` does. Closing
/// only clears the flag. The flag is layout-only and not persisted.
pub(in crate::ui) fn toggle_jump_file(player: &mut WinampPlayer) -> Task<Message> {
    if player.jump_file_visible {
        player.jump_file_visible = false;
        return Task::none();
    }
    if song_list(player).is_none() {
        return Task::none();
    }
    player.jump_file_visible = true;
    player.jump_file_query.clear();
    player.about_visible = false;
    player.file_info_visible = false;
    player.jump_time_visible = false;
    let focus = iced::widget::operation::focus(views::JUMP_FILE_INPUT_ID);
    if player.shaded {
        Task::batch([toggle_window_shade(player), focus])
    } else {
        focus
    }
}

/// A keystroke in the Jump to File filter box: store the typed text so the view
/// re-filters live.
pub(in crate::ui) fn jump_file_changed(player: &mut WinampPlayer, text: String) -> Task<Message> {
    player.jump_file_query = text;
    Task::none()
}

/// Enter or `Jump`: with the panel open, play the first matching row from the
/// showing list through `TrackSelected` and close the panel. No match leaves
/// the panel and the query alone, so a typo does not dismiss it.
pub(in crate::ui) fn jump_file_submitted(player: &mut WinampPlayer) -> Task<Message> {
    if !player.jump_file_visible {
        return Task::none();
    }
    let first_match = song_list(player).and_then(|(titles, epoch)| {
        views::jump_file_matches(titles, &player.jump_file_query)
            .first()
            .map(|&index| (epoch, index))
    });
    let Some((epoch, index)) = first_match else {
        return Task::none();
    };
    player.jump_file_visible = false;
    Task::done(Message::TrackSelected { epoch, index })
}

/// A press on a Jump to File row: play that source index through
/// `TrackSelected` and close the panel. `TrackSelected`'s existing epoch/index
/// checks reject a press that outlived its list.
pub(in crate::ui) fn jump_file_row_selected(
    player: &mut WinampPlayer,
    epoch: u64,
    index: usize,
) -> Task<Message> {
    if !player.jump_file_visible {
        return Task::none();
    }
    player.jump_file_visible = false;
    Task::done(Message::TrackSelected { epoch, index })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_seconds_and_minutes_seconds() {
        let secs = Duration::from_secs;
        assert_eq!(parse_jump_time("95"), Some(secs(95)));
        assert_eq!(parse_jump_time(" 1:35 "), Some(secs(95)));
        assert_eq!(parse_jump_time("01:05"), Some(secs(65)));
        assert_eq!(parse_jump_time("1:5"), Some(secs(65)));
        assert_eq!(parse_jump_time("0:00"), Some(secs(0)));
    }

    #[test]
    fn rejects_malformed_times() {
        for text in [
            "",
            "1:60",
            "1:",
            ":30",
            "1:2:3",
            "-5",
            "+5",
            "1m",
            "1:005",
            "99999999999999999999999",
        ] {
            assert_eq!(parse_jump_time(text), None, "{text:?}");
        }
    }
}
