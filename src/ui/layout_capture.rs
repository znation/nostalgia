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
use iced::{Element, Event, Font, Pixels, Point, Rectangle, Size};

/// The bounds every captured tree is laid out in.
pub(crate) fn limits() -> Limits {
    Limits::new(Size::ZERO, Size::new(1000.0, 1000.0))
}

/// Builds iced's headless tiny-skia renderer.
///
/// tiny-skia measures text in software, so a tree containing the fallback
/// renderer's bevel widgets still lays out without a window or GPU.
pub(crate) fn headless_renderer() -> iced::Renderer {
    futures::executor::block_on(<iced::Renderer as Headless>::new(
        Font::default(),
        Pixels(16.0),
        Some("tiny-skia"),
    ))
    .expect("the headless tiny-skia renderer")
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
}

impl Operation for CapturedText {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn text(&mut self, _id: Option<&Id>, bounds: Rectangle, text: &str) {
        self.0.push((text.to_string(), bounds));
    }
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
    let mut tree = Tree::new(element.as_widget());
    let node = element
        .as_widget_mut()
        .layout(&mut tree, renderer, &limits());
    let size = node.size();
    let mut captured = CapturedText::default();
    element
        .as_widget_mut()
        .operate(&mut tree, Layout::new(&node), renderer, &mut captured);
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
}
