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
    layout::Limits,
    renderer::Headless,
    widget::{Id, Tree, operation::Operation},
};
use iced::{Element, Font, Pixels, Rectangle, Size};

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

impl Operation for CapturedText {
    fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
        operate(self);
    }

    fn text(&mut self, _id: Option<&Id>, bounds: Rectangle, text: &str) {
        self.0.push((text.to_string(), bounds));
    }
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
    let mut tree = Tree::new(element.as_widget());
    let node = element
        .as_widget_mut()
        .layout(&mut tree, renderer, &limits());
    let size = node.size();
    let mut captured = CapturedText::default();
    element
        .as_widget_mut()
        .operate(&mut tree, Layout::new(&node), renderer, &mut captured);
    TextCapture {
        fragments: captured.0,
        size,
    }
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
}
