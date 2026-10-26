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

### Show the current track's track number in the File info panel, like Winamp's ID3 panel (planned 2026-10-26 by the plan role)

**Goal.** Winamp's File info (ID3) dialog lists a track's number within its album beside its
year and genre. Nostalgia's File info panel names the title, artist, album, year, genre, length,
and loaded preview format, but `Song` carries no track number even though Apple Music reports
one. Carry the source's `attributes.trackNumber` through `Song` and the `KnownTrack` cache and
draw a `Track #: <n>` line between `Genre:` and `Length:` while the source supplies one. No new
state, handlers, settings, or playback behavior.

**Approach.**

- `src/library.rs`: add `#[serde(default)] pub track_number: Option<u32>` to `Song`,
  documented as the track's 1-based position on its album (`None` when the source supplied none
  or reported a non-positive number). Add the `"track_number": null` key to the test
  `song_payload`, and add `song_round_trips_its_track_number` (a `Some(3)` number serializes under
  the `track_number` key and survives the round trip) and
  `song_track_number_defaults_to_none_when_the_payload_omits_it` (a payload from
  `song_required_fields_payload` parses with `None`), beside the existing year and genre tests.
  Add `track_number` to every test-built `Song` literal the compiler reports, as the year and
  genre plans did.
- `src/apple_music/rest/wire.rs`: add `#[serde(rename = "trackNumber")] track_number:
  Option<u32>` to the private `Attributes`, documented as the track's 1-based position on its
  album with absence meaning the API omitted it. Carry it through `Resource::into_song`'s
  destructure and set `Song::track_number` to `track_number.filter(|number| *number > 0)`, so an
  absent number or an API-reported `0` (which no album can use) maps to the `None`
  "source supplied none" sentinel. Update `into_song`'s doc comment to name the track number
  among the fields it moves into the model.
- `src/sample_library.rs`: give the five sample songs their album positions — `1`, `2`, and `3`
  for Opening, Middle, and Ending on First Record, and `1` for B-side on Second Record and
  Headliner on Debut — so the signed-out File info panel shows a number too. Add
  `songs_carry_the_documented_track_numbers` beside `songs_carry_the_documented_genres`,
  pinning every song's id and number in library order.
- `src/ui/views/now_playing/text.rs`: add `pub track_number: Option<u32>` to `KnownTrack`,
  documented as copied from `Song::track_number`, with `None` meaning the source supplied none.
  `src/ui/handlers/playback.rs` records `track_number: song.track_number` in the `KnownTrack` it
  inserts.
- `src/ui/views/file_info.rs`: add `const TRACK_NUMBER_LABEL: &str = "Track #: ";` and push a
  `Track #: <n>` line after the conditional genre line and before the `Length:` line only while
  `track.track_number` is `Some`; no line otherwise. Update the module and view doc comments to
  name the track number.
- Tests: add `songs_from_album_maps_the_track_number` (attributes `"trackNumber":3` maps to
  `Some(3)`) and `songs_from_album_maps_a_missing_or_zero_track_number_to_none` (absent, `0`, and
  `null` all map to `None`) to `src/apple_music/rest/tests/library.rs` through the existing
  `song_from_attributes` helper; a focused `known_tracks_record_the_played_songs_track_number` in
  `src/ui/tests/readouts_tests.rs` that plays a song carrying `Some(3)` and asserts the recorded
  `KnownTrack`; and `file_info_overlay_draws_the_tracks_track_number` (the line sits between the
  genre and `Length:` lines) plus
  `file_info_overlay_omits_the_track_number_line_when_the_source_supplied_none` in
  `src/ui/views/tests/chrome.rs`. Add `track_number: None` to the `sample_known_track` fixture in
  `src/ui/views/tests/mod.rs` and to every other `KnownTrack` test literal the compiler reports.
- `docs/usage.md`: in the File info sentence, name the track-number line and say it is left out
  when the source supplies none. `docs/invariants.md` `## File info panel`: add the `Track #: …`
  line (after `Genre: …`, before `Length: …`, omitted when `KnownTrack::track_number` is `None`,
  sourced from `Song::track_number`, itself taken from the resource's `attributes.trackNumber`
  with absent and `0` mapped to `None`), name the new tests in `Pinned by`, and leave every other
  guarantee unchanged.

**Files touched.** `src/library.rs`, `src/apple_music/rest/wire.rs`,
`src/apple_music/rest/tests/library.rs`, `src/sample_library.rs`, `src/test_support.rs`,
`src/ui/views/now_playing/text.rs`, `src/ui/handlers/playback.rs`, `src/ui/views/file_info.rs`,
`src/ui/views/tests/chrome.rs`, `src/ui/views/tests/mod.rs`, `src/ui/tests/readouts_tests.rs`
(plus `track_number` in the other `Song`/`KnownTrack` test literals), `docs/usage.md`,
`docs/invariants.md`.

**Acceptance criteria.**

- A REST song resource carrying `attributes.trackNumber` `3` maps to `Song::track_number ==
  Some(3)`; an absent, `null`, or `0` track number maps to `None`, and a resource without the
  attribute still parses.
- Playing a song whose `Song::track_number` is `Some(3)` records `KnownTrack::track_number ==
  Some(3)`.
- The File info panel draws `Track #: 3` between the `Genre:` and `Length:` lines when the
  number is present and draws no track-number line when it is `None`; the title, artist, album,
  year, genre, length, format, `Close`, and backdrop behavior stay as they are.
- The sample library's songs carry the documented album positions, so the signed-out panel shows
  a track number.
- `make check` passes.

### Show `Custom` in the equalizer preset pick list for a hand-tuned curve (planned 2026-10-26 by the plan role)

**Goal.** Classic Winamp's equalizer preset pick list names the applied preset, and reads
`Custom` once a slider has made the curve the user's own. Nostalgia's pick list instead shows a
lighter `(none)` placeholder whenever `AppState::eq_preset` is `None`, which is also the state
every hand-tuned curve reaches (`set_eq_preamp`, `set_eq_band`, and `set_all_eq_bands` clear the
preset). Show `Custom` as the closed label for that state, in the same text color as a selected
preset, so the pick list reads as a value rather than an empty control. State, handlers, preset
application, and audio forwarding are untouched.

**Approach.**

- `src/ui/views/equalizer.rs`: add a documented `pub(super) const CUSTOM_PRESET_LABEL: &str =
  "Custom";` beside `eq_enabled_label`, and pass it to `PickList::placeholder` in
  `view_equalizer` in place of `"(none)"`. Update `view_equalizer`'s doc comment (the `None`
  branch now draws the `Custom` label) and keep the comment explaining that the slice borrow
  avoids copying `PRESETS` on every rebuild.
- `src/ui/style.rs`: in `chrome_pick_list_style`, change `placeholder_color` to
  [`theme::TEXT`] and rewrite the doc comment: the placeholder now names a real `Custom` curve,
  so it is drawn like the selected preset rather than as an empty box; the hover/opened face and
  border rules stay as they are. Update the assertion in
  `chrome_pick_list_style_sinks_the_face_when_opened` to expect `theme::TEXT`.
- `src/ui/views/tests/equalizer.rs`: add
  `equalizer_pick_list_shows_custom_for_a_hand_tuned_curve`, which lays out
  `view_equalizer(false, 0.0, &[0.0; BAND_COUNT], None)` through
  `layout_capture::capture_text` and asserts the captured texts contain `CUSTOM_PRESET_LABEL`,
  then lays out the same panel with `Some(PRESETS[0])` and asserts it draws that preset's name
  and no `CUSTOM_PRESET_LABEL`. Import `CUSTOM_PRESET_LABEL` in the existing
  `super::equalizer::{…}` list in `src/ui/views/tests/mod.rs`.
- `docs/usage.md`: in the equalizer paragraph, say the preset pick list names the applied
  preset and reads `Custom` once a slider move makes the curve your own.
- `docs/invariants.md`: in `## UI layout`, add a bullet stating the preset pick list shows the
  applied preset's name, or `Custom` when `AppState::eq_preset` is `None`, in `theme::TEXT`;
  name `view_equalizer`/`CUSTOM_PRESET_LABEL` and `chrome_pick_list_style` in `Enforced by`, and
  add the new test to `Pinned by`. Leave the equalizer band-row, tint, and scale guarantees
  unchanged.

**Files touched.** `src/ui/views/equalizer.rs`, `src/ui/views/tests/equalizer.rs`,
`src/ui/views/tests/mod.rs`, `src/ui/style.rs`, `docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**

- With no applied preset (`eq_preset == None`), the equalizer's closed pick list draws `Custom`;
  with `Some(PRESETS[0])` applied it draws `Flat` and not `Custom` (headless layout capture).
- `chrome_pick_list_style` returns `theme::TEXT` for `placeholder_color`; its updated style test
  passes.
- `AppState::eq_preset` still becomes `None` on a hand-tuned preamp or band change and `Some` on
  a preset pick, picking a preset still emits `Message::EqPresetSelected`, and the saved settings
  format is unchanged.
- `make check` passes.

## Done
