//! The transient Save list panel: a modal overlay over the normal-mode
//! screen with a name box that saves the showing song list's ids.

use iced::{
    Element,
    widget::{Column, Row, Text, TextInput},
};

use crate::ui::{Message, style, theme};

use super::{labeled_button, modal::modal_overlay};

/// The Save list box's widget id, so the panel can focus it on opening.
pub const SAVE_LIST_INPUT_ID: &str = "nostalgia-save-list";

/// The panel's empty-name note — the same word the Save preset panel shows
/// for a blank box — named `SAVE_LIST_EMPTY_NAME_NOTE` beside the preset
/// panel's so the two panels' exports cannot collide.
pub const SAVE_LIST_EMPTY_NAME_NOTE: &str = "Type a name";

/// The panel's note for a name containing `:`: Enter refuses it
/// (`Use it without a colon`) for the same reason the Save preset panel
/// does — the settings file's `user_playlist=<name>:<ids>` line uses the
/// colon as its separator.
pub const SAVE_LIST_COLON_NAME_NOTE: &str = "Use it without a colon";

/// Stacks the transient Save list panel over `base`.
///
/// The panel is modal (see [`modal_overlay`]): `Close` and the backdrop each
/// publish [`Message::ToggleSaveList`], while `Save` and Enter in the box
/// publish [`Message::SaveListSubmitted`]. The box shows the typed `name` and
/// `note`, an inline error the submit handler stored after a refused save;
/// `None` draws no note, as on opening. No state or list is touched here.
pub fn view_save_list_overlay<'a>(
    base: Element<'a, Message>,
    name: &'a str,
    note: Option<&'static str>,
) -> Element<'a, Message> {
    let mut content = Column::new()
        .spacing(10)
        .align_x(iced::alignment::Horizontal::Center)
        .push(Text::new("SAVE LIST").size(18).color(theme::TITLE_TAPE))
        .push(
            TextInput::new("Playlist name", name)
                .id(SAVE_LIST_INPUT_ID)
                .on_input(Message::SaveListNameChanged)
                .on_submit(Message::SaveListSubmitted)
                .style(|_theme, status| style::chrome_text_input_style(status)),
        );
    if let Some(note) = note {
        content = content.push(Text::new(note).color(theme::TEXT));
    }
    content = content.push(
        Row::new()
            .spacing(10)
            .push(labeled_button("Save", Message::SaveListSubmitted))
            .push(labeled_button("Close", Message::ToggleSaveList)),
    );
    modal_overlay(base, content, Message::ToggleSaveList)
}
