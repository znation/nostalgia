use crate::ui::{message::Message, player::WinampPlayer, views};
use iced::Element;
use iced::widget::{Column, MouseArea};

pub(super) fn view_shade(player: &WinampPlayer) -> Element<'_, Message> {
    // Shade mode builds only the title bar and its compact transport strip —
    // the Now Playing bar, seek strip, transport row, equalizer, and browse
    // list are all dropped, and the window itself is resized down to that
    // pair. The strip keeps the transport buttons, the elapsed-over-total
    // readout, a seek slider over the loaded preview's own length, and the
    // volume slider. The short state lock resolves only what the strip draws,
    // so the rolled-up frame stays cheap. The About panel never draws here:
    // opening it while shaded restores the window first.
    let (transport_mode, volume, elapsed, total, position, duration) = {
        let state = player.state.blocking_lock();
        (
            state.transport_mode(),
            state.volume(),
            player.now_playing_elapsed(state.current_track.as_deref()),
            player.now_playing_time(state.current_track.as_deref()),
            player.readout_position(),
            player.playback_duration,
        )
    };
    Column::new()
        .push(
            MouseArea::new(
                Column::new()
                    .push(views::view_title_bar(player.about_visible))
                    .push(views::view_shade_strip(
                        transport_mode,
                        player.blink_visible,
                        volume,
                        elapsed,
                        total,
                        position,
                        duration,
                    )),
            )
            .on_scroll(views::volume_wheel_message),
        )
        .into()
}
