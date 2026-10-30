//! The custom Winamp title bar and the decorative section bands: the gold
//! title tape, the centered label between two tapes, the app bar's menu
//! button, and the shade, minimize, and close buttons.

use iced::{
    Color, Element, Length,
    widget::{Button, Column, Container, MouseArea, Row, Space, Text, container},
};

use crate::ui::{Message, bevel, style, theme};

/// The custom title bar's app name.
const TITLE_BAR_TEXT: &str = "NOSTALGIA";

/// The band label's size in pixels: a multiple of 8, so the letterform face
/// draws it crisp, and small enough for the letterform face's crisp 8px stock
/// to fit inside the 12px bar.
const TITLE_BAR_LABEL_SIZE: f32 = 8.0;

/// The custom title bar's height in px: the base skin's 12px title bar
/// (`winamp-screenshots/winamp-2.x-stack.png`'s main-window bar occupies
/// rows 0..11), not the mockups' doubled 24px band.
pub const TITLE_BAR_HEIGHT: f32 = 12.0;

/// The title bar button glyphs' size in px: a multiple of 8, and the largest
/// on-grid size that leaves the glyphs and their 1px bevels inside the 12px
/// bar. The shared [`super::fixed_width_button`] draws at the default size,
/// which is sized for the taller transport and shade strips.
const TITLE_BAR_BUTTON_GLYPH_SIZE: f32 = 8.0;

/// The title tape's height in px: the five ribbon rows the base skin draws
/// between its tapes — gold face, white shine slat, dark groove, brushed
/// gold, gold face again (`winamp-2.x-stack.png` rows 5..9 of the bar).
const TITLE_TAPE_HEIGHT: f32 = 5.0;

/// A raised chrome button like [`super::fixed_width_button`], with its glyph
/// at [`TITLE_BAR_BUTTON_GLYPH_SIZE`] so the window buttons fit inside
/// [`TITLE_BAR_HEIGHT`].
fn title_bar_glyph_button(
    glyph: &'static str,
    width: f32,
    font: iced::Font,
    message: Message,
) -> Button<'static, Message> {
    Button::new(
        Text::new(glyph)
            .font(font)
            .size(TITLE_BAR_BUTTON_GLYPH_SIZE),
    )
    .on_press(message)
    .width(Length::Fixed(width))
    .style(|_theme, status| style::chrome_button_style(status))
}

/// Each title-bar button's pinned width, so its glyph cannot resize it and
/// reflow the drag region beside it.
pub(super) const TITLE_BAR_BUTTON_WIDTH: f32 = 22.0;

/// The title bar's leftmost menu button's pinned width. It is narrower than
/// the window buttons: the M slot is a small square in the classic skin, and
/// it sits at the left edge of the bar rather than beside the window buttons.
pub(super) const TITLE_BAR_LEFT_BUTTON_WIDTH: f32 = 18.0;

/// A 1px full-width strip in `color`, the tape's top light edge and bottom
/// shadow.
fn colored_strip(color: Color) -> Element<'static, Message> {
    Container::new(Space::new())
        .width(Length::Fill)
        .height(Length::Fixed(1.0))
        .style(move |_theme| container::Style {
            background: Some(color.into()),
            ..container::Style::default()
        })
        .into()
}

/// [`TITLE_TAPE_HEIGHT`] tall gold title tape, the base skin's ribbon: its
/// two [`theme::TITLE_TAPE`] gold faces sandwich the white shine slat and the
/// dark groove under it, ending in the brushed-gold lower face. It fills the
/// width it is given and is vertically centered by the row that holds it, so
/// a band's two tapes split the spare width around the label.
fn title_tape() -> Element<'static, Message> {
    Column::new()
        .width(Length::Fill)
        .height(Length::Fixed(TITLE_TAPE_HEIGHT))
        .push(colored_strip(theme::TITLE_TAPE))
        .push(colored_strip(theme::TITLE_TEXT))
        .push(colored_strip(theme::TITLE_TAPE_SHADOW))
        .push(colored_strip(theme::TITLE_TAPE_BRUSHED))
        .push(colored_strip(theme::TITLE_TAPE))
        .into()
}

/// The tape/label/tape row shared by the app bar's drag region and the
/// decorative section bands. The two fill-width tapes split the spare width
/// equally, so the label sits at the row's exact center even though the app
/// bar's drag region is narrower than the window.
fn title_band_content(label: &'static str) -> Row<'static, Message> {
    Row::new()
        .push(title_tape())
        .push(
            Text::new(label)
                .size(TITLE_BAR_LABEL_SIZE)
                .font(theme::ui_font())
                .color(theme::TITLE_TEXT),
        )
        .push(title_tape())
        .spacing(6)
        .padding([0, 6])
        .align_y(iced::alignment::Vertical::Center)
        .width(Length::Fill)
        .height(Length::Fill)
}

/// A decorative section band: a raised [`theme::TITLE_BAR_FILL`] panel
/// carrying the white `label` centered between two gold tapes at
/// [`TITLE_BAR_HEIGHT`].
///
/// It deliberately has no buttons: this app is one real window whose controls
/// live in the app bar, so a second set here would be clickable-looking dead
/// chrome. It emits no messages and paints [`style::title_bar_style`].
pub fn view_title_band(label: &'static str) -> Element<'static, Message> {
    bevel::raised_panel(
        Container::new(title_band_content(label))
            .width(Length::Fill)
            .height(Length::Fixed(TITLE_BAR_HEIGHT))
            .style(|_theme| style::title_bar_style()),
    )
}

/// The custom Winamp title bar: the app name centered between gold tapes on a
/// raised [`theme::TITLE_BAR_FILL`] band, draggable to move the window, with
/// the classic main-menu `M` button at the left and shade, minimize, and close
/// buttons on the right. Double-clicking the band toggles the classic roll-up
/// (shade) mode via [`Message::ToggleWindowShade`].
///
/// The whole band except the buttons is a [`MouseArea`], so a press anywhere
/// on it emits [`Message::WindowDragged`] and the update loop begins the OS
/// window drag, while a double-click emits [`Message::ToggleWindowShade`].
/// The M button toggles the About panel and is sunken while `about_visible`
/// holds and raised otherwise; the shade, minimize, and close buttons are
/// [`title_bar_glyph_button`]s — the same raised chrome as the transport row,
/// with glyphs sized for the 12px bar — pinned so their glyphs cannot resize
/// them. The bar paints [`style::title_bar_style`] and wraps it in
/// [`bevel::raised_panel`], so it carries the base skin's raised bevel.
pub fn view_title_bar(about_visible: bool) -> Element<'static, Message> {
    let drag_region = MouseArea::new(
        Container::new(title_band_content(TITLE_BAR_TEXT))
            .width(Length::Fill)
            .height(Length::Fill),
    )
    .on_press(Message::WindowDragged)
    .on_double_click(Message::ToggleWindowShade);

    let menu_button = Button::new(
        Text::new("M")
            .font(theme::ui_font())
            .size(TITLE_BAR_BUTTON_GLYPH_SIZE),
    )
    .on_press(Message::ToggleAbout)
    .width(Length::Fixed(TITLE_BAR_LEFT_BUTTON_WIDTH))
    .style(move |_theme, status| style::toggle_button_style(about_visible, status));

    bevel::raised_panel(
        Container::new(
            Row::new()
                .push(menu_button)
                .push(drag_region)
                .push(title_bar_glyph_button(
                    // `▭` is uncovered by both embedded faces (see part 1/4's
                    // probe in `PLANS.md`); `□` is the closest covered
                    // substitute and draws crisp in Tiny5.
                    "□",
                    TITLE_BAR_BUTTON_WIDTH,
                    theme::shape_font(),
                    Message::ToggleWindowShade,
                ))
                .push(title_bar_glyph_button(
                    "–",
                    TITLE_BAR_BUTTON_WIDTH,
                    theme::shape_font(),
                    Message::MinimizeWindow,
                ))
                .push(title_bar_glyph_button(
                    "✕",
                    TITLE_BAR_BUTTON_WIDTH,
                    theme::shape_font(),
                    Message::CloseWindow,
                )),
        )
        .width(Length::Fill)
        .height(Length::Fixed(TITLE_BAR_HEIGHT))
        .style(|_theme| style::title_bar_style()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // The title bar's buttons must stay pinned, for the same reason the
    // transport buttons were pinned after the label-reflow bug: a glyph whose
    // metrics change would otherwise resize its button and shift the drag
    // region or the buttons after it. These widths are private to this module
    // and no other test reads them, so a zeroed or widened button could pass
    // the suite while the bar reflowed. Pin the widths; the 22 > 18 split is
    // the documented narrow M slot.
    // Part 2/4 of the pixel-font split re-based every size the chrome hands
    // iced onto the 8px grid — only multiples of 8 keep the embedded faces
    // crisp — so pin each size as a multiple of 8 as well as its exact value.
    #[test]
    fn title_bar_text_sizes_stay_on_the_pixel_grid() {
        assert_eq!(TITLE_BAR_LABEL_SIZE, 8.0);
        assert_eq!(TITLE_BAR_LABEL_SIZE % 8.0, 0.0);
        assert_eq!(TITLE_BAR_BUTTON_GLYPH_SIZE, 8.0);
        assert_eq!(TITLE_BAR_BUTTON_GLYPH_SIZE % 8.0, 0.0);
    }

    // The bar's own metrics are shapes the chrome draws with colored strips,
    // not text a face lays out, so the multiple-of-8 rule above does not pin
    // them: they are pinned straight to the reference rows instead. The 12px
    // bar and its 5px ribbon are `winamp-2.x-stack.png` rows 0..11 with the
    // ribbon on rows 5..9.
    #[test]
    fn title_bar_is_the_base_skins_twelve_px_bar_with_its_five_px_ribbon() {
        assert_eq!(TITLE_BAR_HEIGHT, 12.0);
        assert_eq!(TITLE_TAPE_HEIGHT, 5.0);
    }

    #[test]
    fn title_bar_buttons_keep_their_pinned_widths() {
        assert_eq!(TITLE_BAR_BUTTON_WIDTH, 22.0);
        assert_eq!(TITLE_BAR_LEFT_BUTTON_WIDTH, 18.0);
    }
}
