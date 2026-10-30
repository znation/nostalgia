use crate::ui::{message::Message, player::WinampPlayer, views};
use iced::Element;
use iced::widget::Column;

// Return the footer built from the same snapshot as the LCD.
pub(super) fn view_player_panels(
    player: &WinampPlayer,
) -> (Element<'_, Message>, Option<Element<'_, Message>>) {
    // The now-playing title, artist, playlist number, elapsed time, and total
    // are resolved while the state lock is held, from a borrowed `current_track`
    // against the accumulated `known_tracks` index (see
    // [`WinampPlayer::now_playing_label`], [`WinampPlayer::now_playing_artist`],
    // [`WinampPlayer::now_playing_number`],
    // [`WinampPlayer::now_playing_elapsed`], and
    // [`WinampPlayer::now_playing_time`]). The label, artist, number, and
    // elapsed placeholder borrow from `known_tracks` (or the `"Nothing"`/`""`/
    // `""`/`"00:00"`/`"--:--"` literals); the total borrows the cached length,
    // while a reported elapsed position allocates its formatted `MM:SS` string.
    let (
        now_playing,
        now_playing_artist,
        now_playing_number,
        now_playing_elapsed,
        now_playing_time,
        playback_format,
        spectrum,
        transport_mode,
        volume,
        balance,
        repeat,
        shuffle,
        eq_enabled,
        eq_auto,
        eq_preamp,
        eq_bands,
        eq_preset,
    ) = {
        let state = player.state.blocking_lock();
        (
            player.now_playing_label(state.current_track.as_deref()),
            player.now_playing_artist(state.current_track.as_deref()),
            player.now_playing_number(state.current_track.as_deref()),
            player.now_playing_elapsed(state.current_track.as_deref()),
            player.now_playing_time(state.current_track.as_deref()),
            player.playback_format,
            player.spectrum,
            state.transport_mode(),
            state.volume(),
            state.balance(),
            state.repeat,
            state.shuffle,
            state.eq_enabled,
            state.eq_auto,
            state.eq_preamp(),
            state.eq_bands(),
            state.eq_preset(),
        )
    };

    let library_elapsed = now_playing_elapsed.clone();
    let library_total = now_playing_time.clone();
    let mut player_panels = Column::new()
        .push(views::view_title_bar(player.about_visible))
        .push(views::view_now_playing(
            now_playing_elapsed,
            now_playing,
            now_playing_artist,
            now_playing_number,
            now_playing_time,
            player.marquee_offset,
            playback_format,
            views::visible_spectrum(spectrum, player.visualizer_visible),
            views::visible_spectrum(player.spectrum_peaks, player.visualizer_visible),
            transport_mode,
            player.blink_visible,
            player
                .mixer_drag
                .map(|drag| views::mixer_readout_text(drag, volume, balance, eq_preamp, &eq_bands))
                .or_else(|| {
                    player
                        .seek_preview
                        .zip(player.playback_duration)
                        .map(|(target, length)| views::seek_readout_text(target, length))
                })
                .or_else(|| {
                    (player.mixer_flash_ticks > 0).then(|| {
                        views::mixer_readout_text(
                            player.mixer_flash,
                            volume,
                            balance,
                            eq_preamp,
                            &eq_bands,
                        )
                    })
                }),
            views::IndicatorModes {
                options_visible: player.options_visible,
                always_on_top: player.always_on_top,
                file_info_visible: player.file_info_visible,
                visualizer_visible: player.visualizer_visible,
            },
        ))
        .push(views::view_seek_bar(
            player.readout_position(),
            player.playback_duration,
        ))
        .push(views::view_transport_controls(
            volume,
            balance,
            repeat,
            shuffle,
            player.eq_panel_visible,
            player.library_panel_visible,
        ));
    // The EQ button on the transport row is layout-only: it shows or hides this
    // band and panel, while the shared EQ state keeps driving the audio. The
    // freed height flows to the browse list below because nothing here pins the
    // column's height or resizes the window.
    if player.eq_panel_visible {
        player_panels = player_panels
            .push(views::view_title_band("NOSTALGIA EQUALIZER"))
            .push(views::view_equalizer(
                eq_enabled,
                eq_auto,
                eq_preamp,
                &eq_bands,
                eq_preset,
                &player.custom_eq_presets,
            ));
    }

    let footer = player.library_panel_visible.then(|| {
        views::view_library_strip(
            transport_mode,
            player.blink_visible,
            volume,
            library_elapsed,
            library_total,
        )
    });
    (player_panels.into(), footer)
}
