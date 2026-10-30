use crate::ui::player::CurrentView;
use crate::ui::{message::Message, player::WinampPlayer, views};
use iced::Element;

pub(super) fn apply_overlays<'a>(
    base: Element<'a, Message>,
    player: &'a WinampPlayer,
) -> Element<'a, Message> {
    if player.about_visible && !player.shaded {
        views::view_about_overlay(base, player.apple_music_service.session().is_some())
    } else if player.options_visible && !player.shaded {
        let (repeat, shuffle, manual_advance) = {
            let state = player.state.blocking_lock();
            (state.repeat, state.shuffle, state.manual_advance)
        };
        views::view_options_overlay(
            base,
            views::OptionsState {
                always_on_top: player.always_on_top,
                time_remaining: player.time_display == views::TimeDisplay::Remaining,
                visualizer_visible: player.visualizer_visible,
                eq_panel_visible: player.eq_panel_visible,
                library_panel_visible: player.library_panel_visible,
                stop_after_current: player.stop_after_current,
                repeat,
                shuffle,
                manual_advance,
            },
            player.options_selected,
        )
    } else if player.file_info_visible && !player.shaded {
        match player.file_info_track() {
            Some((track, format)) => views::view_file_info_overlay(base, track, format),
            None => base,
        }
    } else if player.jump_time_visible && !player.shaded {
        match player.playback_duration {
            Some(length) => views::view_jump_time_overlay(base, &player.jump_time_query, length),
            None => base,
        }
    } else if player.save_preset_open && !player.shaded {
        views::view_save_preset_overlay(base, &player.save_preset_name, player.save_preset_note)
    } else if player.jump_file_visible && !player.shaded {
        match player.current_view {
            CurrentView::Songs => views::view_jump_file_overlay(
                base,
                &player.jump_file_query,
                &player.songs.titles,
                player.songs.epoch,
                player.jump_file_selected,
            ),
            CurrentView::AllSongs => views::view_jump_file_overlay(
                base,
                &player.jump_file_query,
                &player.all_songs.titles,
                player.all_songs.epoch,
                player.jump_file_selected,
            ),
            CurrentView::RecentlyPlayed => views::view_jump_file_overlay(
                base,
                &player.jump_file_query,
                &player.recently_played.titles,
                player.recently_played.epoch,
                player.jump_file_selected,
            ),
            _ => base,
        }
    } else {
        base
    }
}
