//! The Winamp 2.x base-skin palette and the custom [`iced::Theme`] built from
//! it.
//!
//! The player's widgets render in iced's default theme unless one is applied
//! app-wide, so this module names the base-skin colours once and exposes them
//! as a single [`winamp_theme`] the `init_ui` builder installs. Keeping the
//! colours as public constants — rather than inlining them in each view —
//! lets later fidelity work (title bar, panel bevels, playlist chrome) style
//! against the same names.

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

/// The base-skin colours as an iced [`Palette`](iced::theme::Palette): the
/// dark window face and light text are the background/foreground pair,
/// [`TITLE_BLUE`] drives interactive accents, and [`LCD_GREEN`] is the
/// success colour. Warning and danger keep conventional amber/red so a
/// future error state reads correctly against the dark face.
pub fn palette() -> iced::theme::Palette {
    iced::theme::Palette {
        background: WINDOW_BACKGROUND,
        text: TEXT,
        primary: TITLE_BLUE,
        success: LCD_GREEN,
        warning: Color::from_rgb(1.0, 0.65, 0.0),
        danger: Color::from_rgb(1.0, 0.2, 0.2),
    }
}

/// The app-wide Winamp theme: a custom [`Theme`] named `"Winamp"` over the
/// base-skin [`palette`]. Applied by `init_ui` so every widget that follows
/// the theme renders on the dark Winamp face.
pub fn winamp_theme() -> Theme {
    Theme::custom("Winamp", palette())
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
}
