//! Winamp widget chrome styles.
//!
//! Every iced widget this player dresses — buttons, the preset pick list and
//! its menu, sliders, playlist rows, the playlist scroll well, and the title
//! bar — gets its base-skin colours and 1px chrome geometry here. Each style
//! is a pure `Status` → `Style` builder, so its colour rule is testable
//! without building a widget. The reusable two-tone bevel the buttons and
//! sliders draw their edges from lives in the sibling `bevel` module.

use iced::{
    Background, Border, Color, Shadow, Vector,
    widget::{button, container, overlay::menu, pick_list, scrollable, slider},
};

use super::bevel::bevel_edges;
use super::theme;

/// A 1px square-cornered [`Border`] in `color`.
///
/// The base skin's chrome edges are square 1px outlines; iced's [`Border`]
/// carries no `Default`, so this shared shape lives here rather than as a
/// struct literal at each style site. Pure, so it needs no widget to test.
fn square_border(color: Color) -> Border {
    Border {
        color,
        width: 1.0,
        radius: 0.0.into(),
    }
}

/// A 1px down-right [`Shadow`] in `color`, the bottom/right half of a bevel.
///
/// iced's [`Border`] draws a single colour all around, so the other half of a
/// two-tone bevel is a no-blur shadow offset one pixel down and right; the
/// shared geometry lives here rather than as a struct literal at each style
/// site. Pure, so it needs no widget to test.
fn square_shadow(color: Color) -> Shadow {
    Shadow {
        color,
        offset: Vector::new(1.0, 1.0),
        blur_radius: 0.0,
    }
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

/// The flat base-skin title-bar fill: a solid [`theme::TITLE_BLUE`] band with
/// the light [`theme::TEXT`] colour for the app name.
///
/// [`super::bevel::raised_panel`] composes with the window face and
/// [`super::bevel::lcd_well`] paints its own background, but neither sets a text
/// colour; the title bar paints both its own opaque background and its own
/// light text. Pure, so both fields are testable without a widget.
pub fn title_bar_style() -> container::Style {
    container::Style {
        background: Some(Background::Color(theme::TITLE_BLUE)),
        text_color: Some(theme::TEXT),
        ..container::Style::default()
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
        border: square_border(top_left),
        shadow: square_shadow(bottom_right),
        ..button::Style::default()
    }
}

/// The chrome style for the equalizer's preset pick list.
///
/// The closed control is flat chrome, like the transport buttons: a
/// [`theme::BUTTON_FACE`] face with the dark 1px outline and [`theme::TEXT`]
/// label. Unlike the buttons and sliders, hovering the closed control does
/// not lift the face — the base skin's combo boxes give no hover feedback —
/// but opening the menu sinks the face to [`theme::BUTTON_FACE_PRESSED`]
/// through [`chrome_face`], the same press shade the other chrome uses. The
/// placeholder (the `(none)` custom-curve label) is drawn in the lighter
/// [`theme::PANEL_EDGE_LIGHT`] so an unselected list reads as empty. Pure, so
/// the colour rule is testable without building a widget.
pub fn chrome_pick_list_style(status: pick_list::Status) -> pick_list::Style {
    let opened = matches!(status, pick_list::Status::Opened { .. });

    pick_list::Style {
        text_color: theme::TEXT,
        placeholder_color: theme::PANEL_EDGE_LIGHT,
        handle_color: theme::TEXT,
        background: Background::Color(chrome_face(false, opened)),
        border: square_border(theme::PANEL_EDGE_DARK),
    }
}

/// The base-skin style for the preset pick list's open menu.
///
/// The dropdown is a [`theme::BUTTON_FACE`] panel with the dark 1px chrome
/// outline; its options are [`theme::TEXT`] on that face, and the highlighted
/// option uses the title-bar [`theme::TITLE_BLUE`] with the same light text,
/// matching Winamp's selection blue. No drop shadow, so the menu stays flat
/// like the rest of the base-skin chrome. Pure, so the colour rule is
/// testable without building a widget.
pub fn preset_menu_style() -> menu::Style {
    menu::Style {
        background: theme::BUTTON_FACE.into(),
        border: square_border(theme::PANEL_EDGE_DARK),
        text_color: theme::TEXT,
        selected_text_color: theme::TEXT,
        selected_background: theme::TITLE_BLUE.into(),
        shadow: Shadow::default(),
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
            border: square_border(theme::PANEL_EDGE_DARK),
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

/// The flat playlist-entry style for a browse row.
///
/// The base skin's playlist rows are flat light text on the near-black well:
/// no bevel, so the resting and disabled faces are transparent and let the
/// well show through, while hover lifts the row to [`theme::BUTTON_FACE`] and
/// press sinks it to [`theme::BUTTON_FACE_PRESSED`] — the same face ladder
/// [`chrome_face`] names. Pure and status-driven, so the row rule is testable
/// without building a widget.
pub fn playlist_row_style(status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered => Some(Background::Color(theme::BUTTON_FACE)),
        button::Status::Pressed => Some(Background::Color(theme::BUTTON_FACE_PRESSED)),
        button::Status::Active | button::Status::Disabled => None,
    };

    button::Style {
        background,
        text_color: theme::TEXT,
        ..button::Style::default()
    }
}

/// The sunken-playlist-well style for the browse list's [`scrollable::Style`].
///
/// The base skin's playlist editor is a near-black recess with a dark track
/// scrollbar carrying a raised chrome scroller. iced's [`scrollable::Style`]
/// has no `Default`, so this builds every field: the `container` paints the
/// [`theme::LCD_BACKGROUND`] well with the same dark-border + light-offset
/// sunken edge [`chrome_button_style`] uses for its pressed state, both rails
/// sit in [`theme::WINDOW_BACKGROUND`] with a [`theme::BUTTON_FACE`] scroller
/// bordered light, and the touch `auto_scroll` overlay matches the well.
/// Classic Winamp scrollbars give no hover/drag feedback, so the style ignores
/// the [`scrollable::Status`] iced passes. Pure, so the colour rule is
/// testable without building a widget.
pub fn playlist_scrollable_style() -> scrollable::Style {
    let rail = scrollable::Rail {
        background: Some(Background::Color(theme::WINDOW_BACKGROUND)),
        border: square_border(theme::PANEL_EDGE_DARK),
        scroller: scrollable::Scroller {
            background: Background::Color(theme::BUTTON_FACE),
            border: square_border(theme::PANEL_EDGE_LIGHT),
        },
    };

    scrollable::Style {
        container: container::Style {
            text_color: Some(theme::TEXT),
            background: Some(theme::LCD_BACKGROUND.into()),
            border: square_border(theme::PANEL_EDGE_DARK),
            shadow: square_shadow(theme::PANEL_EDGE_LIGHT),
            snap: false,
        },
        vertical_rail: rail,
        horizontal_rail: rail,
        gap: None,
        auto_scroll: scrollable::AutoScroll {
            background: Background::Color(theme::LCD_BACKGROUND),
            border: square_border(theme::PANEL_EDGE_LIGHT),
            shadow: square_shadow(theme::PANEL_EDGE_DARK),
            icon: theme::TEXT,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Asserts `status`'s chrome bevel keeps `active`'s 1px square geometry.
    ///
    /// Interaction statuses change the face and edge colours; they must not
    /// change the bevel's shape. The three non-resting button-status tests
    /// each pin that, so the geometry contract lives here once.
    fn assert_same_edge_geometry(active: &button::Style, status: &button::Style) {
        assert_eq!(status.border.width, active.border.width);
        assert_eq!(status.border.radius, active.border.radius);
        assert_eq!(status.shadow.offset, active.shadow.offset);
        assert_eq!(status.shadow.blur_radius, active.shadow.blur_radius);
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
        assert_same_edge_geometry(&active, &hovered);
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
        assert_same_edge_geometry(&active, &pressed);
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
        // The rail's blocky 1px outline: a Winamp sunken groove reads as chrome
        // only with square corners. The whole-`Rail` equality in the
        // hovered/dragged test pins only that the radius is status-independent,
        // not its literal value, so a shared-code change to a rounded groove
        // would clear every other test while the rail lost its chrome shape.
        assert_eq!(style.rail.border.radius, 0.0.into());
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
        assert_same_edge_geometry(&active, &disabled);
    }

    #[test]
    fn playlist_row_style_is_a_flat_entry_that_lifts_and_sinks() {
        assert_eq!(playlist_row_style(button::Status::Active).background, None);
        assert_eq!(
            playlist_row_style(button::Status::Hovered).background,
            Some(Background::Color(theme::BUTTON_FACE))
        );
        assert_eq!(
            playlist_row_style(button::Status::Pressed).background,
            Some(Background::Color(theme::BUTTON_FACE_PRESSED))
        );
        assert_eq!(
            playlist_row_style(button::Status::Disabled).background,
            None
        );

        // Playlist rows are flat: they carry no bevel of their own, so the
        // well's near-black shows through at rest. Pin that the border and
        // shadow stay at the iced default rather than a raised or sunken edge.
        let active = playlist_row_style(button::Status::Active);
        let default = button::Style::default();
        assert_eq!(active.border, default.border);
        assert_eq!(active.shadow, default.shadow);

        for status in [
            button::Status::Active,
            button::Status::Hovered,
            button::Status::Pressed,
            button::Status::Disabled,
        ] {
            assert_eq!(playlist_row_style(status).text_color, theme::TEXT);
        }
    }

    #[test]
    fn playlist_scrollable_style_sinks_the_well_and_raises_the_scroller() {
        let style = playlist_scrollable_style();

        assert_eq!(
            style.container.background,
            Some(Background::Color(theme::LCD_BACKGROUND))
        );
        assert_eq!(style.container.text_color, Some(theme::TEXT));
        assert_eq!(style.container.border.color, theme::PANEL_EDGE_DARK);
        assert_eq!(style.container.border.width, 1.0);
        assert_eq!(style.container.border.radius, 0.0.into());
        assert_eq!(style.container.shadow.color, theme::PANEL_EDGE_LIGHT);
        assert_eq!(style.container.shadow.offset, Vector::new(1.0, 1.0));
        assert_eq!(style.container.shadow.blur_radius, 0.0);
        assert!(!style.container.snap);
        assert_eq!(style.gap, None);

        // Both rails are the same dark track with a raised chrome scroller; a
        // change that styled only one axis would leave a bare track on the
        // other. Pin each field of each rail.
        assert_eq!(style.vertical_rail, style.horizontal_rail);
        for rail in [style.vertical_rail, style.horizontal_rail] {
            assert_eq!(
                rail.background,
                Some(Background::Color(theme::WINDOW_BACKGROUND))
            );
            assert_eq!(rail.border.color, theme::PANEL_EDGE_DARK);
            assert_eq!(rail.border.width, 1.0);
            assert_eq!(
                rail.scroller.background,
                Background::Color(theme::BUTTON_FACE)
            );
            assert_eq!(rail.scroller.border.color, theme::PANEL_EDGE_LIGHT);
            assert_eq!(rail.scroller.border.width, 1.0);
        }

        // The touch auto-scroll overlay matches the well it scrolls.
        assert_eq!(
            style.auto_scroll.background,
            Background::Color(theme::LCD_BACKGROUND)
        );
        assert_eq!(style.auto_scroll.border.color, theme::PANEL_EDGE_LIGHT);
        assert_eq!(style.auto_scroll.icon, theme::TEXT);
    }

    #[test]
    fn title_bar_style_is_title_blue_with_light_text() {
        let style = title_bar_style();
        assert_eq!(style.background, Some(Background::Color(theme::TITLE_BLUE)));
        assert_eq!(style.text_color, Some(theme::TEXT));
    }

    #[test]
    fn chrome_pick_list_style_sinks_the_face_when_opened() {
        let active = chrome_pick_list_style(pick_list::Status::Active);
        assert_eq!(active.background, Background::Color(theme::BUTTON_FACE));
        assert_eq!(active.text_color, theme::TEXT);
        assert_eq!(active.placeholder_color, theme::PANEL_EDGE_LIGHT);
        assert_eq!(active.handle_color, theme::TEXT);
        assert_eq!(active.border.color, theme::PANEL_EDGE_DARK);
        assert_eq!(active.border.width, 1.0);
        assert_eq!(active.border.radius, 0.0.into());

        // Hovering the closed control gives no lift: the pick list only
        // changes face when the menu opens.
        let hovered = chrome_pick_list_style(pick_list::Status::Hovered);
        assert_eq!(hovered.background, Background::Color(theme::BUTTON_FACE));
        assert_eq!(hovered.text_color, theme::TEXT);

        let opened = chrome_pick_list_style(pick_list::Status::Opened { is_hovered: false });
        assert_eq!(
            opened.background,
            Background::Color(theme::BUTTON_FACE_PRESSED)
        );
        assert_eq!(opened.text_color, theme::TEXT);
    }

    #[test]
    fn preset_menu_style_uses_the_base_skin_face_and_selection_blue() {
        let style = preset_menu_style();
        assert_eq!(style.background, Background::Color(theme::BUTTON_FACE));
        assert_eq!(style.border.color, theme::PANEL_EDGE_DARK);
        assert_eq!(style.border.width, 1.0);
        assert_eq!(style.text_color, theme::TEXT);
        assert_eq!(style.selected_text_color, theme::TEXT);
        assert_eq!(
            style.selected_background,
            Background::Color(theme::TITLE_BLUE)
        );
        assert_eq!(style.shadow, Shadow::default());
    }
}
