//! Shared application state, the single source of truth for playback
//! status. Both the Apple Music service and the UI read and mutate it,
//! so it lives in its own module rather than in the binary's entry point.

use crate::audio;
use crate::clamp;
use crate::equalizer;

/// Global playback state shared between the Apple Music service and the UI.
///
/// `Debug` and `PartialEq` support equality assertions on whole `AppState`
/// values, such as the default-state tests below.
#[derive(Debug, PartialEq)]
pub struct AppState {
    /// The id of the track the player currently has loaded, or `None` before
    /// anything has played. The Now Playing bar names it and the browse song
    /// lists mark its row; [`AppState::stop`] leaves it set so the bar keeps
    /// showing the interrupted track's title.
    pub current_track: Option<String>,
    /// Whether playback is running. The LCD's transport glyph, through
    /// [`AppState::transport_mode`], reads it, and [`AppState::stop`] clears it
    /// without clearing the current track.
    pub is_playing: bool,
    /// Whether the loaded preview is paused: true only between Pause and the
    /// next Play or Stop, and false while playing or stopped. Together with
    /// [`AppState::is_playing`] it lets [`AppState::transport_mode`] tell
    /// Pause from Stop, which both clear the playing flag.
    pub paused: bool,
    /// Whether Previous/Next wrap around the ends of the loaded song list
    /// (Repeat on) or stop at the edge (Repeat off). Starts off, as in Winamp.
    pub repeat: bool,
    /// Whether Next picks a random song from the loaded song list (Shuffle on)
    /// or the next in order (Shuffle off). Starts off, as in Winamp.
    pub shuffle: bool,
    /// Whether a song that plays to its natural end stops instead of advancing
    /// (Manual playlist advance on) or follows the Repeat/Shuffle advance
    /// rules (off). Starts off, as in Winamp.
    pub manual_advance: bool,
    /// Playback volume in `[0.0, 1.0]`, never NaN. Kept private so the only
    /// way to change it is [`AppState::set_volume`], which clamps. Read it
    /// with [`AppState::volume`].
    volume: f32,
    /// Stereo balance in `[-1.0, 1.0]`, never NaN: `-1.0` is hard left, `0.0`
    /// center, and `+1.0` hard right. Kept private so the only way to change
    /// it is [`AppState::set_balance`], which clamps. Read it with
    /// [`AppState::balance`].
    balance: f32,
    /// Whether the equalizer is engaged. Starts off, as in Winamp; the band
    /// gains below are stored regardless so turning it back on restores them.
    pub eq_enabled: bool,
    /// Whether selecting a track applies a preset matched to its primary
    /// genre (Winamp's AUTO button). Starts off, as in Winamp.
    /// [`AppState::toggle_eq_auto`] flips it; selecting a track reads it
    /// through `handlers::mixer::apply_auto_preset`.
    pub eq_auto: bool,
    /// The preamp gain in decibels applied ahead of the bands, in
    /// `[equalizer::GAIN_MIN_DB, equalizer::GAIN_MAX_DB]`. Kept private like
    /// `volume`: [`AppState::set_eq_preamp`] and
    /// [`AppState::apply_eq_preset`] are its writers, both through
    /// [`equalizer::clamp_gain`], and [`AppState::eq_preamp`] is its only
    /// reader.
    eq_preamp: f32,
    /// The gain in decibels of each of the [`equalizer::BAND_COUNT`] bands,
    /// low frequency to high; flat (all `0.0`) by default. Kept private like
    /// `volume`: [`AppState::set_eq_band`], [`AppState::set_all_eq_bands`],
    /// and [`AppState::apply_eq_preset`] are its writers, all through
    /// [`equalizer::clamp_gain`], and [`AppState::eq_bands`] is its only
    /// reader.
    eq_bands: [f32; equalizer::BAND_COUNT],
    /// The preset whose whole curve is currently applied, or `None` for a
    /// custom curve. [`AppState::apply_eq_preset`] is its only setter to
    /// `Some`; the hand-moved sliders (`set_eq_preamp`, `set_eq_band`) clear
    /// it to `None`, since a manual move makes the curve custom. Read it with
    /// [`AppState::eq_preset`].
    eq_preset: Option<equalizer::Preset>,
}

/// The three states the transport can be in, resolved from [`AppState`] by
/// [`AppState::transport_mode`]. `Paused` and `Stopped` are distinct even
/// though both leave the player silent, because the Now Playing LCD shows a
/// different glyph for each.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportMode {
    /// The loaded preview is sounding.
    Playing,
    /// The loaded preview is loaded but held at its position.
    Paused,
    /// Nothing is sounding and no preview is held paused.
    Stopped,
}

/// The one production `AppState` is built in the app's entry point (`main`)
/// and shared (as an `Arc<Mutex<_>>`) with both the UI and the Apple Music
/// service; the test modules build their own. A single `Default` keeps every
/// construction site in sync: a new field's startup value is set here, in one
/// place, instead of in a struct literal repeated at each site. The starting
/// volume is 0.5, not the derived 0.0, so a manual impl is required.
///
/// Initial state: nothing loaded, not playing and not paused (stopped),
/// Repeat off, Shuffle off, at 50% volume with a centered balance, and the
/// equalizer off with a flat (all-zero) curve and no preset selected, and
/// AUTO off.
impl Default for AppState {
    fn default() -> Self {
        Self {
            current_track: None,
            is_playing: false,
            paused: false,
            repeat: false,
            shuffle: false,
            manual_advance: false,
            volume: 0.5,
            balance: 0.0,
            eq_enabled: false,
            eq_auto: false,
            eq_preamp: 0.0,
            eq_bands: [0.0; equalizer::BAND_COUNT],
            eq_preset: None,
        }
    }
}

impl AppState {
    /// Flip the play/pause flag in place.
    /// `AppleMusicService::toggle_play_pause` is its only caller — the system
    /// media Play/Pause key reaches it through that service method — so keeping
    /// the flip here (rather than inlined at the call site) puts the toggling
    /// semantics next to the field they mutate.
    pub fn toggle_playing(&mut self) {
        self.is_playing = !self.is_playing;
        self.paused = !self.is_playing;
    }

    /// Flip the Repeat flag in place. The UI's Repeat button and the
    /// keyboard's R key call this (through `Message::ToggleRepeat`); keeping
    /// the flip here (rather than inlined at the call site) puts the toggling
    /// semantics next to the field they mutate, like `toggle_playing`.
    pub fn toggle_repeat(&mut self) {
        self.repeat = !self.repeat;
    }

    /// Flip the Shuffle flag in place. The UI's Shuffle button and the
    /// keyboard's S key call this (through `Message::ToggleShuffle`); keeping
    /// the flip here (rather than inlined at the call site) puts the toggling
    /// semantics next to the field they mutate, like `toggle_repeat`.
    pub fn toggle_shuffle(&mut self) {
        self.shuffle = !self.shuffle;
    }

    /// Flip the Manual playlist advance flag in place. The options menu's
    /// `Manual playlist advance` row calls this (through
    /// `Message::OptionsChoice(OptionsChoice::ManualAdvance)`); keeping the
    /// flip here (rather than inlined at the call site) puts the toggling
    /// semantics next to the field they mutate, like `toggle_repeat`.
    pub fn toggle_manual_advance(&mut self) {
        self.manual_advance = !self.manual_advance;
    }

    /// Clear the playback flag and the paused flag in place, leaving
    /// `current_track` in place so the Now Playing bar keeps showing the interrupted track's title
    /// (matching Winamp, where Stop halts the music and the title stays in
    /// the display). The UI's Stop button and the keyboard's V key call this
    /// (through `Message::Stop`); keeping the semantics here (rather than
    /// inlined at the call site) puts them next to the field they mutate,
    /// like `toggle_playing`.
    pub fn stop(&mut self) {
        self.is_playing = false;
        self.paused = false;
    }

    /// Set the playback flag, leaving `current_track` in place so the Now
    /// Playing bar keeps naming the track. The on-screen Play buttons and the
    /// keyboard's X key call this (through `Message::Play`); unlike
    /// [`AppState::toggle_playing`] it is an explicit, idempotent set, so a
    /// second press keeps playing rather than pausing. It also clears
    /// [`AppState::paused`], so Play after Pause reports `Playing` rather
    /// than `Paused`.
    pub fn play(&mut self) {
        self.is_playing = true;
        self.paused = false;
    }

    /// Clear the playback flag and set [`AppState::paused`], leaving
    /// `current_track` in place so the Now Playing bar keeps showing the
    /// interrupted track's title (matching Winamp). The on-screen Pause
    /// buttons and the keyboard's C key call this (through `Message::Pause`);
    /// unlike [`AppState::toggle_playing`] it is an explicit, idempotent set;
    /// the service resumes a paused preview before calling this.
    pub fn pause(&mut self) {
        self.is_playing = false;
        self.paused = true;
    }

    /// Which of the three transport states the player is in. Pause and Stop
    /// both clear [`AppState::is_playing`], so [paused](AppState::paused)
    /// distinguishes them: the Now Playing LCD draws a paused glyph from this
    /// rather than a stopped one.
    #[must_use]
    pub fn transport_mode(&self) -> TransportMode {
        if self.is_playing {
            TransportMode::Playing
        } else if self.paused {
            TransportMode::Paused
        } else {
            TransportMode::Stopped
        }
    }

    /// Flip the equalizer's on/off flag in place. The equalizer panel's
    /// `EQ: On`/`EQ: Off` button is the only UI caller, through
    /// `Message::ToggleEqualizer`; keeping the flip here (rather than inlined
    /// at the call site) puts the toggling semantics next to the field they
    /// mutate, like `toggle_repeat`. The stored preamp and band gains are left
    /// alone, so re-enabling the equalizer restores the curve the user dialed
    /// in.
    pub fn toggle_equalizer(&mut self) {
        self.eq_enabled = !self.eq_enabled;
    }

    /// Flip the equalizer's AUTO flag in place. The equalizer panel's
    /// `AUTO: On`/`AUTO: Off` button is the only UI caller, through
    /// `Message::ToggleEqAuto`; keeping the flip here (rather than inlined at
    /// the call site) puts the toggling semantics next to the field they
    /// mutate, like `toggle_equalizer`. The stored curve, preset, and on/off
    /// flag are left alone.
    pub fn toggle_eq_auto(&mut self) {
        self.eq_auto = !self.eq_auto;
    }

    /// Store a preamp gain, clamped to the valid range via
    /// [`equalizer::clamp_gain`]. The UI's preamp slider is the only caller;
    /// the field is private and both writers go through `clamp_gain` (this
    /// setter and [`AppState::apply_eq_preset`]), so the clamp holds no
    /// matter which caller stores a gain. A hand-moved slider is a custom
    /// curve, so this clears any applied preset selection.
    pub fn set_eq_preamp(&mut self, gain: f32) {
        self.eq_preamp = equalizer::clamp_gain(gain);
        self.eq_preset = None;
    }

    /// Store a clamped gain into band `band`. An out-of-range `band` index
    /// (a stale slider message after the band count shrinks, say) is ignored
    /// rather than panicking: `get_mut` yields `None` and nothing is stored,
    /// so a bad index can't take the window down mid-drag. The field is
    /// private and every writer goes through `clamp_gain` (this setter,
    /// [`AppState::set_all_eq_bands`], and [`AppState::apply_eq_preset`]), so
    /// every stored band gain is clamped.
    /// A hand-moved slider is a custom curve, so a stored gain clears
    /// any applied preset selection; an ignored out-of-range index stores
    /// nothing and leaves the selection alone.
    pub fn set_eq_band(&mut self, band: usize, gain: f32) {
        if let Some(slot) = self.eq_bands.get_mut(band) {
            *slot = equalizer::clamp_gain(gain);
            self.eq_preset = None;
        }
    }

    /// Store one clamped gain into every band and clear the preset selection,
    /// since the curve is now custom. The equalizer's dB scale labels are the
    /// only caller. It leaves the preamp alone, as Winamp's labels do.
    pub fn set_all_eq_bands(&mut self, gain: f32) {
        self.eq_bands = [equalizer::clamp_gain(gain); equalizer::BAND_COUNT];
        self.eq_preset = None;
    }

    /// Apply a whole preset curve: store its preamp and every band gain
    /// (each still through [`equalizer::clamp_gain`], so a preset can never
    /// push shared state out of range) and remember the selection. The UI's
    /// equalizer pick list is the only caller; a later hand move of the
    /// preamp or a band clears the selection (see [`AppState::set_eq_preamp`]
    /// and [`AppState::set_eq_band`]).
    pub fn apply_eq_preset(&mut self, preset: equalizer::Preset) {
        self.eq_preamp = equalizer::clamp_gain(preset.preamp);
        self.eq_bands = preset.bands.map(equalizer::clamp_gain);
        self.eq_preset = Some(preset);
    }

    /// The current playback volume, always in `[0.0, 1.0]` (see
    /// [`AppState::set_volume`]). The UI's `view` reads it for the slider.
    #[must_use]
    pub fn volume(&self) -> f32 {
        self.volume
    }

    /// The current stereo balance, always in `[-1.0, 1.0]` (see
    /// [`AppState::set_balance`]). The UI's `view` reads it for the balance
    /// slider.
    #[must_use]
    pub fn balance(&self) -> f32 {
        self.balance
    }

    /// The current preamp gain in decibels, always within
    /// `[equalizer::GAIN_MIN_DB, equalizer::GAIN_MAX_DB]` (see
    /// [`AppState::set_eq_preamp`]). The UI's `view` reads it for the preamp
    /// slider.
    #[must_use]
    pub fn eq_preamp(&self) -> f32 {
        self.eq_preamp
    }

    /// The current per-band gains in decibels, low frequency to high, each
    /// always within `[equalizer::GAIN_MIN_DB, equalizer::GAIN_MAX_DB]` (see
    /// [`AppState::set_eq_band`]). The UI's `view` reads them for the band
    /// sliders.
    #[must_use]
    pub fn eq_bands(&self) -> [f32; equalizer::BAND_COUNT] {
        self.eq_bands
    }

    /// The applied equalizer preset, or `None` for a custom curve. The UI's
    /// `view` reads it to show the selected preset's name in the pick list.
    #[must_use]
    pub fn eq_preset(&self) -> Option<equalizer::Preset> {
        self.eq_preset
    }

    /// A copyable snapshot of the stored equalizer curve, for the audio seam.
    /// [`audio::EqSnapshot`] is what crosses to the audio worker, so every
    /// caller that forwards the curve — starting a preview, resuming one, and
    /// the equalizer handlers' mutate-and-forward — reads it through this one
    /// conversion instead of rebuilding the struct field by field.
    #[must_use]
    pub fn eq_snapshot(&self) -> audio::EqSnapshot {
        audio::EqSnapshot {
            enabled: self.eq_enabled,
            preamp_db: self.eq_preamp(),
            bands_db: self.eq_bands(),
        }
    }

    /// Stores `volume`, clamped to `[0.0, 1.0]` with NaN mapped to silence by
    /// [`clamp_volume`]. The field is private and this is its only writer, so
    /// the clamped invariant holds no matter which caller (today, the UI's
    /// `VolumeChange` arm and [`AppState::nudge_volume`]) sets it.
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = clamp_volume(volume);
    }

    /// Stores `balance`, clamped to `[-1.0, 1.0]` with NaN mapped to center
    /// by [`clamp_balance`]. The field is private and this is its only writer,
    /// so the clamped invariant holds no matter which caller (today, the UI's
    /// `BalanceChange` arm) sets it.
    pub fn set_balance(&mut self, balance: f32) {
        self.balance = clamp_balance(balance);
    }

    /// Moves the volume by `delta`, clamped to the valid range by
    /// [`AppState::set_volume`]. The UI's arrow-key arms call this with
    /// `+views::VOLUME_STEP` and `-views::VOLUME_STEP`, so the keyboard and
    /// the slider share one granularity and one clamp.
    pub fn nudge_volume(&mut self, delta: f32) {
        self.set_volume(self.volume + delta);
    }
}

/// Clamps a volume value to the valid `[0.0, 1.0]` range, mapping a NaN to
/// `0.0` (silence).
///
/// iced's slider can emit a value outside the range (a drag beyond the ends,
/// or a stale in-flight change), and the UI must never store an unclamped
/// volume in `AppState`. [`AppState::set_volume`] is the field's only writer
/// and calls this, so the rule lives here as a pure function beside the state
/// it protects rather than at each call site. The NaN-and-bounds behavior
/// itself is [`crate::clamp::clamp_with_nan_fallback`], shared with
/// `equalizer::clamp_gain`; this wrapper only names volume's range and its
/// silence fallback.
#[must_use]
pub fn clamp_volume(volume: f32) -> f32 {
    clamp::clamp_with_nan_fallback(volume, 0.0, 1.0, 0.0)
}

/// Clamps a balance value to the valid `[-1.0, 1.0]` range, mapping a NaN to
/// `0.0` (center).
///
/// The volume wrapper's twin: iced's slider can emit a value outside the
/// range (a drag beyond the ends, or a stale in-flight change), and the UI
/// must never store an unclamped balance in `AppState`.
/// [`AppState::set_balance`] is the field's only writer and calls this, so the
/// rule lives here as a pure function beside the state it protects rather than
/// at each call site. The NaN-and-bounds behavior itself is
/// [`crate::clamp::clamp_with_nan_fallback`], shared with
/// `equalizer::clamp_gain`; this wrapper only names balance's range and its
/// center fallback.
#[must_use]
pub fn clamp_balance(balance: f32) -> f32 {
    clamp::clamp_with_nan_fallback(balance, -1.0, 1.0, 0.0)
}

/// How close to center (exclusive) an inward balance move must land to snap to
/// exactly `0.0`.
pub const BALANCE_CENTER_SNAP: f32 = 0.1;

/// The balance a slider change to `requested` stores, given the `current`
/// position it moves from (a drag's previous requested value, else the stored
/// balance): `requested` clamped by [`clamp_balance`], except that a move
/// toward center landing within [`BALANCE_CENTER_SNAP`] of it stores exactly
/// `0.0`, like the classic player's center detent. Only an inward move snaps
/// (the clamped magnitude must also be below `current.abs()`); otherwise the
/// slider's own arrow keys could never step out of center, since a 1% step
/// out would snap straight back.
#[must_use]
pub fn snap_balance_to_center(current: f32, requested: f32) -> f32 {
    let clamped = clamp_balance(requested);
    if clamped.abs() < BALANCE_CENTER_SNAP && clamped.abs() < current.abs() {
        0.0
    } else {
        clamped
    }
}

#[cfg(test)]
mod tests;
