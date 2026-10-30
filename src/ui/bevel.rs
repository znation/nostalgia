//! Reusable Winamp bevel styling.
//!
//! The base skin's defining 3D edge is a light top/left and dark bottom/right
//! pair on raised chrome, reversed in a sunken well. iced 0.14's `Border` is a
//! single color of uniform width, so this module composes the two-tone bevel
//! from 1px [`rule`] edges and overlays it on the panel content.
//! [`bevel_edges`] is the pure color rule at the heart of the layer;
//! [`lcd_well`] and [`raised_panel`] are the two panel shapes the views use.
//!
//! The widget chrome styles that consume [`bevel_edges`] live in the sibling
//! `style` module; this module owns only the bevel composition.

use iced::{
    Color, Element, Length, Theme,
    widget::{Column, Container, Row, Space, Stack, container, rule},
};

use super::{Message, theme};

/// The `(top_left, bottom_right)` edge colors for a bevel.
///
/// A raised edge catches the light on its top and left and falls dark on its
/// bottom and right; a sunken (recessed) edge reverses that. Pure, so the
/// color rule is testable without building any widgets.
pub fn bevel_edges(raised: bool) -> (Color, Color) {
    if raised {
        (theme::PANEL_EDGE_LIGHT, theme::PANEL_EDGE_DARK)
    } else {
        (theme::PANEL_EDGE_DARK, theme::PANEL_EDGE_LIGHT)
    }
}

/// A 1px [`rule`] in `color`, styled to fill the available space on its axis.
///
/// A bevel's horizontal and vertical edges differ only in which [`rule`]
/// constructor draws them, so the shared full-fill styling lives here once.
fn styled_edge<'a, R>(edge: rule::Rule<'a, Theme>, color: Color) -> Element<'a, Message, Theme, R>
where
    R: iced::advanced::Renderer + 'a,
{
    edge.style(move |_theme| rule::Style {
        color,
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    })
    .into()
}

/// The bevel drawn as a full-size overlay: a top and bottom horizontal edge
/// with a [`Length::Fill`] row between them holding the left and right
/// vertical edges. The [`Space`] spacer keeps the right edge on the far side.
///
/// The overlay is sized by the [`Stack`] that carries it (see [`beveled`]), so
/// its [`Length::Fill`] edges span exactly the panel's resolved size.
fn bevel_overlay<'a, R>(top_left: Color, bottom_right: Color) -> Element<'a, Message, Theme, R>
where
    R: iced::advanced::Renderer + 'a,
{
    Column::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .push(styled_edge::<R>(rule::horizontal(1.0), top_left))
        .push(
            Row::new()
                .width(Length::Fill)
                .height(Length::Fill)
                .push(styled_edge::<R>(rule::vertical(1.0), top_left))
                .push(Space::new().width(Length::Fill))
                .push(styled_edge::<R>(rule::vertical(1.0), bottom_right)),
        )
        .push(styled_edge::<R>(rule::horizontal(1.0), bottom_right))
        .into()
}

/// Wraps `content` in a two-tone bevel, raised or sunken.
///
/// The content is the base layer of a [`Stack`], so the panel's height is the
/// content's intrinsic height; the bevel is an overlay on top of it, laid out
/// against the panel's resolved size. The panel fills the available width but
/// never stretches to the available height.
fn beveled<'a, R>(
    content: Element<'a, Message, Theme, R>,
    raised: bool,
) -> Element<'a, Message, Theme, R>
where
    R: iced::advanced::Renderer + 'a,
{
    let (top_left, bottom_right) = bevel_edges(raised);

    Stack::new()
        .width(Length::Fill)
        .height(Length::Shrink)
        .push(content)
        .push(bevel_overlay::<R>(top_left, bottom_right))
        .into()
}

/// The [`Container`] style the display wells apply,
/// with the fill color set. Pure, so each fill is testable without a
/// renderer — iced exposes no way to introspect an inline style closure.
pub(super) mod well_style {
    use iced::widget::container;

    use crate::ui::theme;

    /// The display recess's style: the well fill is the window face color,
    /// matching the base skin's display surface
    /// (`winamp-2.x-stack.png` (16,20) and (30,40) read the face on it,
    /// with (0,0,0) only inside its inset blocks).
    pub(super) fn lcd_well_style() -> container::Style {
        container::Style {
            background: Some(theme::WINDOW_BACKGROUND.into()),
            ..container::Style::default()
        }
    }

    /// A display inset's style: the pure-black block fill the base skin
    /// draws inside its display's visualizer and time blocks, and behind
    /// its playlist editor.
    pub(super) fn lcd_inset_style() -> container::Style {
        container::Style {
            background: Some(theme::LCD_INSET_BLACK.into()),
            ..container::Style::default()
        }
    }
}

/// Wraps `content` in the base skin's display recess: a full-width
/// face-colored well behind the content with a few px of padding and the
/// reversed (sunken) bevel. The display's pure-black blocks — the time and
/// visualizer sockets, and the playlist editor's well — are
/// [`lcd_inset`] insets broken into this well.
///
/// No text color is set, so the caller's caption keeps the theme text color
/// and only the title's explicit green stays green.
pub fn lcd_well<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    let well = Container::new(content)
        .width(Length::Fill)
        .padding(4)
        .style(|_theme| well_style::lcd_well_style());

    beveled(well.into(), false)
}

/// Wraps `content` in the base skin's pure-black display inset: a block
/// cut into the face-colored [`lcd_well`], painted [`theme::LCD_INSET_BLACK`]
/// at `width` (the visualizer block fills its row, the time block hugs its
/// readout) with no padding, so the content keeps the bounds the block
/// paints to.
pub fn lcd_inset<'a, Theme, R>(
    content: impl Into<Element<'a, Message, Theme, R>>,
    width: Length,
) -> Element<'a, Message, Theme, R>
where
    Theme: container::Catalog + 'a,
    <Theme as container::Catalog>::Class<'a>: From<container::StyleFn<'a, Theme>>,
    R: iced::advanced::Renderer + 'a,
{
    Container::new(content)
        .width(width)
        .padding(0)
        .style(move |_theme| well_style::lcd_inset_style())
        .into()
}

/// Wraps `content` in a raised chrome panel on the window face.
///
/// No extra background is painted, so the panel composes with the existing
/// window theme.
pub fn raised_panel<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    beveled(content.into(), true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::{
        Background, Size,
        advanced::{layout::Limits, widget::Tree},
        widget::Text,
    };

    #[test]
    fn bevel_edges_raised_is_light_top_left_and_dark_bottom_right() {
        assert_eq!(
            bevel_edges(true),
            (theme::PANEL_EDGE_LIGHT, theme::PANEL_EDGE_DARK)
        );
    }

    #[test]
    fn bevel_edges_sunken_reverses_the_pair() {
        assert_eq!(
            bevel_edges(false),
            (theme::PANEL_EDGE_DARK, theme::PANEL_EDGE_LIGHT)
        );
    }

    /// Asserts a panel builder requests the full available width but a
    /// content-driven height.
    ///
    /// The two public builders are concrete over iced's real renderer, so they
    /// cannot be laid out with the test's null `()` renderer; assert their size
    /// strategy instead. A `Fill` height would make a panel swallow the
    /// column's leftover vertical space, so the request must be a
    /// content-driven `Shrink`.
    /// `beveled_tracks_content_height_and_fills_the_available_width` below
    /// resolves the real layout through the generic `beveled`.
    fn assert_intrinsic_height(element: Element<'_, Message>) {
        let size = element.as_widget().size();
        assert_eq!(size.width, Length::Fill);
        assert_eq!(size.height, Length::Shrink);
    }

    #[test]
    fn lcd_well_constructs_with_intrinsic_height() {
        assert_intrinsic_height(lcd_well(Text::new("x")));
    }

    // The well and inset fills are the bugfix's base-skin contract: the
    // display wells sit on the window face with pure-black blocks broken
    // into them, not on one flat near-black field. The constants are pinned
    // for value in `theme.rs`; a pure style function makes the fill each
    // container actually requests observable here.
    #[test]
    fn lcd_well_fills_with_the_face_and_the_inset_with_pure_black() {
        assert_eq!(
            well_style::lcd_well_style().background,
            Some(Background::Color(theme::WINDOW_BACKGROUND))
        );
        assert_eq!(
            well_style::lcd_inset_style().background,
            Some(Background::Color(theme::LCD_INSET_BLACK))
        );
        // The inset must read as a block cut into the face, so the two fills
        // never meet.
        assert_ne!(theme::WINDOW_BACKGROUND, theme::LCD_INSET_BLACK);
    }

    #[test]
    fn raised_panel_constructs_with_intrinsic_height() {
        assert_intrinsic_height(raised_panel(Text::new("x")));
    }

    // Lays the composition out with iced's null `()` renderer (compiled under
    // debug assertions), so the resolved height is asserted without a GPU. A
    // 7px-tall content under a 400px-tall limit must stay 7px tall, and its
    // 20px width must widen to the 500px limit. This is the regression guard
    // the size-strategy tests cannot be: it fails if the panel ever grows past
    // its content.
    #[test]
    fn beveled_tracks_content_height_and_fills_the_available_width() {
        let content: Element<'_, Message, Theme, ()> = Space::new()
            .width(Length::Fixed(20.0))
            .height(Length::Fixed(7.0))
            .into();
        let mut panel = beveled(content, true);
        let mut tree = Tree::new(panel.as_widget());
        let limits = Limits::new(Size::ZERO, Size::new(500.0, 400.0));

        let node = panel.as_widget_mut().layout(&mut tree, &(), &limits);

        assert_eq!(node.size(), Size::new(500.0, 7.0));
    }
}
