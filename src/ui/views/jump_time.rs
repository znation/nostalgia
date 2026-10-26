//! The transient Jump to time panel: a modal overlay over the normal-mode
//! screen with a time box that seeks the loaded preview.

use std::time::Duration;

use iced::{
    Element,
    widget::{Column, Row, Text, TextInput},
};

use crate::ui::{Message, style, theme};

use super::{format_track_time, labeled_button, modal::modal_overlay};

/// The Jump to time box's widget id, so Ctrl+J can focus it on opening.
pub const JUMP_TIME_INPUT_ID: &str = "nostalgia-jump-time";

/// Stacks the transient Jump to time panel over `base`.
///
/// The panel is modal (see [`modal_overlay`]): `Close` and the backdrop each
/// publish [`Message::ToggleJumpTime`], while `Jump` and Enter in the box
/// publish [`Message::JumpTimeSubmitted`]. It names the preview's `length` and
/// shows the typed `query`. No state or playback is touched here.
pub fn view_jump_time_overlay<'a>(
    base: Element<'a, Message>,
    query: &'a str,
    length: Duration,
) -> Element<'a, Message> {
    let length_ms = u64::try_from(length.as_millis()).unwrap_or(u64::MAX);
    let input = TextInput::new("mm:ss", query)
        .id(JUMP_TIME_INPUT_ID)
        .on_input(Message::JumpTimeChanged)
        .on_submit(Message::JumpTimeSubmitted)
        .style(|_theme, status| style::chrome_text_input_style(status));
    let content = Column::new()
        .spacing(10)
        .align_x(iced::alignment::Horizontal::Center)
        .push(Text::new("JUMP TO TIME").size(18).color(theme::TITLE_TAPE))
        .push(
            Text::new(format!("Track length: {}", format_track_time(length_ms))).color(theme::TEXT),
        )
        .push(input)
        .push(
            Row::new()
                .spacing(10)
                .push(labeled_button("Jump", Message::JumpTimeSubmitted))
                .push(labeled_button("Close", Message::ToggleJumpTime)),
        );
    modal_overlay(base, content, Message::ToggleJumpTime)
}
