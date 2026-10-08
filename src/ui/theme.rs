//! The Winamp 2.x base-skin palette and the custom [`iced::Theme`] built from
//! it.
//!
//! The player's widgets render in iced's default theme unless one is applied
//! app-wide, so this module names the base-skin colours once and exposes them
//! as a single [`winamp_theme`] the `init_ui` builder installs. Keeping the
//! colours as public constants — rather than inlining them in each view —
//! lets later fidelity work (title bar, panel bevels, playlist chrome) style
//! against the same names.

use std::sync::OnceLock;

use iced::{Color, Theme};

/// The dark gray window face every panel is drawn on.
pub const WINDOW_BACKGROUND: Color = Color::from_rgb(0.18, 0.18, 0.18);

/// The light chrome text colour used for captions and labels.
pub const TEXT: Color = Color::from_rgb(0.87, 0.87, 0.87);

/// Winamp's title-bar and selection blue.
pub const TITLE_BLUE: Color = Color::from_rgb(0.0, 0.0, 0.55);

/// The playlist/LCD green, used for the Now Playing title.
pub const LCD_GREEN: Color = Color::from_rgb(0.0, 1.0, 0.0);

/// The highlight colour behind the currently playing row in a Songs list,
/// echoing Winamp's playlist selection bar (a saturated blue).
pub const PLAYING_ROW_HIGHLIGHT: Color = Color::from_rgb(0.25, 0.5, 1.0);

/// The light top/left edge of a raised panel bevel, and the dark bottom/right
/// edge of a sunken one.
pub const PANEL_EDGE_LIGHT: Color = Color::from_rgb(0.55, 0.55, 0.55);

/// The dark bottom/right edge of a raised panel bevel, and the light top/left
/// edge of a sunken one.
pub const PANEL_EDGE_DARK: Color = Color::from_rgb(0.05, 0.05, 0.05);

/// The raised chrome button face, lighter than [`WINDOW_BACKGROUND`] so the
/// button reads as raised off the window face.
pub const BUTTON_FACE: Color = Color::from_rgb(0.30, 0.30, 0.30);

/// The button face while hovered.
pub const BUTTON_FACE_HOVERED: Color = Color::from_rgb(0.38, 0.38, 0.38);

/// The pressed (sunken) button face, darker than the resting face.
pub const BUTTON_FACE_PRESSED: Color = Color::from_rgb(0.22, 0.22, 0.22);

/// The near-black recess behind the green Now Playing title.
pub const LCD_BACKGROUND: Color = Color::from_rgb(0.05, 0.05, 0.05);

/// The base-skin colours as an iced [`Palette`](iced::theme::Palette): the
/// dark window face and light text are the background/foreground pair,
/// [`TITLE_BLUE`] drives interactive accents, and [`LCD_GREEN`] is the
/// success colour. Warning and danger keep conventional amber/red so a
/// future error state reads correctly against the dark face.
fn palette() -> iced::theme::Palette {
    iced::theme::Palette {
        background: WINDOW_BACKGROUND,
        text: TEXT,
        primary: TITLE_BLUE,
        success: LCD_GREEN,
        warning: Color::from_rgb(1.0, 0.65, 0.0),
        danger: Color::from_rgb(1.0, 0.2, 0.2),
    }
}

/// The process-wide cache behind [`winamp_theme`], so the theme is built at
/// most once.
static WINAMP_THEME: OnceLock<Theme> = OnceLock::new();

/// Returns the shared Winamp theme, building it at most once.
///
/// iced calls the app's theme function on every UI rebuild — every message,
/// including each slider tick — and `Theme::custom` allocates its `Arc` and
/// generates the full extended palette each time it runs. Caching the theme
/// here turns that per-rebuild construction into an `Arc` clone; the palette
/// never changes, so every rebuild can share one instance.
fn cached_theme() -> &'static Theme {
    WINAMP_THEME.get_or_init(|| Theme::custom("Winamp", palette()))
}

/// The app-wide Winamp theme: a custom [`Theme`] named `"Winamp"` over the
/// base-skin `palette`. Applied by `init_ui` so every widget that follows
/// the theme renders on the dark Winamp face. A cheap clone of the cached
/// [`cached_theme`], so the per-rebuild call allocates nothing new.
pub fn winamp_theme() -> Theme {
    cached_theme().clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palette_uses_the_named_base_skin_colours() {
        let palette = winamp_theme().palette();
        assert_eq!(palette.background, WINDOW_BACKGROUND);
        assert_eq!(palette.text, TEXT);
        assert_eq!(palette.primary, TITLE_BLUE);
        assert_eq!(palette.success, LCD_GREEN);
    }

    // The named-colour test above pins four of the palette's six slots; the
    // remaining two, `warning` and `danger`, are inline literals rather than
    // named constants, so nothing read them and an accidental swap or edit
    // would clear the whole suite while iced's extended palette rendered the
    // wrong amber/red ramp for a future error state. Pin the documented
    // conventional values, and that the two stay distinct from each other and
    // from the success green.
    #[test]
    fn palette_keeps_conventional_warning_and_danger_colours() {
        let palette = winamp_theme().palette();
        assert_eq!(palette.warning, Color::from_rgb(1.0, 0.65, 0.0));
        assert_eq!(palette.danger, Color::from_rgb(1.0, 0.2, 0.2));
        assert_ne!(palette.warning, palette.danger);
        assert_ne!(palette.warning, palette.success);
        assert_ne!(palette.danger, palette.success);
    }

    // The palette tests above read the six palette slots. The playing-row
    // highlight and the LCD well's recess are the two base-skin colours
    // painted from constants no test reads at all: `views::scrollable_list`
    // and `style::lcd_well` apply them inside inline style closures, which
    // iced's `Element` API exposes no way to introspect. Their RGB values are
    // the skin contract — a regression could turn the playlist selection bar
    // or the recess into any other colour and clear the whole suite. Pin
    // both, and that they keep their documented roles: the well is darker
    // than the window face it recesses into, and the highlight stands apart
    // from both.
    #[test]
    fn panel_recess_and_playing_row_highlight_keep_their_base_skin_colours() {
        assert_eq!(LCD_BACKGROUND, Color::from_rgb(0.05, 0.05, 0.05));
        assert_eq!(PLAYING_ROW_HIGHLIGHT, Color::from_rgb(0.25, 0.5, 1.0));

        // The recess reads as sunken only if it is darker than the face it
        // sits in. Bound to locals so the comparison is a runtime check rather
        // than the constant assertion clippy rejects.
        let recess = LCD_BACKGROUND;
        let face = WINDOW_BACKGROUND;
        assert!(recess.r < face.r);
        assert!(recess.g < face.g);
        assert!(recess.b < face.b);

        // The selection bar must stand out from the window face and the well.
        assert_ne!(PLAYING_ROW_HIGHLIGHT, WINDOW_BACKGROUND);
        assert_ne!(PLAYING_ROW_HIGHLIGHT, LCD_BACKGROUND);
    }

    // The tests above read each palette slot against the named constant that
    // feeds it, so a constant and its reader can drift together and clear the
    // suite. The base-skin RGB values are this project's visual contract — a
    // Winamp 2.x clone lives or dies on its palette — and the remaining named
    // colours are read only constant-to-constant (or by inline widget
    // closures iced exposes no way to introspect). Pin every literal here,
    // plus the bevel and button-face orderings their doc comments promise, so
    // an accidental recolour is caught rather than shipped.
    #[test]
    fn base_skin_colours_keep_their_documented_literal_values() {
        assert_eq!(WINDOW_BACKGROUND, Color::from_rgb(0.18, 0.18, 0.18));
        assert_eq!(TEXT, Color::from_rgb(0.87, 0.87, 0.87));
        assert_eq!(TITLE_BLUE, Color::from_rgb(0.0, 0.0, 0.55));
        assert_eq!(LCD_GREEN, Color::from_rgb(0.0, 1.0, 0.0));
        assert_eq!(PANEL_EDGE_LIGHT, Color::from_rgb(0.55, 0.55, 0.55));
        assert_eq!(PANEL_EDGE_DARK, Color::from_rgb(0.05, 0.05, 0.05));
        assert_eq!(BUTTON_FACE, Color::from_rgb(0.30, 0.30, 0.30));
        assert_eq!(BUTTON_FACE_HOVERED, Color::from_rgb(0.38, 0.38, 0.38));
        assert_eq!(BUTTON_FACE_PRESSED, Color::from_rgb(0.22, 0.22, 0.22));

        // The bevel's two edges must stay distinct, with the light edge
        // actually lighter, or a raised panel reads flat and a sunken one
        // disappears. Bound to locals so each comparison is a runtime check
        // rather than the constant assertion clippy rejects.
        let (light, dark) = (PANEL_EDGE_LIGHT, PANEL_EDGE_DARK);
        assert!(light.r > dark.r);

        // The button face must sit above the window face and darken on hover,
        // then sink below the resting face while pressed — the doc comments'
        // promised ordering, which drives whether a button reads as raised.
        let (face, hovered, pressed) = (BUTTON_FACE, BUTTON_FACE_HOVERED, BUTTON_FACE_PRESSED);
        let window = WINDOW_BACKGROUND;
        assert!(face.r > window.r);
        assert!(hovered.r > face.r);
        assert!(pressed.r < face.r);
    }

    #[test]
    fn theme_reads_as_dark() {
        // The dark face is what makes the light chrome text legible; the
        // generated extended palette classifies the theme accordingly.
        assert!(winamp_theme().extended_palette().is_dark);
    }

    #[test]
    fn theme_is_named_winamp() {
        assert_eq!(winamp_theme().to_string(), "Winamp");
    }

    // `winamp_theme` is cached in a `OnceLock` so the app's per-rebuild theme
    // call clones one shared `Arc` instead of constructing a fresh
    // `Theme::custom` (an allocation plus extended-palette generation) each
    // time. Pointer identity across calls is the observable guarantee of that
    // caching.
    #[test]
    fn winamp_theme_is_cached_as_a_singleton() {
        assert!(std::ptr::eq(cached_theme(), cached_theme()));
    }

    // The singleton test above pins `cached_theme`'s address, which the
    // `OnceLock` guarantees on its own. The public contract is that
    // `winamp_theme` *uses* that cache: iced calls it on every UI rebuild, so
    // it must return a clone of the one cached `Theme::Custom` rather than
    // constructing a fresh one. A regression inlining
    // `Theme::custom("Winamp", palette())` into `winamp_theme` would leave
    // `cached_theme`'s identity intact and pass every existing test while
    // rebuilding the theme (an allocation plus extended-palette generation)
    // on each frame. The inner `Arc` is the shared allocation, so pointer
    // identity across two public calls is the observable guarantee.
    #[test]
    fn winamp_theme_returns_clones_of_the_cached_theme() {
        let first = winamp_theme();
        let second = winamp_theme();
        match (&first, &second) {
            (Theme::Custom(first), Theme::Custom(second)) => {
                assert!(std::sync::Arc::ptr_eq(first, second));
            }
            _ => panic!("the Winamp theme must be a custom theme"),
        }
    }
}
