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

## Done
