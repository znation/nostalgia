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
> mockups draw, and the playlist-number title prefix landed too (done
> 2026-10-23), matching `docs/screenshots/player.png`, and the transport
> glyph row landed on the same day. The select-then-open plan below remains
> queued.

## Planned

### Select a browse row, then open it with Open or a second click (planned 2026-10-23 by the plan role)

**Goal.** Match `docs/screenshots/library.png`'s interaction caption — "Click selects,
double-click (or OPEN) drills in, BACK climbs out" — without changing what opening a row does
today. A single press on any browse row selects it and paints the mockup's blue selection bar
instead of drilling or playing; pressing that selected row again (which is what the second press
of a double-click is), or pressing the new `Open` button, opens it through the existing artist,
album, playlist, and song handlers. The playing row keeps its `▶` marker and highlight, keyboard
Next/Previous and the preview-end advance keep playing and scrolling as they do now, and opening
a row keeps its existing fetch/playback path. This is one focused selection layer over every
browse level; it adds no new fetch, playback, sort, or scroll behavior.

**Approach.**

- `src/ui/browse.rs`: give `BrowseList<T>` a `selected: Option<usize>` field beside the rows and
  the epoch. Add `pub(super) fn select_row(&mut self, epoch: u64, index: usize) -> bool`: it
  returns `false` for a stale epoch or out-of-range index (through the existing
  `BrowseList::select` check), otherwise records `Some(index)` and returns whether that row was
  already selected. Add `pub(super) fn selected(&self) -> Option<usize>`. Clear `selected` in
  `replace_rows` (so `store`/`store_with_row_text` reset it), `clear`, and `fail`, and clear it
  in `sort_rows_by` and `restore_library_order` whenever they actually reorder rows, since those
  already bump the epoch because indices changed. A list whose rows and order are unchanged keeps
  its selection, so Back to an unsorted parent can still show the row the user left selected.
- `src/ui/message.rs`: add `SelectRow { epoch, index }`, the row-press message, and
  `OpenSelected`, the Open button's message. The existing `ArtistSelected`, `AlbumSelected`,
  `TrackSelected`, and `PlaylistSelected` remain the open actions, also emitted by Next/Previous
  and the preview-end advance where they already are; they are no longer emitted by a row's
  single press.
- `src/ui/handlers/browse.rs`: add `pub(in crate::ui) fn select_row(player, epoch, index) ->
  Task<Message>`. It calls the current view's list `select_row`, and returns `Task::none()` unless
  that reports the row was already selected; then it opens the pressed row. Add `pub(in
  crate::ui) fn open_selected(player) -> Task<Message>`: read the current view's list's
  `selected()` and `epoch`, and return `Task::none()` when nothing is selected, otherwise open
  that row. Both share one private `open_row(player, epoch, index) -> Task<Message>` that maps
  `current_view` to `Task::done(Message::ArtistSelected { .. })`, `AlbumSelected`, `TrackSelected`
  (for both `Songs` and `AllSongs`), or `PlaylistSelected`. Keeping the open action in the
  existing handlers means the epoch/index staleness checks, fetch clearing, and playback path
  stay exactly where they are.
- `src/ui/mod.rs`: route `Message::SelectRow` to `handlers::browse::select_row` and
  `Message::OpenSelected` to `handlers::browse::open_selected`.
- `src/ui/views/browse/rows.rs`: drop the open `Message` (and the now-unused `epoch`/`index`
  arguments) from `artist_row`, `album_row`, `playlist_row`, and `song_row`; each returns
  `(title, label, is_current)`. `scrollable_list` gains `epoch: u64` and `selected:
  Option<usize>` parameters and puts `Message::SelectRow { epoch, index }` on each row button
  instead of the tuple's removed message. Highlight a row when `is_current || selected ==
  Some(position)` using the existing `current_row_style` blue bar, while keeping `row_prefix`'s
  `▶` strictly tied to `is_current`. Update the doc comments to say a press selects and opening
  goes through `SelectRow`/`OpenSelected`.
- `src/ui/views/browse.rs`: add `pub fn view_open_button() -> Element<'static, Message>` using
  `labeled_button("Open", Message::OpenSelected)`, documented beside the Back/Top/Sort builders.
  Thread `selected: Option<usize>` through `view_artists`, `view_albums`, `view_songs`,
  `view_all_songs`, `view_all_albums`, `view_playlists`, `album_list`, and `song_list` into
  `scrollable_list`. Keep every row's title, label, numbering, playing marker, and footer as they
  are.
- `src/ui/views/mod.rs`: re-export `view_open_button` with the other browse builders.
- `src/ui/view.rs`: pass each list's `selected()` into its view builder in the `current_view`
  match. Push `views::view_open_button()` above the browse list for every browse view — beside
  the Back/Retry controls, before the Top/Sort row — so Open is present at every level and a press
  with no selection is a no-op.
- `README.md`: in the Usage paragraph that describes the browse list, say a click selects a row,
  a second click on the selected row (a double-click) or the `Open` button opens it (drills into
  an artist, album, or playlist, or plays a song), the selected row gets the blue bar, and Back
  still climbs out. Do not edit the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: add a `## Browse selection` section after `## Browse hierarchy`. State
  that each `BrowseList` owns at most one selected row, that only a matching epoch and in-range
  index can select, that a second press on the selected row or `OpenSelected` dispatches the
  existing open message, that a single press never opens/plays/fetches, and that selection is
  cleared when the rows are replaced, cleared, failed, sorted, or restored. Name
  `BrowseList::select_row`/`selected`, `handlers::browse::select_row`/`open_selected`/`open_row`,
  `views::browse::scrollable_list`, and `views::view_open_button` as the enforcers, and list the
  new tests as pinned.
- Tests. `src/ui/browse.rs`: add a `#[cfg(test)] mod tests` covering `select_row`'s
  select-then-report-second-press behavior, stale-epoch and out-of-range rejection, and the
  clearing paths. `src/ui/tests.rs`: add
  `a_single_row_press_selects_without_opening` (artist view unchanged, no task scheduled),
  `a_second_press_on_the_selected_row_opens_it` (the artist view steps to Albums),
  `open_selected_opens_the_selected_song` (playback task/current track follow the existing
  `TrackSelected` path), `open_selected_with_nothing_selected_is_a_noop`, and
  `replacing_or_sorting_rows_clears_the_selection`; reuse `seed_browse_lists`. `src/ui/views/tests.rs`:
  update the row-builder and `scrollable_list` calls to the new signatures, keep their title,
  label, prefix, and current-track assertions, and add
  `browse_open_button_constructs` to the view-construction test. `src/ui/tests.rs`'s root-layout
  tests: add `browse_open_button_sits_above_the_browse_list` using the existing
  `captured_root_view_text_bounds`, and keep Top beside Sort unchanged.

**Files touched.** `src/ui/browse.rs`, `src/ui/message.rs`, `src/ui/mod.rs`,
`src/ui/handlers/browse.rs`, `src/ui/views/browse.rs`, `src/ui/views/browse/rows.rs`,
`src/ui/views/mod.rs`, `src/ui/view.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`,
`README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- One press on any artist, album, song, or playlist row selects it and does not change the view,
  play a song, or schedule a fetch; the selected row draws the same blue bar as the playing row,
  and the `▶` marker stays on the playing row only.
- A second press on the row that is already selected opens it exactly as today's single press
  did: Artists → Albums, Albums/All Albums → Songs, Playlists → Songs, and a song starts playing
  through `handlers::playback::track_selected`; a double-click reaches this on its second press.
- The new `Open` button sits above every browse list and dispatches the same open action for the
  selected row; with no selection it does nothing and schedules no task.
- A stale epoch or out-of-range index cannot select, and replacing, clearing, failing, sorting,
  or restoring a list's rows clears its selection; the playing marker, numbering, scrolling and
  wheel behavior, Sort, Top, Back, and keyboard Next/Previous are unchanged.
- README and `docs/invariants.md` describe the select-then-open interaction, and `make check`
  passes.

### Show the compact transport readout as elapsed over the track's total (planned 2026-10-23 by the plan role)

**Goal.** Match `docs/screenshots/library.png` and `docs/screenshots/hero.png`, whose compact
footer readout reads `4:05/12:33`: show the current time over the playing track's total in the
shared compact row used by the library footer and the window-shade strip. The left value stays
the same `WinampPlayer::now_playing_elapsed` string (so the elapsed/remaining toggle still applies,
and remaining mode reads `-02:37/3:30`), the right value is the same
`WinampPlayer::now_playing_time` total the LCD's right edge shows, and when that total is the
`--:--` placeholder the readout stays that single string rather than `--:--/--:--`. This changes
only the compact row's text: the LCD, playback, messages, glyphs, volume slider, heights, and
layout are unchanged.

**Approach.**

- `src/ui/views/mini_transport.rs`: add `pub(super) fn mini_transport_readout<'a>(elapsed:
  Cow<'a, str>, total: Cow<'a, str>) -> Cow<'a, str>`. It returns `elapsed` unchanged when
  `total` is `"--:--"` (so a missing or unknown track keeps its current placeholder) and
  otherwise returns `Cow::Owned(format!("{elapsed}/{total}"))`. Give `mini_transport_row` a
  `total: Cow<'a, str>` parameter beside `elapsed`, build the drawn `Text` from this helper, and
  give `view_library_strip` the same parameter. Update the module and function docs to name both
  time strings and the placeholder rule.
- `src/ui/views/shade.rs`: give `view_shade_strip` a `total: Cow<'a, str>` parameter beside
  `elapsed` and pass it to `mini_transport_row`; update its docs to say the readout is
  elapsed-over-total and keeps the shared toggle.
- `src/ui/view.rs`: in the shaded branch, resolve `now_playing_time` from the same state lock
  tuple (using `state.current_track.as_deref()`), and pass it to `view_shade_strip`. In the normal
  branch, clone the already-resolved `now_playing_time` before it moves into `view_now_playing`
  (the way `library_elapsed` is cloned) and pass the clone to `view_library_strip`, so the footer
  and the LCD's right edge read the same total.
- `src/ui/views/tests.rs`: add `mini_transport_readout_appends_the_known_total` covering
  `00:53`/`3:30` → `00:53/3:30`, `-02:37`/`3:30` → `-02:37/3:30`, `00:53`/`--:--` → `00:53`, and
  `--:--`/`--:--` → `--:--`. Update `captured_shade_strip`, `captured_library_strip`, and the
  existing height and geometry tests to pass a total (for example `Cow::Borrowed("3:30")`) and
  assert the combined readout reaches the laid-out row. Keep the glyph order, fixed height, and
  right-edge volume assertions unchanged.
- `src/ui/tests.rs`: add `root_view_footer_shows_elapsed_over_total`, using
  `play_song_1_then_browse_to_album_2` and `captured_root_view_text_bounds` with `shaded = false`
  and then `true`, asserting both trees draw `00:00/3:30` (the known song's cached total) and
  that neither draws `--:--/--:--`. This pins the two `view` call sites, not just the builders.
- `README.md`: in the Usage prose, extend the library-footer and shade-strip sentences to say the
  compact readout shows the elapsed (or remaining) time over the track's total, and falls back to
  the plain `--:--` placeholder when no total is known. Do not edit the
  `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in `## Library footer` and `## Window shade`, replace the bullet that
  calls the compact label "the current time readout" / the same elapsed string with the
  elapsed-over-total rule and its single-placeholder fallback. Name
  `views::mini_transport::mini_transport_readout` as the enforcer, note that both strips take the
  same `now_playing_elapsed` and `now_playing_time` strings the LCD resolves, and add the new
  tests to both `Pinned by` lists.

**Files touched.** `src/ui/views/mini_transport.rs`, `src/ui/views/shade.rs`, `src/ui/view.rs`,
`src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- Both the library footer and the shade strip draw `<elapsed>/<total>`, such as `00:53/3:30`, when
  the current track's total is known; remaining mode still prefixes the elapsed value with `-`
  (`-02:37/3:30`), and a known total with no reported position reads `00:00/3:30`.
- When `now_playing_time` returns `--:--` (no current track, or an unknown one), the readout is
  just the elapsed string, so the empty state still reads `--:--` and never `--:--/--:--`.
- The compact row's four glyphs, messages, heights, right-edge `MINI_VOLUME_ID` slider, and
  elapsed/remaining toggle behavior are unchanged, and the LCD's large left readout and
  right-edge total are unchanged.
- The root `view` passes the same total string to the strip that it passes to the LCD, in both
  normal and shaded modes; README and `docs/invariants.md` describe the combined readout, and
  `make check` passes.

## Done
