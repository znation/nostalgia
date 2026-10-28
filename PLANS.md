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

### Mark a song-list footer total that skips unknown lengths with `+`, like Winamp (planned 2026-10-27 by the plan role)

**Goal.** Winamp's playlist editor adds a `+` after a time total when one or more of the
entries it covers has no known length (`3:30+`), so the total does not look exact when it
isn't. Today `song_list_summary` sums `duration_ms` and treats `0` ("the source supplied no
duration", per `Song::duration_ms`) as zero time without saying so. `[210_000, 0]` therefore
reads `2 songs - 3:30`, and a selected unknown-length row reads `0:00` as if it really were zero.

**Approach.** Make the change in `song_list_summary` (`src/ui/views/browse/rows/text.rs`) only.
It is a pure helper, and `views::browse::rows::song_list` already passes its result to the footer.
- Total: append `+` to `format_minutes_seconds(total_ms)` exactly when some song in `songs`
  has `duration_ms == 0` (`songs.iter().any(|song| song.duration_ms == 0)`). The song count
  is unchanged, so every entry still counts.
- Selected part: when the selected song's `duration_ms == 0`, it reads `0:00+` (Winamp's
  selection total is marked the same way for the same reason). A known selected length is
  unchanged.
- An empty list has no unknown entries and still reads `0 songs - 0:00`, so
  `an_empty_browse_level_draws_no_count_footer` (`src/ui/tests/view_tests.rs`) is
  unaffected. Do not touch `format_minutes_seconds`, `format_track_time`, album row totals
  (`--:--`), or the compact transport readouts.
- Update the helper's doc comment, which currently says "a zero-length selection reads
  `0:00`".

**Files touched.** `src/ui/views/browse/rows/text.rs`, `src/ui/views/tests/browse.rs`,
`docs/usage.md` (the footer sentence in the browse-list paragraph, after the
`2 songs - 3:30/4:15` example), `docs/invariants.md` (the `## Song-list footer` section's
Guarantee and Pinned-by list).

**Acceptance criteria.**
- In `src/ui/views/tests/browse.rs`, rename `song_list_summary_counts_unknown_durations_without_adding_time` to
  `song_list_summary_marks_a_total_that_skips_unknown_durations`, asserting
  `song_list_summary(&[summary_song(210_000), summary_song(0)], None) == "2 songs - 3:30+"`
  and an all-unknown `[summary_song(0)]` reads `"1 song - 0:00+"`.
- Rename `song_list_summary_reads_a_zero_length_selection_as_zero` to
  `song_list_summary_marks_an_unknown_selected_length`, asserting
  `[210_000, 0]` with `Some(1)` reads `"2 songs - 0:00+/3:30+"` and with `Some(0)` reads
  `"2 songs - 3:30/3:30+"`.
- The other existing `song_list_summary_*` tests (all-known durations, empty list, selection
  past the list, saturation, sub-second truncation) pass unchanged.
- docs/usage.md and the invariants section describe the `+` marker and name the renamed
  tests. `make check` passes.

## Done
