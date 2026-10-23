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

### Show each album row's running time (planned 2026-10-23 by the plan role)

**Goal.** Match `docs/screenshots/library.png`'s Albums panel: each album row shows its total running time at the right (`12:38`, `2:58`), replacing the current `View Songs` hint. For the bundled sample library the time is the sum of the album's actual sample-song durations, so `First Record` reads `10:30`, `Second Record` `1:35`, and `Debut` `3:20`; the mockup's decorative numbers do not match the sample data and are not copied. An album with no known songs shows `--:--`, the same placeholder song rows use. The time is display data only: the row still emits `Message::AlbumSelected { epoch, index }`, and Sort, Back, and selection behavior are unchanged.

**Approach.** The fetch shape is settled here, without a per-album request or an unproven REST include: one whole-library songs read is grouped by exact `(album title, artist display name)` and summed. This mirrors the queued artist-count plan's use of one extra collection request, and leaves a song resource without `albumName` unmatched (its album shows `--:--`) instead of failing the whole list.

- `src/library.rs`: add `#[serde(skip)] pub duration_ms: u64` to `Album`, documented as the summed duration of the songs the library reports for the album and `0` when none. It is derived display data, not an Apple Music album field, so it deserializes to `0` and stays out of the serialized payload; existing album payload tests need no field change. Add `Album::duration_ms` to every `Album` literal (default `0` in fixtures).
- `src/sample_library.rs`: after building `songs`, build `songs_by_album` first and set each sample album's `duration_ms` by summing `songs_by_album[album.id]` with `saturating_add` before indexing `albums_by_artist`; pin the three totals in the sample-content tests.
- `src/apple_music/rest/wire.rs`: add `album_name` (`#[serde(rename = "albumName")]`) to the private `Attributes`, and add `Resource::into_album_duration(self) -> Option<(String, String, u64)>` returning the exact non-blank album name, the artist display name (empty when blank, matching `into_album`'s sentinel), and the duration (`0` when absent).
- `src/apple_music/rest.rs`: add `RestLibrary::get_album_durations(&self, session) -> Result<HashMap<(String, String), u64>, AppleMusicError>`, reading `/v1/me/library/songs` through the existing `fetch` pagination and summing durations per key with `saturating_add`. Document that it relies on the library song's `albumName` attribute and that a resource without one is skipped.
- `src/apple_music/browse.rs`: add a private `apply_album_durations(albums: &mut [Album], durations: &HashMap<(String, String), u64>)` that sets each album's `duration_ms` from its `(title, artist)` key and `0` on a miss. In the REST closures of `get_albums_by_artist` and `get_all_albums`, fetch the list, then `rest.get_album_durations(session)?`, then apply; a failed songs request fails the album load through the existing Retry path. The signed-out closures keep returning the sample library, whose albums now carry durations.
- `src/ui/views/browse/rows.rs`: make `album_row`'s secondary label `Cow::Owned(format_track_time(album.duration_ms))` instead of the `"View Songs"` constant, and update its doc comment. `playlist_row` keeps its hint.
- Tests. `src/ui/views/tests.rs`: update `album_row_uses_title_and_selects_the_album_by_index` and `album_row_shows_the_title_alone_when_the_artist_is_empty` to set a duration and expect its formatted time; add `album_row_shows_a_placeholder_when_the_duration_is_unknown` for `0` → `--:--`. `src/sample_library.rs`: pin `10:30`, `1:35`, and `3:20` from the derived durations. `src/apple_music/rest/tests.rs`: add a `get_album_durations` test whose stub song data carries `albumName`, `artistName`, and `durationInMillis`, asserting the summed `(title, artist)` map and the `/v1/me/library/songs` URL. `src/apple_music/tests.rs`: update `get_albums_by_artist_uses_the_rest_library_when_signed_in` and `get_all_albums_uses_the_rest_library_when_signed_in` to use `StubTransport::returning_bodies` (album list, then songs) and assert the assigned duration and both URLs; add a test that a failing songs request fails `get_all_albums`.
- `README.md`: in the Usage browse-list sentence, say each album row also shows its running time at the right (`10:30`), with `--:--` when the source supplied none.
- `docs/invariants.md`: add a `## Browse album running times` section after the browse-row sections, naming `Album::duration_ms`, `SampleLibrary::new`, `RestLibrary::get_album_durations`, `browse::apply_album_durations`, and `views::browse::rows::album_row` as the enforcers, and the new tests as pinned.

**Files touched.** `src/library.rs`, `src/sample_library.rs`, `src/apple_music/rest/wire.rs`, `src/apple_music/rest.rs`, `src/apple_music/rest/tests.rs`, `src/apple_music/browse.rs`, `src/apple_music/tests.rs`, `src/test_support.rs`, `src/ui/views/browse/rows.rs`, `src/ui/views/tests.rs`, `src/ui/tests.rs`, `README.md`, `docs/invariants.md`.

**Acceptance criteria.**

- Every album row in the per-artist Albums view and the flat All Albums view shows its running time (or `--:--` for an unknown total) instead of `View Songs`; the row still emits the same `Message::AlbumSelected { epoch, index }`, and sort, Back, and selection are unchanged.
- The sample library reports `First Record` `10:30`, `Second Record` `1:35`, and `Debut` `3:20`, derived from the sample songs rather than hardcoded.
- The signed-in service returns albums whose `duration_ms` is the sum of the matching library songs' durations, fetching the album collection first and the whole-library songs collection second; an unmatched album is `0`, and a failed songs request surfaces as the existing album-load failure.
- `Album::duration_ms` is skipped by serde and defaults to `0`, so the model's existing wire payload tests stay unchanged.
- `README.md` and `docs/invariants.md` document the labels, `make check` passes, and the new tests cover the row formatting, the sample totals, the REST grouping, and the extra-request failure.

## Done
