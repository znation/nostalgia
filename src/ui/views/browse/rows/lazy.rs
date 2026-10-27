//! A cached row element: rebuilds its contents only when its dependency
//! changes.
//!
//! The browse list is the heaviest part of the view tree, and iced rebuilds
//! the whole screen on every message — including the position and spectrum
//! polls while a preview plays. A row's widgets depend only on the list's
//! epoch, its position, whether it is playing, whether it is selected, and its
//! text; a poll reply changes none of those, so rebuilding every row for it
//! throws away identical widgets. [`LazyRow`] keeps the built element in the
//! widget tree and reuses it while its dependency hashes the same.
//!
//! This is a small non-overlay copy of iced's `lazy` widget, which the crate
//! does not enable. A browse row's element is a button with text, so it never
//! opens an overlay; the `overlay` method returns `None` and documents that
//! restriction rather than carrying iced's self-referencing overlay support.

use std::cell::RefCell;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use iced::advanced::{
    Clipboard, Layout, Shell, Widget, layout, mouse, overlay, renderer,
    widget::tree::{self, Tree},
};
use iced::{Element, Event, Length, Rectangle, Size, Vector};

/// The shared cell holding the built element: the widget and its tree state
/// both point at it so a reused row can hand its element to the next view.
type CachedElement<Message, Theme, Renderer> =
    Rc<RefCell<Option<Element<'static, Message, Theme, Renderer>>>>;

/// A widget that builds its `view` from `dependency` once and reuses the
/// built element until the dependency's hash changes.
pub(in crate::ui::views) struct LazyRow<'a, Message, Theme, Renderer, Dependency, View>
where
    Renderer: iced::advanced::Renderer,
{
    dependency: Dependency,
    view: Box<dyn Fn(&Dependency) -> View + 'a>,
    element: RefCell<Option<CachedElement<Message, Theme, Renderer>>>,
}

impl<'a, Message, Theme, Renderer, Dependency, View>
    LazyRow<'a, Message, Theme, Renderer, Dependency, View>
where
    Dependency: Hash + 'a,
    View: Into<Element<'static, Message, Theme, Renderer>>,
    Renderer: iced::advanced::Renderer,
{
    /// The size a row reports before its element is built: a browse row
    /// button is full-width and content-height.
    const UNBUILT_SIZE: Size<Length> = Size {
        width: Length::Fill,
        height: Length::Shrink,
    };

    /// Creates a row whose element is `view(&dependency)`, rebuilt only when
    /// the dependency's hash changes.
    pub(in crate::ui::views) fn new(
        dependency: Dependency,
        view: impl Fn(&Dependency) -> View + 'a,
    ) -> Self {
        Self {
            dependency,
            view: Box::new(view),
            element: RefCell::new(None),
        }
    }

    fn with_element<T>(
        &self,
        f: impl FnOnce(&Element<'_, Message, Theme, Renderer>) -> T,
    ) -> Option<T> {
        let element = self.element.borrow();
        let cell = element.as_ref()?;
        let element = cell.borrow();
        let element = element.as_ref()?;
        Some(f(element))
    }

    fn with_element_mut<T>(
        &self,
        f: impl FnOnce(&mut Element<'_, Message, Theme, Renderer>) -> T,
    ) -> Option<T> {
        let element = self.element.borrow();
        let cell = element.as_ref()?;
        let mut element = cell.borrow_mut();
        let element = element.as_mut()?;
        Some(f(element))
    }

    fn hash(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.dependency.hash(&mut hasher);
        hasher.finish()
    }
}

/// The element and dependency hash kept in the widget tree between views.
struct Internal<Message, Theme, Renderer> {
    element: CachedElement<Message, Theme, Renderer>,
    hash: u64,
}

impl<'a, Message, Theme, Renderer, Dependency, View> Widget<Message, Theme, Renderer>
    for LazyRow<'a, Message, Theme, Renderer, Dependency, View>
where
    View: Into<Element<'static, Message, Theme, Renderer>> + 'static,
    Dependency: Hash + 'a,
    Message: 'static,
    Theme: 'static,
    Renderer: iced::advanced::Renderer + 'static,
{
    fn tag(&self) -> tree::Tag {
        struct Tag<T>(T);
        tree::Tag::of::<Tag<View>>()
    }

    fn state(&self) -> tree::State {
        let element = Rc::new(RefCell::new(Some((self.view)(&self.dependency).into())));
        *self.element.borrow_mut() = Some(element.clone());
        tree::State::new(Internal {
            element,
            hash: self.hash(),
        })
    }

    fn children(&self) -> Vec<Tree> {
        self.with_element(|element| vec![Tree::new(element.as_widget())])
            .unwrap_or_default()
    }

    fn diff(&self, tree: &mut Tree) {
        let current = tree
            .state
            .downcast_mut::<Internal<Message, Theme, Renderer>>();
        let hash = self.hash();
        if current.hash == hash {
            *self.element.borrow_mut() = Some(current.element.clone());
            return;
        }
        current.hash = hash;
        let element = Rc::new(RefCell::new(Some((self.view)(&self.dependency).into())));
        current.element = element.clone();
        *self.element.borrow_mut() = Some(element);
        let _ = self.with_element(|element| {
            tree.diff_children(std::slice::from_ref(&element.as_widget()));
        });
    }

    fn size(&self) -> Size<Length> {
        self.with_element(|element| element.as_widget().size())
            .unwrap_or(Self::UNBUILT_SIZE)
    }

    fn size_hint(&self) -> Size<Length> {
        // `Column::push` asks a child's hint before any tree exists, so this
        // must name the row button's own size instead of waiting for the
        // element `state` builds. A row button is full-width and
        // content-height.
        self.with_element(|element| element.as_widget().size_hint())
            .unwrap_or(Self::UNBUILT_SIZE)
    }

    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.with_element_mut(|element| {
            element
                .as_widget_mut()
                .layout(&mut tree.children[0], renderer, limits)
        })
        .unwrap_or_else(|| layout::Node::new(Size::ZERO))
    }

    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &Renderer,
        operation: &mut dyn iced::advanced::widget::Operation,
    ) {
        let _ = self.with_element_mut(|element| {
            element
                .as_widget_mut()
                .operate(&mut tree.children[0], layout, renderer, operation);
        });
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let _ = self.with_element_mut(|element| {
            element.as_widget_mut().update(
                &mut tree.children[0],
                event,
                layout,
                cursor,
                renderer,
                clipboard,
                shell,
                viewport,
            );
        });
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.with_element(|element| {
            element.as_widget().mouse_interaction(
                &tree.children[0],
                layout,
                cursor,
                viewport,
                renderer,
            )
        })
        .unwrap_or_default()
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let _ = self.with_element(|element| {
            element.as_widget().draw(
                &tree.children[0],
                renderer,
                theme,
                style,
                layout,
                cursor,
                viewport,
            );
        });
    }

    fn overlay<'b>(
        &'b mut self,
        _tree: &'b mut Tree,
        _layout: Layout<'b>,
        _renderer: &Renderer,
        _viewport: &Rectangle,
        _translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        // A browse row is a button with text and never opens an overlay; a
        // cached element that did would need iced's `lazy` widget instead.
        None
    }
}

impl<'a, Message, Theme, Renderer, Dependency, View>
    From<LazyRow<'a, Message, Theme, Renderer, Dependency, View>>
    for Element<'a, Message, Theme, Renderer>
where
    View: Into<Element<'static, Message, Theme, Renderer>> + 'static,
    Message: 'static,
    Theme: 'static,
    Dependency: Hash + 'a,
    Renderer: iced::advanced::Renderer + 'static,
{
    fn from(lazy: LazyRow<'a, Message, Theme, Renderer, Dependency, View>) -> Self {
        Self::new(lazy)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use iced::advanced::Widget;

    use super::*;
    use crate::ui::Message;

    /// A row whose element build count is observable, so the test can prove a
    /// diff with an unchanged dependency reuses the element.
    fn row(
        builds: Rc<Cell<usize>>,
        dependency: u64,
    ) -> LazyRow<'static, Message, iced::Theme, iced::Renderer, u64, Element<'static, Message>>
    {
        LazyRow::new(dependency, move |_| {
            builds.set(builds.get() + 1);
            iced::widget::Text::new("row").into()
        })
    }

    #[test]
    fn reuses_the_element_until_the_dependency_changes() {
        let builds = Rc::new(Cell::new(0));
        let first = row(Rc::clone(&builds), 1);
        let mut tree = Tree::new(&first as &dyn Widget<Message, iced::Theme, iced::Renderer>);
        assert_eq!(builds.get(), 1, "the first view builds the row once");

        let same = row(Rc::clone(&builds), 1);
        tree.diff(&same as &dyn Widget<Message, iced::Theme, iced::Renderer>);
        assert_eq!(builds.get(), 1, "an unchanged dependency reuses the row");

        let changed = row(Rc::clone(&builds), 2);
        tree.diff(&changed as &dyn Widget<Message, iced::Theme, iced::Renderer>);
        assert_eq!(builds.get(), 2, "a changed dependency rebuilds the row");
    }

    // `Column::push` asks for a child's hint before any tree exists, so the
    // unbuilt fallback must report a full-width, content-height size. A
    // shrink-width fallback would collapse the list's column before its rows
    // are ever built.
    #[test]
    fn an_unbuilt_row_reports_a_full_width_content_height_hint() {
        let builds = Rc::new(Cell::new(0));
        let lazy = row(Rc::clone(&builds), 1);

        assert_eq!(
            lazy.size_hint(),
            Size {
                width: Length::Fill,
                height: Length::Shrink,
            }
        );
        assert_eq!(
            builds.get(),
            0,
            "asking for the hint must not build the row"
        );
    }
}
