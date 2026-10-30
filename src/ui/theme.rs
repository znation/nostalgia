//! The Winamp 2.x base-skin palette and the custom [`iced::Theme`] built from
//! it.
//!
//! The player's widgets render in iced's default theme unless one is applied
//! app-wide, so this module names the base-skin colors once and exposes them
//! as a single [`winamp_theme`] the `init_ui` builder installs. Keeping the
//! colors as public constants — rather than inlining them in each view —
//! lets the views and chrome styles share one named palette.

use std::sync::OnceLock;

use iced::{Color, Font, Theme};

/// The steel-blue window face every panel is drawn on, sampled from the
/// mockups' chrome face (`docs/screenshots/player.png` reads (53,53,84) at
/// (5,20)), which matches the base-skin capture's hue (`winamp-2.x-stack.png`
/// reads (24,24,41) on the face) at the mockup's flat-render brightness.
pub const WINDOW_BACKGROUND: Color = Color::from_rgb8(53, 53, 84);

/// The light chrome text color used for captions and labels.
pub const TEXT: Color = Color::from_rgb(0.87, 0.87, 0.87);

/// Winamp's title-bar and selection blue.
pub const TITLE_BLUE: Color = Color::from_rgb(0.0, 0.0, 0.55);

/// The title tape's bright gold face, sampled from the mockups' tape strips
/// that flank each title band's label.
pub const TITLE_TAPE: Color = Color::from_rgb(0.93, 0.81, 0.48);

/// The title tape's darker bottom edge, the shadow that makes the gold face
/// read as a raised strip.
pub const TITLE_TAPE_SHADOW: Color = Color::from_rgb(0.64, 0.58, 0.42);

/// The playlist/LCD green, used for the Now Playing title.
pub const LCD_GREEN: Color = Color::from_rgb(0.0, 1.0, 0.0);

/// The unlit LCD indicator color: a dark green for the channel label that is
/// not the loaded preview's reported mode.
pub const LCD_GREEN_DIM: Color = Color::from_rgb(0.0, 0.35, 0.0);

/// The stopped-transport glyph color on the Now Playing LCD, a red square
/// that tells Stop apart from the green playing and paused glyphs.
pub const LCD_RED: Color = Color::from_rgb(1.0, 0.1, 0.1);

/// The highlight color behind the selected row in a browse list, echoing
/// Winamp's playlist selection bar (a saturated blue).
pub const ROW_HIGHLIGHT: Color = Color::from_rgb(0.25, 0.5, 1.0);

/// The white text of the playing browse row, which stands out from the green
/// text of an ordinary row without a selection bar.
pub const PLAYING_ROW_TEXT: Color = Color::from_rgb(1.0, 1.0, 1.0);

/// The light top/left edge of a raised panel bevel, and the dark bottom/right
/// edge of a sunken one: the mockups' blue-tinted bevel highlight
/// (`docs/screenshots/player.png` reads (98,98,118) on its window-edge
/// bevel), which matches the base-skin capture's bevel hue
/// (`winamp-2.x-stack.png` reads (140,156,181) at (271,13)) at the mockup's
/// flat-render brightness.
pub const PANEL_EDGE_LIGHT: Color = Color::from_rgb8(98, 98, 118);

/// The dark bottom/right edge of a raised panel bevel, and the light top/left
/// edge of a sunken one: the base-skin capture's blue-tinted dark bevel
/// (`winamp-2.x-stack.png` reads (49,57,74) at (140,113), the main window's
/// bottom edge).
pub const PANEL_EDGE_DARK: Color = Color::from_rgb8(49, 57, 74);

/// The near-black ring the base skin draws on every window edge *outside*
/// the light bevel (`winamp-2.x-stack.png` reads (0,0,8) on the outline,
/// `docs/screenshots/player.png` (20,20,31) at the same mockup edge).
pub const WINDOW_EDGE_OUTLINE: Color = Color::from_rgb8(20, 20, 31);

/// The raised chrome button face: the mockups' light metal
/// (`docs/screenshots/player.png` reads (185,196,209) on a resting transport
/// button), matching the base skin's light metal faces
/// (`winamp-2.x-stack.png` reads (189,198,198) and (173,181,181) on its
/// transport row) at the mockup's flat-render brightness.
pub const BUTTON_FACE: Color = Color::from_rgb8(185, 196, 209);

/// The button face while hovered, a slightly lighter lift of
/// [`BUTTON_FACE`].
pub const BUTTON_FACE_HOVERED: Color = Color::from_rgb8(195, 206, 219);

/// The pressed (sunken) button face, darker than the resting face.
pub const BUTTON_FACE_PRESSED: Color = Color::from_rgb8(167, 178, 191);

/// The glyph color engraved into the light metal button faces: the mockups'
/// dark navy text (`docs/screenshots/player.png` reads (57,57,90) on a
/// transport button's glyph), dark against [`BUTTON_FACE`] as the base
/// skin's near-black engraved glyphs read against their light metal
/// (`winamp-2.x-stack.png` (16..40, 88..103)).
pub const BUTTON_TEXT: Color = Color::from_rgb8(57, 57, 90);

/// The near-white top-left bevel highlight of a light metal button
/// (`docs/screenshots/player.png` reads (239,255,255) on a resting transport
/// button's top edge). The window's own [`PANEL_EDGE_LIGHT`] bevel — a
/// mid-value blue-gray — disappears into the metal face, so light metal
/// chrome takes this highlight instead.
pub const BUTTON_EDGE_LIGHT: Color = Color::from_rgb8(239, 255, 255);

/// The transport row's Repeat and Shuffle buttons carry a small square LED on
/// their light metal face. This is its fill while the flag it marks is on: the
/// mockups' lit LED (`docs/screenshots/transport.png` reads (0,232,0) inside
/// the repeat button's LED in its Repeat-on panel, where the shuffle LED
/// beside it stays dark), a dimmer stop than the pure [`LCD_GREEN`] the LCD
/// readouts use, so a lit LED cannot read as an ordinary green readout.
pub const LED_ON: Color = Color::from_rgb8(0, 232, 0);

/// The square LED on the transport row's Repeat and Shuffle faces while the
/// flag it marks is off: the mockups' unlit LED
/// (`docs/screenshots/player.png` reads (16,20,15) inside the resting
/// Shuffle button's LED), near-black against the [`BUTTON_FACE`] metal.
pub const LED_OFF: Color = Color::from_rgb8(16, 20, 15);

/// The near-black recess behind the green Now Playing title.
pub const LCD_BACKGROUND: Color = Color::from_rgb(0.05, 0.05, 0.05);

/// The transport and shade volume sliders' groove fill: the mockup's orange
/// rail, distinct from the neutral [`LCD_BACKGROUND`] groove of the seek bar
/// and equalizer preamp.
pub const VOLUME_SLIDER_FILL: Color = Color::from_rgb(0.88, 0.49, 0.15);

/// The transport balance slider's groove fill: the mockup's green rail.
pub const BALANCE_SLIDER_FILL: Color = Color::from_rgb(0.09, 0.57, 0.04);

/// The equalizer band tint at [`GAIN_MIN_DB`](crate::equalizer::GAIN_MIN_DB):
/// a full cut, matching the LCD green's "quiet" reading.
pub const EQ_GAIN_LOW: Color = LCD_GREEN;

/// The equalizer band tint at `0 dB`, the flat gain: yellow, so a neutral
/// band reads neither boosted nor cut.
pub const EQ_GAIN_MID: Color = Color::from_rgb(0.85, 0.85, 0.0);

/// The equalizer band tint at [`GAIN_MAX_DB`](crate::equalizer::GAIN_MAX_DB):
/// a full boost, a hot red-orange.
pub const EQ_GAIN_HIGH: Color = Color::from_rgb(1.0, 0.2, 0.1);

/// The family name of the vendored letterform face the chrome's labels
/// render in: the bitmap-text look of the reference captures. The TTF ships
/// as `assets/fonts/Silkscreen-Regular.ttf` with its source recorded in
/// `assets/fonts/README.md`.
///
// Nothing in production names the family yet — part 2/4 of the pixel-font
// split adopts these faces control by control, and until it lands only the
// layout-capture seam test — through `ui_font` — reaches them.
pub const UI_FONT_FAMILY: &str = "Silkscreen";

/// The family name of the vendored shape-glyph face the transport's markers
/// render in. The TTF ships as `assets/fonts/Tiny5.ttf` with its source
/// recorded in `assets/fonts/README.md`.
///
// Referenced only by `shape_font`, which stays dead until part 2/4 adopts
// it; see [`UI_FONT_FAMILY`].
pub const SHAPE_FONT_FAMILY: &str = "Tiny5";

/// The letterform face the chrome's text renders in, addressed by
/// [`UI_FONT_FAMILY`] so no view spells the family as a literal.
///
// Dead only until part 2/4 adopts it; see [`UI_FONT_FAMILY`].
#[cfg_attr(not(test), expect(dead_code))]
pub fn ui_font() -> Font {
    Font::with_name(UI_FONT_FAMILY)
}

/// The shape-glyph face the chrome's markers render in, addressed by
/// [`SHAPE_FONT_FAMILY`] so no view spells the family as a literal.
///
// Dead only until part 2/4 adopts it; see [`UI_FONT_FAMILY`].
#[cfg_attr(not(test), expect(dead_code))]
pub fn shape_font() -> Font {
    Font::with_name(SHAPE_FONT_FAMILY)
}

/// The base-skin colors as an iced [`Palette`](iced::theme::Palette): the
/// dark window face and light text are the background/foreground pair,
/// [`TITLE_BLUE`] drives interactive accents, and [`LCD_GREEN`] is the
/// success color. Warning and danger keep conventional amber/red so a
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
    fn palette_uses_the_named_base_skin_colors() {
        let palette = winamp_theme().palette();
        assert_eq!(palette.background, WINDOW_BACKGROUND);
        assert_eq!(palette.text, TEXT);
        assert_eq!(palette.primary, TITLE_BLUE);
        assert_eq!(palette.success, LCD_GREEN);
    }

    // The named-color test above pins four of the palette's six slots; the
    // remaining two, `warning` and `danger`, are inline literals rather than
    // named constants, so nothing read them and an accidental swap or edit
    // would clear the whole suite while iced's extended palette rendered the
    // wrong amber/red ramp for a future error state. Pin the documented
    // conventional values, and that the two stay distinct from each other and
    // from the success green.
    #[test]
    fn palette_keeps_conventional_warning_and_danger_colors() {
        let palette = winamp_theme().palette();
        assert_eq!(palette.warning, Color::from_rgb(1.0, 0.65, 0.0));
        assert_eq!(palette.danger, Color::from_rgb(1.0, 0.2, 0.2));
        assert_ne!(palette.warning, palette.danger);
        assert_ne!(palette.warning, palette.success);
        assert_ne!(palette.danger, palette.success);
    }

    // The palette tests above read the six palette slots. The row highlight,
    // the playing row's white text, and the LCD well's recess are base-skin
    // colors that inline style closures apply: `views::browse::rows::scrollable_list`
    // and `bevel::lcd_well` read them where iced's `Element` API exposes no
    // way to introspect. Their RGB values are the skin contract — a
    // regression could turn the playlist selection bar or the recess into
    // any other color and clear the whole suite. Pin all three, and that they
    // keep their documented roles: the well is darker than the window face it
    // recesses into, the selection bar stands apart from both, and the
    // playing text stands apart from an ordinary row's green.
    #[test]
    fn panel_recess_and_row_highlight_keep_their_base_skin_colors() {
        assert_eq!(LCD_BACKGROUND, Color::from_rgb(0.05, 0.05, 0.05));
        assert_eq!(ROW_HIGHLIGHT, Color::from_rgb(0.25, 0.5, 1.0));
        assert_eq!(PLAYING_ROW_TEXT, Color::from_rgb(1.0, 1.0, 1.0));
        // The playing row relies on white text alone, so it must not look like
        // an ordinary green row.
        assert_ne!(PLAYING_ROW_TEXT, LCD_GREEN);

        // The recess reads as sunken only if it is darker than the face it
        // sits in. Bound to locals so the comparison is a runtime check rather
        // than the constant assertion clippy rejects.
        let recess = LCD_BACKGROUND;
        let face = WINDOW_BACKGROUND;
        assert!(recess.r < face.r);
        assert!(recess.g < face.g);
        assert!(recess.b < face.b);

        // The selection bar must stand out from the window face and the well.
        assert_ne!(ROW_HIGHLIGHT, WINDOW_BACKGROUND);
        assert_ne!(ROW_HIGHLIGHT, LCD_BACKGROUND);
    }

    // The tests above read each palette slot against the named constant that
    // feeds it, so a constant and its reader can drift together and clear the
    // suite. The base-skin RGB values are this project's visual contract — a
    // Winamp 2.x clone lives or dies on its palette — and the remaining named
    // colors are read only constant-to-constant (or by inline widget
    // closures iced exposes no way to introspect). Pin every literal here,
    // plus the bevel and button-face orderings their doc comments promise, so
    // an accidental recolor is caught rather than shipped.
    #[test]
    fn base_skin_colors_keep_their_documented_literal_values() {
        assert_eq!(WINDOW_BACKGROUND, Color::from_rgb8(53, 53, 84));
        assert_eq!(TEXT, Color::from_rgb(0.87, 0.87, 0.87));
        assert_eq!(TITLE_BLUE, Color::from_rgb(0.0, 0.0, 0.55));
        assert_eq!(TITLE_TAPE, Color::from_rgb(0.93, 0.81, 0.48));
        assert_eq!(TITLE_TAPE_SHADOW, Color::from_rgb(0.64, 0.58, 0.42));
        assert_eq!(LCD_GREEN, Color::from_rgb(0.0, 1.0, 0.0));
        assert_eq!(LCD_GREEN_DIM, Color::from_rgb(0.0, 0.35, 0.0));
        assert_eq!(LCD_RED, Color::from_rgb(1.0, 0.1, 0.1));
        assert_eq!(PANEL_EDGE_LIGHT, Color::from_rgb8(98, 98, 118));
        assert_eq!(PANEL_EDGE_DARK, Color::from_rgb8(49, 57, 74));
        assert_eq!(WINDOW_EDGE_OUTLINE, Color::from_rgb8(20, 20, 31));
        assert_eq!(BUTTON_FACE, Color::from_rgb8(185, 196, 209));
        assert_eq!(BUTTON_FACE_HOVERED, Color::from_rgb8(195, 206, 219));
        assert_eq!(BUTTON_FACE_PRESSED, Color::from_rgb8(167, 178, 191));
        assert_eq!(BUTTON_TEXT, Color::from_rgb8(57, 57, 90));
        assert_eq!(BUTTON_EDGE_LIGHT, Color::from_rgb8(239, 255, 255));
        assert_eq!(LED_ON, Color::from_rgb8(0, 232, 0));
        assert_eq!(LED_OFF, Color::from_rgb8(16, 20, 15));
        // The LED must read as its own status marker, not as one of the LCD's
        // pure green readouts used beside it on the same row.
        assert_ne!(LED_ON, LCD_GREEN);
        // The light-metal convention the transport row is measured against: the
        // face and its bezel highlight are lighter than the engraved glyph color.
        const {
            assert!(
                BUTTON_FACE.r + BUTTON_FACE.g + BUTTON_FACE.b
                    > BUTTON_TEXT.r + BUTTON_TEXT.g + BUTTON_TEXT.b
            );
            assert!(
                BUTTON_EDGE_LIGHT.r + BUTTON_EDGE_LIGHT.g + BUTTON_EDGE_LIGHT.b
                    > BUTTON_TEXT.r + BUTTON_TEXT.g + BUTTON_TEXT.b
            );
            // The LED square's two states must stay distinguishable against
            // the light metal face they sit on and the dark engraved glyph
            // beside them: the dark square dimmer than the engraved color, so
            // it cannot read as a third glyph, and the lit square brighter
            // green than the face, so it cannot disappear into the metal.
            const {
                assert!(LED_OFF.r + LED_OFF.g + LED_OFF.b < LED_ON.r + LED_ON.g + LED_ON.b);
                assert!(LED_OFF.g < BUTTON_TEXT.g);
                assert!(LED_ON.g > BUTTON_FACE.g);
            }
        }
        assert_eq!(VOLUME_SLIDER_FILL, Color::from_rgb(0.88, 0.49, 0.15));
        assert_eq!(BALANCE_SLIDER_FILL, Color::from_rgb(0.09, 0.57, 0.04));
        assert_eq!(EQ_GAIN_LOW, LCD_GREEN);
        assert_eq!(EQ_GAIN_MID, Color::from_rgb(0.85, 0.85, 0.0));
        assert_eq!(EQ_GAIN_HIGH, Color::from_rgb(1.0, 0.2, 0.1));

        // The two level fills must read apart from each other and from the
        // neutral groove, or the volume and balance sliders would look
        // identical again.
        let (volume_fill, balance_fill, groove) =
            (VOLUME_SLIDER_FILL, BALANCE_SLIDER_FILL, LCD_BACKGROUND);
        assert_ne!(volume_fill, balance_fill);
        assert_ne!(volume_fill, groove);
        assert_ne!(balance_fill, groove);

        // The title tapes must read apart from the blue band they cross, or
        // the strip disappears into the title bar.
        let (tape, tape_shadow, title_blue) = (TITLE_TAPE, TITLE_TAPE_SHADOW, TITLE_BLUE);
        assert_ne!(tape, title_blue);
        assert_ne!(tape_shadow, title_blue);
        assert_ne!(tape, tape_shadow);

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
