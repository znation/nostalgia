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
> (both 2026-10-20); the 2026-10-22 note below records drift that has since
> been resolved.

> **Steward drift note (2026-10-22 by the steward role; resolved 2026-10-23
> by the steward role).** That note recorded the built Now Playing bar and
> browse rows drifting from the brief's mockups — no elapsed readout, no
> `O A I D V` strip, no status row, album rows without running time, and a
> queued playback-position plan that put an `<elapsed>/<total>` readout at the
> LCD's right edge. Those drifts have since landed: "Add the classic elapsed
> readout to the Now Playing LCD and restore the plain total at its right
> edge", "Draw the classic O A I D V strip at the LCD's left edge", "Show the
> loaded preview's bitrate in the LCD status row", "Show each album row's
> running time", and "Show each artist row's album count" are done, and the
> playback-position plan is no longer queued. No current brief-vs-build drift
> is known. The LCD `MONO`/`STEREO` pair has since landed (done 2026-10-23),
> which brought the status row to `docs/screenshots/player.png` and
> `docs/screenshots/transport.png`, and the volume and balance slider fill
> has since landed (done 2026-10-23) with the orange and green grooves those
> mockups draw. The only planned entry is now the playlist-number prefix
> below.

## Planned

### Prefix the Now Playing LCD title with the track's playlist number (planned 2026-10-23 by the plan role)

**Goal.** Match the LCD title line in `docs/screenshots/player.png`, which reads `1. THE SAMPLE BAND - OPENING` before the right-edge total, while the build draws only `Now Playing: The Sample Band - Opening`. Record the 1-based position of the row that selected the current track and draw its prefix (`1. `) after the `Now Playing: ` caption and before the artist/title. The number follows every new selection, including Next/Previous and the preview-end advance, stays with the track after browsing to another list (it is cached in `KnownTrack` beside the title and artist), and is absent when no track is current or the id is unknown. This is presentation only: playback, browse order, selection resolution, sorting, and the marquee timer's schedule are unchanged.

**Approach.**

- `src/ui/views/now_playing/text.rs`: add `pub number: String` to `KnownTrack`, documented as the cached `"<n>. "` prefix (`""` when none). Add `pub fn now_playing_number<'a>(tracks, current_track) -> Cow<'a, str>` mirroring `now_playing_artist`: borrow the known track's `number`, or `""` when no track is current or the id is unknown. Change `display_len` to take the number prefix and add its character count, and `needs_marquee` to take and pass it, so the prefix counts against `MARQUEE_WIDTH` and the timer runs exactly when the drawn line overflows. Update the doc comments to mention the prefix.
- `src/ui/handlers/playback.rs`: in `track_selected`, write the resolved row's number into its `KnownTrack` entry as `format!("{}. ", index + 1)`; replace the `.entry(...).or_insert_with` with an insert/entry update so re-selecting the same id from a different row or list refreshes the number with the other cached fields. Keep `track_played`'s prune-to-current behavior, which retains the current track's number.
- `src/ui/views/now_playing.rs`: re-export `now_playing_number`; add a `number: Cow<'a, str>` parameter after `artist` to `view_now_playing`, `now_playing_content`, and `now_playing_row`; and thread it through. Measure `display_len(&label, &artist, &number)`. In the fitting branch, push `lcd_text(number)` after the caption only when it is non-empty, then the artist and title as today. In the scrolling branch, build `format!("{number}{}", display_title_for(&artist, &label))` and window that, so the number scrolls with the line instead of overflowing the budget. Update the module and function docs.
- `src/ui/player.rs`: add a `now_playing_number` method beside `now_playing_artist`, and make `needs_marquee` resolve the number from the same known-track index before calling `views::needs_marquee(&label, &artist, &number)`.
- `src/ui/view.rs`: resolve `now_playing_number` inside the existing state-lock block and pass it to `views::view_now_playing`; the bar therefore uses the cached number, not the currently drawn list.
- `src/ui/views/tests.rs`: extend `captured_now_playing_texts` with a number argument (empty in the existing calls) and add coverage that `1. ` draws between the `Now Playing: ` caption and the artist; that an overlong line's marquee window starts with the prefix; that `display_len`/`needs_marquee` count the prefix; and that an artist-less, unknown, or absent track draws no prefix. Add the number field to every `KnownTrack` literal.
- `src/ui/tests.rs`: update the `KnownTrack` literals and add `track_selected_records_the_selected_rows_number` (index 0 → `"1. "`, index 1 → `"2. "`), `replaying_a_track_from_another_row_updates_its_number`, and `now_playing_number_survives_browsing_to_another_album` (reuse the `play_song_1_then_browse_to_album_2` fixture). Extend `the_player_knows_when_the_now_playing_line_needs_the_marquee` so the prefix can push a borderline line over the budget.
- `README.md`: in the Usage sentence that describes the LCD title, say the line is prefixed by the playing track's 1-based position in the list it was played from (`1. The Sample Band - Opening`); do not touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Now Playing label`, add the number guarantee: the known track's line draws `<n>. ` before its artist/title from the row that selected it, the number follows every new selection (Next/Previous and the preview-end advance included), persists across browsing away and Stop, and is absent for an unknown id or no current track; the prefix counts toward the marquee budget. Name `KnownTrack::number`, `views::now_playing_number`, `handlers::playback::track_selected`, `now_playing_row`, `display_len`, and `needs_marquee` as the enforcers and list the new tests.

**Files touched.** `src/ui/views/now_playing/text.rs`, `src/ui/views/now_playing.rs`, `src/ui/handlers/playback.rs`, `src/ui/player.rs`, `src/ui/view.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- Selecting the first row of any song list makes the LCD read `Now Playing: 1. <artist> - <title>`, and the second row reads `2. `; Next/Previous and the preview-end advance update the number with the new row, while re-playing the same track from another row updates it too.
- The number survives browsing to another list and Stop, and is absent when no track is current or the id is unknown.
- The prefix counts toward `MARQUEE_WIDTH`, and an overlong line scrolls a window that includes it; no playback, browse, sort, selection, or audio behavior changes.
- The new view and handler tests pin the prefix, persistence, and marquee behavior, `make check` passes, and the README and invariant section describe the same behavior.

### Draw the main transport row with the mockup's classic glyphs (planned 2026-10-23 by the plan role)

**Goal.** Match `docs/screenshots/player.png`, `docs/screenshots/hero.png`, and `docs/screenshots/transport.png`, whose main transport row draws Previous, Play/Pause, Stop, and Next as compact glyphs in that order. The build draws the same four controls as the words `Play`/`Pause`, `Stop`, `Previous`, and `Next`. Replace those four faces with `◀◀`, `▶`/`II`, `■`, and `▶▶`, sharing the exact glyphs, order, widths, and messages the shade strip already uses, and keep a word label on each glyph button's hover tooltip. Repeat and Shuffle remain the existing visible `Repeat: On/Off` and `Shuffle: On/Off` text toggles, and every button keeps its fixed width so the glyph swap cannot reflow the row. This is presentation only: no message, state, playback, or audio behavior changes, and no dependency is added.

**Approach.**

- `src/ui/views/transport.rs`: add `pub(super) fn play_pause_glyph(is_playing: bool) -> &'static str` (`"II"` while playing, `"▶"` otherwise) and `pub(super) fn play_pause_word(is_playing: bool) -> &'static str` (`"Pause"` while playing, `"Play"` otherwise), moving the glyph helper out of `shade`. Add `pub(super) fn transport_glyphs(is_playing: bool) -> [(&'static str, &'static str, f32, Message); 4]`, the pure row order and spec: Previous `("◀◀", "Previous", 34.0, Message::PreviousTrack)`, Play/Pause `(play_pause_glyph, play_pause_word, 30.0, Message::PlayPause)`, Stop `("■", "Stop", 30.0, Message::Stop)`, Next `("▶▶", "Next", 34.0, Message::NextTrack)`. Rebuild `transport_buttons` from that spec, wrapping each `fixed_width_button` in `iced::widget::Tooltip` with the word as a small `Text` tooltip above it; then push the unchanged fixed-width `repeat_label` and `shuffle_label` buttons (104 and 112 px) in that order. Update the module and function docs for the glyph row and tooltips.
- `src/ui/views/shade.rs`: remove the local `play_pause_glyph` and import `transport::play_pause_glyph`; build `shade_controls` from `transport_glyphs` by dropping each entry's tooltip word, so the strip and the full row cannot drift on glyph, order, width, or message. Update the shade doc comment and keep the strip's own no-tooltip layout.
- `src/ui/views/mod.rs`: update the `labeled_button`/`fixed_width_button` doc references from `play_pause_label` to the glyph helpers; no code change is needed there.
- `src/ui/views/tests.rs`: change the import to `play_pause_glyph`/`play_pause_word`/`transport_glyphs`; replace `play_pause_label_mirrors_playing_state` with glyph and word assertions; add `transport_glyphs_match_the_mockup_order_and_widths` asserting the four glyph/word/width/message tuples exactly; keep `transport_buttons_keep_a_fixed_width_across_label_changes` and update `transport_buttons_keep_their_measured_face_widths` to `[34.0, 30.0, 30.0, 34.0, 104.0, 112.0]`; update the shade tests for the moved helper and add `main_transport_and_shade_strip_share_glyphs_order_and_widths`, comparing `transport_glyphs` with `shade_controls`.
- `src/ui/tests.rs`: change `transport_row_does_not_reflow_when_the_play_label_changes` to `transport_row_does_not_reflow_when_the_play_glyph_changes`, asserting the `Repeat: Off` and `Shuffle: Off` x positions stay fixed across stopped and playing (the `Stop` word is no longer drawn), and add `root_view_draws_the_transport_glyph_row`, which lays out the root `view` through `captured_root_view_text_bounds` and asserts `◀◀` and `▶▶` draw in the same row and in Previous-before-Next order. Update any existing root-view text lookup that used a removed transport word.
- `README.md`: in the Usage paragraph's transport sentence, say the bar sits above Previous, Play/Pause, Stop, and Next glyph controls with word tooltips, then the Repeat and Shuffle toggles, the sliders, the equalizer, and the browse list. Do not touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## UI layout`, replace the transport-label bullet with the glyph-row guarantee (order, exact glyphs, fixed widths, tooltip words, Repeat/Shuffle text unchanged) and name `views::transport::transport_glyphs`/`transport_buttons` and the new tests. In `## Window shade`, change the enforcer reference from `views::shade::play_pause_glyph` to the shared `views::transport::play_pause_glyph` and state that the strip and full row share the glyph/order/width spec.

**Files touched.** `src/ui/views/transport.rs`, `src/ui/views/shade.rs`, `src/ui/views/mod.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- The main transport row draws, in order, `◀◀`, the state-dependent `▶`/`II`, `■`, and `▶▶`, each with the same `Message` as today and a hover tooltip reading `Previous`, `Play`/`Pause`, `Stop`, or `Next`; Repeat and Shuffle remain the existing text toggles.
- The four glyph buttons use the fixed widths 34, 30, 30, and 34 px, so Play/Pause swapping `▶` and `II` leaves every later control's x position unchanged; Repeat and Shuffle keep their 104 and 112 px widths.
- The shade strip builds its four buttons from the same `transport_glyphs` spec, so its glyphs, order, widths, and messages match the main row.
- The root layout test finds the `◀◀` and `▶▶` glyphs in one row, and the reflow test confirms the later toggles do not move across the playback-state swap.
- No playback, state, browse, or audio behavior changes, no dependency is added, `make check` passes, and the README and both invariant sections describe the same row.

## Done
