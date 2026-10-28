use crate::ui::{message::Message, player::WinampPlayer, views};
use iced::Element;
use iced::widget::{Column, MouseArea};

pub(super) fn view_normal(player: &WinampPlayer) -> Element<'_, Message> {
    // The PL button on the transport row is layout-only too: it shows or
    // hides the whole library column below, while the shared browse, search,
    // selection, playback, and player state keep their values. The player
    // panels above stay drawn, and nothing resizes the window. Hiding the
    // library skips building its column at all: the browse rows are the
    // heaviest part of the tree (one per row, up to a whole-library list),
    // and an element that is never pushed into the tree is discarded anyway.
    let (player_panels, footer) = super::player_panels::view_player_panels(player);
    // Player panels share the volume wheel and the options right-press area;
    // the library stays outside so its Scrollable can consume the wheel and a
    // right-press over it opens nothing.
    let mut column = Column::new().push(
        MouseArea::new(player_panels)
            .on_scroll(views::volume_wheel_message)
            .on_right_press(Message::ToggleOptions),
    );
    if let Some(footer) = footer {
        column = column.push(super::library::view_library(player, footer));
    }
    column.into()
}
