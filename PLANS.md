# Plans

Planned features, written by the plan loop and implemented by the feature loop.
Each plan: goal, approach, files touched, acceptance criteria. Move finished plans to Done.

> **Steward drift note (2026-10-06; updated 2026-10-20).** The initial prompt
> names two commitments — a "very close if not pixel-perfect" classic Winamp UI
> and Apple Music as the library. The first is now moving: the base-skin
> palette landed ("Add a Winamp 2.x base-skin palette and apply it as the app
> theme", done 2026-10-06), and the reusable bevel layer followed ("Add a
> Winamp two-tone bevel layer and frame the Now Playing and equalizer panels",
> done 2026-10-06), framing those two panels through `bevel::lcd_well` and
> `bevel::raised_panel` in `src/ui/bevel.rs`. The transport-button chrome plan
> landed ("Style the transport buttons as raised Winamp chrome", done
> 2026-10-07), and both transport toggles are reachable: the Repeat toggle
> landed 2026-10-06 and the Shuffle toggle 2026-10-08, and their R and S
> keyboard bindings followed ("Add the R and S keyboard bindings for Repeat
> and Shuffle", done 2026-10-20). The rest of widget-level fidelity has
> landed too: slider chrome ("Style the volume and
> equalizer sliders as sunken Winamp grooves with raised chrome thumbs", done
> 2026-10-07), a custom title bar ("Add a Winamp custom title bar and drop the
> OS window frame", done 2026-10-07), and playlist chrome ("Frame the browse
> list as a sunken Winamp playlist editor", done 2026-10-07). The library side
> is real:
> `AppleMusicService` answers its browse and search queries through the REST
> client once a `MusicKit` session is stored, falling back to
> `sample_library()` otherwise — playlist queries included — and `play_track`
> plays the selected preview through the audio seam. Risk: the Winamp chrome
> arrived piecemeal, so later changes still need one consistent vocabulary.
> The equalizer-audio and seek plans this note pointed at have since landed
> (both 2026-10-20); the 2026-10-22 note below records drift that has since
> been resolved.

> **Steward drift note (2026-10-22 by the steward role; resolved 2026-10-23
> by the steward role).** That note recorded the built Now Playing bar and
> browse rows drifting from the brief's mockups — no elapsed readout, no
> `O A I D V` strip, no status row, album rows without running time, and a
> queued playback-position plan that put an `<elapsed>/<total>` readout at the
> LCD's right edge. Those drifts have since landed: "Add the classic elapsed
> readout to the Now Playing LCD and restore the plain total at its right
> edge", "Draw the classic O A I D V strip at the LCD's left edge", "Show the
> loaded preview's bitrate in the LCD status row", "Show each album row's
> running time", and "Show each artist row's album count" are done, and the
> playback-position plan is no longer queued. No current brief-vs-build drift
> is known. The LCD `MONO`/`STEREO` pair has since landed (done 2026-10-23),
> which brought the status row to `docs/screenshots/player.png` and
> `docs/screenshots/transport.png`, and the volume and balance slider fill
> has since landed (done 2026-10-23) with the orange and green grooves those
> mockups draw, and the playlist-number title prefix landed too (done
> 2026-10-23), matching `docs/screenshots/player.png`, and the transport
> glyph row landed on the same day. No planned entry remains.

## Planned

### Save volume, balance, Repeat, Shuffle, and the equalizer across launches (planned 2026-10-23 by the plan role)

**Goal.** The player already keeps its mixer and equalizer settings in `AppState` while it runs —
volume, balance, Repeat, Shuffle, the equalizer's on/off state, preamp, ten band gains, and the
selected preset — but every launch starts again at the `AppState::default()` values, and the README
says plainly that there are no config files. Persist exactly those user-set values in one small text
file, load them before the first frame, and rewrite the file whenever one of them changes, so a
restart restores the mix and curve. Playback position, the showing browse level, the search box,
window placement, and the current track are deliberately not persisted. No new dependency, no
network, and no UI change; this is independent of the title-band work (that change touched the
view tree only).

**Approach.**

- `src/settings.rs` (new): define `pub struct Settings` with private fields `volume`, `balance`,
  `repeat`, `shuffle`, `eq_enabled`, `eq_preamp`, `eq_bands`, and `eq_preset`, deriving
  `Debug, Clone, PartialEq`. Its `Default` mirrors `AppState::default` (0.5 volume, centered balance,
  both toggles off, equalizer off with a flat curve and no preset); document that the two defaults
  must change together. Add `pub fn from_state(&AppState) -> Settings`, reading the existing public
  getters and fields, and `pub fn apply_to(&self, &mut AppState)`, writing through the existing
  clamped setters (`set_volume`, `set_balance`, `set_eq_preamp`, each `set_eq_band`, then
  `apply_eq_preset` when `eq_preset` is `Some`) rather than assigning the private state fields.
- In the same module add private `to_text`/`from_text` for a line-based `key=value` format with keys
  `volume`, `balance`, `repeat`, `shuffle`, `eq_enabled`, `eq_preamp`, `eq_band_0` through
  `eq_band_9`, and `eq_preset`. Booleans are `true`/`false`; the preset is its `Preset::name` or an
  empty value for a custom curve. `from_text` starts from `Settings::default()`, ignores unknown
  keys, malformed numbers, bad booleans, out-of-range band indexes, and an unrecognized preset name,
  and matches a preset name against `equalizer::PRESETS`.
- In the same module add `pub trait SettingsWriter: Send + Sync { fn save(&self, settings: &Settings); }`
  and `pub struct FileStore { path: Option<PathBuf> }` (deriving `Clone`). `FileStore::from_environment`
  resolves `NOSTALGIA_SETTINGS_PATH` when set and non-empty, else `$XDG_CONFIG_HOME/nostalgia/settings.ini`,
  else `$HOME/.config/nostalgia/settings.ini`, else no path (persistence disabled); document that the
  environment variable is the cross-platform override and is read once at startup. Add
  `FileStore::at(PathBuf)` for tests, `FileStore::load(&self) -> Option<Settings>` (a missing or
  unreadable file returns `None`; the text is parsed with `from_text`), and the `SettingsWriter` impl
  whose `save` creates the parent directory when it has one, writes `to_text`, and logs a failed
  create/write with `eprintln!` instead of panicking.
- `src/main.rs`: declare `pub mod settings;`. Build the state, then load once before wrapping it in
  the shared mutex: `let store = settings::FileStore::from_environment(); let saved =
  store.load().unwrap_or_default(); saved.apply_to(&mut state);` and pass both the store and `saved`
  to `ui::init_ui`.
- `src/ui/mod.rs`: extend `init_ui` to take the store and the loaded `Settings`. In the iced
  application builder closure, after `boot`, set `player.settings_store = Some(Box::new(store.clone()))`
  and `player.saved_settings = saved.clone()`. In `update`, bind the dispatch `match` to a variable,
  call `player.persist_settings_if_changed()` once after it, and return the task, so every message
  path is covered without adding a save call to each handler.
- `src/ui/player.rs`: add `pub(in crate::ui) settings_store: Option<Box<dyn SettingsWriter>>` and
  `pub(in crate::ui) saved_settings: Settings` fields, initialized in `WinampPlayer::new` to `None`
  and `Settings::default()` (production `init_ui` replaces both; tests keep the no-op default). Add
  `pub(in crate::ui) fn persist_settings_if_changed(&mut self)`: return immediately when no store is
  attached; otherwise take one state lock, build `Settings::from_state`, return when it equals
  `saved_settings`, else call the store's `save` and store the new snapshot. Document that the
  snapshot comparison is what keeps timer and polling messages from writing.
- `src/ui/tests.rs`: add a test-only `MemorySettingsStore` over `std::sync::Mutex<Vec<Settings>>`
  implementing `SettingsWriter`. Add `a_changed_setting_is_saved_once_and_unchanged_updates_are_not`
  (attach the store, send `Message::VolumeChange(0.8)`, assert the saved snapshot equals
  `Settings::from_state` of the shared state, then send `Message::Ignored` and assert no second
  entry) and `repeat_and_equalizer_changes_reach_the_saved_snapshot` (send `Message::ToggleRepeat`
  and `Message::EqPresetSelected(rock_preset())` and assert the saved snapshot carries both). Keep
  these as plain `#[test]` functions, because `persist_settings_if_changed` uses `blocking_lock`.
- `src/settings.rs` module tests: pin `to_text`/`from_text` round-tripping every field including a
  selected preset; unknown keys, a non-numeric volume, a bad boolean, a bad band index, and an
  unknown preset name each leaving the default; `apply_to` clamping out-of-range file values through
  the state setters; and `FileStore::save` followed by `load` returning the same `Settings` through a
  unique path under `std::env::temp_dir()` (with the file removed afterward).
- `README.md`: replace the closing "There are no CLI flags or config files yet." sentence with a
  description of the settings file: volume, balance, Repeat, Shuffle, and the equalizer's on/off
  state, curve, and preset are restored at the next launch, the path is `$XDG_CONFIG_HOME/nostalgia/settings.ini`
  (or `$HOME/.config/...`), `NOSTALGIA_SETTINGS_PATH` overrides it, and missing or malformed entries
  fall back to the defaults. Do not touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: add a `## Settings persistence` section after `## UI layout` stating that the
  listed mixer/equalizer values are loaded once before the first view and rewritten after any update
  that changes their snapshot; that reads and writes go through the state's clamped setters; that a
  missing/unreadable file or a malformed line falls back per key without failing startup; and that
  playback position, browse level, search, current track, and window placement are not persisted.
  Name `settings::Settings::from_state`/`apply_to`, `settings::FileStore::load`/`save`,
  `ui::init_ui`, and `WinampPlayer::persist_settings_if_changed` as enforcers and the new test names
  as the pins.

**Files touched.** `src/settings.rs` (new), `src/main.rs`, `src/ui/mod.rs`, `src/ui/player.rs`,
`src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- At startup the player restores the volume, balance, Repeat and Shuffle flags, equalizer on/off
  state, preamp, ten band gains, and selected preset from the resolved settings file before the
  first view is drawn; a missing or unreadable file leaves `AppState::default()` values, and a
  malformed line affects only its own key.
- Changing any of those values through the UI writes the current snapshot to the file once; an
  update that leaves the snapshot unchanged writes nothing; the parent directory is created when
  needed and a write failure is logged rather than crashing.
- The text format round-trips every field (preset included), out-of-range values are clamped when
  loaded, and `NOSTALGIA_SETTINGS_PATH`, `$XDG_CONFIG_HOME`, and `$HOME/.config` resolve in that
  order with the first two honored only when non-empty.
- The settings tests pin the round-trip, the ignored malformed input, the clamping, and the file
  save/load; the UI tests pin save-on-change and no-save-on-unchanged; `make check` passes; the
  README and the new invariant section describe the same behavior; no dependency is added.

## Done
