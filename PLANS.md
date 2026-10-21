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
> Recommend the plan loop keep scheduling the open equalizer and seek plans.

## Planned

_None yet._

### Show the item count beneath artist, album, and playlist lists (planned 2026-10-21 by the plan role)

**Goal.** Every loaded non-song browse list shows how many rows it holds beneath the rows — `3 artists`, `2 albums`, `1 playlist` — as the library mockup's status area does (`docs/screenshots/library.png`). The song lists keep their existing count-and-time footer and gain nothing new. This is independent of the pending row numbering: it reads only the slice length each view already receives.

**Approach.**

- `src/ui/views/browse.rs`: add a pure `pub(super) fn list_count_summary(count: usize, singular: &str, plural: &str) -> String` returning `"<count> <noun>"`, with `singular` chosen exactly when `count == 1` and `plural` otherwise. It formats any count, including `0`, so the wording is testable without an iced renderer (`format_list_time` is the sibling formatter for the song footer).
- Add a private `fn list_with_count<'a>(list: Element<'a, Message>, count: usize, singular: &str, plural: &str) -> Element<'a, Message>`: when `count` is non-zero, wrap `list` in a `Column::new().push(list).push(Text::new(list_count_summary(...)).size(14))` with `Length::Fill` width and `Length::FillPortion(3)` height, mirroring the song footer in `song_list`; when `count` is zero, return `list` unchanged so a loading, error, or empty placeholder gets no `0 artists` footer.
- Wrap the three non-song list builders at their row-construction sites: `view_artists` passes `artists.len()` with `"artist"`/`"artists"`, `album_list` passes `albums.len()` with `"album"`/`"albums"` (so both `view_albums` and `view_all_albums` gain the footer once), and `view_playlists` passes `playlists.len()` with `"playlist"`/`"playlists"`. Do not wrap inside `scrollable_list` itself: `song_list` already appends its own count-and-time footer, so wrapping there would give every song list a second footer.
- Update the doc comments of `view_artists`, `album_list` (which `view_albums` and `view_all_albums` both delegate to), and `view_playlists` to say a non-empty list appends its item-count footer, and note in `list_with_count` why it skips an empty list.
- `src/ui/views/tests.rs`: import `list_count_summary` and add `list_count_summary_uses_the_singular_and_plural`, asserting `"1 artist"` for a count of one, `"3 artists"` for a count of three, `"1 playlist"`, and `"0 albums"`. The existing `browse_views_construct_over_the_loaded_library` and `browse_views_construct_over_an_empty_list` build the populated and empty branches of every wrapped builder through `construct_every_browse_view`.
- `README.md`: extend the existing Usage sentence — which already says each song list shows its song count and total playing time — to also say artist, album, and playlist lists show their item count beneath the rows. Keep the hunk to that sentence.
- `docs/invariants.md`: the browse-hierarchy and reply-freshness guarantees do not change, and the document makes no claim about footers, so leave it unchanged.

**Files touched.** `src/ui/views/browse.rs`, `src/ui/views/tests.rs`, `README.md`.

**Acceptance criteria.**

- `make check` passes.
- `list_count_summary` returns `"1 artist"` for `(1, "artist", "artists")`, `"3 artists"` for `(3, "artist", "artists")`, `"1 playlist"` for `(1, "playlist", "playlists")`, and `"0 albums"` for `(0, "album", "albums")`.
- A loaded Artists, Albums, All Albums, or Playlists view renders its rows followed by exactly one footer whose count equals the number of rows passed in; an empty, loading, or failed one still renders only its placeholder, with no `0 …` footer.
- Song lists keep the existing `song_list_summary` footer and call no new footer helper, so no song list shows two footers.
- Row order, row labels, selection messages, epochs, and placeholder wording are unchanged, and both construct tests build every browse view without panicking.
- The change touches only the three files listed above (plus the `PLANS.md` edit and backlog moves), and the README hunk is limited to the item-count clause.

## Done
