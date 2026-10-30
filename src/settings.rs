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

/// A song list the user saved from the browse view's `Save list` button, by
/// name, through the naming panel. It stores only the song ids in showing
/// order — the same ids a row press resolves to `play_track` — because the
/// song rows come back through `resolve_song` at open time, and a saved list
/// opened after a relaunch shows whatever the library still answers.
#[derive(Debug, Clone, PartialEq)]
pub struct SavedPlaylist {
    /// The display name the Playlists view renders it under.
    pub name: String,
    /// The showing list's song ids, in showing order.
    pub song_ids: Vec<String>,
}

/// The persisted values the next launch restores: the mixer and equalizer
/// subset of [`AppState`], the last-played track id, and the UI-owned readout
/// mode, visualizer, marquee autoscroll, equalizer and library panel choices,
/// always-on-top flag, shade mode, window screen position, top-level
/// browse list, and the saved `MusicKit` sign-in's user token.
///
/// The fields are private; build a state-backed snapshot with [`Settings::from_state`] and restore
/// it with [`Settings::apply_to`]. `time_display_remaining`, `visualizer_visible`,
/// `autoscroll`, `eq_panel_visible`, `library_panel_visible`, `always_on_top`, `shade`, `window_position`,
/// `library_view`, and `music_kit_user_token` are not held in [`AppState`]: `from_state` leaves them elapsed, shown and
/// scrolling (all four), unpinned, unshaded, without a known position, on Artists, and without a token, and the
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
    saved_playlists: Vec<SavedPlaylist>,
    /// The signed-out Recently Played list's song ids in play order, newest
    /// first — the order the Recently Played view shows — so a relaunch
    /// picks the list up where the last signed-out run left it.
    recently_played: Vec<String>,
    time_display_remaining: bool,
    visualizer_visible: bool,
    autoscroll: bool,
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
    /// readout mode, the shown visualizer, an autoscrolling title, both
    /// panels, and an unpinned,
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
            saved_playlists: Vec::new(),
            recently_played: Vec::new(),
            time_display_remaining: false,
            visualizer_visible: true,
            autoscroll: true,
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
            // The player's saved song lists and played ids: same UI-owned
            // rule as the saved curves above.
            saved_playlists: Vec::new(),
            recently_played: Vec::new(),
            time_display_remaining: false,
            visualizer_visible: true,
            autoscroll: true,
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

    /// Whether the Now Playing title scrolls on its own while it is too long
    /// for the LCD. The player owns this value, so it reads it back when
    /// saving and restores it from the loaded snapshot at boot.
    #[must_use]
    pub fn autoscroll(&self) -> bool {
        self.autoscroll
    }

    /// Set whether the Now Playing title autoscrolls. The player calls this
    /// on its fresh snapshot before saving; [`Self::apply_to`] deliberately
    /// leaves the UI-owned flag alone.
    pub fn set_autoscroll(&mut self, autoscroll: bool) {
        self.autoscroll = autoscroll;
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

    /// The song lists the player saved through the browse `Save list`
    /// button, in saved order. The player owns this value, so it overwrites
    /// it on its fresh snapshot before saving and restores it from the loaded
    /// snapshot at boot.
    #[must_use]
    pub fn saved_playlists(&self) -> &[SavedPlaylist] {
        &self.saved_playlists
    }

    /// Set the saved song lists. The player calls this on its fresh snapshot
    /// before saving; [`Self::apply_to`] deliberately leaves the list alone.
    pub fn set_saved_playlists(&mut self, playlists: Vec<SavedPlaylist>) {
        self.saved_playlists = playlists;
    }

    /// The signed-out Recently Played list's song ids, newest first — the
    /// order the list is shown in. Ids arrived with by the player (an id the
    /// library no longer answers is dropped at restore time) are kept
    /// verbatim here, like a saved list's ids are. The player owns this
    /// value, so it overwrites it on its fresh snapshot before saving and
    /// restores it at boot like the other player-owned values;
    /// [`Self::apply_to`] deliberately leaves it alone.
    #[must_use]
    pub fn recently_played(&self) -> &[String] {
        &self.recently_played
    }

    /// Set the signed-out Recently Played list's ids, newest first. The
    /// player calls this on its fresh snapshot before saving;
    /// [`Self::apply_to`] deliberately leaves the list alone.
    pub fn set_recently_played(&mut self, ids: Vec<String>) {
        self.recently_played = ids;
    }
}

// Tests live in a sibling file (`src/audio/tests/`, `src/apple_music/tests/`).
#[cfg(test)]
mod tests;
