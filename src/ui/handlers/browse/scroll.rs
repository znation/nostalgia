//! The browse list's scroll operations: the Top/Home and End/End-key snaps,
//! the Page Up and Page Down steps, and the minimal reveal of a moved or
//! playing row.
//!
//! Each handler is a pure `iced` scroll task against the list's stable widget
//! id. They touch only the list's scroll offset — never its rows, selection,
//! view, search state, or playback — so the navigation in the parent [`super`]
//! module and the keyboard shortcuts can share them without changing browse
//! state.

use iced::Task;

use crate::ui::views;
use crate::ui::{CurrentView, Message, WinampPlayer};

/// Snap the showing browse list to `offset` through its stable widget id.
/// The list carries `views::BROWSE_LIST_ID` whichever level is showing, so
/// one operation serves the Top button, the Home and End keys, and every
/// navigation that opens a fresh list.
fn snap_browse_list(offset: iced::widget::scrollable::RelativeOffset) -> Task<Message> {
    iced::widget::operation::snap_to(views::BROWSE_LIST_ID, offset)
}

/// The browse view's Top button and the Home key: snap the current list back
/// to its first row. This touches only the list's scroll offset; it changes
/// no rows, selection, view, search state, or playback.
pub(in crate::ui) fn scroll_browse_to_top() -> Task<Message> {
    snap_browse_list(iced::widget::scrollable::RelativeOffset::START)
}

/// The End key's counterpart to [`scroll_browse_to_top`]: snap the current
/// list to its last row. Like Home it touches only the list's scroll offset;
/// it changes no rows, selection, view, search state, or playback.
pub(in crate::ui) fn scroll_browse_to_bottom() -> Task<Message> {
    snap_browse_list(iced::widget::scrollable::RelativeOffset::END)
}

/// Scroll the showing browse list by `delta` vertical pixels through its
/// stable widget id. Unlike [`reveal_browse_row`], which moves the list only
/// as far as one row needs, the step is relative to the list's current offset.
/// It touches only the scroll offset and has no target in shade mode, where no
/// browse list is built.
fn scroll_browse_by(delta: f32) -> Task<Message> {
    iced::widget::operation::scroll_by(
        views::BROWSE_LIST_ID,
        iced::widget::scrollable::AbsoluteOffset { x: 0.0, y: delta },
    )
}

/// The Page Up key: scroll the current list back by one
/// [`views::BROWSE_PAGE_HEIGHT`], relative to where it sits now. Like Home and
/// End it changes no rows, selection, view, search state, or playback, and
/// iced clamps a page that would pass the list's top.
pub(in crate::ui) fn scroll_browse_page_up() -> Task<Message> {
    scroll_browse_by(-views::BROWSE_PAGE_HEIGHT)
}

/// The Page Down key's counterpart to [`scroll_browse_page_up`]: scroll the
/// current list forward by one [`views::BROWSE_PAGE_HEIGHT`], relative to
/// where it sits now. It likewise touches only the scroll offset, and iced
/// clamps a page that would pass the list's end.
pub(in crate::ui) fn scroll_browse_page_down() -> Task<Message> {
    scroll_browse_by(views::BROWSE_PAGE_HEIGHT)
}

/// Keep the stepped-to playing row visible: reveal the browse row at `index`
/// in the list, scrolling only as far as needed and not at all when the row
/// is already fully visible, and only while a song list (`Songs`,
/// `AllSongs` or `RecentlyPlayed`) is showing, since those are the levels whose rows carry the
/// `▶` playing marker. Another browse level leaves the list where it is, and
/// so does a direct row press — the pressed row is already in view — which
/// deliberately does not call this.
pub(in crate::ui) fn scroll_browse_to_playing_row(
    player: &WinampPlayer,
    index: usize,
) -> Task<Message> {
    match player.current_view {
        CurrentView::Songs | CurrentView::AllSongs | CurrentView::RecentlyPlayed => {
            reveal_browse_row(index)
        }
        _ => Task::none(),
    }
}

/// The rounding slack a reveal comparison gives a row's flush edge: the row
/// pitch is not a f32-exact value (the pixel-font split's part 3/4 re-based
/// the row text onto 16-px grid sizes, moving it to 30.8), so an edge that
/// measurement puts exactly at the viewport's line can compare one ulp
/// wider and schedule a scroll of that width. A real miss misses by a whole
/// row — the pitch itself — so half a pixel of slack separates the edges
/// the rule treats as flush from the misses it reveals.
const REVEAL_SLACK: f32 = 0.5;

/// The scroll offset that shows a row fully in a viewport of
/// `viewport_height` currently scrolled to `scroll_top`, or `None` when the
/// row is already fully visible. The row spans `row_height` pixels from
/// `row_top`. A row scrolled above the viewport's top snaps to the row's
/// top; a row starting inside the viewport but ending below it snaps to
/// the viewport's bottom edge. So a viewport shorter than one row whose
/// row starts inside it scrolls to the row's bottom, not its top — no
/// caller passes such a viewport (both rows are smaller than the lists'
/// viewport heights). The browse list and the Jump to File list share
/// this minimal-reveal rule through [`reveal_row_offset`] and the Jump to
/// File handler's `reveal_jump_file_row`.
pub(in crate::ui) fn reveal_offset(
    row_top: f32,
    row_height: f32,
    scroll_top: f32,
    viewport_height: f32,
) -> Option<f32> {
    let row_bottom = row_top + row_height;
    if row_top < scroll_top {
        Some(row_top)
    } else if row_bottom > scroll_top + viewport_height + REVEAL_SLACK {
        Some(row_bottom - viewport_height)
    } else {
        None
    }
}

/// The scroll offset that shows browse row `index` fully in a viewport of
/// `viewport_height` currently scrolled to `scroll_top`, or `None` when the
/// row is already fully visible. Row `index` spans from
/// [`views::BROWSE_LIST_PADDING`] plus `index` times
/// [`views::BROWSE_ROW_HEIGHT`] for one [`views::BROWSE_ROW_HEIGHT`].
pub(in crate::ui) fn reveal_row_offset(
    index: usize,
    scroll_top: f32,
    viewport_height: f32,
) -> Option<f32> {
    let row_top = views::BROWSE_LIST_PADDING + index as f32 * views::BROWSE_ROW_HEIGHT;
    reveal_offset(
        row_top,
        views::BROWSE_ROW_HEIGHT,
        scroll_top,
        viewport_height,
    )
}

/// Scrolls the browse list only as far as needed to show row `index` fully,
/// and not at all when it is already visible. It reads the list's current
/// offset and viewport from the widget operation, touches only the scroll
/// offset, and has no target in shade mode, where no browse list is built.
pub(in crate::ui) fn reveal_browse_row(index: usize) -> Task<Message> {
    use iced::advanced::widget::operation::Scrollable;
    use iced::advanced::widget::{Id, Operation};

    struct Reveal {
        index: usize,
    }

    impl<T> Operation<T> for Reveal {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation<T>)) {
            operate(self);
        }

        fn scrollable(
            &mut self,
            id: Option<&Id>,
            bounds: iced::Rectangle,
            _content_bounds: iced::Rectangle,
            translation: iced::Vector,
            state: &mut dyn Scrollable,
        ) {
            if id != Some(&Id::new(views::BROWSE_LIST_ID)) {
                return;
            }
            if let Some(y) = reveal_row_offset(self.index, translation.y, bounds.height) {
                state.scroll_to(iced::widget::scrollable::AbsoluteOffset {
                    x: None,
                    y: Some(y),
                });
            }
        }
    }

    iced::advanced::widget::operate(Reveal { index })
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROW: f32 = views::BROWSE_ROW_HEIGHT;
    const PAD: f32 = views::BROWSE_LIST_PADDING;

    #[test]
    fn a_row_above_the_viewport_reveals_at_its_top() {
        assert_eq!(reveal_row_offset(2, 500.0, 100.0), Some(PAD + 2.0 * ROW));
    }

    #[test]
    fn a_row_below_the_viewport_reveals_at_its_bottom() {
        let bottom = PAD + 10.0 * ROW + ROW;
        assert_eq!(reveal_row_offset(10, 0.0, 100.0), Some(bottom - 100.0));
    }

    #[test]
    fn a_visible_row_needs_no_scroll() {
        assert_eq!(reveal_row_offset(1, 0.0, 1000.0), None);
        let viewport = PAD + 3.0 * ROW + ROW;
        assert_eq!(reveal_row_offset(3, 0.0, viewport), None);
    }

    // A viewport shorter than one row cannot fit it, so both edges are out of
    // view at once. The top check runs first, so a row scrolled past above the
    // viewport is revealed at its top rather than its bottom.
    #[test]
    fn a_row_above_a_viewport_shorter_than_one_row_reveals_at_its_top() {
        let scroll_top = PAD + 4.0 * ROW;
        assert_eq!(
            reveal_row_offset(3, scroll_top, ROW / 2.0),
            Some(PAD + 3.0 * ROW)
        );
    }

    // A row below a viewport shorter than one row cannot fit either; it is
    // revealed with its bottom edge at the viewport's bottom, since the top
    // check does not fire for a row below.
    #[test]
    fn a_row_below_a_viewport_shorter_than_one_row_reveals_at_its_bottom() {
        let row_bottom = PAD + 2.0 * ROW + ROW;
        assert_eq!(
            reveal_row_offset(2, 0.0, ROW / 2.0),
            Some(row_bottom - ROW / 2.0)
        );
    }

    // Both edges are inclusive: a row whose top sits exactly at the scroll
    // offset, or whose bottom sits exactly at the viewport's bottom, is fully
    // visible and must not nudge the list. One ulp of rounding sits in the
    // exact-edge comparisons once the pitch is not f32-exact, so these pins
    // assert the reveal chooses the flush row's own edge — the row's top —
    // over a whole-row miss, and [`REVEAL_SLACK`] rounds the edge itself out.
    #[test]
    fn a_row_flush_with_either_viewport_edge_needs_no_scroll() {
        let row_top = PAD + 3.0 * ROW;
        assert_eq!(reveal_row_offset(3, row_top, 2.0 * ROW), None);
        assert_eq!(reveal_row_offset(3, row_top - ROW, 2.0 * ROW), None);
    }

    // The Jump to File list calls `reveal_offset` with its own row geometry.
    #[test]
    fn reveal_offset_uses_the_callers_row_geometry() {
        assert_eq!(reveal_offset(10.0, 20.0, 50.0, 100.0), Some(10.0));
        assert_eq!(reveal_offset(200.0, 20.0, 50.0, 100.0), Some(120.0));
        assert_eq!(reveal_offset(60.0, 20.0, 50.0, 100.0), None);
    }
}
