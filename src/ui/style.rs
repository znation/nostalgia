//! Reusable Winamp bevel styling.
//!
//! The base skin's defining 3D edge is a light top/left and dark bottom/right
//! pair on raised chrome, reversed in a sunken well. iced 0.14's `Border` is a
//! single colour of uniform width, so this module composes the two-tone bevel
//! from 1px [`rule`] edges and overlays it on the panel content.
//! [`bevel_edges`] is the pure colour rule at the heart of the layer;
//! [`lcd_well`] and [`raised_panel`] are the two panel shapes the views use.

use iced::{
    Background, Border, Color, Element, Length, Shadow, Theme, Vector,
    widget::{Column, Container, Row, Space, Stack, button, container, rule},
};

use super::{Message, theme};

/// The `(top_left, bottom_right)` edge colours for a bevel.
///
/// A raised edge catches the light on its top and left and falls dark on its
/// bottom and right; a sunken (recessed) edge reverses that. Pure, so the
/// colour rule is testable without building any widgets.
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
fn styled_edge<'a, R>(rule: rule::Rule<'a, Theme>, color: Color) -> Element<'a, Message, Theme, R>
where
    R: iced::advanced::Renderer + 'a,
{
    rule.style(move |_theme| rule::Style {
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

/// Wraps `content` in a sunken LCD well: a full-width near-black recess behind
/// the content with a few px of padding and the reversed (sunken) bevel.
///
/// No text colour is set, so the caller's caption keeps the theme text colour
/// and only the title's explicit green stays green.
pub fn lcd_well<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    let well = Container::new(content)
        .width(Length::Fill)
        .padding(4)
        .style(|_theme| container::Style {
            background: Some(theme::LCD_BACKGROUND.into()),
            ..container::Style::default()
        });

    beveled(well.into(), false)
}

/// Wraps `content` in a raised chrome panel on the window face.
///
/// No extra background is painted, so the panel composes with the existing
/// window theme.
pub fn raised_panel<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    beveled(content.into(), true)
}

/// The raised (Active/Hovered) and sunken (Pressed) chrome style for a button.
///
/// iced's [`Border`] carries a single colour, so the two-tone edge reuses
/// [`bevel_edges`]: raised, it is a 1px light border on the top/left and a dark
/// no-blur [`Shadow`] offset down-right on the bottom/right; pressing sinks the
/// pair. The face darkens on hover and sinks while pressed. Pure and
/// theme-free, so the colour rule is testable without building a widget.
pub fn chrome_button_style(status: button::Status) -> button::Style {
    let (top_left, bottom_right) = bevel_edges(!matches!(status, button::Status::Pressed));
    let face = match status {
        button::Status::Active | button::Status::Disabled => theme::BUTTON_FACE,
        button::Status::Hovered => theme::BUTTON_FACE_HOVERED,
        button::Status::Pressed => theme::BUTTON_FACE_PRESSED,
    };

    let text_color = match status {
        button::Status::Disabled => theme::TEXT.scale_alpha(0.5),
        _ => theme::TEXT,
    };

    button::Style {
        background: Some(Background::Color(face)),
        text_color,
        border: Border {
            color: top_left,
            width: 1.0,
            radius: 0.0.into(),
        },
        shadow: Shadow {
            color: bottom_right,
            offset: Vector::new(1.0, 1.0),
            blur_radius: 0.0,
        },
        ..button::Style::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::{
        Size,
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

    #[test]
    fn chrome_button_style_active_is_raised_chrome() {
        let style = chrome_button_style(button::Status::Active);
        assert_eq!(
            style.background,
            Some(Background::Color(theme::BUTTON_FACE))
        );
        assert_eq!(style.text_color, theme::TEXT);
        assert_eq!(style.border.color, theme::PANEL_EDGE_LIGHT);
        assert_eq!(style.border.width, 1.0);
        assert_eq!(style.shadow.color, theme::PANEL_EDGE_DARK);
        assert_eq!(style.shadow.offset, Vector::new(1.0, 1.0));
        assert_eq!(style.shadow.blur_radius, 0.0);
    }

    #[test]
    fn chrome_button_style_hovered_lifts_the_face() {
        let active = chrome_button_style(button::Status::Active);
        let hovered = chrome_button_style(button::Status::Hovered);
        assert_eq!(
            hovered.background,
            Some(Background::Color(theme::BUTTON_FACE_HOVERED))
        );
        assert_eq!(hovered.border.color, active.border.color);
        assert_eq!(hovered.shadow.color, active.shadow.color);
    }

    #[test]
    fn chrome_button_style_pressed_sinks_and_reverses_the_edge() {
        let pressed = chrome_button_style(button::Status::Pressed);
        assert_eq!(
            pressed.background,
            Some(Background::Color(theme::BUTTON_FACE_PRESSED))
        );
        assert_eq!(pressed.border.color, theme::PANEL_EDGE_DARK);
        assert_eq!(pressed.shadow.color, theme::PANEL_EDGE_LIGHT);
    }

    #[test]
    fn chrome_button_style_disabled_dims_the_text_but_keeps_the_edges() {
        let active = chrome_button_style(button::Status::Active);
        let disabled = chrome_button_style(button::Status::Disabled);
        assert_eq!(disabled.border.color, active.border.color);
        assert_eq!(disabled.shadow.color, active.shadow.color);
        assert_eq!(disabled.background, active.background);
        assert!(disabled.text_color.a < theme::TEXT.a);
    }

    // The two public builders are concrete over iced's real renderer, so they
    // cannot be laid out with the test's null `()` renderer; these assert
    // their size strategy instead. A `Fill` height would make a panel swallow
    // the column's leftover vertical space, so the request must be a
    // content-driven `Shrink`. `beveled_tracks_content_height` below resolves
    // the real layout through the generic `beveled`.
    #[test]
    fn lcd_well_constructs_with_intrinsic_height() {
        let well = lcd_well(Text::new("x"));
        let size = well.as_widget().size();
        assert_eq!(size.width, Length::Fill);
        assert_eq!(size.height, Length::Shrink);
    }

    #[test]
    fn raised_panel_constructs_with_intrinsic_height() {
        let panel = raised_panel(Text::new("x"));
        let size = panel.as_widget().size();
        assert_eq!(size.width, Length::Fill);
        assert_eq!(size.height, Length::Shrink);
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
