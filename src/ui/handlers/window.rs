//! The window message handlers: the app window's resolved id and the custom
//! title bar's actions — drag, minimize, close, shade, always-on-top, and the
//! About panel.
//!
//! Every action acts on the resolved window id and is a no-op until it
//! exists; `with_window_id` holds that guard once, so each handler names only
//! the `iced::window` call it schedules.

use iced::Task;

use crate::ui::views;
use crate::ui::{Message, WinampPlayer};

/// The app window's id, resolved once at boot by `iced::window::latest()`:
/// store it, so the title bar's window actions can act on the window. Until
/// it lands (or if the query fails) those actions are no-ops (see
/// `with_window_id`). A saved always-on-top choice waits for the same id, so
/// its level is applied once the window exists instead of before boot.
pub(in crate::ui) fn window_id_resolved(
    player: &mut WinampPlayer,
    id: Option<iced::window::Id>,
) -> Task<Message> {
    player.window_id = id;
    match id {
        Some(id) if player.always_on_top => {
            iced::window::set_level(id, window_level(player.always_on_top))
        }
        _ => Task::none(),
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
/// the title bar plus its compact transport strip. Unshading resizes back to
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
    let opening = !player.about_visible;
    player.about_visible = opening;
    if opening {
        player.file_info_visible = false;
        player.jump_time_visible = false;
        player.jump_file_visible = false;
    }
    if opening && player.shaded {
        toggle_window_shade(player)
    } else {
        Task::none()
    }
}

/// Alt+3 or the LCD strip's `I`: flip the transient File info panel. Opening
/// needs a current track
/// resolved in `known_tracks` (as `window_title` resolves it); without one the
/// flag stays as it was. Opening closes the About panel, and opening while
/// shaded first restores the window exactly as [`toggle_about`] does. The flag
/// is layout-only and not persisted.
pub(in crate::ui) fn toggle_file_info(player: &mut WinampPlayer) -> Task<Message> {
    let opening = !player.file_info_visible;
    if opening && player.current_known_track().is_none() {
        return Task::none();
    }
    player.file_info_visible = opening;
    if opening {
        player.about_visible = false;
        player.jump_time_visible = false;
        player.jump_file_visible = false;
    }
    if opening && player.shaded {
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

/// Rolls the window up to the shade strip at `width`: the title bar plus the
/// compact transport strip, keeping the window's current width so the
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
/// height to [`views::SHADED_HEIGHT`], the title bar plus the compact transport
/// strip. Pure, so the resize target can be pinned by a test rather than only
/// observing that [`resize_to_shade`] schedules some work.
pub(in crate::ui) fn shade_target_size(width: f32) -> iced::Size {
    iced::Size::new(width, views::SHADED_HEIGHT)
}
