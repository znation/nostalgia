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
> (both 2026-10-20); the 2026-10-22 note below carries the current queue.

> **Steward drift note (2026-10-22 by the steward role): the built Now
> Playing bar and browse rows do not match the brief's mockups, and the two
> queued plans settle that difference in opposite directions — the
> playback-position plan adds a readout the mockup shows nowhere, while the
> album-row plan matches the mockup's `<artist> - <title>` rows but leaves the
> running time it draws out of scope.**
>
> The brief promises a "very close if not pixel-perfect" Winamp, and the plan
> role has been citing `docs/screenshots/` as the spec — the landed plans for
> the scrolling title, the artist-prefixed rows, the `TOP` button and the list
> counts each name a mockup feature. The main window's mockup
> (`docs/screenshots/player.png`) draws the classic LCD: a vertical `O C I D V`
> strip at the left, a large elapsed-time readout (`00:53`) over a green
> visualizer, a song-title box reading `<ARTIST> - <TITLE> (<length>)`, and a
> `256 KBPS 44 KHZ` / `MONO STEREO` status row. The build renders one row
> instead: `now_playing_row` (`src/ui/views/now_playing.rs`) pushes a
> `Now Playing: ` caption, the artist and title, a fill, and the track length
> at the right edge of the LCD well, and `view` (`src/ui/mod.rs`) stacks that
> bar above the transport row — no elapsed-time readout, no `O C I D V` strip,
> no status row.
>
> The first entry under `## Planned`, "Add the playback-position readout to
> the Now Playing bar (Playback position, part 3/3)", widens that right-edge
> slot to `<elapsed>/<total>`, a value the mockup shows nowhere: its title
> line carries the length in parentheses and the elapsed time in the big left
> readout. Recommend the plan role schedule that elapsed-time display and
> settle where part 3/3's readout belongs before the queue drains.
>
> The browse rows drift the same way: `docs/screenshots/library.png` draws
> each artist row with its album count at the right (`2 albums`) and each
> album row with its running time (`12:38`), while the build shows the
> `View Albums` and `View Songs` hints (`src/ui/views/browse.rs`) and the
> total count beneath the rows. The queued album-row plan, "Show each album
> row's artist beside its title", puts album running times out of scope
> because an album query does not supply its songs' durations, so the
> mockup's per-album time would need a songs fetch per row — a decision for
> the plan role, not a detail to drop.

## Planned

### Retire the `unclear-invariant` validation gap by pinning subsystem contracts in docs/invariants.md (planned 2026-10-23 by the steward role)

**Goal.** Three Fixed entries (the stale browse reply store-time invariant, the bidirectional browse contract, and the Now Playing title contract) each reconstructed a guarantee that nothing had written down, recorded as `gap: unclear-invariant`. Promote that recurring tag into infrastructure: make `docs/invariants.md` the single, discoverable home for every subsystem contract that a bug has had to reconstruct, so the next bug in those areas finds the invariant instead of rebuilding it, and so the tag stops recurring.

**Approach.**

- Audit `docs/invariants.md` against the three Fixed entries cited below and ensure each reconstructed guarantee is stated as a named, test-cited section (Browse hierarchy, Now Playing label, stale reply store-time window), adding any that are missing.
- Add a short "How to use" note at the top of `docs/invariants.md` pointing readers (and the bugfix role) at it before reconstructing an invariant, and cross-reference it from the BUGS.md header's Validation gap description.
- No code change; this is a docs-only plan whose acceptance is `make check` plus the three cited guarantees each having a section.

**Cited entries.** `BUGS-FIXED.md`: "A browse reply is stored after a newer request was issued …" (unclear-invariant, store-time invariant), "Now Playing bar shows the track id instead of the song title" (unclear-invariant, Now Playing contract), and the bidirectional-browse entry (unclear-invariant, browse hierarchy).

**Files touched.** `docs/invariants.md`, `BUGS.md`.

**Acceptance criteria.**

- `make check` passes.
- Every `unclear-invariant` Fixed entry cited above has a matching section in `docs/invariants.md` naming the enforcing code and the tests that fail if it is removed.
- The BUGS.md header cross-references `docs/invariants.md` for the `unclear-invariant` tag.

### Add the classic elapsed readout to the Now Playing LCD and restore the plain total at its right edge (planned 2026-10-23 by the plan role)

**Goal.** Split the LCD well the way `docs/screenshots/player.png` draws it: a large left-hand elapsed-time readout (`00:53`, zero-padded minutes and seconds) and the plain total length at the right edge. The landed playback-position series already supplies the position, but it currently renders `<elapsed>/<total>` in the right-edge slot — a value the mockup shows nowhere. Move that elapsed value into the big left readout and put the right edge back to the plain total, settling the drift recorded in the steward's 2026-10-22 note. The `O C I D V` strip, the visualizer, and the `256 KBPS 44 KHZ` / `MONO STEREO` status row stay out of scope, as do artist-row album counts.

**Approach.**

- `src/ui/views/now_playing.rs`:
  - Replace `format_position_time` and `now_playing_progress` with `format_lcd_elapsed(position_ms: u64) -> String`, which zero-pads both fields: `0` → `"00:00"`, `53_000` → `"00:53"`, `192_000` → `"03:12"`. Minutes stay unbounded, so an hour reads `"60:00"`. Delete the two replaced helpers and their tests rather than leaving dead public functions.
  - Add `now_playing_elapsed<'a>(tracks, current_track, position: Option<Duration>) -> Cow<'a, str>` beside [`now_playing_time`]: `"--:--"` when `known_track_for` finds nothing for `current_track`; `"00:00"` when the track is known but the backend reports no position (no preview, a failed start, or the silent backend); `Cow::Owned(format_lcd_elapsed(duration_millis(position)))` when a position is reported.
  - Change `now_playing_time` to drop its `position` parameter and always return the cached total (`KnownTrack::time`, borrowed) or `"--:--"` for an unknown/absent track. Update its doc comment to say the right edge is the plain total.
  - Add `LCD_ELAPSED_SIZE` (34) and an `lcd_elapsed_text` helper beside `lcd_text`, using the same `theme::LCD_GREEN`. Change `now_playing_row` to take `elapsed: Cow<'a, str>` as its first argument and push `lcd_elapsed_text(elapsed)` followed by a fixed-width `Space` (12 px) before the existing title line. Reduce `MARQUEE_WIDTH` from 28 to 22 and update its doc comment so the scrolling title line still fits the well beside the new readout; the `display_len`/`needs_marquee` tests key off the constant, so adjust only the comments that hard-code 28.
- `src/ui/mod.rs`: add `WinampPlayer::now_playing_elapsed` borrowing from `views::now_playing_elapsed`; change `WinampPlayer::now_playing_time` to call the two-argument view helper. In `view`, add `now_playing_elapsed` to the state-lock destructuring tuple and pass it as the new first argument to `views::view_now_playing`; update the `now_playing_time` doc comment there.
- Tests. `src/ui/views/tests.rs`: update the `captured_now_playing_texts` helper to take an elapsed string and assert the readout's position in the captured fragments; add `format_lcd_elapsed_pads_minutes_and_seconds` and `now_playing_elapsed_reads_placeholder_zero_or_position` (unknown id → `"--:--"`, known track with `None` → `"00:00"`, `Some(53s)` → `"00:53"`); add `now_playing_time_returns_the_plain_total_even_with_a_position` (the helper no longer takes a position). `src/ui/tests.rs`: drop the position argument from every `player.now_playing_time(...)` call; repoint the elapsed assertions at `player.now_playing_elapsed(...)` (known position → `"00:53"`-style, pause keeps the frozen value, Stop and a new selection read `"00:00"`, stale-epoch replies are still dropped); keep the epoch/poll tests unchanged. Update `now_playing_bar_and_browse_action_buttons_construct` for the new `view_now_playing` arity.
- `README.md`: in the Usage paragraph that describes the Now Playing bar, say the big readout at the left shows the loaded preview's elapsed time and the length at the right is the plain total; remove the elapsed/total sentence. Do not touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Now Playing label`, replace the `<elapsed>/<total>` guarantee with the split one (left readout `MM:SS`, plain total at the right, `00:00` for a known track without a position, `--:--` with no current track), name `views::now_playing_elapsed`/`format_lcd_elapsed`/`now_playing_time` and the `PlaybackPosition` handler as the enforcers, and cite the new tests plus the existing position-epoch tests.

**Files touched.** `src/ui/views/now_playing.rs`, `src/ui/mod.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- The LCD row's first drawn text is the elapsed readout: `00:53` for a 53-second position, `03:12` for 192 seconds, `00:00` for a known track with no reported position, and `--:--` when no track or an unknown id is current.
- The right edge shows the cached plain total (`3:30`) for a known track regardless of any reported position, and `--:--` for no/unknown track; no code path renders `<elapsed>/<total>` and `now_playing_progress`/`format_position_time` no longer exist.
- Pausing keeps the frozen elapsed readout, Stop and a new selection reset it to `00:00`, and a `PlaybackPosition` reply stamped with an older epoch is still dropped (the existing position tests pass unchanged apart from the elapsed accessor).
- `make check` passes (fmt, clippy, rustdoc, tests).
- The README Usage paragraph and the `docs/invariants.md` Now Playing guarantee describe the split readout in the same change.


## Done
