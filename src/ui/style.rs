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
    widget::{Column, Container, Row, Space, Stack, button, container, rule, slider},
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

/// The chrome face shade for an interaction state: pressed when `pressed`,
/// hovered when `hovered`, and the resting shade otherwise. Both chrome styles
/// map their own status enum onto this same ladder — the button by
/// Active/Hovered/Pressed and the slider by Active/Hovered/Dragged — so the
/// ladder lives here once and the two stay in lockstep. `pressed` wins when
/// both are true, though no current status is both. Pure, so it needs no
/// widget to test.
fn chrome_face(hovered: bool, pressed: bool) -> Color {
    if pressed {
        theme::BUTTON_FACE_PRESSED
    } else if hovered {
        theme::BUTTON_FACE_HOVERED
    } else {
        theme::BUTTON_FACE
    }
}

/// The raised (Active/Hovered) and sunken (Pressed) chrome style for a button.
///
/// iced's [`Border`] carries a single colour, so the two-tone edge reuses
/// [`bevel_edges`]: raised, it is a 1px light border on the top/left and a dark
/// no-blur [`Shadow`] offset down-right on the bottom/right; pressing sinks the
/// pair. The face comes from [`chrome_face`], which darkens it on hover and
/// sinks it while pressed. Pure, so the colour rule is testable without
/// building a widget.
pub fn chrome_button_style(status: button::Status) -> button::Style {
    let (top_left, bottom_right) = bevel_edges(!matches!(status, button::Status::Pressed));
    let face = chrome_face(
        matches!(status, button::Status::Hovered),
        matches!(status, button::Status::Pressed),
    );

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

/// The sunken-groove / raised-thumb chrome style for a slider.
///
/// The base skin's slider is a dark recess with a blocky chrome thumb. iced's
/// [`slider::Rail`] paints a single-colour [`Border`] and no shadow, so the
/// sunken edge is the dark 1px outline around a near-black groove; the
/// [`slider::Handle`] carries the light 1px border that reads as raised
/// chrome, the same light-border trick [`chrome_button_style`] uses. The rail
/// is uniform (`LCD_BACKGROUND` on both sides of the handle) because the thumb,
/// not a fill colour, marks the value. The handle face comes from
/// [`chrome_face`], the same ladder [`chrome_button_style`] uses. Pure, so the
/// colour rule is testable without building a widget.
pub fn chrome_slider_style(status: slider::Status) -> slider::Style {
    let face = chrome_face(
        matches!(status, slider::Status::Hovered),
        matches!(status, slider::Status::Dragged),
    );

    slider::Style {
        rail: slider::Rail {
            backgrounds: (
                Background::Color(theme::LCD_BACKGROUND),
                Background::Color(theme::LCD_BACKGROUND),
            ),
            width: 4.0,
            border: Border {
                color: theme::PANEL_EDGE_DARK,
                width: 1.0,
                radius: 0.0.into(),
            },
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Rectangle {
                width: 8,
                border_radius: 0.0.into(),
            },
            background: Background::Color(face),
            border_width: 1.0,
            border_color: theme::PANEL_EDGE_LIGHT,
        },
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
    fn chrome_face_is_the_rest_hover_press_ladder() {
        assert_eq!(chrome_face(false, false), theme::BUTTON_FACE);
        assert_eq!(chrome_face(true, false), theme::BUTTON_FACE_HOVERED);
        assert_eq!(chrome_face(false, true), theme::BUTTON_FACE_PRESSED);
        // No current status is both hovered and pressed, so pin the precedence
        // here rather than through a style.
        assert_eq!(chrome_face(true, true), theme::BUTTON_FACE_PRESSED);
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
        assert_eq!(style.border.radius, 0.0.into());
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
        // Only Disabled dims the text (its own test pins the alpha drop), so a
        // hovered button keeps full-strength chrome text. The Active test above
        // pins `TEXT` for the shared `_` arm's first status; assert it here for
        // Hovered too, or a match split that dimmed Hovered would clear every
        // other test while the hovered label faded.
        assert_eq!(hovered.text_color, theme::TEXT);
        // "Lifts the face" is the name's contract: hovering swaps only the
        // background. The edge colours are asserted unchanged above; assert the
        // geometry too, or a change that made (say) a hovered button's border
        // thicker or its shadow offset would clear every other test while the
        // chrome distorted under the cursor.
        assert_eq!(hovered.border.width, active.border.width);
        assert_eq!(hovered.border.radius, active.border.radius);
        assert_eq!(hovered.shadow.offset, active.shadow.offset);
        assert_eq!(hovered.shadow.blur_radius, active.shadow.blur_radius);
    }

    #[test]
    fn chrome_button_style_pressed_sinks_and_reverses_the_edge() {
        let active = chrome_button_style(button::Status::Active);
        let pressed = chrome_button_style(button::Status::Pressed);
        assert_eq!(
            pressed.background,
            Some(Background::Color(theme::BUTTON_FACE_PRESSED))
        );
        assert_eq!(pressed.border.color, theme::PANEL_EDGE_DARK);
        assert_eq!(pressed.shadow.color, theme::PANEL_EDGE_LIGHT);
        // The Pressed twin of the Hovered text-colour assertion above.
        assert_eq!(pressed.text_color, theme::TEXT);
        // "Sinks and reverses the edge" is the name's contract: pressing swaps
        // the two edge colours and the face, but the 1px geometry stays. Assert
        // it, or a status-dependent border width, radius, or shadow offset
        // could pass every test while the pressed bevel changed shape.
        assert_eq!(pressed.border.width, active.border.width);
        assert_eq!(pressed.border.radius, active.border.radius);
        assert_eq!(pressed.shadow.offset, active.shadow.offset);
        assert_eq!(pressed.shadow.blur_radius, active.shadow.blur_radius);
    }

    #[test]
    fn chrome_slider_style_active_is_a_sunken_groove_with_a_raised_thumb() {
        let style = chrome_slider_style(slider::Status::Active);
        assert_eq!(
            style.rail.backgrounds,
            (
                Background::Color(theme::LCD_BACKGROUND),
                Background::Color(theme::LCD_BACKGROUND)
            )
        );
        assert_eq!(style.rail.width, 4.0);
        assert_eq!(style.rail.border.color, theme::PANEL_EDGE_DARK);
        assert_eq!(style.rail.border.width, 1.0);
        assert_eq!(
            style.handle.shape,
            slider::HandleShape::Rectangle {
                width: 8,
                border_radius: 0.0.into()
            }
        );
        assert_eq!(
            style.handle.background,
            Background::Color(theme::BUTTON_FACE)
        );
        assert_eq!(style.handle.border_width, 1.0);
        assert_eq!(style.handle.border_color, theme::PANEL_EDGE_LIGHT);
    }

    #[test]
    fn chrome_slider_style_hovered_and_dragged_only_swap_the_handle_face() {
        let active = chrome_slider_style(slider::Status::Active);
        let hovered = chrome_slider_style(slider::Status::Hovered);
        let dragged = chrome_slider_style(slider::Status::Dragged);

        assert_eq!(
            hovered.handle.background,
            Background::Color(theme::BUTTON_FACE_HOVERED)
        );
        assert_eq!(
            dragged.handle.background,
            Background::Color(theme::BUTTON_FACE_PRESSED)
        );
        assert_eq!(hovered.rail, active.rail);
        assert_eq!(dragged.rail, active.rail);
        // The name's "only" is the contract: hovering and dragging swap the
        // handle face and nothing else. The Active test above pins the handle's
        // shape and border for the shared fields; assert the two non-resting
        // statuses keep them unchanged too, or a change that made (say) the
        // dragged thumb's border a status-dependent colour would clear every
        // other test while the thumb gained a colour no test read.
        assert_eq!(hovered.handle.shape, active.handle.shape);
        assert_eq!(dragged.handle.shape, active.handle.shape);
        assert_eq!(hovered.handle.border_width, active.handle.border_width);
        assert_eq!(dragged.handle.border_width, active.handle.border_width);
        assert_eq!(hovered.handle.border_color, active.handle.border_color);
        assert_eq!(dragged.handle.border_color, active.handle.border_color);
    }

    #[test]
    fn chrome_button_style_disabled_dims_the_text_but_keeps_the_edges() {
        let active = chrome_button_style(button::Status::Active);
        let disabled = chrome_button_style(button::Status::Disabled);
        assert_eq!(disabled.border.color, active.border.color);
        assert_eq!(disabled.shadow.color, active.shadow.color);
        assert_eq!(disabled.background, active.background);
        assert!(disabled.text_color.a < theme::TEXT.a);
        // "Keeps the edges" is the name's contract: Disabled dims only the
        // text, so the face and both edge colours (asserted above) and the 1px
        // geometry all stay. Assert the geometry too, or a status-dependent
        // width, radius, or shadow offset could pass every other test while the
        // disabled bevel changed shape.
        assert_eq!(disabled.border.width, active.border.width);
        assert_eq!(disabled.border.radius, active.border.radius);
        assert_eq!(disabled.shadow.offset, active.shadow.offset);
        assert_eq!(disabled.shadow.blur_radius, active.shadow.blur_radius);
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
