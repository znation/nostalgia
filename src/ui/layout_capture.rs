//! Test-only harness for laying a widget tree out headlessly and reading the
//! text fragments it would draw.
//!
//! `iced`'s `Element` API gives no way to read the text a tree carries, so the
//! harness lays the tree out under a fixed [`Limits`] and records every `text`
//! operation it performs with its absolute [`Rectangle`]. The same entry point
//! serves the null renderer and the headless tiny-skia renderer — the latter
//! measures text for real, so a test can assert where a label actually lands.
//!
//! Layout bugs previously needed a hand-run window measurement; a test can
//! now lay out the real root view and confirm the geometry offline.

use iced::advanced::{
    Layout,
    layout::{Limits, Node},
    renderer::Headless,
    widget::{Id, Tree, operation::Operation},
};
use iced::{Element, Event, Pixels, Point, Rectangle, Size};

use super::theme;

/// The bounds every captured tree is laid out in.
pub(crate) fn limits() -> Limits {
    Limits::new(Size::ZERO, Size::new(1000.0, 1000.0))
}

/// Builds iced's headless tiny-skia renderer.
///
/// tiny-skia measures text in software, so a tree containing the fallback
/// renderer's bevel widgets still lays out without a window or GPU.
pub(crate) fn headless_renderer() -> iced::Renderer {
    load_vendored_fonts();
    futures::executor::block_on(<iced::Renderer as Headless>::new(
        theme::ui_font(),
        Pixels(16.0),
        Some("tiny-skia"),
    ))
    .expect("the headless tiny-skia renderer")
}

/// Registers the vendored pixel faces — the letterform and shape-glyph TTFs
/// under `assets/fonts/` — in the process-wide [`font_system`] every headless
/// capture resolves texts through, so a test can address either face with
/// `iced::Font::with_name` without a per-test load. Name-less text resolves
/// through [`headless_renderer`]'s default, which is [`theme::ui_font`] —
/// the same face the app's `.default_font` draws — so the captured tests
/// measure exactly what the app renders.
/// Idempotent: the `OnceLock` runs it once, and `FontSystem::load_font`
/// itself records its borrowed bytes by address and skips repeats.
fn load_vendored_fonts() {
    use std::borrow::Cow;

    static VENDORED: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    VENDORED.get_or_init(|| {
        let mut font_system = iced::advanced::graphics::text::font_system()
            .write()
            .expect("the global font system the headless captures record texts through");
        font_system.load_font(Cow::Borrowed(
            include_bytes!("../../assets/fonts/Silkscreen-Regular.ttf").as_slice(),
        ));
        font_system.load_font(Cow::Borrowed(
            include_bytes!("../../assets/fonts/Tiny5.ttf").as_slice(),
        ));
    });
}

/// The text fragments a laid-out tree draws and the resolved root size.
#[derive(Debug, Default)]
pub(crate) struct TextCapture {
    fragments: Vec<(String, Rectangle)>,
    size: Size,
}

impl TextCapture {
    /// The fragments in draw order, each with its absolute bounds.
    pub(crate) fn fragments(&self) -> &[(String, Rectangle)] {
        &self.fragments
    }

    /// The fragments' text in draw order.
    pub(crate) fn texts(&self) -> impl Iterator<Item = &str> {
        self.fragments.iter().map(|(text, _)| text.as_str())
    }

    /// The root's resolved size under [`limits`].
    pub(crate) fn size(&self) -> Size {
        self.size
    }

    /// The bounds of `label`'s first fragment.
    ///
    /// Panics with every captured fragment when the label was not drawn, so a
    /// missing control shows what the tree drew instead.
    pub(crate) fn find(&self, label: &str) -> Rectangle {
        self.fragments
            .iter()
            .find(|(text, _)| text == label)
            .map(|(_, bounds)| *bounds)
            .unwrap_or_else(|| {
                panic!(
                    "no {label} text fragment was drawn: {:?}",
                    self.texts().collect::<Vec<_>>()
                )
            })
    }
}

#[derive(Default)]
struct CapturedText(Vec<(String, Rectangle)>);

/// The rendered RGBA pixels a laid-out tree draws and their pixel size.
#[derive(Debug)]
pub(crate) struct PixelCapture {
    pixels: Vec<u8>,
    size: Size<u32>,
}

impl PixelCapture {
    /// The strongest green channel value drawn inside `bounds`.
    ///
    /// Text is antialiased, so a label's glyph interiors carry its fill color
    /// while its edges blend toward the background. Taking the maximum green
    /// over the label's bounds recovers the fill it was drawn with without
    /// depending on any single glyph pixel.
    pub(crate) fn max_green_in(&self, bounds: Rectangle) -> u8 {
        self.max_channel_in(bounds, 1)
    }

    /// The strongest blue channel value drawn inside `bounds`, for telling the
    /// visualizer's gray peak markers from its blue-poor ramp colors. Like
    /// [`Self::max_red_in`], it returns the maximum over the covered pixels.
    pub(crate) fn max_blue_in(&self, bounds: Rectangle) -> u8 {
        self.max_channel_in(bounds, 2)
    }

    /// The strongest red channel value drawn inside `bounds`.
    ///
    /// Used where a fill's hue rather than its brightness carries the claim,
    /// such as the visualizer's ramp running from dark green to orange. Like
    /// [`Self::max_green_in`], it returns the maximum over the covered pixels
    /// so an antialiased edge cannot hide the fill color.
    pub(crate) fn max_red_in(&self, bounds: Rectangle) -> u8 {
        self.max_channel_in(bounds, 0)
    }

    /// The strongest `channel` (0 = red, 1 = green, 2 = blue) value drawn
    /// inside `bounds`, clipped to the image's edges.
    fn max_channel_in(&self, bounds: Rectangle, channel: usize) -> u8 {
        let width = self.size.width as f32;
        let height = self.size.height as f32;
        let x_start = bounds.x.max(0.0).floor() as u32;
        let y_start = bounds.y.max(0.0).floor() as u32;
        let x_end = (bounds.x + bounds.width).min(width).ceil() as u32;
        let y_end = (bounds.y + bounds.height).min(height).ceil() as u32;
        let mut value = 0;
        for y in y_start..y_end {
            for x in x_start..x_end {
                let index = ((y * self.size.width + x) * 4 + channel as u32) as usize;
                value = value.max(self.pixels[index]);
            }
        }
        value
    }

    /// The RGBA bytes in `bounds`, row by row, clipped to the image's edges.
    ///
    /// Two captures of the same widget can be compared region by region even
    /// when the renderer paints a label directly instead of through a text
    /// operation, as iced's closed `PickList` does.
    pub(crate) fn region(&self, bounds: Rectangle) -> Vec<u8> {
        let width = self.size.width as f32;
        let height = self.size.height as f32;
        let x_start = bounds.x.max(0.0).floor() as u32;
        let y_start = bounds.y.max(0.0).floor() as u32;
        let x_end = (bounds.x + bounds.width).min(width).ceil() as u32;
        let y_end = (bounds.y + bounds.height).min(height).ceil() as u32;
        let mut bytes = Vec::new();
        for y in y_start..y_end {
            for x in x_start..x_end {
                let index = ((y * self.size.width + x) * 4) as usize;
                bytes.extend_from_slice(&self.pixels[index..index + 4]);
            }
        }
        bytes
    }
}

impl Operation for CapturedText {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn text(&mut self, _id: Option<&Id>, bounds: Rectangle, text: &str) {
        self.0.push((text.to_string(), bounds));
    }
}

/// Lays `element` out under [`limits`], returning its state tree and resolved
/// layout node so a test can run one or more widget operations over the same
/// layout.
pub(crate) fn layout_tree<'a, Message, Theme, Renderer>(
    element: &mut Element<'a, Message, Theme, Renderer>,
    renderer: &Renderer,
) -> (Tree, Node)
where
    Renderer: iced::advanced::Renderer,
{
    let mut tree = Tree::new(element.as_widget());
    let node = element
        .as_widget_mut()
        .layout(&mut tree, renderer, &limits());
    (tree, node)
}

/// Applies `operation` to `element`'s already laid-out `tree` and `node`.
pub(crate) fn apply_operation<'a, Message, Theme, Renderer>(
    element: &mut Element<'a, Message, Theme, Renderer>,
    renderer: &Renderer,
    tree: &mut Tree,
    node: &Node,
    operation: &mut dyn Operation,
) where
    Renderer: iced::advanced::Renderer,
{
    element
        .as_widget_mut()
        .operate(tree, Layout::new(node), renderer, operation);
}

/// Lays `element` out under [`limits`] and applies `operation` to its tree.
pub(crate) fn operate<'a, Message, Theme, Renderer>(
    element: &mut Element<'a, Message, Theme, Renderer>,
    renderer: &Renderer,
    operation: &mut dyn Operation,
) where
    Renderer: iced::advanced::Renderer,
{
    let (mut tree, node) = layout_tree(element, renderer);
    apply_operation(element, renderer, &mut tree, &node, operation);
}

/// Lays `element` out under [`limits`] and records its draw operations,
/// returning the state tree, the resolved layout node, and the text fragments
/// plus root size. Both capture entry points start from this same tree, node,
/// and text capture; the pixel variant then also draws the laid-out tree.
fn layout_and_capture<'a, Message, Theme, Renderer>(
    element: &mut Element<'a, Message, Theme, Renderer>,
    renderer: &Renderer,
) -> (Tree, Node, TextCapture)
where
    Renderer: iced::advanced::Renderer,
{
    let (mut tree, node) = layout_tree(element, renderer);
    let size = node.size();
    let mut captured = CapturedText::default();
    apply_operation(element, renderer, &mut tree, &node, &mut captured);
    (
        tree,
        node,
        TextCapture {
            fragments: captured.0,
            size,
        },
    )
}

/// Lays `element` out under [`limits`] with `renderer` and returns the text
/// fragments it would draw plus its resolved root size.
pub(crate) fn capture_text<'a, Message, Theme, Renderer>(
    element: &mut Element<'a, Message, Theme, Renderer>,
    renderer: &Renderer,
) -> TextCapture
where
    Renderer: iced::advanced::Renderer,
{
    let (_tree, _node, capture) = layout_and_capture(element, renderer);
    capture
}

/// Builds a user interface for `element` under [`limits`] and dispatches
/// `events` with the cursor at `point`, returning the messages the tree
/// publishes.
///
/// The headless mouse-driven tests build the same `UserInterface`, wire the
/// same null clipboard, and collect the same message vector around each
/// event batch; this helper keeps that boilerplate in one place so a test
/// only names the events and where they land.
pub(crate) fn publish_at<'a, Message, Theme, Renderer>(
    element: Element<'a, Message, Theme, Renderer>,
    renderer: &mut Renderer,
    events: &[Event],
    point: Point,
) -> Vec<Message>
where
    Renderer: iced::advanced::Renderer,
{
    let mut user_interface = iced_runtime::user_interface::UserInterface::build(
        element,
        limits().max(),
        iced_runtime::user_interface::Cache::new(),
        renderer,
    );
    let mut messages = Vec::new();
    let mut clipboard = iced::advanced::clipboard::Null;
    let _ = user_interface.update(
        events,
        iced::mouse::Cursor::Available(point),
        renderer,
        &mut clipboard,
        &mut messages,
    );
    messages
}

/// Dispatches one left-button press and release at `point`, returning the
/// messages the tree publishes.
///
/// A control that acts on release needs both events, so this is the batch the
/// title bar, overlay, and shade-strip click tests dispatch at a control's
/// center. Tests that need a different batch — a press alone, a wheel, or two
/// clicks in one batch — call [`publish_at`] directly.
pub(crate) fn click_at<'a, Message, Theme, Renderer>(
    element: Element<'a, Message, Theme, Renderer>,
    renderer: &mut Renderer,
    point: Point,
) -> Vec<Message>
where
    Renderer: iced::advanced::Renderer,
{
    publish_at(
        element,
        renderer,
        &[
            Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
            Event::Mouse(iced::mouse::Event::ButtonReleased(
                iced::mouse::Button::Left,
            )),
        ],
        point,
    )
}

/// Lays `element` out and draws it with `renderer`, returning the text
/// fragments it would draw, the RGBA pixels it actually paints, and the root
/// size. A test can use the rendered pixels to check a label's color, which
/// the text operation alone does not carry.
pub(crate) fn capture_text_and_pixels<'a, Message, Theme, Renderer>(
    element: &mut Element<'a, Message, Theme, Renderer>,
    renderer: &mut Renderer,
    theme: &Theme,
) -> (TextCapture, PixelCapture)
where
    Renderer: iced::advanced::Renderer + iced::advanced::text::Renderer + Headless,
{
    let (tree, node, capture) = layout_and_capture(element, renderer);

    let style = iced::advanced::renderer::Style {
        text_color: iced::Color::BLACK,
    };
    let viewport = Rectangle::with_size(node.size());
    element.as_widget().draw(
        &tree,
        renderer,
        theme,
        &style,
        Layout::new(&node),
        iced::mouse::Cursor::Unavailable,
        &viewport,
    );

    let size = capture.size();
    let pixel_size = Size::new(size.width.ceil() as u32, size.height.ceil() as u32);
    let pixels = renderer.screenshot(pixel_size, 1.0, iced::Color::TRANSPARENT);
    (
        capture,
        PixelCapture {
            pixels,
            size: pixel_size,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::theme;

    /// The `headless_renderer` seam itself: the vendored pixel faces
    /// (`assets/fonts/`) load into the global font system the captures
    /// resolve texts through, so a capture can address either face, and the
    /// widths and colors match the probes the vendoring run recorded on the
    /// 8-px grid. A capture that resolves the faces to tofu or the system
    /// sans fails here instead of as a shifted number in a geometry test
    /// elsewhere.
    #[test]
    fn headless_captures_resolve_the_vendored_pixel_faces() {
        let renderer = headless_renderer();
        let mut word = text(
            iced::widget::Text::new("NOSTALGIA")
                .font(theme::ui_font())
                .size(16.0)
                .width(iced::Length::Shrink),
        );
        let word_capture = capture_text(&mut word, &renderer);

        let word_bounds = word_capture.find("NOSTALGIA");
        assert_eq!(word_bounds.width, 100.0);

        let mut glyph = centered_text(
            iced::widget::Text::new("▶")
                .font(theme::shape_font())
                .size(16.0),
        );
        let theme = iced::Theme::Dark;
        let mut renderer = headless_renderer();
        let (glyph_capture, pixels) = capture_text_and_pixels(&mut glyph, &mut renderer, &theme);

        let glyph_bounds = glyph_capture.find("▶");
        assert_eq!(glyph_bounds.width, 8.0);

        let whole = Rectangle::new(
            Point::ORIGIN,
            Size::new(pixels.size.width as f32, pixels.size.height as f32),
        );
        let colors = pixels
            .region(whole)
            .chunks(4)
            .map(|rgba| [rgba[0], rgba[1], rgba[2], rgba[3]])
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(colors.len(), 2, "glyph colors: {colors:?}");
    }

    /// The split's end-state proof: with no explicit font at all — the same
    /// default the app's `.default_font(theme::ui_font())` and this renderer's
    /// default seed — the text resolves to the letterform face, not the
    /// system sans, and draws at Silkscreen's probed advance. A future
    /// default-font regression fails here, not as a shifted number in a
    /// geometry test elsewhere.
    #[test]
    fn headless_captures_resolve_unlabeled_text_to_the_letterform_face() {
        let renderer = headless_renderer();
        let mut word = text(
            iced::widget::Text::new("NOSTALGIA")
                .size(16.0)
                .width(iced::Length::Shrink),
        );
        let word_capture = capture_text(&mut word, &renderer);

        let word_bounds = word_capture.find("NOSTALGIA");
        assert_eq!(word_bounds.width, 100.0);
    }

    /// A bare text shrunk to its measured face width.
    fn text(
        text: iced::widget::Text<'static>,
    ) -> Element<'static, (), iced::Theme, iced::Renderer> {
        text.into()
    }

    /// A text centered in a fixed 32x32 square, so the capture's pixels span
    /// one glyph and its margins.
    fn centered_text(
        text: iced::widget::Text<'static>,
    ) -> Element<'static, (), iced::Theme, iced::Renderer> {
        iced::widget::container(text)
            .width(iced::Length::Fixed(32.0))
            .height(iced::Length::Fixed(32.0))
            .into()
    }

    fn two_label_column() -> Element<'static, (), iced::Theme, iced::Renderer> {
        iced::widget::container(
            iced::widget::Column::new()
                .push(iced::widget::Text::new("alpha"))
                .push(iced::widget::Text::new("beta")),
        )
        .width(iced::Length::Fixed(200.0))
        .height(iced::Length::Fixed(100.0))
        .into()
    }

    // The harness itself: a known fixed-size column must report both labels in
    // draw order, a root size equal to the fixed container, and bounds that
    // place the second label below the first. A broken capture would fail
    // here directly instead of as a confusing geometry assertion elsewhere.
    #[test]
    fn capture_text_returns_fragments_with_bounds_and_the_root_size() {
        let renderer = headless_renderer();
        let mut element = two_label_column();
        let capture = capture_text(&mut element, &renderer);

        assert_eq!(capture.texts().collect::<Vec<_>>(), vec!["alpha", "beta"]);
        assert_eq!(capture.size(), Size::new(200.0, 100.0));

        let alpha = capture.find("alpha");
        let beta = capture.find("beta");
        assert!(
            beta.y >= alpha.y + alpha.height,
            "beta must sit below alpha: {alpha:?}, {beta:?}"
        );
    }

    // A missing label names every fragment the tree did draw, so a failing
    // geometry test points at what the tree actually rendered.
    #[test]
    #[should_panic(expected = "alpha")]
    fn find_lists_the_captured_fragments_when_a_label_is_missing() {
        let renderer = headless_renderer();
        let mut element = two_label_column();
        let capture = capture_text(&mut element, &renderer);

        capture.find("missing");
    }

    /// A 4x2 image whose green channel is `10 * (y * 4 + x + 1)`.
    fn gradient_pixels() -> PixelCapture {
        let mut pixels = Vec::new();
        for n in 0..8u8 {
            pixels.extend_from_slice(&[255, 10 * (n + 1), 255, 255]);
        }
        PixelCapture {
            pixels,
            size: Size::new(4, 2),
        }
    }

    /// A 4x2 image whose red channel is `10 * (y * 4 + x + 1)`.
    fn red_gradient_pixels() -> PixelCapture {
        let mut pixels = Vec::new();
        for n in 0..8u8 {
            pixels.extend_from_slice(&[10 * (n + 1), 255, 255, 255]);
        }
        PixelCapture {
            pixels,
            size: Size::new(4, 2),
        }
    }

    /// A 4x2 image whose blue channel is `10 * (y * 4 + x + 1)`.
    fn blue_gradient_pixels() -> PixelCapture {
        let mut pixels = Vec::new();
        for n in 0..8u8 {
            pixels.extend_from_slice(&[255, 255, 10 * (n + 1), 255]);
        }
        PixelCapture {
            pixels,
            size: Size::new(4, 2),
        }
    }

    // Only the blue channel inside the bounds counts, clipped to the image.
    #[test]
    fn max_blue_in_reads_only_the_blue_of_pixels_inside_the_bounds() {
        let capture = blue_gradient_pixels();

        assert_eq!(
            capture.max_blue_in(Rectangle::new(Point::new(0.0, 0.0), Size::new(2.0, 1.0))),
            20
        );
        assert_eq!(
            capture.max_blue_in(Rectangle::new(
                Point::new(-5.0, -5.0),
                Size::new(50.0, 50.0)
            )),
            80
        );
    }

    // Only the green channel inside the bounds counts: the maximum of the
    // covered pixels, not of the whole image.
    #[test]
    fn max_green_in_reads_only_the_green_of_pixels_inside_the_bounds() {
        let capture = gradient_pixels();

        assert_eq!(
            capture.max_green_in(Rectangle::new(Point::new(0.0, 0.0), Size::new(2.0, 1.0))),
            20
        );
        assert_eq!(
            capture.max_green_in(Rectangle::new(Point::new(1.0, 1.0), Size::new(2.0, 1.0))),
            70
        );
    }

    // Bounds hanging off any edge of the image are clipped, not indexed out
    // of range.
    #[test]
    fn max_green_in_clips_bounds_that_overhang_the_image() {
        let capture = gradient_pixels();

        let whole = Rectangle::new(Point::new(-5.0, -5.0), Size::new(50.0, 50.0));
        assert_eq!(capture.max_green_in(whole), 80);
    }

    // Bounds wholly outside the image cover no pixels, so there is no green.
    #[test]
    fn max_green_in_is_zero_for_bounds_outside_the_image() {
        let capture = gradient_pixels();

        let right = Rectangle::new(Point::new(10.0, 0.0), Size::new(3.0, 1.0));
        assert_eq!(capture.max_green_in(right), 0);
    }

    // The red channel gets its own maximum so a fill's hue can be told apart
    // from its brightness; the same bounds clipping applies to it.
    #[test]
    fn max_red_in_reads_only_the_red_of_pixels_inside_the_bounds() {
        let capture = red_gradient_pixels();

        assert_eq!(
            capture.max_red_in(Rectangle::new(Point::new(0.0, 0.0), Size::new(2.0, 1.0))),
            20
        );
        assert_eq!(
            capture.max_red_in(Rectangle::new(
                Point::new(-5.0, -5.0),
                Size::new(50.0, 50.0)
            )),
            80
        );
    }

    // A region reads its RGBA bytes row by row and clips bounds that overhang
    // an edge, so two captures can be compared without indexing out of range.
    #[test]
    fn region_reads_the_covered_pixels_in_row_order_and_clips() {
        let capture = gradient_pixels();
        let left = capture.region(Rectangle::new(Point::new(0.0, 0.0), Size::new(2.0, 1.0)));
        assert_eq!(left, vec![255, 10, 255, 255, 255, 20, 255, 255]);

        let whole = capture.region(Rectangle::new(
            Point::new(-5.0, -5.0),
            Size::new(50.0, 50.0),
        ));
        assert_eq!(whole.len(), 8 * 4);
        assert_eq!(&whole[..4], &[255, 10, 255, 255]);
    }

    fn button() -> Element<'static, &'static str, iced::Theme, iced::Renderer> {
        iced::widget::button(iced::widget::Text::new("go"))
            .on_press("pressed")
            .width(iced::Length::Fixed(100.0))
            .height(iced::Length::Fixed(40.0))
            .into()
    }

    // The click helper's own contract: a press and release over a button
    // publish its message once, and the same batch outside the button's
    // bounds publishes nothing, so the mouse tests that rely on it fail for
    // the control and not for the harness.
    #[test]
    fn click_at_publishes_inside_the_control_and_not_outside() {
        let mut renderer = headless_renderer();
        let inside = click_at(button(), &mut renderer, Point::new(50.0, 20.0));
        assert_eq!(inside, vec!["pressed"]);

        let outside = click_at(button(), &mut renderer, Point::new(150.0, 20.0));
        assert!(outside.is_empty(), "outside click published {outside:?}");
    }

    // `publish_at` dispatches exactly the events it is given: a button acts
    // on release, so a press alone publishes nothing.
    #[test]
    fn publish_at_dispatches_only_the_given_events() {
        let mut renderer = headless_renderer();
        let press = Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left));
        let messages = publish_at(button(), &mut renderer, &[press], Point::new(50.0, 20.0));
        assert!(messages.is_empty(), "a press alone published {messages:?}");
    }
}
