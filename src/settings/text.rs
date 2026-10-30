//! The line-based `key=value` text form of [`Settings`]: rendering a snapshot
//! for the settings file and parsing one back, key by key, with a bad line
//! falling back to its default.

use super::Settings;
use super::library_view::LibraryView;
use crate::equalizer;

impl Settings {
    /// Render the snapshot as the line-based `key=value` text the settings
    /// file holds. Floats use their shortest round-tripping `Display` form,
    /// and a custom curve stores an empty `eq_preset` value. Visible crate-
    /// wide so the UI-level round-trip tests can drive the file format too.
    pub(crate) fn to_text(&self) -> String {
        let mut text = String::new();
        text.push_str(&format!("volume={}\n", self.volume));
        text.push_str(&format!("balance={}\n", self.balance));
        text.push_str(&format!("repeat={}\n", self.repeat));
        text.push_str(&format!("shuffle={}\n", self.shuffle));
        text.push_str(&format!("manual_advance={}\n", self.manual_advance));
        text.push_str(&format!("eq_enabled={}\n", self.eq_enabled));
        text.push_str(&format!("eq_auto={}\n", self.eq_auto));
        text.push_str(&format!("eq_preamp={}\n", self.eq_preamp));
        for (band, gain) in self.eq_bands.iter().enumerate() {
            text.push_str(&format!("eq_band_{band}={gain}\n"));
        }
        text.push_str(&format!(
            "eq_preset={}\n",
            self.eq_preset.map_or("", |preset| preset.name)
        ));
        text.push_str(&format!(
            "time_display={}\n",
            if self.time_display_remaining {
                "remaining"
            } else {
                "elapsed"
            }
        ));
        text.push_str(&format!("visualizer_visible={}\n", self.visualizer_visible));
        text.push_str(&format!("eq_panel_visible={}\n", self.eq_panel_visible));
        text.push_str(&format!(
            "library_panel_visible={}\n",
            self.library_panel_visible
        ));
        text.push_str(&format!("always_on_top={}\n", self.always_on_top));
        text.push_str(&format!("shade={}\n", self.shade));
        if let Some((x, y)) = self.window_position {
            text.push_str(&format!("window_position={x},{y}\n"));
        }
        text.push_str(&format!(
            "current_track={}\n",
            self.current_track.as_deref().unwrap_or("")
        ));
        text.push_str(&format!("library_view={}\n", self.library_view.key()));
        // A conditional line, like `window_position` and unlike
        // `current_track`'s always-line: an absent token's absence is itself
        // the clear-the-line documentation, and a token that was never stored
        // must not put an empty line into the file.
        if let Some(token) = &self.music_kit_user_token {
            text.push_str(&format!("music_kit_user_token={token}\n"));
        }
        text
    }

    /// Parse `text` into a snapshot, starting from [`Settings::default`].
    ///
    /// Unknown keys, malformed numbers, booleans other than `true`/`false`,
    /// out-of-range band indexes, and unrecognized preset names are ignored,
    /// so one bad line falls back to its default without failing the load. A
    /// leading UTF-8 BOM is an encoding marker, not part of the first key, so
    /// it is dropped before the lines are read. A `NaN` is malformed rather
    /// than a number: `f32::from_str` accepts it, but it is not a value the
    /// player can hold, so the field keeps its default. Visible crate-wide so
    /// the UI-level round-trip tests can drive the file format too.
    pub(crate) fn from_text(text: &str) -> Self {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let mut settings = Self::default();
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            let value = value.trim();
            match key {
                "volume" => parse_number_into(&mut settings.volume, value),
                "balance" => parse_number_into(&mut settings.balance, value),
                "repeat" => parse_into(&mut settings.repeat, value),
                "shuffle" => parse_into(&mut settings.shuffle, value),
                // A malformed value leaves the default advance-along list.
                "manual_advance" => parse_into(&mut settings.manual_advance, value),
                "eq_enabled" => parse_into(&mut settings.eq_enabled, value),
                // A malformed value leaves the default AUTO off.
                "eq_auto" => parse_into(&mut settings.eq_auto, value),
                "eq_preamp" => parse_number_into(&mut settings.eq_preamp, value),
                "eq_preset" => settings.parse_eq_preset(value),
                // Only the two written words are recognized; any other value
                // leaves the default elapsed mode.
                "time_display" => match value {
                    "remaining" => settings.time_display_remaining = true,
                    "elapsed" => settings.time_display_remaining = false,
                    _ => {}
                },
                // A malformed value leaves the default shown visualizer.
                "visualizer_visible" => {
                    parse_into(&mut settings.visualizer_visible, value);
                }
                // A malformed value leaves the default shown panel.
                "eq_panel_visible" => parse_into(&mut settings.eq_panel_visible, value),
                "library_panel_visible" => {
                    parse_into(&mut settings.library_panel_visible, value);
                }
                // An empty or missing value leaves no token; any other text is
                // the token verbatim (`split_once` stops at the first `=`).
                // The shape check is `seed_session`'s job at boot, so the
                // codec stays value-blind, like every other string the file
                // carries.
                "music_kit_user_token" => {
                    settings.music_kit_user_token = if value.is_empty() {
                        None
                    } else {
                        Some(value.to_string())
                    };
                }
                // A malformed value leaves the default unpinned window.
                "always_on_top" => parse_into(&mut settings.always_on_top, value),
                // A malformed value leaves the default unshaded window.
                "shade" => parse_into(&mut settings.shade, value),
                // A malformed pair leaves the position unset.
                "window_position" => parse_window_position(&mut settings.window_position, value),
                // An empty or missing line leaves no track loaded; a
                // non-empty value is the id verbatim (`split_once` stops at
                // the first `=`), and an unknown or stale id falls back to
                // the view-level id label downstream.
                "current_track" => {
                    settings.current_track = if value.is_empty() {
                        None
                    } else {
                        Some(value.to_string())
                    };
                }
                // An unrecognized word leaves the default Artists list.
                "library_view" => {
                    if let Some(view) = LibraryView::from_key(value) {
                        settings.library_view = view;
                    }
                }
                _ => settings.parse_eq_band(key, value),
            }
        }
        settings
    }

    /// Apply an `eq_preset` line: empty clears the preset, a known name selects
    /// it, and an unrecognized name leaves the current preset.
    fn parse_eq_preset(&mut self, value: &str) {
        if value.is_empty() {
            self.eq_preset = None;
            return;
        }
        if let Some(preset) = equalizer::PRESETS
            .iter()
            .find(|preset| preset.name == value)
        {
            self.eq_preset = Some(*preset);
        }
    }

    /// Apply an `eq_band_<index>` line; any other key, or an index past the
    /// last band, is ignored.
    fn parse_eq_band(&mut self, key: &str, value: &str) {
        let Some(index) = key
            .strip_prefix("eq_band_")
            .and_then(|index| index.parse::<usize>().ok())
        else {
            return;
        };
        if let Some(slot) = self.eq_bands.get_mut(index) {
            parse_number_into(slot, value);
        }
    }
}

/// Assign `value`'s parsed form to `slot`, or leave `slot` unchanged when the
/// value is malformed. [`Settings::from_text`]'s boolean fields follow this
/// rule, so a bad line falls back to that field's default without failing the
/// load; the float fields, `eq_band_*`, and `eq_preset` need their own
/// handling for the NaN, index, and preset lookup.
fn parse_into<T: std::str::FromStr>(slot: &mut T, value: &str) {
    if let Ok(parsed) = value.parse() {
        *slot = parsed;
    }
}

/// Assign `value`'s parsed number to `slot` unless it is malformed or `NaN`,
/// leaving `slot` unchanged.
///
/// `f32::from_str` accepts `NaN` and the infinities. An infinity is a value
/// outside every documented range, so [`Settings::apply_to`]'s state setters
/// clamp it like any other out-of-range number. `NaN` is not a number at all,
/// and passing it on would make each field's fallback depend on its state
/// setter's neutral value — volume's is silence, not its default — so it is
/// treated as malformed and the field keeps its default.
fn parse_number_into(slot: &mut f32, value: &str) {
    if let Ok(parsed) = value.parse::<f32>()
        && !parsed.is_nan()
    {
        *slot = parsed;
    }
}

/// Assign an `x,y` pair to `slot` when both halves parse as finite numbers,
/// leaving `slot` unchanged otherwise. Negative coordinates are valid: a
/// monitor left of or above the primary has them.
fn parse_window_position(slot: &mut Option<(f32, f32)>, value: &str) {
    let Some((x, y)) = value.split_once(',') else {
        return;
    };
    if let (Ok(x), Ok(y)) = (x.trim().parse::<f32>(), y.trim().parse::<f32>())
        && x.is_finite()
        && y.is_finite()
    {
        *slot = Some((x, y));
    }
}
