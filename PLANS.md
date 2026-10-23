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
> at the right edge of the LCD well, and `view` (`src/ui/view.rs`) stacks that
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

### Preview a seek drag's target time in the Now Playing readout (planned 2026-10-23 by the plan role)

**Goal.** While the user drags the seek slider beneath the Now Playing LCD, show the time the thumb points at in the large left-hand readout, so seeking a long preview is not blind. Today `WinampPlayer::now_playing_elapsed` in `src/ui/player.rs` reads only `playback_position`, while the slider thumb reads `seek_preview.or(playback_position)` inline in `src/ui/view.rs`; a drag therefore moves the thumb but leaves the readout on the backend's last position. Releasing already seeks to the drag and records that position, so the readout lands where the thumb pointed with no playback change. It touches only the Now Playing readout and the seek bar's shared position source.

**Approach.**

- `src/ui/player.rs`: add `pub(in crate::ui) fn readout_position(&self) -> Option<Duration>` returning `self.seek_preview.or(self.playback_position)`. Document it as the one position the seek thumb and the large readout both show: an in-progress drag wins, otherwise the backend-reported position. Use it in `WinampPlayer::now_playing_elapsed` in place of `self.playback_position`, so elapsed mode previews the drag and remaining mode counts down from it through the existing `views::now_playing_elapsed` paths. Extend the `seek_preview` field's doc to say the held drag also previews in the readout.
- `src/ui/view.rs`: pass `player.readout_position()` to `views::view_seek_bar` instead of the inline `player.seek_preview.or(player.playback_position)`, so the thumb and readout cannot drift apart.
- Leave the right-edge total, marquee, paused blink, position/format/duration/spectrum tick batches, and the seek-on-release path unchanged. `reset_position_readout` still clears `seek_preview` on Stop and a new selection, so the readout returns to the cleared state then.
- `src/ui/tests.rs`: add `now_playing_elapsed_previews_the_seek_drag` beside `now_playing_elapsed_shows_the_position_while_it_is_known`. Build a player with `play_song_1_then_browse_to_album_2()` (song 1 is 3:30), set `playback_position = Some(Duration::from_secs(45))`, then send `Message::SeekPreviewChanged(12.0)`; assert `player.readout_position()` is `Some(Duration::from_secs(12))`, the elapsed readout is `"00:12"`, and `player.now_playing_time(...)` is still `"3:30"`. Set `player.time_display = views::TimeDisplay::Remaining` and assert `"-03:18"`. Set `player.seek_preview = None` and assert the readout falls back to `"00:45"` in elapsed mode (and `"-02:45"` in remaining mode), proving the drag is a preview, not a replacement.
- `README.md`: in the Usage sentence about the seek slider, replace "dragging moves only the thumb, and releasing seeks to that position" with wording that dragging moves the thumb and previews that position in the large elapsed readout (counting down from it in remaining-time mode), and releasing seeks to it. Do not touch the `<!-- tumwater:status:start -->` block.
- `docs/invariants.md`: in the `## Now Playing label` seek-slider bullet, add that the large readout follows the in-progress drag in both time modes while the right-edge total does not, and that the thumb and readout share `WinampPlayer::readout_position`; add the new test to that bullet's Pinned-by list.

**Files touched.** `src/ui/player.rs`, `src/ui/view.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- While `seek_preview` holds a drag, the large readout shows that time in elapsed mode and counts down from it in remaining mode; the right-edge total is unchanged.
- The seek slider's thumb and the large readout both resolve their position through `WinampPlayer::readout_position`, so an in-progress drag wins over `playback_position` and clearing the drag falls back to the reported position.
- A release still sends exactly one `seek_output_to` with the dragged position and records it; Stop and a new selection still clear the drag and readout through `reset_position_readout`. No marquee, blink, tick, format, duration, spectrum, or playback behavior changes.
- `README.md` and `docs/invariants.md` describe the preview, `make check` passes, and the new test covers the drag, the remaining-mode countdown, and the fallback.

### Show each artist row's album count (planned 2026-10-23 by the plan role)

**Goal.** Match `docs/screenshots/library.png`'s Artists panel: each artist row shows how many albums the library holds for that artist at the right (`2 albums`, `1 album`), replacing the current `View Albums` hint. An artist with no albums shows no count, as the mockup's third row does. The count is display data only: the row still selects the artist by index and epoch, Sort still orders artists by name, and no browse or playback behavior changes. The count is derived from albums the service already knows, so this adds no new Apple Music endpoint shape.

**Approach.**

- `src/library.rs`: add `pub album_count: usize` to `Artist`, documented as the number of albums the loaded library holds for the artist and `0` when none. Mark it `#[serde(skip)]` so it stays out of the wire payload (a derived value, not an Apple Music field) and deserializes to `0`. Add a pure `pub fn album_counts_by_artist(albums: &[Album]) -> HashMap<&str, usize>` that counts non-empty `Album::artist` display names, ignoring empty names.
- `src/sample_library.rs`: build `albums` first, then derive each sample artist's `album_count` from `albums` by `artist_id` (The Sample Band 2, Echo Chamber 1, Mono Tones 0) before storing the artist list. Add a test pinning those three counts.
- `src/apple_music/rest/wire.rs`: `Resource::into_artist` initializes `album_count: 0`; the artist resource carries no album count.
- `src/apple_music/browse.rs`: in the REST branch of `AppleMusicService::get_favorite_artists`, after `rest.get_favorite_artists(session)?`, also call `rest.get_all_albums(session)?`, run `album_counts_by_artist`, and assign each artist's count by matching `Artist::name` to the albums' `Album::artist` name. Keep the signed-out fallback at `sample_library().artists.clone()`, whose counts are now populated. Document that the signed-in load makes one extra paginated album request, that a failure in either request fails the artists load through the existing Retry path, and that exact display-name matching is the only link because `get_all_albums` deliberately leaves `artist_id` empty.
- `src/ui/views/browse/rows.rs`: add `pub(in crate::ui::views) fn album_count_label(count: usize) -> Cow<'static, str>` returning `""` for 0, `"1 album"` for 1, and `"N albums"` otherwise; use it as `artist_row`'s secondary label in place of the `"View Albums"` constant and update that row's doc comment.
- Tests. `src/ui/views/tests.rs`: update `artist_row_uses_name_and_selects_the_artist_by_index` to give its artist two albums and expect `"2 albums"`; add `album_count_label_singular_plural_and_none` covering 0, 1, and 2. `src/library.rs` tests: cover `album_counts_by_artist` (zero, one, many, and the empty-name skip) and that `album_count` is not serialized. `src/sample_library.rs`: pin the three sample counts. `src/apple_music/tests.rs`: add `get_favorite_artists_counts_albums_by_artist_name_from_the_rest_library`, using a two-body `StubTransport` (artists then albums) and asserting the resulting counts and both requested URLs; update `get_favorite_artists_uses_the_rest_library_when_signed_in` for the second request and its zero counts. Add `album_count: 0` to the `Artist` literals in `src/test_support.rs`, `src/ui/tests.rs`, and `src/apple_music/rest/tests.rs` unless the literal is about the count.
- `README.md`: in the Usage browse-list sentence, say each artist row shows its album count at the right (`2 albums`, `1 album`, and no count for an artist with none), while song rows keep their lengths and album rows keep `<artist> - <title>`.
- `docs/invariants.md`: add a `## Browse row labels` section after `## Browse row order` stating the artist-row count guarantee, naming `Artist::album_count`, `album_counts_by_artist`, `AppleMusicService::get_favorite_artists`, and `views::browse::rows::album_count_label` as the enforcers, and listing the new tests as pinned.

**Files touched.** `src/library.rs`, `src/sample_library.rs`, `src/apple_music/browse.rs`, `src/apple_music/rest/wire.rs`, `src/apple_music/tests.rs`, `src/test_support.rs`, `src/ui/views/browse/rows.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `src/apple_music/rest/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- The Artists list renders `1 album`, `N albums`, or no count; the row still emits the same `Message::ArtistSelected { epoch, index }`, and Sort and Back behavior are unchanged.
- The signed-out service returns the sample artists with counts 2, 1, and 0; the signed-in service fetches the artist list first and the all-albums collection second, counts albums by exact `Album::artist` name, and surfaces a failed album request as the existing artists load failure (Retry).
- `Artist::album_count` is skipped by serde and defaults to `0` when absent, so the model's wire payload tests stay unchanged.
- `README.md` and `docs/invariants.md` document the labels, `make check` passes, and the new tests cover the labels, the sample counts, the REST count, and the serialization skip.

## Done
