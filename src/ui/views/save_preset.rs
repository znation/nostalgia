//! The transient Save preset panel: a modal overlay over the normal-mode
//! screen with a name box that saves the equalizer's applied curve.

use iced::{
    Element,
    widget::{Column, Row, Text, TextInput},
};

use crate::ui::{Message, style, theme};

use super::{labeled_button, modal::modal_overlay};

/// The Save preset box's widget id, so the panel can focus it on opening.
pub const SAVE_PRESET_INPUT_ID: &str = "nostalgia-save-preset";

/// The panel's empty-name note: Enter with a blank box keeps it open and
/// asks for a name instead of saving a nameless curve.
pub const EMPTY_NAME_NOTE: &str = "Type a name";

/// The panel's note for a name that is already a built-in preset's: Enter
/// refuses the name (`Already a preset`) and saves nothing, because a saved
/// custom with the same name could never be told apart from the built-in.
pub const BUILT_IN_NAME_NOTE: &str = "Already a preset";

/// The panel's note for a name containing `:`: Enter refuses it
/// (`Use it without a colon`), because the settings file's
/// `eq_custom_preset=<name>:<curve>` line uses the colon as its separator.
pub const COLON_NAME_NOTE: &str = "Use it without a colon";

/// The panel's note for a name containing a line break: Enter refuses it
/// (`Use one line`), because the settings file is line-based and a name
/// carrying `\n` or `\r` would split its `eq_custom_preset=` row in two and
/// let the name's tail be read as settings lines of its own.
pub const LINE_BREAK_NAME_NOTE: &str = "Use one line";

/// The panel's note for a name no saved custom matches: `Delete` with it
/// keeps the panel open with this note and deletes nothing.
pub const NO_SUCH_PRESET_NOTE: &str = "No such preset";

/// Stacks the transient Save preset panel over `base`.
///
/// The panel is modal (see [`modal_overlay`]): `Close` and the backdrop each
/// publish [`Message::ToggleSavePreset`], while `Save` and Enter in the box
/// publish [`Message::SavePresetSubmitted`] and `Delete` publishes
/// [`Message::DeletePresetSubmitted`]. The box shows the typed `name`
/// and `note`, an inline error the submit handler stored after a refused
/// save; `None` draws no note, as on opening. No state or curve is touched
/// here.
pub fn view_save_preset_overlay<'a>(
    base: Element<'a, Message>,
    name: &'a str,
    note: Option<&'static str>,
) -> Element<'a, Message> {
    let mut content = Column::new()
        .spacing(10)
        .align_x(iced::alignment::Horizontal::Center)
        .push(Text::new("SAVE PRESET").size(18).color(theme::TITLE_TAPE))
        .push(
            TextInput::new("Preset name", name)
                .id(SAVE_PRESET_INPUT_ID)
                .on_input(Message::SavePresetNameChanged)
                .on_submit(Message::SavePresetSubmitted)
                .style(|_theme, status| style::chrome_text_input_style(status)),
        );
    if let Some(note) = note {
        content = content.push(Text::new(note).color(theme::TEXT));
    }
    content = content.push(
        Row::new()
            .spacing(10)
            .push(labeled_button("Save", Message::SavePresetSubmitted))
            .push(labeled_button("Delete", Message::DeletePresetSubmitted))
            .push(labeled_button("Close", Message::ToggleSavePreset)),
    );
    modal_overlay(base, content, Message::ToggleSavePreset)
}
