//! The window message handlers: the app window's resolved id and the custom
//! title bar's actions — drag, minimize, close, shade, always-on-top, and the
//! About and Options panels — plus the equalizer and library panels' visibility.
//!
//! Native window actions use the resolved window id and are no-ops until it
//! exists; `with_window_id` holds that guard. Panel visibility toggles update
//! player layout flags without scheduling native window work.

use iced::Task;

use crate::ui::views;
use crate::ui::{Message, OptionsChoice, OptionsRow, WinampPlayer};

/// The window system moved the window: remember its screen position. The
/// update loop's settings comparison then saves it; a drag's burst of moves
/// collapses in the background writer.
pub(in crate::ui) fn window_moved(player: &mut WinampPlayer, point: iced::Point) -> Task<Message> {
    player.window_position = Some((point.x, point.y));
    Task::none()
}

/// Where the window opens: the saved screen position, or the window
/// system's choice when none was saved. The position is not clamped to the
/// monitors, since iced cannot query them before the window opens.
pub(in crate::ui) fn launch_position(saved: Option<(f32, f32)>) -> iced::window::Position {
    saved.map_or(iced::window::Position::Default, |(x, y)| {
        iced::window::Position::Specific(iced::Point::new(x, y))
    })
}

/// The app window's id, resolved once at boot by `iced::window::latest()`:
/// store it, so the title bar's window actions can act on the window. Until
/// it lands (or if the query fails) those actions are no-ops (see
/// `with_window_id`). A saved always-on-top choice waits for the same id, so
/// its level is applied once the window exists instead of before boot. A
/// restored shade mode also waits for the id: because no pre-shade size was
/// captured at boot, this schedules the launch size query that
/// [`window_shade_measured`] turns into the size the first unshade restores.
/// When both the saved level and the shade measurement apply, their tasks run
/// together as one batch.
pub(in crate::ui) fn window_id_resolved(
    player: &mut WinampPlayer,
    id: Option<iced::window::Id>,
) -> Task<Message> {
    player.window_id = id;
    let Some(id) = id else {
        return Task::none();
    };
    let level = player
        .always_on_top
        .then(|| iced::window::set_level(id, window_level(true)));
    let measurement = (player.shaded && player.unshaded_size.is_none())
        .then(|| iced::window::size(id).map(Message::WindowShadeMeasured));
    match (level, measurement) {
        (Some(level), Some(measurement)) => Task::batch([level, measurement]),
        (Some(task), None) | (None, Some(task)) => task,
        (None, None) => Task::none(),
    }
}

/// The title bar's drag region: drag the window with the pointer.
pub(in crate::ui) fn window_dragged(player: &mut WinampPlayer) -> Task<Message> {
    with_window_id(player, iced::window::drag)
}

/// The title bar's minimize button.
pub(in crate::ui) fn minimize_window(player: &mut WinampPlayer) -> Task<Message> {
    with_window_id(player, |id| iced::window::minimize(id, true))
}

/// The title bar's close button.
pub(in crate::ui) fn close_window(player: &mut WinampPlayer) -> Task<Message> {
    with_window_id(player, iced::window::close)
}

/// The title bar's shade toggle: double-clicking the drag region rolls the
/// window up into shade mode, or restores it. Shading measures the window
/// only until the pre-shade size is known; after that it rolls straight up to
/// the base skin's single 14px shade bar. Unshading resizes back to
/// the captured size. `with_window_id`
/// keeps both no-ops before the window id resolves, exactly like the other
/// title-bar actions; the flag still flips so the view switches to the
/// shaded tree even where the resize is ignored.
pub(in crate::ui) fn toggle_window_shade(player: &mut WinampPlayer) -> Task<Message> {
    if player.shaded {
        player.shaded = false;
        match player.unshaded_size {
            Some(size) => with_window_id(player, |id| iced::window::resize(id, size)),
            None => Task::none(),
        }
    } else {
        player.shaded = true;
        match player.unshaded_size {
            // The size to restore is already known, so roll straight up;
            // re-measuring could read the strip a prior shade left.
            Some(size) => resize_to_shade(player, size.width),
            // First shade: measure so the size can be restored.
            None => with_window_id(player, |id| {
                iced::window::size(id).map(Message::WindowShadeMeasured)
            }),
        }
    }
}

/// The window's inner size, captured just before the first shade: store it
/// and roll the window up to the strip. A measurement only belongs to a shade
/// still waiting for its first capture: one that lands after an unshade would
/// clamp the restored window and record the strip as the size to restore.
pub(in crate::ui) fn window_shade_measured(
    player: &mut WinampPlayer,
    size: iced::Size,
) -> Task<Message> {
    if !player.shaded || player.unshaded_size.is_some() {
        return Task::none();
    }
    player.unshaded_size = Some(size);
    resize_to_shade(player, size.width)
}

/// The title bar's `M` button: flip the transient About panel. Opening it
/// while the window is shaded first calls [`toggle_window_shade`] and returns
/// that task, so the window restores before the panel draws; closing it never
/// touches the window. The flag is layout-only and not persisted.
pub(in crate::ui) fn toggle_about(player: &mut WinampPlayer) -> Task<Message> {
    if player.about_visible {
        player.about_visible = false;
        return Task::none();
    }
    player.close_transient_panels();
    player.about_visible = true;
    if player.shaded {
        toggle_window_shade(player)
    } else {
        Task::none()
    }
}

/// The LCD strip's `O` or the transport row's options (`≡`) button: flip the
/// transient options menu. Opening it while the window is shaded first calls
/// [`toggle_window_shade`] and returns that task, so the window restores before
/// the menu draws; opening it also closes every other transient panel. Closing
/// it never touches the window, and the flag is layout-only and not persisted.
pub(in crate::ui) fn toggle_options(player: &mut WinampPlayer) -> Task<Message> {
    if player.options_visible {
        player.options_visible = false;
        return Task::none();
    }
    player.close_transient_panels();
    player.options_visible = true;
    player.options_selected = 0;
    if player.shaded {
        toggle_window_shade(player)
    } else {
        Task::none()
    }
}

/// The player row's EQ button and the E key: shows or hides the normal-mode
/// equalizer section. This is a layout-only toggle, distinct from
/// [`super::mixer::toggle_equalizer`], which flips the audio equalizer's on/off state; it
/// changes no shared EQ value and schedules no work. Shade mode builds no
/// equalizer panel, so there the toggle leaves the flag alone instead of
/// flipping a hidden setting.
pub(in crate::ui) fn toggle_equalizer_panel(player: &mut WinampPlayer) -> Task<Message> {
    if player.shaded {
        return Task::none();
    }
    player.eq_panel_visible = !player.eq_panel_visible;
    Task::none()
}

/// The player row's PL button: shows or hides the normal-mode library section.
/// This is a layout-only toggle that changes no playback, equalizer, browse,
/// search, or selection state and schedules no work. Shade mode draws no PL
/// button, so the Alt+E key does nothing there, like `toggle_equalizer_panel`.
pub(in crate::ui) fn toggle_library_panel(player: &mut WinampPlayer) -> Task<Message> {
    if player.shaded {
        return Task::none();
    }
    player.library_panel_visible = !player.library_panel_visible;
    Task::none()
}

/// Move the menu highlight up, clamping at the first row; closed menus ignore it.
pub(in crate::ui) fn options_selection_up(player: &mut WinampPlayer) -> Task<Message> {
    if player.options_visible {
        player.options_selected = player.options_selected.saturating_sub(1);
    }
    Task::none()
}

/// Move the menu highlight down, clamping at the last row; closed menus ignore it.
pub(in crate::ui) fn options_selection_down(player: &mut WinampPlayer) -> Task<Message> {
    if player.options_visible {
        player.options_selected = player
            .options_selected
            .saturating_add(1)
            .min(OptionsRow::ALL.len() - 1);
    }
    Task::none()
}

/// Invoke the highlighted row through its mouse handler while the menu is open.
pub(in crate::ui) fn options_activate(player: &mut WinampPlayer) -> Task<Message> {
    if !player.options_visible {
        return Task::none();
    }
    match OptionsRow::ALL
        .get(player.options_selected)
        .map(|row| row.message())
    {
        Some(Message::OptionsChoice(choice)) => options_choice(player, choice),
        Some(Message::ToggleAbout) => toggle_about(player),
        Some(Message::ToggleOptions) => toggle_options(player),
        _ => Task::none(),
    }
}

/// Close the menu and apply the row's existing toggle.
pub(in crate::ui) fn options_choice(
    player: &mut WinampPlayer,
    choice: OptionsChoice,
) -> Task<Message> {
    player.options_visible = false;
    match choice {
        OptionsChoice::AlwaysOnTop => toggle_always_on_top(player),
        // The OPTIONS menu row is `Message::ToggleAutoscroll`'s only
        // publisher, so the choice re-dispatches through the shared toggle
        // message (like a browse reply does in `browse::fetch`) rather than
        // reaching past `update`'s dispatch to the handler.
        OptionsChoice::TimeDisplay => super::playback::toggle_time_display(player),
        OptionsChoice::Autoscroll => crate::ui::update(player, Message::ToggleAutoscroll),
        OptionsChoice::Visualizer => super::playback::toggle_visualizer(player),
        OptionsChoice::EqualizerPanel => toggle_equalizer_panel(player),
        OptionsChoice::LibraryPanel => toggle_library_panel(player),
        OptionsChoice::FadeOutAndStop => super::playback::fade::fade_out_and_stop(player),
        OptionsChoice::StopAfterCurrent => super::playback::toggle_stop_after_current(player),
        OptionsChoice::Repeat => super::playback::toggle_repeat(player),
        OptionsChoice::Shuffle => super::playback::toggle_shuffle(player),
        OptionsChoice::ManualAdvance => super::playback::toggle_manual_advance(player),
        OptionsChoice::JumpTracksBack => super::playback::stepping::jump_ten_tracks(player, false),
        OptionsChoice::JumpTracksForward => {
            super::playback::stepping::jump_ten_tracks(player, true)
        }
        OptionsChoice::SeekBackward => super::mixer::seek_backward(player),
        OptionsChoice::SeekForward => super::mixer::seek_forward(player),
        OptionsChoice::StartOfList => super::playback::stepping::jump_to_edge_track(player, false),
        OptionsChoice::EndOfList => super::playback::stepping::jump_to_edge_track(player, true),
        OptionsChoice::JumpTime => super::jump::toggle_jump_time(player),
        OptionsChoice::JumpFile => super::jump::toggle_jump_file(player),
        OptionsChoice::FileInfo => toggle_file_info(player),
    }
}

/// A plain or Shift-held Escape press: close every transient panel, leaving
/// only the visibility flags changed. `ui::subscriptions` delivers this message
/// only while [`WinampPlayer::transient_panel_open`] holds, but with no panel
/// open it is still a no-op. It is not a toggle, so it never opens a panel.
pub(in crate::ui) fn escape_pressed(player: &mut WinampPlayer) -> Task<Message> {
    player.close_transient_panels();
    Task::none()
}

/// Alt+3 or the LCD strip's `I`: flip the transient File info panel. Opening
/// needs either a current track resolved in `known_tracks` or a selected song row
/// in the showing song list (as `WinampPlayer::file_info_track` resolves them);
/// with neither the flag stays as it was. Opening closes the About and Options
/// panels, and opening while shaded first restores the window exactly as
/// [`toggle_about`] does. The flag is layout-only and not persisted.
pub(in crate::ui) fn toggle_file_info(player: &mut WinampPlayer) -> Task<Message> {
    if player.file_info_visible {
        player.file_info_visible = false;
        return Task::none();
    }
    if player.file_info_track().is_none() {
        return Task::none();
    }
    player.close_transient_panels();
    player.file_info_visible = true;
    if player.shaded {
        toggle_window_shade(player)
    } else {
        Task::none()
    }
}

/// The LCD strip's always-on-top toggle: flip the flag and schedule the
/// window-level change. `with_window_id` makes the level call a no-op before the window id
/// resolves, exactly like the other title-bar actions, while the flag still
/// flips so the view redraws the toggle.
pub(in crate::ui) fn toggle_always_on_top(player: &mut WinampPlayer) -> Task<Message> {
    player.always_on_top = !player.always_on_top;
    let level = window_level(player.always_on_top);
    with_window_id(player, move |id| iced::window::set_level(id, level))
}

/// The window level for `always_on_top`: pinned above other windows when set,
/// otherwise the normal level. Both the title bar's toggle and boot's saved
/// choice schedule `iced::window::set_level` with it, so the mapping lives here
/// once.
pub(in crate::ui) fn window_level(always_on_top: bool) -> iced::window::Level {
    if always_on_top {
        iced::window::Level::AlwaysOnTop
    } else {
        iced::window::Level::Normal
    }
}

/// Runs `action` with the resolved window id, or returns no task when it has
/// not resolved yet. The title bar's window actions — drag, minimize, close,
/// the shade size query and resize, and the always-on-top level — all guard on
/// [`WinampPlayer::window_id`] the same way: a `None` (before boot's
/// `iced::window::latest()` query resolves, or if it fails) makes the action a
/// no-op. The guard lives here once, so each handler names only the
/// `iced::window` call it schedules.
fn with_window_id(
    player: &WinampPlayer,
    action: impl FnOnce(iced::window::Id) -> Task<Message>,
) -> Task<Message> {
    match player.window_id {
        Some(id) => action(id),
        None => Task::none(),
    }
}

/// Rolls the window up to the single shade bar at `width`, keeping the
/// window's current width so the
/// rolled-up window matches the one it replaces. Both shade paths — rolling
/// straight up with a known size, and the first shade once its measurement
/// lands — resize this way, so the expression lives here once. A no-op before
/// the window id resolves, via [`with_window_id`], like the other title-bar
/// actions.
fn resize_to_shade(player: &WinampPlayer, width: f32) -> Task<Message> {
    with_window_id(player, |id| {
        iced::window::resize(id, shade_target_size(width))
    })
}

/// The size of the rolled-up window at `width`: keep the width and shrink the
/// height to [`views::SHADED_HEIGHT`], the base skin's single 14px shade bar.
/// Pure, so the resize target can be pinned by a test rather than only
/// observing that [`resize_to_shade`] schedules some work.
pub(in crate::ui) fn shade_target_size(width: f32) -> iced::Size {
    iced::Size::new(width, views::SHADED_HEIGHT)
}
