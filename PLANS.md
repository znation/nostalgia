# Plans

Planned features, written by the plan loop and implemented by the feature loop.
Each plan: goal, approach, files touched, acceptance criteria. Move finished plans to Done.

> **Steward drift note (2026-10-06; condensed 2026-10-23 by the steward role).** This note
> tracked the initial prompt's two commitments — a "very close if not pixel-perfect" classic
> Winamp UI and Apple Music as the library — while the build caught up. Both are now reflected
> in the shipped app and the usage reference: the Winamp chrome (base-skin palette,
> bevel layer, custom title bar, title bands, Now Playing LCD, browse selection and row
> styling, compact and mini transport strips) and the Apple Music library (REST browse and
> search behind the `AppleMusicService` seam with the sample library as fallback, and preview
> playback through the audio seam). The per-landing detail this note once carried is retired
> here; `PLANS-DONE.md` holds the individual entries, and no current brief-vs-build drift is
> known. Standing lesson: the chrome arrived piecemeal, so later changes should reuse the
> existing vocabulary — `bevel::lcd_well`, `bevel::raised_panel`, and the shared style
> helpers — rather than inventing new chrome. The README's detailed Usage reference moved to
> `docs/usage.md` on 2026-10-26, so plans that change usage prose edit that file, not the README.
> Record any new drift here rather than appending another note.

## Planned

_None yet._


### Show the current track's content rating (Explicit or Clean) in the File info panel (planned 2026-10-28 by the plan role)

**Goal.** Apple Music marks each song `explicit` or `clean` in `attributes.contentRating`.
`library-songs` resources carry this attribute, and the Apple Music apps show it as the "E" badge.
Nostalgia drops it. The File info panel (Alt+3) already lists the tag-like
fields Winamp's ID3 panel shows (title through composer). Add one more line, `Rating: Explicit` or
`Rating: Clean`, so the panel names what the library knows about the track. Leave the line out
when the source supplied no rating.

**Approach.** Follow the path the `Composer:` line took (see `docs/invariants.md`, the File info
section), one field end to end:
1. **Model (`src/library.rs`).** Add `pub enum ContentRating { Explicit, Clean }` deriving
   `Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize` with
   `#[serde(rename_all = "lowercase")]`. Give it `pub fn label(self) -> &'static str`, which returns
   `"Explicit"` or `"Clean"`. Add `Song::content_rating: Option<ContentRating>` with
   `#[serde(default)]`, after `composer`, with a doc comment in the style of the `composer` field's.
2. **Wire (`src/apple_music/rest/wire.rs`).** Add `#[serde(rename = "contentRating")]
   content_rating: Option<String>` to `Attributes`, beside `composer_name`.
3. **Mapping (`src/apple_music/rest/wire/mapping.rs`).** In `Resource::into_song`, destructure the
   new attribute (extend the tuple and its `None` arm). Map it through a new private
   `fn content_rating(raw: Option<String>) -> Option<ContentRating>`. That function trims the value,
   then matches `explicit` → `Explicit` and `clean` → `Clean` ignoring ASCII case
   (`eq_ignore_ascii_case`). Anything else, including absent, blank, or an unknown value, maps to
   `None`. Add "content rating" to `into_song`'s doc list of fields that fall back to their "source
   supplied none" sentinel.
4. **Known track.** Add `pub content_rating: Option<ContentRating>` to `KnownTrack`
   (`src/ui/views/now_playing/text.rs`), documented like `disc_number`. In
   `src/ui/handlers/playback.rs`, the `known_tracks.insert` in the track-selection handler copies it
   from `song.content_rating`.
5. **Panel (`src/ui/views/file_info.rs`).** Add `const CONTENT_RATING_LABEL: &str = "Rating: ";`.
   In `view_file_info_overlay`, push `Rating: <label>` after the composer line and before
   `Length:`, only while `track.content_rating` is `Some`. Update the module doc and the function
   doc's list of lines.
6. **Literals.** Every `Song { … }` and `KnownTrack { … }` struct literal gains
   `content_rating: None`. The `composer: ` literals show where they are: `src/sample_library.rs`,
   `src/test_support/fixtures.rs`, `src/apple_music/tests/mod.rs`,
   `src/apple_music/rest/tests/library.rs`, `src/ui/tests/mod.rs`, `src/ui/views/tests/mod.rs`, and
   `src/ui/views/tests/chrome.rs` (`sample_known_track`). Literals that end in `..` need nothing.
   The sample library keeps `None` on every song, because its songs are invented and have no rating.

**Files touched.** `src/library.rs`, `src/apple_music/rest/wire.rs`,
`src/apple_music/rest/wire/mapping.rs`, `src/ui/views/now_playing/text.rs`,
`src/ui/handlers/playback.rs`, `src/ui/views/file_info.rs`, the test files and fixtures listed in
step 6, `src/ui/tests/readouts_tests.rs`, `docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**
- `src/apple_music/rest/tests/library.rs`: `songs_from_album_maps_the_content_rating` checks that
  `"explicit"`, `"clean"` and `" Explicit "` map to `Some(Explicit)`, `Some(Clean)` and
  `Some(Explicit)`. `songs_from_album_maps_a_missing_blank_or_unknown_content_rating_to_none`
  covers an absent key, `null`, `""`, `"   "` and `"notExplicit"`. Both go through the existing
  `song_from_attributes` helper.
- `src/library.rs` tests: `song_round_trips_its_content_rating` checks that `Some(Explicit)`
  serializes as `"explicit"` and round-trips. `song_content_rating_defaults_to_none_when_the_payload_omits_it`
  covers the omitted field. The existing every-field-required and unknown-field tests pass.
- `src/ui/tests/readouts_tests.rs`: `known_tracks_record_the_played_songs_content_rating`, in the
  style of `known_tracks_record_the_played_songs_composer`.
- `src/ui/views/tests/chrome.rs`: `file_info_overlay_draws_the_tracks_content_rating` checks that
  `Rating: Explicit` sits after `Composer: …` and before `Length: …`.
  `file_info_overlay_omits_the_content_rating_line_when_the_source_supplied_none` uses
  `assert_no_file_info_line(&texts, "Rating:")`.
- `docs/usage.md`'s Alt+3 sentence names the content rating among the panel's lines and among the
  lines left out when the source supplies none. The File info section of `docs/invariants.md`
  describes the `Rating: …` line and its `contentRating` mapping, and its Pinned-by list names the
  new tests.
- `make check` passes.

### Remember the window's screen position across launches, like Winamp (planned 2026-10-28 by the plan role)

**Goal.** Winamp reopens where it was last left on screen. Nostalgia saves every other layout
choice (the readout mode, visualizer, EQ and library panels, always-on-top, shade mode), but the
window opens wherever the window manager puts it on every launch. Persist the window's last
reported screen position in the settings file and open the next launch there. The window is not
resizable (`init_ui` calls `.resizable(false)`), so position is the only placement worth saving;
size stays unpersisted. On Wayland iced reports no moves and ignores a requested position, so
there the line is never written and behavior is unchanged.

**Approach.**

- `src/settings.rs`: add a private `window_position: Option<(f32, f32)>` field, kept as plain
  floats so the settings module stays free of iced types. `Settings::default` and
  `Settings::from_state` leave it `None`. Add `window_position()` / `set_window_position(Option<(f32,
  f32)>)` accessors in the same style as `shaded()` / `set_shaded`. `apply_to` leaves it alone (it is
  player-owned like the other six). Update the module, struct, and `shaded()` docs: the "window's
  size and position are deliberately not persisted" sentences now say the position is persisted
  and the size is not; the UI-owned count goes from six to seven.
- `src/settings/text.rs`: `to_text` writes `window_position=<x>,<y>\n` (f32 `Display`, so `120,80`
  or `120.5,-40`) only when the field is `Some`, so files written without a known position stay
  as they are. `from_text` handles a `"window_position"` key with a new `parse_window_position`
  helper: `split_once(',')`, trim each half, parse both as `f32`, and accept the pair only when
  both are finite (`f32::is_finite`, which rejects NaN and the infinities). A malformed value
  leaves the field unchanged, matching `parse_into`'s rule. Negative coordinates are valid,
  because a monitor left of or above the primary has them.
- `src/ui/player.rs`: add a `window_position: Option<(f32, f32)>` field on `WinampPlayer` (initialized
  `None` in `WinampPlayer::new`). `persist_settings_if_changed` calls
  `snapshot.set_window_position(self.window_position)`, and `adopt_saved_settings` restores
  `self.window_position = saved.window_position()`. Update both doc comments.
- `src/ui/message.rs`: add `WindowMoved(iced::Point)`. `src/ui/mod.rs`'s `update` routes it to a new
  `handlers::window::window_moved(player, point)` in `src/ui/handlers/window.rs`, which stores
  `Some((point.x, point.y))` and returns `Task::none()`. `update`'s existing trailing
  `persist_settings_if_changed` call then saves it. A drag-move sends many events, and
  `BackgroundWriter` already collapses a burst to its newest snapshot.
- `src/ui/subscriptions/listeners.rs`: add `window_moved_message(event, status, window)`, mapping
  `iced::Event::Window(iced::window::Event::Moved(point))` to `Some(Message::WindowMoved(point))`
  and every other event to `None`. Re-export it from `src/ui/subscriptions/mod.rs` beside
  `modifiers_message`, and push `iced::event::listen_with(window_moved_message)` into the
  always-on `subscriptions` vector there. Extend the `subscriptions` doc comment.
- Restore at launch: add `pub(in crate::ui) fn launch_position(saved: Option<(f32, f32)>) ->
  iced::window::Position` in `src/ui/handlers/window.rs`, returning `Position::Specific(Point::new(x,
  y))` for `Some` and `Position::Default` for `None`. `init_ui` in `src/ui/mod.rs` chains
  `.position(handlers::window::launch_position(saved.window_position()))` on the
  `iced::application` builder (iced 0.14's `Application::position`) before `.run()`. The
  `saved` snapshot is already in scope there; read it before the boot closure moves it, or clone it.
  Decided, not left open: no clamping to monitor bounds, because iced cannot query monitors
  before the window opens. A position saved on a since-removed monitor is restored as saved,
  and the usage doc names deleting the `window_position` line as the fix.

**Files touched.** `src/settings.rs`, `src/settings/text.rs`, `src/ui/player.rs`, `src/ui/message.rs`,
`src/ui/mod.rs`, `src/ui/handlers/window.rs`, `src/ui/subscriptions/listeners.rs`,
`src/ui/subscriptions/mod.rs`, `src/ui/tests/settings_tests.rs` (or the listener tests' module),
`docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `src/settings.rs` tests: a new `window_position_round_trips_and_ignores_malformed_values` shows
  that `Some((120.0, -40.5))` survives `to_text` → `from_text`, that a default snapshot's text
  has no `window_position` line, and that `window_position=abc`, `window_position=10`,
  `window_position=NaN,5`, and `window_position=inf,5` each leave it `None`.
  `to_text_round_trips_every_field` (with `state_with_every_field_set` or its caller) sets a
  position so the full round trip covers it.
- `src/ui/tests/settings_tests.rs`: `window_moved_reaches_the_saved_snapshot` attaches a
  `MemorySettingsStore`, sends `Message::WindowMoved(Point::new(300.0, 200.0))`, and sees one
  snapshot with `window_position() == Some((300.0, 200.0))`. A repeat of the same point saves
  nothing new. `saved_window_position_is_restored_on_boot` shows that `adopt_saved_settings` sets
  `player.window_position` from the snapshot and back to `None` from `Settings::default()`.
- A listener test shows that `window_moved_message` maps a `Moved(point)` window event to
  `Message::WindowMoved(point)` and returns `None` for `window::Event::Focused` and a key press.
- A test of `launch_position` shows `Some((10.0, 20.0))` gives `Position::Specific` at that point
  (checked with `matches!`, since `Position` has no `PartialEq`) and `None` gives
  `Position::Default`.
- `docs/usage.md`'s settings sentence names the window's screen position among the saved values,
  says it is not restored on Wayland, and says deleting the `window_position` line resets an
  off-screen window. The "There are no CLI flags yet" sentence stays. `docs/invariants.md`'s
  `## Settings persistence` section names the position in its Guarantee and its malformed-line
  fallback. It changes the "size and position are not persisted" sentences there to say only
  the size is not, and its Pinned-by list names the new tests.
- `make check` passes.

## Done
