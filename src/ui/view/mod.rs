//! The root screen assembly: the title bar, Now Playing bar, seek strip,
//! transport row, the player's EQ-button-controlled equalizer section, and the
//! browse level stacked into the tree `init_ui` hands to iced. Widget
//! construction itself lives in `views`.

use super::{message::Message, player::WinampPlayer};
use crate::ui::theme;
use iced::widget::{Container, container};
use iced::{Border, Color, Element, Length};

mod library;
mod normal;
mod options_shortcut;
mod overlay;
mod player_panels;
mod shade;

/// Assembles the app screen: the title bar, Now Playing bar, seek strip,
/// transport row, and — while [`WinampPlayer::eq_panel_visible`] holds — the
/// equalizer band and panel, all built in `views` from the resolved title and
/// the shared playback, volume, Repeat, Shuffle, and EQ state — above, while
/// [`WinampPlayer::library_panel_visible`] holds, the current browse list and
/// the compact library transport footer beneath it. The transport row's `EQ`
/// and `PL` buttons toggle the two sections independently. Hiding the
/// equalizer section or the library section moves the rest up into the freed
/// height; nothing resizes the window. When the window is shaded,
/// the tree is the title bar over the compact transport strip and the rest of
/// the screen is dropped. The strip's time readout publishes the same
/// [`Message::ToggleTimeDisplay`] as the hidden LCD's, so the elapsed/remaining
/// choice stays switchable in shade mode. While [`WinampPlayer::about_visible`]
/// or [`WinampPlayer::options_visible`] holds outside shade mode, the
/// normal-mode tree is the base of [`super::views::view_about_overlay`] or
/// [`super::views::view_options_overlay`], whose About panel also names the active
/// library source from the Apple Music service's current session and whose
/// options menu lists the existing on/off choices plus About.
pub(in crate::ui) fn view(player: &WinampPlayer) -> Element<'_, Message> {
    let base = if player.shaded {
        shade::view_shade(player)
    } else {
        normal::view_normal(player)
    };
    Element::new(options_shortcut::OptionsShortcut(
        Container::new(overlay::apply_overlays(base, player))
            .width(Length::Fill)
            .padding(1.0)
            // Shrink, not Fill: the shaded window is sized to
            // [`views::SHADED_HEIGHT`], exactly the tree content plus the
            // outline's two rows, so the strip's bottom edge stays visible.
            .height(Length::Shrink)
            .style(|_theme| container::Style {
                border: window_edge_border(theme::WINDOW_EDGE_OUTLINE),
                ..container::Style::default()
            })
            .into(),
    ))
}

/// A 1px square ring in `color`: the outer edge of the base skin's two-part
/// window border (the dark [`theme::WINDOW_EDGE_OUTLINE`] ring outside the
/// panels' light bevel).
fn window_edge_border(color: Color) -> Border {
    Border {
        color,
        width: 1.0,
        radius: 0.0.into(),
    }
}
