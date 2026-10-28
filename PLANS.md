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

## Done
