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

_None yet._

### Draw the classic O A I D V strip at the LCD's left edge (planned 2026-10-23 by the plan role)

**Goal.** Draw the mockup's still-missing static LCD element (`docs/screenshots/player.png`): a vertical column of the classic `O`, `A`, `I`, `D`, `V` letters down the well's left edge, beside the large elapsed readout and the title/status rows. The strip is decorative — it emits no messages and toggles nothing, because the app already exposes always-on-top through the title bar's `A` button — and it is drawn in every Now Playing state, including the empty `--:--` bar, so the LCD matches the mockup without duplicating controls. It has a fixed width so adding it does not shift the title or the right-edge total. This is independent of the queued seek slider: that plan adds a strip below the LCD well, while this one adds content inside it.

**Approach.**

- `src/ui/views/now_playing.rs`: add `pub(super) const LCD_INDICATOR_LABELS: [&str; 5] = ["O", "A", "I", "D", "V"]` (the mockup's order; the steward's 2026-10-22 note spells the strip `O C I D V`, which the screenshot shows is a typo), a fixed-width constant (`LCD_INDICATOR_WIDTH = 12.0`), and a small text-size constant (`LCD_INDICATOR_SIZE = 10.0`). Add `pub(super) fn lcd_indicator_strip<'a, Theme, Renderer>() -> Column<'a, Message, Theme, Renderer>` with the same text-catalog and renderer bounds as `lcd_text`, building one small [`theme::LCD_GREEN`] `Text` per label in a fixed-width column with tight spacing, top-aligned and vertically arranged. Change `view_now_playing` to put the strip and the existing title/status `Column` in a `Row` before wrapping that row in `bevel::lcd_well`, so the letters sit inside the same recessed well; keep `now_playing_row`, `now_playing_status_row`, and all readout logic unchanged. Update the function's doc comment to mention the decorative strip.
- `src/ui/views/tests.rs`: import `LCD_INDICATOR_LABELS`, `LCD_INDICATOR_WIDTH`, and `lcd_indicator_strip`. Add a `captured_indicator_texts` helper that lays out `lcd_indicator_strip()` with iced's null renderer and the existing `CapturedText` operation, returning the drawn text and the laid-out width. Add `lcd_indicator_strip_draws_each_label_in_order_at_a_fixed_width`, asserting the captured text equals `["O", "A", "I", "D", "V"]` and the width equals `LCD_INDICATOR_WIDTH`. Add `view_now_playing_draws_the_indicator_strip_beside_the_title`, which lays out `view_now_playing` itself (a fitting track, e.g. `"00:53"` / `"Opening"` / `"The Sample Band"` / `"3:30"` with no format) through the same operation and asserts every indicator letter plus the elapsed value, the title, and the total appear, so a future refactor cannot drop the strip while leaving its builder tested.
- `README.md`: in the Usage sentence that describes the Now Playing bar, say a small vertical `O A I D V` strip runs down the LCD's left edge as in the mockup and is decorative. Keep the hunk to that sentence; do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in the `## Now Playing label` section, add a bullet stating that the LCD's left edge always draws the five classic indicator letters `O`, `A`, `I`, `D`, `V` in that order at a fixed width, in every playback state, and that the strip is decorative (it emits no message and changes nothing); name `views::now_playing::LCD_INDICATOR_LABELS` and `lcd_indicator_strip` as the enforcers and add the two new test names to the Pinned-by list.

**Files touched.** `src/ui/views/now_playing.rs`, `src/ui/views/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `LCD_INDICATOR_LABELS` is exactly `["O", "A", "I", "D", "V"]`, and `lcd_indicator_strip` draws them in that order, one letter per line, in the LCD green at a fixed `LCD_INDICATOR_WIDTH` width.
- `view_now_playing` lays the strip out at the left of the existing elapsed/title/status content inside `bevel::lcd_well`; laying out the whole bar captures all five letters and the unchanged elapsed, title, and total text.
- The strip is present in the empty and loaded states, has no `Message` and no event handler, and does not alter the large readout, the marquee, the status row, or the right-edge total.
- The README and `docs/invariants.md` describe the strip as decorative, and `make check` passes (fmt, clippy, rustdoc, tests).

### Show the loaded preview's bitrate in the LCD status row (planned 2026-10-23 by the plan role)

**Goal.** Complete the mockup's LCD status line (`docs/screenshots/player.png`, `256 KBPS 44 KHZ`) by showing the loaded preview's average bitrate beside the sample rate and channel mode: `256 KBPS  44 KHZ  STEREO`. When the backend cannot compute it, the row falls back to today's `44 KHZ  STEREO`. The value is presentation only, derived from the downloaded encoded preview body and the decoded source's own total duration; it follows the loaded preview as the sample rate does (Pause keeps it, Stop and a new selection clear it). The landed status-row plan left bitrate out because `preview::download_and_decode` discards the body length, so this plan carries that length through to `PlaybackFormat`. It is independent of the queued `O A I D V` strip: that adds a column inside the well, while this changes the format value and the existing status text.

**Approach.**

- `src/audio/preview.rs`: add `pub(super) struct DecodedPreview { pub source: rodio::Decoder<Cursor<Vec<u8>>>, pub encoded_bytes: u64 }`. In `download_and_decode`, record `bytes.len()` as `encoded_bytes` before the bytes move into `Cursor`, and return the struct after `rodio::Decoder::new_mp4`. Keep the existing status check, wire/decoded caps, and error wording unchanged.
- `src/audio.rs`: add `pub bitrate_kbps: Option<u32>` to `PlaybackFormat`, documented as the preview's average encoded bitrate in kilobits per second and `None` when the backend cannot report the decoded source's length.
- `src/audio/player.rs`: change `RodioPlayer`'s `WorkerPlayer::Preview` to `DecodedPreview`. Add the pure helper `pub(super) fn preview_bitrate_kbps(encoded_bytes: u64, duration: Duration) -> Option<u32>`: multiply bytes by 8, divide by `duration.as_millis()` (one bit per millisecond is one kilobit per second), round to nearest, and return `None` for zero duration or a result that does not fit `u32`. Change `format_for_source` to take the encoded byte count, fill `bitrate_kbps` by calling `preview_bitrate_kbps` with `source.total_duration()` and passing through `None` when the source reports no length, and keep reading sample rate and channels as it does today. In `append`, destructure `DecodedPreview`, pass `encoded_bytes` to `format_for_source`, and read the same source's duration for `duration_for_source` as today.
- `src/ui/views/now_playing.rs`: change `format_playback_status` to prefix `"{kbps} KBPS  "` when `bitrate_kbps` is `Some`; keep `"{khz} KHZ  {mode}"` when it is `None`. Remove the doc sentence that says bitrate is deliberately absent, and note the fallback. `now_playing_status_row` and its fixed height stay as they are.
- `src/audio/tests.rs`: add `preview_bitrate_kbps_computes_encoded_bits_over_the_preview_length` (`960_000` bytes over 30 seconds is `Some(256)`; zero duration is `None`). Update `format_for_source_reads_sample_rate_and_channels` for the new argument and assert the computed bitrate for a buffer whose total duration is known, and add `bitrate_kbps: None` to every other `PlaybackFormat` literal in the file unless the test is about bitrate.
- `src/ui/views/tests.rs`: extend `format_playback_status_formats_khz_and_channel_mode` with `Some(256)` reading `"256 KBPS  44 KHZ  STEREO"` and a `None` case reading `"44 KHZ  STEREO"`; extend `now_playing_status_row_draws_the_format_or_nothing` to capture the bitrate text; add `bitrate_kbps` to the remaining literals.
- `src/ui/tests.rs`, `src/apple_music/tests.rs`, and `src/test_support.rs`: add `bitrate_kbps: None` (or a value where the fake is meant to represent a loaded preview with a known rate) to every `PlaybackFormat` literal so the suite compiles.
- `README.md`: in the Usage sentence describing the status row, say it shows the preview's bitrate, sample rate, and channel mode (`256 KBPS  44 KHZ  STEREO`) when the backend reports them, and falls back to the sample rate and mode alone when it cannot compute the bitrate. Do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Now Playing label`, replace the status-row bullet's "Bitrate is not shown" sentence with the guarantee that a loaded preview's average bitrate is shown when the download length and decoded duration are both known, is omitted otherwise, and follows the same Pause/Stop/epoch rules as the sample rate. Name `audio::preview::DecodedPreview`, `audio::player::preview_bitrate_kbps`/`format_for_source`, and `views::now_playing::format_playback_status` as the enforcers, and add the new test names to the Pinned-by list.

**Files touched.** `src/audio/preview.rs`, `src/audio.rs`, `src/audio/player.rs`, `src/audio/tests.rs`, `src/ui/views/now_playing.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `src/apple_music/tests.rs`, `src/test_support.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- `preview_bitrate_kbps(960_000, Duration::from_secs(30)) == Some(256)`, a zero-length duration returns `None`, and `format_for_source` fills `bitrate_kbps` from the encoded byte count and the source's own total duration.
- `download_and_decode` returns a `DecodedPreview` whose `encoded_bytes` is the length of the downloaded body and whose `source` is the decoded preview; the existing download-failure and cap errors are unchanged.
- `format_playback_status` renders `256 KBPS  44 KHZ  STEREO` for `Some(256)`, 44_100 Hz, two channels, and keeps `44 KHZ  STEREO` when the bitrate is `None`; the status row draws that text and keeps its fixed height and blank state.
- Bitrate follows the existing preview lifecycle with no new state: Pause keeps it, Stop and a new selection clear it through the existing `PlaybackFormat` reset and epoch guards, and the silent fallback or a backend without a duration reports no bitrate.
- `make check` passes (fmt, clippy, rustdoc, tests).

## Done
