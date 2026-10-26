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
