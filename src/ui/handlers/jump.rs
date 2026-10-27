//! The transient panels' jump handlers: Ctrl+J opens Jump to time, plain J
//! opens Jump to File, each keystroke stores the typed text, Up, Down, Page Up,
//! and Page Down move the Jump to File highlight, and Enter or `Jump` seeks or
//! plays it.

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
/// clears the query, closes About, Options, File info, and Jump to File, focuses the
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
    player.close_transient_panels();
    player.jump_time_visible = true;
    player.jump_time_query.clear();
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
/// filter, closes About, Options, File info, and Jump to time, focuses the filter box,
/// and while shaded first restores the window as `toggle_about` does. Closing
/// only clears the flag. The flag is layout-only and not persisted.
pub(in crate::ui) fn toggle_jump_file(player: &mut WinampPlayer) -> Task<Message> {
    if player.jump_file_visible {
        player.jump_file_visible = false;
        player.jump_file_selected = None;
        return Task::none();
    }
    if song_list(player).is_none() {
        return Task::none();
    }
    player.close_transient_panels();
    player.jump_file_visible = true;
    player.jump_file_query.clear();
    player.jump_file_selected = None;
    let focus = iced::widget::operation::focus(views::JUMP_FILE_INPUT_ID);
    if player.shaded {
        Task::batch([toggle_window_shade(player), focus])
    } else {
        focus
    }
}

/// A keystroke in the Jump to File filter box: store the typed text so the view
/// re-filters live, and put the highlight back on the first match.
pub(in crate::ui) fn jump_file_changed(player: &mut WinampPlayer, text: String) -> Task<Message> {
    player.jump_file_query = text;
    player.jump_file_selected = None;
    Task::none()
}

/// Moves the Jump to File highlight `step` matches from its current position
/// (a `None` selection starts from the first match), clamped to the first or
/// last match without wrapping. No-op unless the panel is open; an empty match
/// list clears the highlight. The highlight stores the match's source index,
/// so a filtered row keeps its source-list number, and the returned task
/// scrolls the match list only as far as needed to show the landed row.
fn move_jump_file_selection(player: &mut WinampPlayer, step: isize) -> Task<Message> {
    if !player.jump_file_visible {
        return Task::none();
    }
    let Some((titles, _)) = song_list(player) else {
        return Task::none();
    };
    let matches = views::jump_file_matches(titles, &player.jump_file_query);
    if matches.is_empty() {
        player.jump_file_selected = None;
        return Task::none();
    }
    let position = views::jump_file_highlighted(&matches, player.jump_file_selected)
        .and_then(|selected| matches.iter().position(|&index| index == selected))
        .unwrap_or(0);
    let next = (position as isize + step).clamp(0, matches.len() as isize - 1) as usize;
    player.jump_file_selected = Some(matches[next]);
    reveal_jump_file_row(next)
}

/// Up while the Jump to File panel is open: highlight the previous match,
/// stopping at the first.
pub(in crate::ui) fn jump_file_selection_up(player: &mut WinampPlayer) -> Task<Message> {
    move_jump_file_selection(player, -1)
}

/// Down while the Jump to File panel is open: highlight the next match,
/// stopping at the last.
pub(in crate::ui) fn jump_file_selection_down(player: &mut WinampPlayer) -> Task<Message> {
    move_jump_file_selection(player, 1)
}

/// Page Up while the Jump to File panel is open: highlight the match one page
/// of visible rows above, stopping at the first.
pub(in crate::ui) fn jump_file_selection_page_up(player: &mut WinampPlayer) -> Task<Message> {
    move_jump_file_selection(player, -views::JUMP_FILE_PAGE_ROWS)
}

/// Page Down while the Jump to File panel is open: highlight the match one
/// page of visible rows below, stopping at the last.
pub(in crate::ui) fn jump_file_selection_page_down(player: &mut WinampPlayer) -> Task<Message> {
    move_jump_file_selection(player, views::JUMP_FILE_PAGE_ROWS)
}

/// The top edge of the Jump to File row at `position`, measured from the top
/// of the match list's scrollable. It counts the list padding and each row's
/// drawn height plus the gap before it.
pub(in crate::ui) fn jump_file_row_top(position: usize) -> f32 {
    views::JUMP_FILE_LIST_PADDING
        + position as f32 * (views::JUMP_FILE_ROW_HEIGHT + views::JUMP_FILE_ROW_SPACING)
}

/// Scrolls the Jump to File match list only as far as needed to show the row
/// at `position` fully, and not at all when it is already visible. It reads
/// the match list's current offset and viewport from the widget operation and
/// touches only that list's scroll offset.
fn reveal_jump_file_row(position: usize) -> Task<Message> {
    use iced::advanced::widget::operation::Scrollable;
    use iced::advanced::widget::{Id, Operation};

    struct Reveal {
        position: usize,
    }

    impl<T> Operation<T> for Reveal {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
            operate(self);
        }

        fn scrollable(
            &mut self,
            id: Option<&Id>,
            bounds: iced::Rectangle,
            _content_bounds: iced::Rectangle,
            translation: iced::Vector,
            state: &mut dyn Scrollable,
        ) {
            if id != Some(&Id::new(views::JUMP_FILE_LIST_ID)) {
                return;
            }
            if let Some(y) = super::browse::scroll::reveal_offset(
                jump_file_row_top(self.position),
                views::JUMP_FILE_ROW_HEIGHT,
                translation.y,
                bounds.height,
            ) {
                state.scroll_to(iced::widget::scrollable::AbsoluteOffset {
                    x: None,
                    y: Some(y),
                });
            }
        }
    }

    iced::advanced::widget::operate(Reveal { position })
}

/// Enter or `Jump`: with the panel open, play the highlighted row from the
/// showing list through `TrackSelected` and close the panel. The highlight
/// defaults to the first match, so a submit before any Up or Down behaves as
/// before. No match leaves the panel and the query alone, so a typo does not
/// dismiss it.
pub(in crate::ui) fn jump_file_submitted(player: &mut WinampPlayer) -> Task<Message> {
    if !player.jump_file_visible {
        return Task::none();
    }
    let target = song_list(player).and_then(|(titles, epoch)| {
        let matches = views::jump_file_matches(titles, &player.jump_file_query);
        views::jump_file_highlighted(&matches, player.jump_file_selected)
            .map(|index| (epoch, index))
    });
    let Some((epoch, index)) = target else {
        return Task::none();
    };
    player.jump_file_visible = false;
    player.jump_file_selected = None;
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
        // The largest minutes count whose seconds still fit u64.
        assert_eq!(
            parse_jump_time("307445734561825860:15"),
            Some(secs(307_445_734_561_825_860 * 60 + 15))
        );
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
            // Minutes that fit u64 but overflow once multiplied by 60.
            "18446744073709551615:00",
            // Non-ASCII digits are not accepted.
            "٣٠",
            "1:٣٠",
        ] {
            assert_eq!(parse_jump_time(text), None, "{text:?}");
        }
    }
}
