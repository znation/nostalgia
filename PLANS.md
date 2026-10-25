# Plans

Planned features, written by the plan loop and implemented by the feature loop.
Each plan: goal, approach, files touched, acceptance criteria. Move finished plans to Done.

> **Steward drift note (2026-10-06; condensed 2026-10-23 by the steward role).** This note
> tracked the initial prompt's two commitments — a "very close if not pixel-perfect" classic
> Winamp UI and Apple Music as the library — while the build caught up. Both are now reflected
> in the shipped app and the README's Usage section: the Winamp chrome (base-skin palette,
> bevel layer, custom title bar, title bands, Now Playing LCD, browse selection and row
> styling, compact and mini transport strips) and the Apple Music library (REST browse and
> search behind the `AppleMusicService` seam with the sample library as fallback, and preview
> playback through the audio seam). The per-landing detail this note once carried is retired
> here; `PLANS-DONE.md` holds the individual entries, and no current brief-vs-build drift is
> known. Standing lesson: the chrome arrived piecemeal, so later changes should reuse the
> existing vocabulary — `bevel::lcd_well`, `bevel::raised_panel`, and the shared style
> helpers — rather than inventing new chrome. Record any new drift here rather than appending
> another note.

## Planned

_None yet._

### Show the selected song's length before the total in the song-list footer, like Winamp (planned 2026-10-24 by the plan role)

**Goal.** Classic Winamp's playlist editor status line reads `<selected time>/<total time>`.
Nostalgia's song-list footer (`song_list_summary`, `src/ui/views/browse/rows/text.rs`) shows
only `<count> song(s) - <total>`. While a song row is selected (its blue bar painted), the footer
should read `<count> song(s) - <selected length>/<total>`, for example `2 songs - 3:30/4:15`.
With no selection, or a selected index outside the list, it stays exactly as today. This touches
only the footer text: no message, handler, selection, or playback change. It is independent of
the waiting Ctrl+Z/Ctrl+B plan (no shared files besides README and `docs/invariants.md`, in
different paragraphs and sections).

**Approach.**
- `src/ui/views/browse/rows/text.rs` `song_list_summary`: change the signature to
  `song_list_summary(songs: &[Song], selected: Option<usize>) -> String`. Compute the total as
  today. When `selected.and_then(|index| songs.get(index))` is `Some(song)`, format
  `"{count} {noun} - {selected}/{total}"` where `selected` is
  `format_minutes_seconds(song.duration_ms)` (so a zero-length selected song reads `0:00`, never
  `--:--`); otherwise keep the current `"{count} {noun} - {total}"`. Update its doc comment to
  describe both forms and the out-of-range fallback.
- `src/ui/views/browse/rows.rs` `song_list`: pass its existing `selected` argument:
  `with_footer(list, song_list_summary(songs, selected))`. Add one clause to its doc comment
  saying the footer includes the selected row's length.
- Do not change `list_count_summary`, the artist/album/playlist footers, or `view_songs` /
  `view_all_songs` (they already pass `selected` through to `song_list`).

**Files touched.** `src/ui/views/browse/rows/text.rs`, `src/ui/views/browse/rows.rs`,
`src/ui/views/tests/browse.rs`, `docs/invariants.md`, `README.md`.

**Acceptance criteria.**
- Every existing `song_list_summary` test in `src/ui/views/tests/browse.rs` passes `None` as
  the new argument and keeps its expected string.
- New tests in `src/ui/views/tests/browse.rs`, built on `summary_song`:
  - `song_list_summary_puts_the_selected_length_before_the_total`: songs of 210_000 and 45_000
    ms with `Some(1)` give `"2 songs - 0:45/4:15"`, and `Some(0)` gives `"2 songs - 3:30/4:15"`.
  - `song_list_summary_reads_a_zero_length_selection_as_zero`: songs of 210_000 and 0 ms with
    `Some(1)` give `"2 songs - 0:00/3:30"`.
  - `song_list_summary_ignores_a_selection_past_the_list`: one 210_000 ms song with `Some(1)`
    gives `"1 song - 3:30"`, and `&[]` with `Some(0)` gives `"0 songs - 0:00"`.
- `docs/invariants.md`: add a `## Song-list footer` section (Guarantee / Enforced by / Pinned
  by / History, the same shape as `## Library footer`) stating the two footer forms, that the
  selected length comes from the list's own `selected` index, and that an out-of-range index
  falls back to the plain form; name `song_list_summary` and `song_list` under Enforced by, and
  the existing and new `song_list_summary_*` tests under Pinned by.
- README Usage: after the clause "with each song list showing its song count and total playing
  time beneath the rows", add that while a song row is selected the footer shows that
  song's length before the total (`2 songs - 3:30/4:15`), like Winamp's playlist editor. Do not
  touch the brief or status blocks.
- `make check` passes.

## Done
