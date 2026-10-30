//! The user's mixer and equalizer settings, persisted across launches.
//!
//! [`Settings`] is a small snapshot of the values the player keeps while it runs: the volume,
//! balance, Repeat and Shuffle flags, the equalizer's on/off and AUTO states, preamp, band gains,
//! and selected preset, the Now Playing readout's elapsed/remaining mode, the visualizer's
//! shown/hidden choice, the equalizer and library panels' shown/hidden choices, the always-on-top
//! flag, and the window-shade mode, the window's last screen position, the library's top-level
//! browse list, and the saved `MusicKit` sign-in's user token. All but the last nine live in
//! [`AppState`]; the readout mode, visualizer choice,
//! panel choices, always-on-top flag, shade mode, window position, browse list, and sign-in token
//! live on the UI's
//! `WinampPlayer` (the token through its [`crate::apple_music::AppleMusicService`]), so the player sets them on its
//! snapshot before saving and restores them after
//! loading. The file-backed store lives in [`file_store`], the text format in the private `text`
//! module, the writer seam in [`seam`], and [`writer::BackgroundWriter`] runs the blocking write on
//! a worker thread in production.
//!
//! [`file_store::FileStore`] loads the snapshot from a line-based text file
//! at startup and rewrites it whenever the UI changes one of those values.
//! Playback position, any browse level below the top-level list, the search box, the current track,
//! and the window's size are deliberately not persisted.

pub mod atomic_write;
pub mod file_store;
pub mod library_view;
pub mod seam;
mod text;
pub mod writer;

use crate::equalizer::{self, CustomPreset, Preset};
use crate::state::AppState;
use library_view::LibraryView;

/// The persisted values the next launch restores: the mixer and equalizer
/// subset of [`AppState`], the last-played track id, and the UI-owned readout
/// mode, visualizer choice, equalizer and library panel choices,
/// always-on-top flag, shade mode, window screen position, top-level
/// browse list, and the saved `MusicKit` sign-in's user token.
///
/// The fields are private; build a state-backed snapshot with [`Settings::from_state`] and restore
/// it with [`Settings::apply_to`]. `time_display_remaining`, `visualizer_visible`,
/// `eq_panel_visible`, `library_panel_visible`, `always_on_top`, `shade`, `window_position`,
/// `library_view`, and `music_kit_user_token` are not held in [`AppState`]: `from_state` leaves them elapsed, shown (all
/// three), unpinned, unshaded, without a known position, on Artists, and without a token, and the
/// player overwrites them from its own values (the token through its [`crate::apple_music::AppleMusicService`]) before
/// saving. `current_track` is the last-played
/// track id, held in [`AppState`]: the snapshot carries it straight from the
/// state, and `apply_to` restores the track stopped with a blank elapsed
/// readout, the state Stop leaves; the position within the track stays out
/// of the file. `Default` mirrors [`AppState::default`] plus elapsed,
/// shown, unpinned, unshaded, and unsigned-in, so if one starting value changes the other must
/// change with it.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    volume: f32,
    balance: f32,
    current_track: Option<String>,
    repeat: bool,
    shuffle: bool,
    manual_advance: bool,
    eq_enabled: bool,
    eq_auto: bool,
    eq_preamp: f32,
    eq_bands: [f32; equalizer::BAND_COUNT],
    eq_preset: Option<Preset>,
    eq_custom_presets: Vec<CustomPreset>,
    time_display_remaining: bool,
    visualizer_visible: bool,
    eq_panel_visible: bool,
    library_panel_visible: bool,
    always_on_top: bool,
    shade: bool,
    window_position: Option<(f32, f32)>,
    library_view: LibraryView,
    music_kit_user_token: Option<String>,
}

impl Default for Settings {
    /// The same starting values as [`AppState::default`]: half volume, a
    /// centered balance, no track loaded, the toggles off, and the equalizer
    /// off with AUTO off, a flat curve, and no preset, plus the elapsed
    /// readout mode, the shown visualizer and both panels, and an unpinned,
    /// unshaded window. A missing or unreadable settings file starts here.
    fn default() -> Self {
        Self {
            volume: 0.5,
            balance: 0.0,
            current_track: None,
            repeat: false,
            shuffle: false,
            manual_advance: false,
            eq_enabled: false,
            eq_auto: false,
            eq_preamp: 0.0,
            eq_bands: [0.0; equalizer::BAND_COUNT],
            eq_preset: None,
            eq_custom_presets: Vec::new(),
            time_display_remaining: false,
            visualizer_visible: true,
            eq_panel_visible: true,
            library_panel_visible: true,
            always_on_top: false,
            shade: false,
            window_position: None,
            library_view: LibraryView::Artists,
            music_kit_user_token: None,
        }
    }
}

impl Settings {
    /// Build the persisted snapshot of `state`, reading its public flags and
    /// its getters for the private clamped values.
    #[must_use]
    pub fn from_state(state: &AppState) -> Self {
        Self {
            volume: state.volume(),
            balance: state.balance(),
            current_track: state.current_track.clone(),
            repeat: state.repeat,
            shuffle: state.shuffle,
            manual_advance: state.manual_advance,
            eq_enabled: state.eq_enabled,
            eq_auto: state.eq_auto,
            eq_preamp: state.eq_preamp(),
            eq_bands: state.eq_bands(),
            eq_preset: state.eq_preset(),
            // The player's own saved list: `AppState` does not hold it, so the
            // player overwrites it before saving, like the UI-owned flags
            // below.
            eq_custom_presets: Vec::new(),
            time_display_remaining: false,
            visualizer_visible: true,
            eq_panel_visible: true,
            library_panel_visible: true,
            always_on_top: false,
            shade: false,
            window_position: None,
            library_view: LibraryView::Artists,
            music_kit_user_token: None,
        }
    }

    /// Whether the Now Playing readout shows the track's remaining time
    /// rather than its elapsed time. The player owns this value, so it reads
    /// it back when saving and restores it from the loaded snapshot at boot.
    #[must_use]
    pub fn time_display_remaining(&self) -> bool {
        self.time_display_remaining
    }

    /// Set whether the Now Playing readout shows remaining time. The player
    /// calls this on its fresh snapshot before saving; [`Self::apply_to`]
    /// deliberately leaves the UI-owned mode alone.
    pub fn set_time_display_remaining(&mut self, remaining: bool) {
        self.time_display_remaining = remaining;
    }

    /// Whether the LCD's spectrum visualizer draws its bars. The player owns
    /// this value, so it reads it back when saving and restores it from the
    /// loaded snapshot at boot. The stored spectrum and its poll are separate
    /// and are not persisted.
    #[must_use]
    pub fn visualizer_visible(&self) -> bool {
        self.visualizer_visible
    }

    /// Set whether the LCD's spectrum visualizer draws its bars. The player
    /// calls this on its fresh snapshot before saving; [`Self::apply_to`]
    /// deliberately leaves the UI-owned flag alone.
    pub fn set_visualizer_visible(&mut self, visible: bool) {
        self.visualizer_visible = visible;
    }

    /// Whether the normal-mode equalizer panel is shown. The player owns this
    /// value, so it reads it back when saving and restores it from the loaded
    /// snapshot at boot. It is a layout choice only; the EQ's own state is
    /// separate.
    #[must_use]
    pub fn eq_panel_visible(&self) -> bool {
        self.eq_panel_visible
    }

    /// Set whether the equalizer panel is shown. The player calls this on its
    /// fresh snapshot before saving; [`Self::apply_to`] deliberately leaves
    /// the UI-owned flag alone.
    pub fn set_eq_panel_visible(&mut self, visible: bool) {
        self.eq_panel_visible = visible;
    }

    /// Whether the normal-mode library section is shown. The player owns this
    /// value, so it reads it back when saving and restores it from the loaded
    /// snapshot at boot.
    #[must_use]
    pub fn library_panel_visible(&self) -> bool {
        self.library_panel_visible
    }

    /// Set whether the library section is shown. The player calls this on its
    /// fresh snapshot before saving; [`Self::apply_to`] deliberately leaves
    /// the UI-owned flag alone.
    pub fn set_library_panel_visible(&mut self, visible: bool) {
        self.library_panel_visible = visible;
    }

    /// Whether the window is pinned above other windows. The player owns this
    /// value, so it reads it back when saving and restores it from the loaded
    /// snapshot at boot.
    #[must_use]
    pub fn always_on_top(&self) -> bool {
        self.always_on_top
    }

    /// Set whether the window is pinned above other windows. The player calls
    /// this on its fresh snapshot before saving; [`Self::apply_to`]
    /// deliberately leaves the UI-owned flag alone.
    pub fn set_always_on_top(&mut self, always_on_top: bool) {
        self.always_on_top = always_on_top;
    }

    /// Whether the window is rolled up into shade mode. The player owns this
    /// value, so it reads it back when saving and restores it from the loaded
    /// snapshot at boot. The window's position is separate, and its size is not
    /// persisted.
    #[must_use]
    pub fn shaded(&self) -> bool {
        self.shade
    }

    /// Set whether the window is rolled up into shade mode. The player calls
    /// this on its fresh snapshot before saving; [`Self::apply_to`]
    /// deliberately leaves the UI-owned flag alone.
    pub fn set_shaded(&mut self, shaded: bool) {
        self.shade = shaded;
    }

    /// The window's last reported screen position, if one was saved. The
    /// player owns this value, so it reads it back when saving and restores
    /// it from the loaded snapshot at boot.
    #[must_use]
    pub fn window_position(&self) -> Option<(f32, f32)> {
        self.window_position
    }

    /// Set the window's screen position. The player calls this on its fresh
    /// snapshot before saving; [`Self::apply_to`] deliberately leaves the
    /// UI-owned value alone.
    pub fn set_window_position(&mut self, position: Option<(f32, f32)>) {
        self.window_position = position;
    }

    /// The saved `MusicKit` sign-in's user token, if the settings file carries
    /// one. The player owns this value — it reads it back through its
    /// [`crate::apple_music::AppleMusicService`] when saving and [`apply_to`](Self::apply_to)
    /// deliberately leaves it alone — so the launch that follows can reseed the
    /// session instead of re-running the browser flow.
    #[must_use]
    pub fn music_kit_user_token(&self) -> Option<&str> {
        self.music_kit_user_token.as_deref()
    }

    /// Set the saved `MusicKit` sign-in's user token. The player calls this on
    /// its fresh snapshot before saving; [`Self::apply_to`] deliberately
    /// leaves the UI-owned value alone.
    pub fn set_music_kit_user_token(&mut self, token: Option<String>) {
        self.music_kit_user_token = token;
    }

    /// The top-level browse list the library reopens on. The player owns this
    /// value, so it reads it back when saving and restores it from the loaded
    /// snapshot at boot.
    #[must_use]
    pub fn library_view(&self) -> LibraryView {
        self.library_view
    }

    /// Set the top-level browse list. The player calls this on its fresh
    /// snapshot before saving; [`Self::apply_to`] deliberately leaves the
    /// UI-owned value alone.
    pub fn set_library_view(&mut self, view: LibraryView) {
        self.library_view = view;
    }

    /// Restore this snapshot into `state`.
    ///
    /// The private values go through the state's clamped setters
    /// (`set_volume`, `set_balance`, `set_eq_preamp`, and `set_eq_band`), so a
    /// hand-edited file cannot push the state out of range. An applied preset
    /// is restored last through `apply_eq_preset`, after the per-band writes
    /// have cleared the selection. The AUTO flag and the last-played track id
    /// (`current_track`, held in [`AppState`]) are plain restored fields: the
    /// track comes back stopped with a blank elapsed readout.
    /// The UI-owned readout mode, visualizer
    /// choice, panel choices, always-on-top flag, shade mode, window
    /// position, and browse list are deliberately left alone.
    pub fn apply_to(&self, state: &mut AppState) {
        state.set_volume(self.volume);
        state.set_balance(self.balance);
        state.current_track = self.current_track.clone();
        state.repeat = self.repeat;
        state.shuffle = self.shuffle;
        state.manual_advance = self.manual_advance;
        state.eq_enabled = self.eq_enabled;
        state.eq_auto = self.eq_auto;
        state.set_eq_preamp(self.eq_preamp);
        for (band, gain) in self.eq_bands.iter().enumerate() {
            state.set_eq_band(band, *gain);
        }
        if let Some(preset) = self.eq_preset {
            state.apply_eq_preset(preset);
        }
        // The saved customs deliberately apply nothing: the applied curve
        // comes back through the `eq_preamp`/`eq_band_*` lines above, and the
        // list only re-feeds the pick list through the player's restored
        // value (`adopt_saved_settings`), exactly like the UI-owned flags
        // `apply_to` leaves alone.
    }

    /// The player's saved custom equalizer curves, in stored order. The
    /// player owns this value, so it overwrites it on its fresh snapshot
    /// before saving and restores it from the loaded snapshot at boot.
    #[must_use]
    pub fn eq_custom_presets(&self) -> &[CustomPreset] {
        &self.eq_custom_presets
    }

    /// Set the saved custom equalizer curves. The player calls this on its
    /// fresh snapshot before saving; [`Self::apply_to`] deliberately leaves
    /// the list alone.
    pub fn set_eq_custom_presets(&mut self, presets: Vec<CustomPreset>) {
        self.eq_custom_presets = presets;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_with_every_field_set() -> AppState {
        let mut state = AppState::default();
        state.set_volume(0.8);
        state.current_track = Some("song-1".to_string());
        state.set_balance(-0.35);
        state.repeat = true;
        state.shuffle = true;
        state.manual_advance = true;
        state.eq_enabled = true;
        state.eq_auto = true;
        state.set_eq_preamp(3.5);
        state.set_eq_band(0, -4.0);
        state.set_eq_band(9, 7.5);
        state
    }

    #[test]
    fn to_text_round_trips_every_field() {
        let mut state = state_with_every_field_set();
        let rock = *equalizer::PRESETS
            .iter()
            .find(|preset| preset.name == "Rock")
            .unwrap();
        state.apply_eq_preset(rock);

        let mut settings = Settings::from_state(&state);
        settings.set_time_display_remaining(true);
        settings.set_visualizer_visible(false);
        settings.set_eq_panel_visible(false);
        settings.set_library_panel_visible(false);
        settings.set_always_on_top(true);
        settings.set_shaded(true);
        settings.set_window_position(Some((120.0, -40.5)));
        settings.set_music_kit_user_token(Some("a.b.c".to_string()));
        assert_eq!(Settings::from_text(&settings.to_text()), settings);
        assert!(Settings::from_text(&settings.to_text()).time_display_remaining());
        assert!(!Settings::from_text(&settings.to_text()).visualizer_visible());
        assert!(!Settings::from_text(&settings.to_text()).eq_panel_visible());
        assert!(!Settings::from_text(&settings.to_text()).library_panel_visible());
        assert!(Settings::from_text(&settings.to_text()).always_on_top());
        assert!(Settings::from_text(&settings.to_text()).shaded());

        let custom = Settings {
            eq_preset: None,
            ..settings
        };
        assert_eq!(Settings::from_text(&custom.to_text()), custom);
    }

    #[test]
    fn time_display_round_trips_both_words_and_ignores_unknown_values() {
        let elapsed = Settings::default();
        assert!(!elapsed.time_display_remaining());
        assert!(!Settings::from_text(&elapsed.to_text()).time_display_remaining());

        let mut remaining = elapsed.clone();
        remaining.set_time_display_remaining(true);
        assert!(remaining.to_text().contains("time_display=remaining"));
        assert!(
            Settings::from_text(&remaining.to_text()).time_display_remaining(),
            "remaining must survive the round trip"
        );

        assert!(
            !Settings::from_text("time_display=sideways\n").time_display_remaining(),
            "an unknown word must fall back to elapsed"
        );
    }

    #[test]
    fn visualizer_visibility_round_trips_and_ignores_malformed_values() {
        let shown = Settings::default();
        assert!(shown.visualizer_visible());
        assert!(Settings::from_text(&shown.to_text()).visualizer_visible());

        let mut hidden = shown.clone();
        hidden.set_visualizer_visible(false);
        assert!(hidden.to_text().contains("visualizer_visible=false"));
        assert!(
            !Settings::from_text(&hidden.to_text()).visualizer_visible(),
            "a hidden visualizer must survive the round trip"
        );

        assert!(
            Settings::from_text("visualizer_visible=true\n").visualizer_visible(),
            "an explicit true must stay shown"
        );
        assert!(
            Settings::from_text("visualizer_visible=maybe\n").visualizer_visible(),
            "an unknown value must fall back to shown"
        );
    }

    #[test]
    fn panel_visibility_round_trips_and_ignores_malformed_values() {
        let shown = Settings::default();
        assert!(shown.eq_panel_visible());
        assert!(shown.library_panel_visible());

        let mut eq_hidden = shown.clone();
        eq_hidden.set_eq_panel_visible(false);
        let back = Settings::from_text(&eq_hidden.to_text());
        assert!(!back.eq_panel_visible());
        assert!(back.library_panel_visible());

        let mut library_hidden = shown.clone();
        library_hidden.set_library_panel_visible(false);
        let back = Settings::from_text(&library_hidden.to_text());
        assert!(back.eq_panel_visible());
        assert!(!back.library_panel_visible());

        let unknown = Settings::from_text("eq_panel_visible=maybe\nlibrary_panel_visible=maybe\n");
        assert!(
            unknown.eq_panel_visible(),
            "an unknown value must fall back to shown"
        );
        assert!(unknown.library_panel_visible());
    }

    #[test]
    fn always_on_top_round_trips_and_ignores_malformed_values() {
        let unpinned = Settings::default();
        assert!(!unpinned.always_on_top());
        assert!(!Settings::from_text(&unpinned.to_text()).always_on_top());

        let mut pinned = unpinned.clone();
        pinned.set_always_on_top(true);
        assert!(pinned.to_text().contains("always_on_top=true"));
        assert!(
            Settings::from_text(&pinned.to_text()).always_on_top(),
            "a pinned window must survive the round trip"
        );

        assert!(
            !Settings::from_text("always_on_top=false\n").always_on_top(),
            "an explicit false must stay unpinned"
        );
        assert!(
            !Settings::from_text("always_on_top=yes\n").always_on_top(),
            "an unknown word must fall back to unpinned"
        );
    }

    #[test]
    fn from_text_ignores_a_leading_utf8_bom() {
        let settings = Settings::from_text("\u{feff}volume=0.25\nbalance=0.25\n");

        assert_eq!(
            settings.volume, 0.25,
            "a leading BOM is an encoding marker, not part of the first key"
        );
        assert_eq!(settings.balance, 0.25);
    }

    #[test]
    fn shade_round_trips_and_ignores_malformed_values() {
        let unshaded = Settings::default();
        assert!(!unshaded.shaded());
        assert!(!Settings::from_text(&unshaded.to_text()).shaded());
        assert!(
            !Settings::from_text("volume=0.5\n").shaded(),
            "a missing shade line must fall back to unshaded"
        );

        let mut shaded = unshaded.clone();
        shaded.set_shaded(true);
        assert!(shaded.to_text().contains("shade=true"));
        assert!(
            Settings::from_text(&shaded.to_text()).shaded(),
            "a shaded window must survive the round trip"
        );

        assert!(
            !Settings::from_text("shade=false\n").shaded(),
            "an explicit false must stay unshaded"
        );
        assert!(
            !Settings::from_text("shade=\n").shaded(),
            "an empty value must fall back to unshaded"
        );
        assert!(
            !Settings::from_text("shade=yes\n").shaded(),
            "an unknown word must fall back to unshaded"
        );
    }

    #[test]
    fn manual_advance_round_trips_and_ignores_malformed_values() {
        let off = Settings::default();
        assert!(!off.manual_advance);
        assert!(
            !Settings::from_text("volume=0.5\n").manual_advance,
            "a missing manual_advance line must fall back to off"
        );
        assert!(!Settings::from_text(&off.to_text()).manual_advance);

        let mut on = off.clone();
        on.manual_advance = true;
        assert!(on.to_text().contains("manual_advance=true"));
        assert!(
            Settings::from_text(&on.to_text()).manual_advance,
            "an enabled manual advance must survive the round trip"
        );

        assert!(
            !Settings::from_text("manual_advance=false\n").manual_advance,
            "an explicit false must stay off"
        );
        assert!(
            !Settings::from_text("manual_advance=\n").manual_advance,
            "an empty value must fall back to off"
        );
        assert!(
            !Settings::from_text("manual_advance=yes\n").manual_advance,
            "an unknown word must fall back to off"
        );
    }

    #[test]
    fn eq_auto_round_trips_and_ignores_malformed_values() {
        let off = Settings::default();
        assert!(!off.eq_auto);
        assert!(!Settings::from_text(&off.to_text()).eq_auto);
        assert!(
            !Settings::from_text("volume=0.5\n").eq_auto,
            "a missing eq_auto line must fall back to off"
        );

        let mut on = off.clone();
        on.eq_auto = true;
        assert!(on.to_text().contains("eq_auto=true"));
        assert!(
            Settings::from_text(&on.to_text()).eq_auto,
            "an enabled AUTO must survive the round trip"
        );

        assert!(
            !Settings::from_text("eq_auto=false\n").eq_auto,
            "an explicit false must stay off"
        );
        assert!(
            !Settings::from_text("eq_auto=\n").eq_auto,
            "an empty value must fall back to off"
        );
        assert!(
            !Settings::from_text("eq_auto=yes\n").eq_auto,
            "an unknown word must fall back to off"
        );
    }

    #[test]
    fn from_text_ignores_nan_numbers() {
        // `f32::from_str` accepts `NaN`, but it is not a value the player can
        // use: an ignored line must fall back to its default, not to whichever
        // neutral value a later clamp happens to choose (volume's is silence).
        let text = "volume=NaN\n\
                    balance=NaN\n\
                    eq_preamp=NaN\n\
                    eq_band_0=NaN\n";
        assert_eq!(Settings::from_text(text), Settings::default());

        let recovered = Settings::from_text("volume=NaN\nvolume=0.25\n");
        assert_eq!(recovered.volume, 0.25, "a later valid line still applies");
    }

    #[test]
    fn from_text_ignores_malformed_and_unknown_lines() {
        let text = "volume=not-a-number\n\
                    balance=abc\n\
                    repeat=yes\n\
                    shuffle=1\n\
                    manual_advance=maybe\n\
                    eq_enabled=maybe\n\
                    eq_auto=maybe\n\
                    eq_auto=maybe\n\
                    eq_preamp=high\n\
                    eq_band_10=3\n\
                    eq_band_x=2\n\
                    eq_band_0=NaN\n\
                    eq_preset=Unknown\n\
                    time_display=sideways\n\
                    visualizer_visible=maybe\n\
                    eq_panel_visible=maybe\n\
                    library_panel_visible=maybe\n\
                    always_on_top=yes\n\
                    shade=yes\n\
                    mystery=1\n";
        assert_eq!(Settings::from_text(text), Settings::default());
    }

    #[test]
    fn a_later_eq_preset_line_clears_or_keeps_an_earlier_one() {
        let rock = *equalizer::PRESETS
            .iter()
            .find(|preset| preset.name == "Rock")
            .unwrap();

        let cleared = Settings::from_text("eq_preset=Rock\neq_preset=\n");
        assert_eq!(cleared.eq_preset, None);

        let kept = Settings::from_text("eq_preset=Rock\neq_preset=Unknown\n");
        assert_eq!(kept.eq_preset, Some(rock));
    }

    // The saved custom curves are a repeated `eq_custom_preset=` line in the
    // same family as `eq_preamp`/`eq_band_*`: every valid line loads in file
    // order, `from_curve` clamps out-of-range gains, and a line without a
    // `:`, a blank name, the wrong field count, or a malformed-or-NaN gain
    // is skipped without failing the load. The whole list must round-trip
    // through `to_text` → `from_text` untouched.
    #[test]
    fn eq_custom_preset_lines_round_trip_and_skip_the_malformed_ones() {
        let text = "eq_custom_preset=Very Fuzzed:2.5,1,2,3,4,5,6,7,8,9,10\n\
                    eq_custom_preset:missing=separator\n\
                    eq_custom_preset=:1,2,3,4,5,6,7,8,9,10,11\n\
                    eq_custom_preset=Short:1,2,3\n\
                    eq_custom_preset=NaN gain:0,1,2,3,4,5,6,7,8,9,NaN\n\
                    eq_custom_preset=Clamped:99,-99,0,0,0,0,0,0,0,0,0\n";
        let settings = Settings::from_text(text);
        assert_eq!(settings.eq_custom_presets().len(), 2);
        let fuzzed = &settings.eq_custom_presets()[0];
        assert_eq!(fuzzed.name, "Very Fuzzed");
        assert_eq!(fuzzed.preamp, 2.5);
        assert_eq!(
            fuzzed.bands,
            [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]
        );
        let clamped = &settings.eq_custom_presets()[1];
        assert_eq!(clamped.name, "Clamped");
        assert_eq!(clamped.preamp, equalizer::GAIN_MAX_DB);
        assert_eq!(clamped.bands[0], equalizer::GAIN_MIN_DB);

        assert_eq!(Settings::from_text(&settings.to_text()), settings);
        assert!(
            settings
                .to_text()
                .contains("eq_custom_preset=Very Fuzzed:2.5")
        );
        // A default snapshot writes no custom lines at all.
        assert!(!Settings::default().to_text().contains("eq_custom_preset"));
    }

    // A saved custom's name is free text, but the file is line-based: a
    // pasted line break in the name would split its `eq_custom_preset=`
    // row in two and let the name's tail be read as settings lines of its
    // own (a name like `Fuzz\neq_auto=on` would even flip a visible
    // setting). The writer therefore flattens line breaks to spaces, so
    // every saved curve keeps one whole line and round-trips under a
    // line-break-free name.
    #[test]
    fn a_line_broken_preset_name_writes_one_whole_settings_line() {
        let mut bands = [0.0; equalizer::BAND_COUNT];
        bands[0] = 2.0;
        let mut settings = Settings::default();
        settings.set_eq_custom_presets(vec![
            equalizer::CustomPreset::from_curve(String::from("Fuzz\nBass"), 2.5, bands),
            equalizer::CustomPreset::from_curve(String::from("Fuzz\rBass"), -1.0, bands),
        ]);
        let mut scrubbed = Settings::default();
        scrubbed.set_eq_custom_presets(vec![
            equalizer::CustomPreset::from_curve(String::from("Fuzz Bass"), 2.5, bands),
            equalizer::CustomPreset::from_curve(String::from("Fuzz Bass"), -1.0, bands),
        ]);

        assert_eq!(settings.to_text(), scrubbed.to_text());
        assert_eq!(Settings::from_text(&settings.to_text()), scrubbed);
    }

    #[test]
    fn from_text_reads_the_last_band_and_trims_around_the_equals_sign() {
        let last = equalizer::BAND_COUNT - 1;
        let settings = Settings::from_text(&format!("  eq_band_{last} = 4.5 \n"));
        assert_eq!(settings.eq_bands[last], 4.5);
        assert_eq!(settings.eq_bands[0], 0.0);
    }

    #[test]
    fn apply_to_clamps_out_of_range_values_through_the_state_setters() {
        let settings = Settings {
            volume: 2.0,
            balance: -2.0,
            eq_preamp: 99.0,
            eq_bands: [99.0; equalizer::BAND_COUNT],
            ..Settings::default()
        };
        let mut state = AppState::default();
        settings.apply_to(&mut state);

        assert_eq!(state.volume(), 1.0);
        assert_eq!(state.balance(), -1.0);
        assert_eq!(state.eq_preamp(), equalizer::GAIN_MAX_DB);
        assert!(
            state
                .eq_bands()
                .iter()
                .all(|gain| *gain == equalizer::GAIN_MAX_DB)
        );
    }

    #[test]
    fn infinite_numbers_load_and_clamp_to_the_range_ends() {
        // Unlike `NaN`, an infinity is kept by the parser and clamped by the
        // state setters, so it lands on the nearest range end.
        let settings =
            Settings::from_text("volume=inf\nbalance=-inf\neq_preamp=infinity\neq_band_0=-inf\n");
        let mut state = AppState::default();
        settings.apply_to(&mut state);

        assert_eq!(state.volume(), 1.0);
        assert_eq!(state.balance(), -1.0);
        assert_eq!(state.eq_preamp(), equalizer::GAIN_MAX_DB);
        assert_eq!(state.eq_bands()[0], equalizer::GAIN_MIN_DB);
    }

    #[test]
    fn apply_to_restores_a_saved_preset() {
        let rock = *equalizer::PRESETS
            .iter()
            .find(|preset| preset.name == "Rock")
            .unwrap();
        let settings = Settings {
            repeat: true,
            eq_enabled: true,
            eq_preset: Some(rock),
            ..Settings::default()
        };
        let mut state = AppState::default();
        settings.apply_to(&mut state);

        assert!(state.repeat);
        assert!(state.eq_enabled);
        assert_eq!(state.eq_preset(), Some(rock));
        assert_eq!(state.eq_preamp(), rock.preamp);
        assert_eq!(state.eq_bands(), rock.bands);
    }

    #[test]
    fn library_view_round_trips_and_ignores_unknown_words() {
        assert_eq!(Settings::default().library_view(), LibraryView::Artists);
        let mut settings = Settings::default();
        settings.set_library_view(LibraryView::Playlists);
        assert!(settings.to_text().contains("library_view=playlists"));
        assert_eq!(Settings::from_text(&settings.to_text()), settings);
        assert_eq!(
            Settings::from_text("library_view=bogus\n").library_view(),
            LibraryView::Artists
        );
        assert_eq!(
            Settings::from_text("volume=0.5\n").library_view(),
            LibraryView::Artists
        );
    }

    #[test]
    fn window_position_round_trips_and_ignores_malformed_values() {
        let mut settings = Settings::default();
        assert!(!settings.to_text().contains("window_position"));

        settings.set_window_position(Some((120.0, -40.5)));
        assert!(settings.to_text().contains("window_position=120,-40.5"));
        assert_eq!(
            Settings::from_text(&settings.to_text()).window_position(),
            Some((120.0, -40.5))
        );

        for bad in ["abc", "10", "NaN,5", "inf,5", "5,-inf", "1,2,3"] {
            assert_eq!(
                Settings::from_text(&format!("window_position={bad}\n")).window_position(),
                None,
                "{bad:?} must leave the position unset"
            );
        }
    }

    #[test]
    fn current_track_round_trips_and_defaults_to_none() {
        let none = Settings::default();
        assert_eq!(none.current_track, None);
        assert!(
            none.to_text().lines().any(|line| line == "current_track="),
            "a missing track writes an empty current_track line"
        );
        assert_eq!(
            Settings::from_text(&none.to_text()).current_track,
            None,
            "an empty value must fall back to no track"
        );
        assert_eq!(
            Settings::from_text("volume=0.5\n").current_track,
            None,
            "a missing current_track line must fall back to no track"
        );

        let mut saved = none.clone();
        saved.current_track = Some("song-42".to_string());
        assert!(saved.to_text().contains("current_track=song-42"));
        assert_eq!(
            Settings::from_text(&saved.to_text()).current_track,
            Some("song-42".to_string()),
            "a saved id must survive the round trip"
        );
        assert_eq!(
            Settings::from_text("current_track=a=b\n").current_track,
            Some("a=b".to_string()),
            "an id containing '=' must keep everything past the first '='"
        );
    }

    #[test]
    fn music_kit_user_token_round_trips_and_defaults_to_none() {
        let none = Settings::default();
        assert_eq!(none.music_kit_user_token(), None);
        assert!(
            !none.to_text().contains("music_kit_user_token"),
            "an absent token writes no token line"
        );
        assert_eq!(
            Settings::from_text(&none.to_text()).music_kit_user_token(),
            None,
            "a file with no token line must fall back to none"
        );
        assert_eq!(
            Settings::from_text("volume=0.5\n").music_kit_user_token(),
            None,
            "a missing token line must fall back to none"
        );
        // An empty value means the line was saved blank (a hand-edited
        // deletion, say): that is no token.
        assert_eq!(
            Settings::from_text("music_kit_user_token=\n").music_kit_user_token(),
            None
        );

        let mut saved = none.clone();
        saved.set_music_kit_user_token(Some("a.b.c".to_string()));
        assert!(saved.to_text().contains("music_kit_user_token=a.b.c"));
        assert_eq!(
            Settings::from_text(&saved.to_text()).music_kit_user_token(),
            Some("a.b.c")
        );
        // Formatting is not the codec's job (`seed_session` validates at
        // boot), so unJWT-shaped text is stored verbatim.
        assert_eq!(
            Settings::from_text("music_kit_user_token=leftover\n").music_kit_user_token(),
            Some("leftover")
        );
    }

    #[test]
    fn apply_to_restores_the_saved_current_track_stopped() {
        let saved = Settings {
            current_track: Some("song-1".to_string()),
            ..Settings::default()
        };
        let mut state = AppState::default();
        saved.apply_to(&mut state);
        assert_eq!(state.current_track.as_deref(), Some("song-1"));
        assert!(!state.is_playing, "the restored track sits stopped");
        assert!(!state.paused, "the restored track is not paused");

        let mut second = AppState::default();
        second.current_track = Some("song-1".to_string());
        Settings::default().apply_to(&mut second);
        assert_eq!(
            second.current_track, None,
            "a boot with no saved track loads none"
        );
    }
}
