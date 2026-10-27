//! Hand-scrolling the overlong Now Playing title, like Winamp: pressing on the scrolling line and
//! dragging moves its window one character per 10 pixels. The
//! marquee timer pauses for the drag (see `WinampPlayer::needs_marquee`) and resumes from the
//! dragged offset on release.

use iced::Task;

use crate::ui::views;
use crate::ui::{Message, WinampPlayer};

/// A title drag in progress.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::ui) struct TitleDrag {
    /// The marquee offset when the button went down.
    start_offset: usize,
    /// The cursor's x at the first move after the press, or `None` before it; distances are
    /// measured from here because the press itself carries no position.
    anchor_x: Option<f32>,
}

/// Starts a drag from the current marquee offset.
pub(in crate::ui) fn title_drag_pressed(player: &mut WinampPlayer) -> Task<Message> {
    player.title_drag = Some(TitleDrag {
        start_offset: player.marquee_offset,
        anchor_x: None,
    });
    Task::none()
}

/// Scrolls the title by the cursor's distance from the drag's anchor; the first move only records
/// the anchor. Does nothing without a drag.
pub(in crate::ui) fn title_drag_moved(player: &mut WinampPlayer, x: f32) -> Task<Message> {
    let Some(drag) = player.title_drag else {
        return Task::none();
    };
    let Some(anchor) = drag.anchor_x else {
        player.title_drag = Some(TitleDrag {
            anchor_x: Some(x),
            ..drag
        });
        return Task::none();
    };
    let cycle = {
        let state = player.state.blocking_lock();
        let current = state.current_track.as_deref();
        let label = player.now_playing_label(current);
        let artist = player.now_playing_artist(current);
        let number = player.now_playing_number(current);
        views::marquee_cycle_len(&label, &artist, &number)
    };
    player.marquee_offset = views::dragged_marquee_offset(drag.start_offset, x - anchor, cycle);
    Task::none()
}

/// Ends the drag; the marquee resumes from the dragged offset.
pub(in crate::ui) fn title_drag_released(player: &mut WinampPlayer) -> Task<Message> {
    player.title_drag = None;
    Task::none()
}
