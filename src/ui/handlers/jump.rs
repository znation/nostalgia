//! The Jump to time panel's handlers: Ctrl+J opens it, each keystroke stores
//! the typed text, and Enter or `Jump` seeks the loaded preview to it.

use std::time::Duration;

use iced::Task;

use super::playback::preview::seek_preview_to;
use super::window::toggle_window_shade;
use crate::ui::views;
use crate::ui::{Message, WinampPlayer};

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

/// Ctrl+J: flip the transient Jump to time panel. Opening needs a loaded
/// preview with a known length; without one the flag stays as it was. Opening
/// clears the query, closes About and File info, focuses the time box, and
/// while shaded first restores the window as `toggle_about` does. Closing only
/// clears the flag. The flag is layout-only and not persisted.
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
