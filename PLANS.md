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

### Show the current track's genre in the File info panel, like Winamp's ID3 panel (planned 2026-10-26 by the plan role)

**Goal.** Winamp's File info dialog lists a track's genre beside its year. Nostalgia's File info
panel already names the title, artist, album, year, length, and loaded preview format, but
`Song` carries no genre, so no genre can reach the panel. Carry the source's primary genre through
`Song` and the `KnownTrack` cache and draw a `Genre: <name>` line between `Year:` and `Length:`
while the source supplies one. No new state, handlers, settings, or playback behavior.

**Approach.**

- `src/library.rs`: add `#[serde(default)] pub genre: String` to `Song`, documented as the primary
  genre name with `""` meaning the source supplied none (the same sentinel `Song::album` uses);
  the File info panel draws a line only while it is non-empty. Add the `"genre": ""` key to the
  test `song_payload`, add a round-trip test pinning that `"Rock"` serializes and deserializes
  under the `genre` key, and add a test that a payload without `genre` defaults to `""`.
- `src/apple_music/rest/wire.rs`: add `#[serde(rename = "genreNames")] genre_names:
  Option<Vec<String>>` to the private `Attributes`, carry it through `Resource::into_song`'s
  destructure, and set `Song::genre` from a new private
  `primary_genre(genres: Option<Vec<String>>) -> String` helper. The helper returns the first
  entry whose trimmed form is non-empty, trimmed, and `String::new()` when the array is absent,
  empty, or all blank, so Apple's `["Rock", "Music"]` yields `"Rock"` without the umbrella genre
  and a blank entry never hides a later real one. Document it beside `artist_or_empty`.
- `src/sample_library.rs`: give the five sample songs explicit genres (`"Rock"` for the three
  First Record tracks, `"Alternative"` for B-side, and `"Electronic"` for Headliner), so the
  signed-out File info panel shows one too. Add `songs_carry_the_documented_genres` beside
  `songs_carry_the_documented_years`, pinning every song's id and genre in library order.
- `src/ui/views/now_playing/text.rs`: add `pub genre: String` to `KnownTrack`, documented as
  copied from `Song::genre` with `""` meaning the source supplied none. `src/ui/handlers/playback.rs`
  records `genre: song.genre.clone()` in the `KnownTrack` it inserts.
- `src/ui/views/file_info.rs`: add `const GENRE_LABEL: &str = "Genre: ";` and push a
  `Genre: <name>` line after the year line and before the length line only while
  `!track.genre.is_empty()`; no line otherwise. Update the module and view doc comments to name
  the genre.
- Tests: add `songs_from_album_maps_the_genre` (a resource with `"genreNames":["Rock","Music"]`
  maps to `"Rock"`) and `songs_from_album_maps_a_missing_or_blank_genre_to_empty` (absent, `[]`,
  `[""]`, and `["  "]` map to `""`, while `["", "Rock"]` maps to `"Rock"`) to
  `src/apple_music/rest/tests/library.rs`, through the existing `song_from_attributes` helper. Add
  a focused `known_tracks_record_the_played_songs_genre` to `src/ui/tests/readouts_tests.rs` that
  plays a song carrying a genre and asserts the recorded `KnownTrack`. In
  `src/ui/views/tests/chrome.rs`, add `file_info_overlay_draws_the_tracks_genre` (asserting
  `Year:` < `Genre:` < `Length:` in the drawn text) and
  `file_info_overlay_omits_the_genre_line_when_the_source_supplied_none`. Add `genre` to every
  test-built `Song` and `KnownTrack` literal the compiler reports, using `String::new()` where the
  fixture is not about genre.
- `docs/usage.md`: in the File info sentence, name the genre line and say the album, year, and
  genre lines are left out when the source supplies none. `docs/invariants.md` `## File info
  panel`: add the `Genre: …` line (after `Year: …`, omitted when `KnownTrack::genre` is empty,
  sourced from `Song::genre`, itself parsed from `attributes.genreNames` by `Resource::into_song`
  through `primary_genre`), name the new tests in `Pinned by`, and leave every other guarantee
  unchanged.

**Files touched.** `src/library.rs`, `src/apple_music/rest/wire.rs`,
`src/apple_music/rest/tests/library.rs`, `src/sample_library.rs`, `src/test_support.rs`,
`src/ui/views/now_playing/text.rs`, `src/ui/handlers/playback.rs`, `src/ui/views/file_info.rs`,
`src/ui/views/tests/chrome.rs`, `src/ui/tests/readouts_tests.rs` (plus `genre` in the other
`Song`/`KnownTrack` test literals the compiler reports), `docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**

- A REST song resource carrying `attributes.genreNames` `["Rock", "Music"]` maps to
  `Song::genre == "Rock"`; a leading blank or empty entry is skipped; an absent, empty, or
  all-blank `genreNames` maps to `""`.
- `Song` round-trips a non-empty genre under the `genre` key, and a payload without `genre`
  defaults to `""`.
- Playing a song whose `Song::genre` is `"Rock"` records `KnownTrack::genre == "Rock"`.
- The File info panel draws `Genre: Rock` between `Year:` and `Length:` when a genre is present
  and draws no Genre line when it is empty; the title, artist, album, year, length, format,
  Close, and backdrop behavior stay as they are.
- The sample library's songs carry genres, so the signed-out panel shows one.
- `make check` passes.

### Number the Jump to File rows and mark the match Enter plays (planned 2026-10-26 by the plan role)

**Goal.** The Jump to File panel lists matching titles flat, with no indication of which row
Enter or `Jump` will play, and unlike the browse rows they carry no position. Prefix each row
with its 1-based position in the showing list (the same number the browse rows carry) and draw
the first match with the browse list's blue selection bar and white text, making the Enter target
visible. Filtering, playback, row presses, and state stay as they are.

**Approach.**

- `src/ui/views/jump_file.rs`:
  - Add `pub(crate) fn jump_file_row_label(index: usize, title: &str) -> String` returning
    `format!("{}. {title}", index + 1)`, documented as the 1-based source position of the same
    0-based `index` that `jump_file_matches` returns and a row press dispatches, so a filtered
    list keeps the number its source list shows.
  - Add `pub(crate) fn jump_file_row_style(is_default: bool, status: iced::widget::button::Status)
    -> iced::widget::button::Style` delegating to
    `super::browse::rows::browse_row_style(status, false, is_default)`. Document `is_default` as
    the row Enter/`Jump` plays: it gets `theme::ROW_HIGHLIGHT` behind `theme::PLAYING_ROW_TEXT`,
    while every other row keeps the flat `style::playlist_row_style` colors.
  - In `view_jump_file_overlay`, record `matches.first().copied()` before consuming `matches`,
    build each button's text with `jump_file_row_label(index, &titles[index])`, and style it with
    `jump_file_row_style(first_match == Some(index), status)`. Keep the button width, the
    `JumpFileRowSelected { epoch, index }` message, the heading, the placeholders, and the
    `Jump`/`Close` buttons unchanged.
- `src/ui/views/mod.rs`: include both new helpers in the existing `pub(crate) use jump_file::…`
  re-export so the view tests can call them.
- `src/ui/views/tests/jump_file.rs`: import the helpers; add
  `jump_file_row_label_numbers_the_showing_list_position` (indices `0`, `1`, and `9`, and a title
  that already contains a number, pinning that the label adds exactly one `"<n>. "` prefix) and
  `jump_file_row_style_highlights_only_the_default_row` (default `Active` carries
  `theme::ROW_HIGHLIGHT` and `theme::PLAYING_ROW_TEXT`; non-default `Active` has no background and
  `theme::LCD_GREEN`; a non-default `Hovered` row still gets the playlist hover face). Update
  `overlay_draws_the_heading_matching_rows_and_buttons` to expect `"2. Middle"` and to omit
  `"1. Opening"`/`"3. Closing"`, and `row_and_buttons_publish_their_messages` to press
  `"2. Middle"`; the empty and no-match wordings stay asserted as they are.
- `docs/usage.md`: in the Jump to File sentence, say each row is numbered like the browse list and
  the first match — the row Enter or `Jump` plays — carries the selection bar.
- `docs/invariants.md` `## Jump to File panel`: add the numbered-label and default-row guarantees
  (the label uses `index + 1`; the first match takes the browse selection style; an empty or
  no-match list draws no row), name the two new tests in `Pinned by`, and leave the shortcut,
  filter, dispatch, and panel-exclusion guarantees unchanged.

**Files touched.** `src/ui/views/jump_file.rs`, `src/ui/views/mod.rs`,
`src/ui/views/tests/jump_file.rs`, `docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**

- Every match row reads `<n>. <cached title>`, where `n` is the source index plus one; filtering
  and reordering the list do not renumber the matches, and a title that already starts with a
  number gains only the new prefix.
- The first match is drawn with the browse selection style (blue `theme::ROW_HIGHLIGHT` bar and
  `theme::PLAYING_ROW_TEXT` text); every other match keeps the flat green playlist-row style. An
  empty list and a list with no match draw no row and keep `No songs`/`No matching songs`.
- Enter or `Jump` still plays the first match and row presses still dispatch
  `JumpFileRowSelected { epoch, index }` with the same epoch and source index; no handler, player
  state, library, REST, audio, or persistence behavior changes.
- `make check` passes.

## Done
