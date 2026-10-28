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

### Show the current track's disc number in the File info panel, like Winamp's ID3 panel (planned 2026-10-28 by the plan role)

**Goal.** Winamp's File info dialog lists a track's position in a multi-disc set beside its track
number. Nostalgia's File info panel shows the title, artist, album, year, genre, track number,
composer, length, and loaded preview format. `Song` carries no disc number even though Apple
Music reports one as `attributes.discNumber`. This change carries that attribute through `Song`
and the `KnownTrack` cache. The panel then draws a `Disc #: <n>` line between `Track #:` and
`Composer:` while the source supplies a positive number. It adds no new state, handlers,
settings, or playback behavior, and it leaves search and sorting alone. It mirrors the finished
"Show the current track's composer in the File info panel" and "Show the current track's track
number in the File info panel" plans in PLANS-DONE.md. It does not depend on the Recently Played
series. Both edit `src/apple_music/rest/wire/mapping.rs`, but this plan touches only
`Resource::into_song` (plus a new private helper) there, and Recently Played 1/2 adds
`Resource::is_song` beside `Resource::is_library_album`.

**Approach.**

- `src/library.rs`: add `#[serde(default)] pub disc_number: Option<u32>` to `Song`, after
  `track_number`. Its doc comment follows `Self::track_number`'s: the track's 1-based disc in its
  album's set; `None` when the source supplied none or reported a non-positive number; the File
  info panel draws a `Disc #:` line only while it is `Some`. In the test module:
  - add the `"disc_number": null` key to `song_payload`;
  - add `song_round_trips_its_disc_number` and
    `song_disc_number_defaults_to_none_when_the_payload_omits_it` (built from
    `song_required_fields_payload`) beside `song_round_trips_its_track_number`.
- `src/apple_music/rest/wire.rs`: add `#[serde(rename = "discNumber")] disc_number:
  Option<serde_json::Number>` to the private `Attributes`, after `track_number`. Its doc comment
  matches `track_number`'s (parsed as a JSON number so an out-of-`u32` value maps to `None`
  instead of failing the whole response).
- `src/apple_music/rest/wire/mapping.rs`: move the `track_number` chain in `Resource::into_song`
  (`as_u64`, `> 0`, `u32::try_from`) into a private
  `fn positive_position(number: Option<serde_json::Number>) -> Option<u32>` beside
  `primary_genre`, carrying the existing comment as its doc comment. Map both
  `track_number` and `disc_number` through it. Carry `disc_number` through the attributes
  destructure (add one more `None` to the fallback tuple). Name the disc number in
  `into_song`'s doc comment among the fields it falls back for.
- `src/sample_library.rs`: give Opening, Middle, and Ending (First Record) `disc_number:
  Some(1)`, and give B-side and Headliner `None`. Signed out, the panel then shows both the line
  and its omission. Add `songs_carry_the_documented_disc_numbers` beside
  `songs_carry_the_documented_composers`, pinning every song's id and disc number in library
  order.
- `src/ui/views/now_playing/text.rs`: add `pub disc_number: Option<u32>` to `KnownTrack`, after
  `track_number`. Its doc comment says it is copied from `Song::disc_number`, and that `None`
  means the panel leaves the line out. In `src/ui/handlers/playback.rs`, the `KnownTrack` literal
  that records `track_number: song.track_number` also records `disc_number: song.disc_number`.
- `src/ui/views/file_info.rs`: add `const DISC_NUMBER_LABEL: &str = "Disc #: ";` between
  `TRACK_NUMBER_LABEL` and `COMPOSER_LABEL`. In `view_file_info_overlay`, push a
  `Disc #: <n>` line after the conditional track-number line and before the conditional composer
  line, only while `track.disc_number` is `Some`. Name the disc number in the module and
  `view_file_info_overlay` doc comments.
- Tests:
  - In `src/apple_music/rest/tests/library.rs`, use the existing `song_from_attributes` helper.
    `songs_from_album_maps_the_disc_number` checks that `"discNumber":2` maps to `Some(2)`.
    `songs_from_album_maps_a_missing_zero_negative_or_oversized_disc_number_to_none` checks that
    an absent, `null`, `0`, `-1`, and `4294967296` disc number each map to `None`. The existing
    track-number tests stay and pin that the helper extraction left that mapping unchanged.
  - In `src/ui/tests/readouts_tests.rs`, add `known_tracks_record_the_played_songs_disc_number`,
    modelled on `known_tracks_record_the_played_songs_track_number`.
  - In `src/ui/views/tests/chrome.rs`, add `file_info_overlay_draws_the_tracks_disc_number` (with
    a track number, disc number, and composer all set, the `Disc #: 2` line sits after
    `Track #:` and before `Composer:`) and
    `file_info_overlay_omits_the_disc_number_line_when_the_source_supplied_none`, modelled on the
    two composer tests there.
  - Add `disc_number: None` to `sample_known_track` in `src/ui/views/tests/chrome.rs`, and to every
    other `Song` and `KnownTrack` literal the compiler reports (the composer commit touched
    `src/test_support/fixtures.rs`, `src/apple_music/tests/mod.rs`,
    `src/apple_music/rest/tests/library.rs`, `src/ui/views/tests/chrome.rs`, and several
    `src/ui/tests/*.rs` files).
- `docs/usage.md`: in the Alt+3 File info sentence, add the disc number after the track number in
  the list of lines, and add it to the parenthetical of lines left out when the source supplies
  none.
- `docs/invariants.md` `## File info panel`: add `Disc #: …` (left out when
  `KnownTrack::disc_number` is `None`) after `Track #: …` and before `Composer: …`. Add a
  sentence that it is copied from `Song::disc_number`, which `Resource::into_song` takes from
  `attributes.discNumber` with the same absent/`null`/non-positive/above-`u32::MAX` → `None`
  rule as the track number. Name the new tests in its `Pinned by`, and leave every other
  guarantee unchanged.

**Files touched.** `src/library.rs`, `src/apple_music/rest/wire.rs`,
`src/apple_music/rest/wire/mapping.rs`, `src/apple_music/rest/tests/library.rs`,
`src/sample_library.rs`, `src/ui/views/now_playing/text.rs`, `src/ui/handlers/playback.rs`,
`src/ui/views/file_info.rs`, `src/ui/views/tests/chrome.rs`,
`src/ui/tests/readouts_tests.rs`, plus the one-line `disc_number` field in the other test-built
`Song`/`KnownTrack` literals, `docs/usage.md`, and `docs/invariants.md`.

**Acceptance criteria.**

- A REST song resource whose `attributes.discNumber` is `2` maps to `Song::disc_number ==
  Some(2)`. An absent, `null`, zero, negative, or above-`u32::MAX` disc number maps to `None`,
  and a resource without the attribute still parses.
- The track-number mapping behaves as before (its existing tests pass unchanged).
- A serialized `Song` round-trips its disc number, and a payload without the `disc_number` key
  parses with `None`.
- The sample library's First Record songs carry `Some(1)`, and B-side and Headliner carry `None`.
- Playing a song records its disc number in the `KnownTrack` the File info panel reads.
- With `Some(n)`, the File info panel draws `Disc #: n` after the `Track #:` line (or after
  whichever earlier line is present) and before `Composer:` (or `Length:` when there is no
  composer). With `None` it draws no `Disc #:` line, and every other line is unchanged.
- `docs/usage.md` and `docs/invariants.md` name the new line as above.
- `make check` passes.

### Open the Jump to time box holding the current position, selected, like Winamp (planned 2026-10-28 by the plan role)

**Goal.** Winamp's Jump to time dialog opens with its box already holding the playing track's
current position, so you can see where you are and type over it. Nostalgia's Ctrl+J panel opens
with an empty box (`handlers::jump::toggle_jump_time` calls `player.jump_time_query.clear()`).
Opening should instead fill the box with the current position in the `M:SS` form that
`parse_jump_time` already accepts, and select all of it, so typing replaces it and Enter on the
untouched text seeks to where playback already is.

**Approach.**
- `src/ui/views/mod.rs`: make `format_minutes_seconds` `pub(in crate::ui)`, so the handler can
  reuse it. Keep its doc comment and add the Jump to time prefill to the list of its users. Do not
  use `format_track_time`, because it turns `0` into `--:--`, which `parse_jump_time` rejects.
- `src/ui/handlers/jump.rs` `toggle_jump_time`: replace `player.jump_time_query.clear()` with:
  set `jump_time_query` to `views::format_minutes_seconds(ms)` when `player.playback_position` is
  `Some(position)`, where `ms` is `u64::try_from(position.as_millis()).unwrap_or(u64::MAX)` (the
  same conversion `view_jump_time_overlay` uses for the length). When it is `None`, clear the query.
  Leave the position unclamped: `jump_time_submitted` already clamps to `playback_duration`.
  Replace the `focus` task with
  `Task::batch([iced::widget::operation::focus(views::JUMP_TIME_INPUT_ID), iced::widget::operation::select_all(views::JUMP_TIME_INPUT_ID)])`
  (`select_all` is in iced_runtime 0.14's `widget::operation`, next to `focus`). Keep the
  existing shaded branch, which batches `toggle_window_shade` before the focus task. Update the
  doc comment: opening now fills the query with the position, or clears it when there is none,
  where it used to just clear the query. The Options menu's `Jump to time` row calls this same
  function, so it gets the prefill with no change of its own.
- `src/ui/tests/jump_time_tests.rs`:
  - Keep `toggle_opens_with_an_empty_query_and_closes_again` for the `None` position.
    `player_with_duration` sets no position, so it must still see an empty query after a stale one.
  - Add `toggle_opens_holding_the_current_position`. With `playback_position = Some(95 s)`, opening
    leaves `jump_time_query == "1:35"` and schedules work.
  - Add `toggle_opens_holding_zero_at_the_start`. `Some(Duration::ZERO)` gives `"0:00"`, not `--:--`.
  - Add `submitting_the_prefilled_query_seeks_to_the_current_position`. Open at `Some(12 s)` with
    the existing 30 s duration, send `JumpTimeSubmitted` without typing, and assert three things,
    the way `submit_seeks_records_the_position_and_closes` does: `playback_position` is still
    `Some(12 s)`, `position_epoch` went up by one (so a seek really ran), and the panel is closed.
- `docs/usage.md`: in the Ctrl+J sentence, say the box opens holding the current position (`1:35`),
  selected so typing replaces it, or empty when no position is known.
- `docs/invariants.md` `## Jump to time panel`: change "Opening clears the query" to "Opening
  sets the query to `format_minutes_seconds` of `playback_position`, or clears it when that is
  `None`, and focuses and selects the box". Add the new tests to **Pinned by**.

**Files touched.** `src/ui/handlers/jump.rs`, `src/ui/views/mod.rs`,
`src/ui/tests/jump_time_tests.rs`, `docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**
- With a known preview length and a playback position of 95 s, Ctrl+J (or the Options menu's
  `Jump to time` row) opens the panel with `1:35` in the box. A position of 0 shows `0:00`.
- With no known position, the box opens empty, as it does today.
- Opening focuses the box and selects its text, so a typed time replaces the prefill.
- Enter on the untouched prefill seeks to the current position and closes the panel.
- Closing, the other panels' mutual closing, and the shaded restore behave as before. The existing
  tests in `jump_time_tests.rs` pass unchanged.
- `docs/usage.md` and `docs/invariants.md` describe the prefill.
- `make check` passes.

## Done
