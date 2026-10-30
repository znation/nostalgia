use crate::ui::{message::Message, player::WinampPlayer, views};
use iced::Element;
use iced::widget::MouseArea;

pub(super) fn view_shade(player: &WinampPlayer) -> Element<'_, Message> {
    // Shade mode builds only the base skin's single 14px shade bar — the
    // title bar's own chrome with the compact transport's controls on it —
    // and drops the Now Playing bar, seek strip, transport row, equalizer,
    // and browse list, while the window itself is resized down to that bar.
    // The bar keeps the transport buttons, the elapsed-over-total readout, a
    // seek slider over the loaded preview's own length, and the volume
    // slider. The short state lock resolves only what the bar draws, so the
    // rolled-up frame stays cheap. The About panel never draws here: the `M`
    // button toggles it, and opening it while shaded restores the window
    // first.
    let (about_visible, transport_mode, volume, elapsed, total, position, duration) = {
        let state = player.state.blocking_lock();
        (
            player.about_visible,
            state.transport_mode(),
            state.volume(),
            player.now_playing_elapsed(state.current_track.as_deref()),
            player.now_playing_time(state.current_track.as_deref()),
            player.readout_position(),
            player.playback_duration,
        )
    };
    MouseArea::new(views::view_shade_strip(
        transport_mode,
        player.blink_visible,
        volume,
        elapsed,
        total,
        position,
        duration,
        about_visible,
    ))
    .on_scroll(views::volume_wheel_message)
    .on_right_press(Message::ToggleOptions)
    .into()
}
