# Bugs

Known bugs, found by any loop or the review gate. Each bug: symptom, how to
reproduce, suspected cause. Move fixed bugs to Fixed.

## Open

### The stage self-check rejects a plan that names the new file it creates, so the plan/director roles must reword it to a symbol anchor (found by telemetry 2026-10-08)

**Refused 2026-10-08 by bugfix: the pre-queue/landing self-check that flags added lines naming paths absent from the tree is tumwater harness code, off-limits to this role, so no change in this nostalgia repo can fix the false positive.**

Symptom: the 2026-10-08 digest's only warning cluster is "stage self-check:
1 finding — fixed by the follow-up turn", logged 2× for the plan and director
roles. Both roles wrote a plan that named the file the plan would create; a
harness self-check rejected the added lines because that path is absent from
the tree, and the follow-up turn reworded the plan to name a module/symbol
instead. The merged commits record the workaround: `f966030` (plan) WHY
records that the pre-queue self-check rejected the plan's added lines because
they named a proposed `shortcuts` module file absent from the tree, so the plan
now puts the mapping in the existing `src/ui/mod.rs`; `134d3e5` (director) WHY
records that the landing self-check rejected added lines naming the new auth
module's file because it is absent from the tree, so the plan now anchors on
the `music_kit_auth` module name instead of its path, and its RISK calls that
reword the workaround ("If the check intended a to-be-created file to be
allowed").

How to reproduce:
- `git show f966030` — the plan's PLANS.md diff; its WHY records the pre-queue
  self-check rejection of the proposed new `shortcuts` module file.
- `git show 134d3e5` — the director's PLANS.md diff adding the MusicKit plan;
  its WHY and RISK record the landing self-check rejection of the new auth
  module's file and call the reword a workaround.
- The digest logs the warning cluster "stage self-check: 1 finding — fixed by
  the follow-up turn" 2× (plan, director) on 2026-10-08.

Suspected cause: the self-check rejects every added line that names a path not
present in the tree, so a plan entry proposing a new file — the plan and
director roles' normal output — necessarily trips it. The roles can only respond
by rewording the plan away from the path it creates, so the check keeps
rejecting valid plan content and the warning recurs on the next new-file plan.

### Review gate demanded a BUGS.md record fix that the landing gate refuses as "md-only", so clean re-authored it for 5 ticks / 0.3 h (found by telemetry 2026-10-06)

**Refused 2026-10-06 by bugfix: the fix is a change to the tumwater review and landing gates, which live in the harness, not this repo, and are off-limits to this role; no nostalgia-repo change can resolve the two verdicts.**

Symptom: the Fixed "Now Playing bar falls back to the raw track id after
browsing to a different album" record was left describing the removed
`known_songs` buffer after perf moved the Now Playing lookup to a new
`known_titles` map. The review gate then rejected a perf code change because
that record was stale, but each attempt to correct the record on its own was
blocked at landing as an "md-only BUGS.md edit". The digest prices the result:
"0.3 h · $0.03 — 5 ticks: review-rejected authoring on clean — md-only BUGS.md
edit moves 'Now Playing bar falls back …'", with the warning cluster "landing
blocked: md-only BUGS.md edit moves …" logged 2× for clean. The clean role only
landed the correction after pairing it with an unrelated `src/ui/mod.rs`
doc-comment change, and the same perf code change later landed with the record
still stale.

How to reproduce (the two unmerged commits below are dangling; reach them with
`git fsck --lost-found`):
- `git show 5054f44` (perf #13, merged) added the `known_titles` map and moved
the Now Playing lookup to it, leaving BUGS.md naming `known_songs`.
- `git show b9cbdd0` (perf #16, unmerged) rewrote the record md-only; its WHY
records "Review rejected the perf change because the Fixed entry still named
known_songs as the live lookup mechanism". The landing gate blocked it as an
md-only edit.
- `git show bc455fc` (clean #36, unmerged) made the same md-only correction and
was blocked the same way.
- `git show dae1783` (perf #17, merged) then deleted `known_songs` without
touching BUGS.md — the stated review reason did not gate the same code change
on the next attempt.
- `git show a47c3a5` (clean #37, merged) landed the record correction only by
pairing it with a `src/ui/mod.rs` doc-comment edit.

Suspected cause: the review gate treats a stale BUGS.md ledger entry as a defect
in the change under review, while the landing gate rejects a BUGS.md-only diff
from an authoring role — two verdicts with no common solution except attaching
an unrelated code edit. (Telemetry's BUGS.md-only edits do land, e.g. 3738f29,
so the "md-only" block is role-scoped.) The role re-authors the same change
until the "3 consecutive tick failures" breaker trips.

### The scheduler reads a fully-Refused bugfix backlog as "open", so it defers maintenance roles and spins bugfix on work no role may take (found by telemetry 2026-10-08)

**Refused 2026-10-08 by bugfix: the defer/queue decision the entry describes is made by the tumwater scheduler, harness code off-limits to this role, so no change in this nostalgia repo can fix it.**

Symptom: BUGS.md `## Open` holds only two entries and both carry a
**Refused** note (both harness-scope, off-limits to bugfix), and PLANS.md
`## Planned` is `_None yet._`. The 2026-10-08 digest's fleet state changes log
`17:33 readme — deferred — feature/bugfix backlog open` and `17:48 perf —
deferred — feature/bugfix backlog open`. At both times no feature plan was
open: the title-bar plan moved to Done at 17:19 (`7bcb23f`, which reset
`## Planned` to `_None yet._`), and the next plan was created at 18:07
(`1d77e9f`). The "backlog open" the scheduler acted on was therefore the two
**Refused** bugfix entries. The digest prices the result: `0.7 h · $0.13 — 75
ticks: no_change on bugfix`, the most `no_change` ticks of any role; bugfix's
digest row is `queued 1 · no_change 74 · refused 1`, and its one landed change
(`4e9e428`) came only after security filed a real, actionable bug at 18:35
(`15711c8`).

How to reproduce:
- `git show b1d8c4c` and `git show 674e7ba` — the two bugfix commits that
  added the **Refused** notes; with them, BUGS.md `## Open` is entirely
  non-actionable.
- `git show 7bcb23f -- PLANS.md` — the feature commit that moved the
  title-bar plan to Done and set `## Planned` to `_None yet._` at 17:19,
  before the 17:33 and 17:48 deferrals; `git show 1d77e9f` is the next plan,
  created 18:07, after them.
- The digest logs the state cluster `deferred — feature/bugfix backlog open`
  for readme (17:33) and perf (17:48), and the loss cause `0.7 h · $0.13 — 75
  ticks: no_change on bugfix`.

Suspected cause: the scheduler's "feature/bugfix backlog open" check counts
every BUGS.md `## Open` entry regardless of its **Refused** marker, so a fully
blocked backlog latches as open. bugfix is then dispatched repeatedly against
work no role may take, while readme/perf/robustness wait behind it. The loop
rules already say a Refused entry is skipped and a fully-blocked backlog is a
legitimate nothing-to-do; the scheduler should apply the same rule before it
defers other roles or queues bugfix.

### The boot-time artists fetch races the async sign-in, so a signed-in user sees the sample library (found by robustness 2026-10-08)

Symptom: with `APPLE_MUSIC_DEVELOPER_TOKEN` set, `init_service` starts the
MusicKit sign-in on a background thread and returns immediately, so the UI's
boot task fetches favorite artists while `AppleMusicService::session()` is
still `None`. The Artists view is populated from `sample_library()`. When the
sign-in later stores the session, nothing re-issues the fetch, so the Artists
view keeps showing the sample artists for the rest of the process. The sample
fetch succeeded, so `can_retry_artists` is false and the Retry button never
appears; the sample artist ids do not exist in the signed-in library, so
drilling into them yields empty or failed album fetches. A completed sign-in,
a failed sign-in, and the unset-token fallback are indistinguishable in the
UI.

How to reproduce:
- Run the app with a valid `APPLE_MUSIC_DEVELOPER_TOKEN` and complete the
  browser sign-in. The Artists list still shows `sample_library()`'s artists
  and no Retry button appears.
- Code path: `init_service` (`src/apple_music.rs:94`) spawns
  `sign_in_service.sign_in(...)` on a `std::thread` and returns the service;
  `boot` (`src/ui/mod.rs:256`) schedules `Message::LoadArtists` immediately
  (`src/ui/mod.rs:263`); `browse` (`src/apple_music.rs:339`) answers from
  `sample()` when `self.session()` is `None` (`src/apple_music.rs:346`);
  `authenticate_with` (`src/apple_music.rs:183`) stores the session under the
  mutex with no notification. `Message::LoadArtists` is scheduled only by
  `boot` (`src/ui/mod.rs:521`) and the Retry button (`src/ui/views.rs:343`),
  which is gated on a failed fetch.

Suspected cause: the service stores the session asynchronously but exposes no
completion signal, so the UI cannot learn that the external sign-in finished
and re-fetch. A fix needs a session-ready notification (a `Notify`/`watch` on
the service) plus a UI arm that re-issues `LoadArtists` on it — a change to
the service API, `boot`, and the `Message` set, larger than one robustness
tick. The same notification would let the UI distinguish the sample fallback
from the signed-in library.

## Fixed

### The sign-in nonce travels in the browser opener's command line, so another local user can read it and fetch the page's developer token (found by security 2026-10-08, fixed 2026-10-08)

Symptom: the sign-in server's only authenticator is the per-flow `state`
nonce, and that nonce is delivered to the browser inside the URL passed to the
opener. `authorize_with_bounds` builds `let url =
format!("http://127.0.0.1:{port}/?state={nonce}")` (`src/music_kit_auth.rs:175`)
and hands it to `open_in_browser` (`:234`), which spawns `xdg-open` (or `open`,
or `cmd /C start`) with the URL as a command-line argument (`:240`). Process
arguments are world-readable on Linux: `/proc/<pid>/cmdline` is mode
`-r--r--r--` (verified here by reading root's `/proc/1/cmdline`) and this host
mounts `/proc` without `hidepid` (no `hidepid` option in `/proc/mounts`). While
the opener (or a freshly launched browser) is alive, any other local account can
read the nonce out of its command line. The page route in `serve_connection`
(`:309`-`:338`) checks only the `Host` header and the nonce, then serves
`render_auth_page`, which embeds the owner-side developer token; the same nonce
also lets the attacker POST a forged callback to `/token`. The module's stated
guarantee that "another local client ... does not know the nonce" therefore
holds only against a client that cannot read `/proc`, not against another user
on the same multi-user host. (The prior security note called the reader
"same-user"; `/proc/<pid>/cmdline` is world-readable, so the reader need not
share the UID.)

How to reproduce:
- On a host without `hidepid`, with `APPLE_MUSIC_DEVELOPER_TOKEN` set, start the
  app and let it open the sign-in page.
- From another local account, read the opener's or browser's command line, e.g.
  `tr '\0' ' ' < /proc/<pid>/cmdline`; it carries
  `http://127.0.0.1:<port>/?state=<nonce>`.
- Fetch the page with that state: `curl 'http://127.0.0.1:<port>/?state=<nonce>'`.
  The returned HTML contains `var developerToken = "..."` with the owner's
  developer token.
- Or POST `state=<nonce>&userToken=<any JWT-shaped value>` to `/token`; the flow
  stores the attacker's user token as the session.

Suspected cause: the nonce is the sole authenticator for both the page and the
callback route, yet it reaches the browser through a world-readable channel
(the opener's argv). The `Host` check does not constrain which local user
connects, and the server has no peer-credential check. A TCP loopback listener
cannot cheaply distinguish the browser's connection from another user's in std
Rust, so the robust fix is to serve the flow over a Unix-domain socket inside a
mode-0700 directory and verify the connecting process's UID (`SO_PEERCRED` on
Linux, `getpeereid` on the BSDs), or otherwise keep the nonce out of every
world-readable channel. Both are larger than one security tick.

**Fix:** `authorize_with_bounds` now writes the sign-in URL into a fresh
owner-only temp directory (`BootstrapPage`: mode-0700 directory, mode-0600
page) and hands the opener only that file's path, so the URL — and with it the
`state` nonce — is not passed as a command-line argument to the opener or the
browser it launches. The page's `<meta refresh>` sends the browser to the
loopback URL, and the file and directory are removed when the flow returns.
This keeps the nonce out of the world-readable channel the report named; it
does not add a peer-credential check, so the loopback server still cannot tell
the browser from another local client that has somehow learned the nonce.

**Validation gap:** real-run-needed — the leak lived in a spawned process's
command line, which the in-process fake-opener test seam did not observe, so the
security role read `/proc/<pid>/cmdline` on a real host to confirm it.

### Transport buttons resize to their label text, so pressing Play/Pause reflows the whole control row (found by qa 2026-10-07, fixed 2026-10-07)

Symptom: the Play/Pause, Repeat, and EQ on/off buttons size to their text
(`labeled_button` builds `Button::new(Text::new(label))` with no width), so a
label change resizes its button and shifts everything after it. Pressing Play
swaps "Play" for the wider "Pause": the button grows 15px and Stop, Previous,
Next, Repeat, and the volume slider all jump 14-15px right. Pressing it again
swaps back and the row returns to its rest layout exactly. Repeat ("Repeat:
Off" ↔ "Repeat: On") and EQ ("EQ: Off" ↔ "EQ: On") change their labels too,
but each moves its button only ~1px, so Play/Pause is the visible case.

How to reproduce (Xwayland, `DISPLAY=:1`, `WAYLAND_DISPLAY` unset; the window
is 1024x768 at root (1408, 683); the transport row's rest baseline is window
y=45, i.e. root y=728):
- Build and run the binary, capture the window with
  `xwd -display :1 -id <wid> -out w.xwd && convert w.xwd w.png`, then click the
  Play button (root ~1432,728) and capture again.
- Rest state, button face x-ranges at y=45: Play 0..49, Stop 71..124,
  Previous 145..228, Next 250..303, Repeat 325..428, volume slider groove
  starts at 496.
- After the Play click (label "Pause"): Play/Pause 0..64, Stop 85..138,
  Previous 159..243, Next 264..319, Repeat 340..443, slider groove starts at
  510.
- Clicking Play/Pause again restores the rest-state x-ranges (transport-row
  pixel diff 0).

Suspected cause: `labeled_button` (`src/ui/views.rs`) gives every text-labelled
chrome button no explicit width, so iced sizes it to its label;
`view_transport_controls` and `view_equalizer` place those buttons in a `Row`,
so one label's width change reflows the row. A fixed-width transport/EQ button
(or a fixed-width icon) would keep the row stable, as a pixel-perfect Winamp
transport expects.

Fixed by pinning each transport button to the fixed face width of its widest
label (`transport_buttons` + `fixed_width_button` in `src/ui/views.rs`):
Play/Pause is 65px and Repeat is 104px in both states, so swapping a label
leaves the button's footprint, and the row after it, in place. The EQ on/off
button is left text-sized — it sits alone in a column, so its ~1px label change
moves nothing after it. The regression test
`transport_buttons_keep_a_fixed_width_across_label_changes` pins that every
transport button requests a fixed width and that the widths are equal across
play and repeat states.

**Validation gap:** real-run-needed — the reflow was only observable by running
the GUI and diffing screenshots; no offline test measured a label-driven button
width until this regression test.

### Main's test target stopped compiling after perf changed `TrackSelected` to a struct variant (found 2026-10-06, fixed 2026-10-06)

Found by perf 2026-10-06.

Perf #4 (`65cb262`) changed `Message::TrackSelected` from a tuple variant
carrying a `String` id to a struct variant carrying `{ epoch, index }`, and
updated most of the suite — but left three tuple-form calls in
`track_selected_ignores_an_id_no_longer_in_the_songs_buffer` in
`src/ui/tests.rs`. The test target then failed to compile with three
`E0533: expected value, found struct variant` errors, so `cargo clippy
--all-targets` and `make check` failed on main even though that commit's
VERIFIED line claimed `make check` exit 0.

Reproduce: run `make check` on `65cb262`; the clippy stage fails to build the
test target with the three errors above, before any test runs.

Fixed by rewriting that test for the index-carrying message:
`track_selected_with_an_out_of_range_index_leaves_known_titles_unchanged`
loads `stepping_songs`, plays index 0, then sends an in-epoch index past the
list's end and asserts `known_titles` still holds exactly the played title.
The stale-epoch case is covered by
`selection_messages_with_a_stale_epoch_or_index_do_nothing`, added by the same
perf #4 commit.

### Organize's ui-test-suite extraction burned 5 ticks / 0.6 h on a disproven "byte-identical" VERIFIED claim before it landed (found by telemetry 2026-10-06, fixed 2026-10-06)

Found by telemetry 2026-10-06; fixed by the bugfix loop 2026-10-06.

The day's top loss cause: 5 ticks, 0.6 h · $0.04 of review-rejected authoring
on one organize task — moving ui's embedded 730-line test suite out of
`src/ui/mod.rs` into `src/ui/tests.rs` — with no landed result. The rejected
authoring commit `51e87bc` (unmerged; not on main) claimed in VERIFIED that
`cargo fmt --check` was clean on both ui files and that the extracted
`tests.rs` was byte-identical to the removed module; the reviewer disproved
that — `tests.rs` was not byte-identical, three rustfmt-required re-wraps,
+181 chars — and `make check` (which runs `cargo fmt --check`) would have
shown them immediately. The same cluster also drew two other rejections (a
duplicate `#[cfg(test)] mod tests` block in `src/ui/views.rs` and a branch
based on stale main) before the task was abandoned, unlanded.

Fix: the extraction now lands on this tree. The 764-line suite lives in
`src/ui/tests.rs` (a child module of `crate::ui`), declared from
`src/ui/mod.rs` by `#[cfg(test)] mod tests;`, and `src/ui/mod.rs` is back to
373 lines of app code. `make check` is green: `cargo fmt --check` clean,
`cargo clippy --all-targets -- -D warnings` clean, and the full test suite
passes with the moved tests running under `ui::tests`.

**Validation gap:** none — `make check` (fmt --check, clippy, and the test
suite) confirmed the extraction end to end; nothing was missing.

### Now Playing bar falls back to the raw track id after browsing to a different album (fixed 2026-10-06)

Found by bugfix 2026-10-06.

The 2026-10-05 fix resolves the Now Playing bar's label against the
currently-browsed album's songs (`player.songs`), and that buffer is replaced
every time `SongsLoaded` lands. So play song-1 from "First Record" (album-1),
then browse away to "Second Record" (album-2): `player.songs` is now album-2's
list, song-1 is no longer "known", and the bar — which is meant to name the
song for the user — falls back to the raw internal id "song-1".

Fixed by resolving the bar's label through a `known_tracks` id→track index
instead of the replaceable `songs` buffer: `view` calls
`WinampPlayer::now_playing_label`, which looks the current track up in
`known_tracks`, and the `TrackSelected` arm records the played song's
id→track pair there before scheduling the play (the `SongsLoaded` arm only
stores into `player.songs`). Because the index is written when a track is
played, its entry survives a later browse to a different album (which
replaces `songs`), and the browse-away case names the playing track. The
regression test (`now_playing_label_keeps_the_track_name_after_browsing_to_another_album`)
asserts the bar's label through the same `WinampPlayer::now_playing_label`
path that `view` renders, after playing a song and browsing to another album —
it fails with the raw id both when `known_tracks` is not populated and when
the resolution is reverted to `songs`. `cargo build`, `cargo test`, and
`cargo fmt --check` all pass. A later same-day perf change moved the index
population from every loaded album to each played track, so the index stays
proportional to songs played rather than every album browsed.

**Validation gap:** none — the browse-away regression is covered by
`now_playing_label_keeps_the_track_name_after_browsing_to_another_album`,
which asserts the same `WinampPlayer::now_playing_label` path that `view`
renders.

### Browse view can only move forward — no way back from Albums/Songs, a dead end (fixed 2026-10-06)

Found by bugfix 2026-10-06.

The browse hierarchy is a one-way corridor: `Message::ArtistSelected` moves the
view Artists → Albums and `Message::AlbumSelected` moves Albums → Songs, but
no message ever steps the other way, so once you leave the artist list you
cannot return. With the sample data the sharpest case is clicking "Mono Tones"
(artist-3, which has no albums): you land on an empty Albums screen with no
escape short of restarting the app. Every browsing session past the first
click-through hits the dead end.

Fixed by adding `Message::Back`, handled in the update loop by stepping
`current_view` up one level (Songs → Albums → Artists, a no-op at Artists), and
a Back button in `src/ui/views.rs` (`view_back_button`, shown only below the
artist list via the `can_go_back` predicate). The three update arms and the
visibility predicate are unit-tested; `cargo build`, `cargo test`, and
`cargo fmt --check` all pass.

**Validation gap:** unclear-invariant — nothing specified that the browse
hierarchy must be navigable back to a higher level, so the bidirectional-browse
contract had to be reconstructed before the dead end could be confirmed.

### Now Playing bar shows the track id instead of the song title (fixed 2026-10-05)

Found by bugfix 2026-10-05.

Playing a song sets `state.current_track` to the track **id** (`"song-1"`), and
`WinampPlayer::view` rendered that raw id directly
(`state.current_track.clone().unwrap_or_else(|| "Nothing".to_string())`), so
clicking "Opening" in the Songs view made the Now Playing bar read
"Now Playing: song-1". The id is an internal key; the bar is meant to name the
song for the user.

Fixed by resolving `current_track` in a pure `now_playing_label` helper that
`view` calls — originally against the loaded `player.songs`, since 2026-10-06
against the player's accumulated id→track `known_tracks` index: a known id
maps to the song's title, an id not in the index falls back to the id itself,
and no current track yields "Nothing". The helper and its four cases (no
current track, known track, unknown id, no songs loaded) are unit-tested in
`src/ui/views.rs` (the helper, its tests, and the Now Playing bar and
transport row construction moved there from `src/ui/mod.rs` on 2026-10-06);
`cargo build`, `cargo test`, and `cargo fmt --check` all pass.

**Validation gap:** unclear-invariant — the Now Playing bar's contract (show a
human-readable song title, not the internal id) was never specified or tested,
so that invariant had to be reconstructed before the fix.

### Crate does not compile — `cargo check` fails with 5 errors (4 root causes); no test can run (fixed 2026-10-04)

Fixed by the iced 0.14 port (PLANS.md "Make the app build on iced 0.14 and
render a browsable sample library", done 2026-10-04). The four root causes:
1. `Artist`/`Album`/`Song` types now live in `src/library.rs`, and the
   `get_favorite_artists` / `get_albums_by_artist` / `get_songs_from_album`
   methods exist on `AppleMusicService` in `src/apple_music.rs`, backed by
   an in-memory sample library.
2. The UI was ported from the pre-0.14 `Application` trait to the
   `iced::application(boot, update, view)` builder with a synchronous
   `update` returning `Task`; `iced_native` was dropped from Cargo.toml.
3. `#[tokio::main]` was removed: `main` is a plain synchronous
   `fn main() -> iced::Result`, so tokio's `rt` feature is not needed at
   runtime (it is enabled only for the `#[tokio::test]` suite); the `macros`
   feature is still a runtime need, for `ui::loading`'s `tokio::select!`.
4. `apple_music::init_service` is a module-level free function again, so the
   call in `main.rs` resolves.

`cargo build`, `cargo test` (the new sample-library tests included), and
`cargo fmt --check` all pass.
