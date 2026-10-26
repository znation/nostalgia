# Fixed bugs

Fixed bugs, newest first, moved here by tumwater from BUGS.md's `## Fixed`. This file
only grows: search it with grep, or read one entry by its line range. Never read it whole.

## Fixed

### Stop leaves the Jump to time panel open but invisible (found by the bugfix role 2026-10-25) — fixed 2026-10-25 by the bugfix role

Stop (or a new selection) cleared `playback_duration`, which the view needs to draw the Jump to time panel, but left `jump_time_visible` set. The panel vanished while the flag stayed true, so the next Ctrl+J only closed the invisible panel, and the panel reappeared with its old query once a later preview reported a length. `reset_position_readout` now clears the flag; regression test `stop_closes_the_panel_whose_preview_length_it_clears` in `src/ui/tests/jump_time_tests.rs` failed before the fix.

**Validation gap:** none — a unit test on the existing player fixture reproduced it.

### The largest test modules are flat files, so a failure's fixtures are far from the test (found by the steward role 2026-10-24) — fixed 2026-10-24 by the bugfix role

**Symptom.** Four test modules are still single flat files with no inner `mod` splits: `src/ui/tests.rs`
(9,087 lines), `src/ui/views/tests.rs` (3,613), `src/apple_music/tests.rs` (3,185), and
`src/music_kit_auth/tests.rs` (1,840). A test in
`src/ui/tests.rs` can sit thousands of lines from the fixture it uses, so finding that fixture means
grepping or scanning the whole file, and a full read costs most of a role's reading budget. Concurrent
worktrees also keep adding tests to the same file, so their merges collide there. This is a
maintainability risk, not a runtime defect: nothing fails because of it today.

**How to reproduce.** `wc -l` on the paths above reports those lengths, and
`grep -c '^\s*mod '` reports 0 for each of the four, so none is split by concern (re-measured 2026-10-24).

**Suspected cause.** Each `tests.rs` grew by appending tests for every feature that landed in its module;
nothing divides it into submodules the way the production code is divided, so the file keeps growing
without a boundary that prompts a split.

**Direction.** When a bug is next hard to locate inside one of these files, split that module by concern
(for example browse, playback, settings, and layout tests) into sibling submodules or a `tests/`
directory, keeping shared fixtures in `test_support`. Split the file that is being worked in rather than
all four at once, and move tests without changing their assertions.

**Progress.** `src/audio/tests.rs`, `src/apple_music/rest/tests.rs`, and `src/music_kit_auth/tests.rs` are now split into `tests/` directories by concern (the last into `bootstrap`, `tokens`, `authorize`, `http_request`, `server`, and `opener`, with shared helpers in `mod.rs`), tests moved without changing assertions. Also `src/apple_music/tests.rs`, now `tests/` (`validation`, `browse_sample`, `browse_rest`, `play_track`, `audio_forwarding`, `transport`, `threads_and_session`, shared helpers in `mod.rs`). Also `src/ui/views/tests.rs`, now `tests/` (`browse`, `now_playing`, `lcd_display`, `transport`, `equalizer`, `shade`, `chrome`, `seek_wheel`, shared helpers in `mod.rs`). Finally `src/ui/tests.rs`, now `tests/` (one `*_tests.rs` file per concern: `settings`, `loading`, `transport`, `shortcuts`, `mixer`, `browse`, `scroll`, `flat_lists`, `search`, `view`, `stepping`, `window`, `sort`, `readouts`, `timers`, `panels`; shared helpers in `mod.rs`). All four modules are split.

**Validation gap:** none — the failure was structural (file size), and `wc -l` plus the unchanged test run confirmed the split.

### Preview download rejects a body of exactly the 10 MiB cap (found by the bugfix role 2026-10-23) — fixed 2026-10-23 by the bugfix role

Symptom: a preview response whose body is exactly `MAX_PREVIEW_BYTES` (10 MiB) fails with
`reading the preview failed: the response body is larger than request limit: 10485760`, even
though the download's documented cap accepts a body of the cap.

How to reproduce: `a_preview_of_exactly_the_cap_reaches_the_decoder` in `src/audio/tests.rs`
serves a `Content-Length: 10485760` body and calls `download_and_decode`; before the fix it
returned that read error instead of reaching the decoder.

Suspected cause: `download_and_decode` set `ureq`'s wire limit to exactly `MAX_PREVIEW_BYTES`,
so reading the last byte of an exactly-at-cap body tripped the limiter. The REST transport had
the same boundary and was fixed to `MAX_PREVIEW_BYTES + 1`; this preview call kept the old
limit. The wire limit now sits one byte above the cap, and the decoded-length check still
rejects anything over the cap.

**Validation gap:** unclear-invariant — the preview section stated a cap but not whether a body
of exactly the cap is legal; that accepted boundary was carried over from the REST transport's
reconstructed section and is now stated in `docs/invariants.md` under "Preview download".

### Preview URL guard accepts IPv4 multicast and reserved ranges (found by the bugfix role 2026-10-23) — fixed 2026-10-23 by the bugfix role

Symptom: `ip_is_internal` — the preview fetch's SSRF guard — returned `false` for IPv4
multicast addresses such as `224.0.0.1`, `224.0.0.251` (mDNS), and `239.255.255.250` (SSDP), and
for the reserved `240.0.0.0/4`, benchmarking `198.18.0.0/15`, IETF protocol-assignment
`192.0.0.0/24`, and deprecated 6to4 relay anycast `192.88.99.0/24` ranges. IPv6 multicast was
already refused, so a preview URL whose host was an address literal in one of those IPv4 ranges
passed `preview_url_problem` and the audio agent's resolver and would be fetched, reaching a
link-local service.

How to reproduce: the table test
`ip_is_internal_refuses_every_internal_range_and_allows_public_addresses` in
`src/apple_music/tests.rs`; before the fix `ip_is_internal(224.0.0.1)` was `false` and the test
failed with `ip_is_internal(224.0.0.1) should be true`.

Suspected cause: the IPv4 branch enumerated `is_private`, `is_loopback`, `is_link_local`,
`is_unspecified`, `is_broadcast`, `is_documentation`, `0.0.0.0/8`, and CGNAT, but omitted the
non-public ranges above; the IPv6 branch did call `is_multicast`.

**Validation gap:** unclear-invariant — the guard's written contract named only "private,
loopback, link-local, or unspecified" while the code also rejected documentation, `0.0.0.0/8`,
and CGNAT, so the intended "public unicast only" rule had to be reconstructed; it now lives in
`docs/invariants.md` under "Preview host guard".

### Shuffle's Previous skips a play paused before its completion (found by the bugfix role 2026-10-23) — fixed 2026-10-23 by the bugfix role

Symptom: with Shuffle on, Previous retraces past a track the user paused, so the step lands on an
older song than the one actually played before the current one. `README.md` says Shuffle's Previous
retraces "the tracks actually played before the current one".

How to reproduce: select a song, then press Pause after the selection's play has committed but
before its `TrackPlayed` completion reaches `update` (the Pause's `TransportSettled` can be
processed first). The play committed, but `update` sees `is_playing == false`. Deterministically,
the regression test `track_played_records_a_play_paused_before_its_completion` in
`src/ui/tests.rs` drives that state order: record song-1 and song-2, set `current_track` to song-3
and call `AppState::pause`, then feed `TrackPlayed`, record song-4, and ask
`play_history.previous` from song-4. Before the fix it returned song-2, skipping the paused song-3.

Suspected cause: `handlers::playback::track_played` inferred the play's outcome from the shared
`is_playing` flag (`if state.is_playing && let Some(current) ...`), but Pause and Stop clear that
flag after a play has committed, and Pause's `paused` flag is gone again after a Stop. The
completion now carries its own `committed` outcome from `play_into` (`true` only for `Ok`), so the
handler never reads mutable transport state to decide whether the play happened.

**Validation gap:** unclear-invariant — the rule for a play paused before its completion was not
written down, and the handler inferred it from mutable shared state; the reconstructed guarantee
now lives in `docs/invariants.md` under "Play history".

### Equalizer frequency labels run together at the default window size (found by qa 2026-10-23) — fixed 2026-10-23 by the bugfix role

Symptom: the equalizer's ten band labels are unreadable as separate frequencies. The row reads
`60 1703106001K 3K 6K 12K14K16K`: `170`, `310`, `600`, and `1K` touch, and so do `12K`, `14K`,
and `16K`. `docs/screenshots/player.png` shows the same ten labels separated under their own
sliders.

How to reproduce (X11/Xwayland, `DISPLAY=:1`, `WAYLAND_DISPLAY` unset):

1. `cargo build --locked` and run `target/debug/nostalgia` from a scratch directory. The app
   opens one window titled `nostalgia`; at this machine's 1.5x scale it is 1536x1152 at root
   +1152+471.
2. Capture the window (`DISPLAY=:1 import -window <id> app.png`; `xwd` also works but its
   frames can lag several seconds behind the app, so use `import` and re-capture before
   measuring).
3. Crop the band-label row, roughly x 0-420, y 458-503 in image pixels:
   `convert app.png -crop 420x45+0+458 +repage -resize 200% labels.png`.

Expected: ten separated labels, `60`, `170`, `310`, `600`, `1K`, `3K`, `6K`, `12K`, `14K`,
`16K`, each readable under its own slider as in `docs/screenshots/player.png`.

Actual: adjacent three-character labels have no visible gap and read as one string,
`1703106001K` and `12K14K16K`.

Suspected cause: `view_equalizer` in `src/ui/views/equalizer.rs` pushes each band as a
`Column` holding a `VerticalSlider` and a size-12 `Text` label into a `Row` with the default
zero spacing and no label width, so the labels touch wherever the text is wider than the
slider column.

**Validation gap:** none — at fix time the suite had no layout assertion for the equalizer
labels, so qa confirmed the collision only from the captured window; the shared
`src/ui/layout_capture.rs` harness now lays the band row out with iced's headless tiny-skia
renderer and asserts a gap between every pair of neighbouring labels.

### Sort renders below Top instead of beside it, contradicting the documented layout (found by qa 2026-10-23) — fixed 2026-10-23 by the bugfix role

Symptom: the README's Usage section says "A `Sort` button beside `Top` reorders the showing
list", and the Sort invariant in `docs/invariants.md` says `views::view_sort_button` "draws
the button beside Top", but on the Artists list the two buttons render in consecutive rows at
the left edge, one above the other. `src/ui/mod.rs` pushes each button into the root `Column`
separately (`view_top_button()` around line 737, `view_sort_button()` around line 739), so no
width ever places them side by side. This is a layout-only divergence: the buttons are both
visible and labelled.

How to reproduce (X11/Xwayland, `DISPLAY=:1`, `WAYLAND_DISPLAY` unset):

1. `cargo build --locked` and run `target/debug/nostalgia` from a scratch directory, with the
   display exported. The app opens one window titled `nostalgia`; at the 1.5x scale of this
   machine it is 1536x1152 at root +1152+471 (window id `0x1600002` in this run).
2. Capture it: `DISPLAY=:1 xwd -id 0x1600002 -out app.xwd && convert app.xwd app.png`.
3. Inspect the left edge above the `Search library` box: `Top` and `Sort` occupy the same
   narrow x range at the far left (roughly x 0-73 image pixels) in consecutive ~40-pixel-tall
   rows, Top roughly y 440-480 and Sort roughly y 480-520 (image pixels; divide by 1.5 for
   logical layout pixels). Both share the same left edge because each is its own row.

Expected: `Top` and `Sort` on the same row, Sort immediately to the right of Top, as the README
and the invariant's "beside Top" wording promise.

Actual: Sort is the next row below Top, and `All Songs` is the row below that; every browse
control stacks vertically at the left edge.

Suspected cause: the root `Column` in `src/ui/mod.rs` gets the two buttons as two separate
`.push()` calls, and a `Column` stacks its children. Wrapping `view_top_button()` and
`view_sort_button()` in one `Row` before pushing it would render them beside each other.

**Validation gap:** none — at fix time the existing suite had no layout assertion, so qa could confirm the divergence only by running the app and measuring the captured window; the shared `src/ui/layout_capture.rs` harness now lays the root `view` out offline and `browse_nav_buttons_place_sort_beside_top` pins their laid-out bounds.

### Sorted browse lists stay sorted after Back, contradicting the one-shot guarantee (found by the bugfix role 2026-10-23) — fixed 2026-10-23 by the bugfix role

**Symptom.** Pressing Sort, opening a child list, then pressing Back showed the parent list still in
sorted order. `docs/invariants.md` ("Browse row order") and the README both promise the reorder is
one-shot, so navigating away and back restores the library's order.

**Reproduce.** Seed `player.artists` with rows out of library order, press `Message::SortBrowse`,
switch to `CurrentView::Albums` (what opening an artist leaves behind), then press
`Message::Back`. The artists came back sorted instead of in their seeded order.
`back_restores_a_sorted_artists_list_to_library_order` and
`back_from_songs_restores_a_sorted_albums_list_to_library_order` fail without the fix.

**Cause.** `browse::back` only switched `current_view` and scrolled, so a parent list that stayed
in memory kept the order Sort put it in. Back cannot refetch cheaply (it has no artist id for the
Albums list), so `BrowseList` now records the cumulative permutation back to library order when
Sort runs, composes it on a second Sort, and `restore_library_order` inverts it on Back for the
list being returned to. Any transition that replaces the rows drops the saved permutation, since a
fresh fetch is already in library order.

**Validation gap:** unclear-invariant — the guarantee was written in `docs/invariants.md` and the
README but no test pinned Back, so I first had to confirm the documented one-shot rule (not Back's
no-refetch behavior) was the intended contract; the two new tests now pin it.

### Sort reorders a loading browse list instead of ignoring the press (found by the bugfix role 2026-10-23) — fixed 2026-10-23 by the bugfix role

**Symptom.** Pressing the browse `Sort` button while a list is marked `loading` but still holds
rows reorders those rows and bumps the list's epoch. The documented guarantee
(`docs/invariants.md`, "Browse row order") and the sort plan's acceptance criteria both say a
loading list is a no-op: no reorder, no epoch bump.

**Reproduce.** The app's own transitions always empty a list in `clear` before setting `loading`,
so the state is constructed: give `player.artists` two out-of-order rows and set
`player.artists.loading = true`, then drive `Message::SortBrowse`. The rows came back sorted
(`["Apple", "banana"]`) and the epoch advanced, instead of staying `["banana", "Apple"]` with
the epoch unchanged. `sort_with_an_empty_or_loading_list_is_a_noop` now seeds this state and
fails without the fix.

**Cause.** `BrowseList::sort_rows_by` guarded only `self.items.len() < 2`. It relied on every
loading list also being empty, so the documented loading rule held by accident of transition
order rather than by the guard itself. The guard now returns early when `self.loading` too, and
the two artist sort tests (which seeded rows into the startup list that is still marked loading)
now mark their list loaded, as a list with rows really is.

**Validation gap:** no-repro — the app cannot leave a loading list holding rows, so the failure
had to be reproduced with a crafted list state rather than a real run; the closest tag fits
because no transition reaches the state, and the regression test now pins it anyway.

### A stalled connection is served for its whole budget before the next one, so repeated stalls starve the sign-in callback (found by security 2026-10-22) — fixed 2026-10-22 by the bugfix role

Symptom: the loopback sign-in server serves accepted connections one at a
time, and a connection that never finishes its request is held for the whole
per-connection budget (`CONNECTION_READ_TIMEOUT`, five seconds in production)
before the next connection is accepted. One stalled connection therefore delays
the browser's callback by five seconds; the existing test
`authorize_serves_the_callback_after_a_stalled_connection_is_dropped`
(`src/music_kit_auth/tests.rs`) pins that a single stall is dropped and the
callback that follows is served. The repeated case is not bounded, because the
cost of each hostile connection is *serialized* behind the previous one: N
stalled connections cost N × the budget of the flow's own deadline. With the
production constants (`CONNECTION_READ_TIMEOUT` = 5 s, `AUTH_TIMEOUT` = 300 s)
sixty stalled connections consume the whole five-minute sign-in window, the
browser's `POST /token` callback is never served, and the flow fails with
"timed out waiting for the Apple Music sign-in callback after 300s" — the
owner cannot sign in while the stalls are held open.

Source: any local process that can open TCP connections to the sign-in port.
The port is ephemeral but enumerable from `/proc/net/tcp`, which every local
user can read, and the attacker never has to present the `state` nonce: the
stall happens before any request is routed.

Sink: the single-threaded accept loop in `authorize_with_bounds`
(`src/music_kit_auth.rs`). It calls `listener.accept()` and then serves that
connection inline through `serve_connection`, which blocks in
`read_http_request` until the connection's deadline expires
(`arm_read_timeout` arms `CONNECTION_READ_TIMEOUT`). Only then is the next
connection accepted, so the browser's callback waits in the kernel's accept
backlog behind every hostile connection queued before it.

Path: `authorize_with_bounds` -> `listener.accept()` -> `serve_connection` ->
`read_http_request` -> `read_or_none` -> `arm_read_timeout` (the five-second
per-connection budget) -> the stalled connection is dropped -> the next
`accept()` happens only then -> the browser's `POST /token` is served last.

Missing check: the per-connection budget bounds one connection, not the queue
behind it. There is no aggregate bound on the time hostile connections may
consume and no parallelism, so the budget multiplies by the number of
stalls.

How to reproduce:
- Code path: `src/music_kit_auth.rs`, `authorize_with_bounds` serves each
  accepted connection inline (`serve_connection(&mut stream, ...)`), so the
  next `accept()` happens only after the current connection's deadline.
- Measured (temporary test, since removed): ten connections that each sent
  `POST /token HTTP/1.1\r\nHost: 127.0.0.1\r\n` and then stalled, followed by
  the real callback, with a 100 ms per-connection budget and a 600 ms flow
  deadline. The flow returned `Err("timed out waiting for the Apple Music
  sign-in callback after 600ms")` after 607 ms, and the callback connection
  read an empty response — the callback was never served. The same run with a
  single stalled connection serves the callback (the existing test's case).
- Manual: run with `APPLE_MUSIC_DEVELOPER_TOKEN` set, find the sign-in port
  (`ss -ltnp`), open sixty connections that send nothing, then complete the
  sign-in in the browser. The browser's callback waits behind the queued
  connections and the flow times out.

Suspected cause: the accept loop is single-threaded by design ("Connections
are served one at a time, so each is bounded by `CONNECTION_READ_TIMEOUT` from
the moment it is accepted"), and the module doc draws from that
single-connection bound the conclusion that a local client "cannot starve the
browser's callback until the flow gives up". The bound holds for one
connection; the queue behind it is unbounded, so repeated stalls do starve the
callback. Fixing it means changing how the accept loop dispatches work —
serving accepted connections concurrently, or multiplexing the pending ones —
and the obvious form of that change (a thread per connection) hands the
attacker control of the thread count, so the choice between the options needs
a maintainer rather than a one-line guard.

Fix: the sign-in server now reads the connections it has accepted together instead of one after
another. `authorize_with_bounds` keeps the accepted connections in a pending list, and each pass
polls every one of them without blocking: `http::RequestReader` owns a non-blocking socket and
parses the request from the bytes received so far, so a connection that never finishes its request
no longer holds the accept loop while it waits. Each connection is still dropped when its own
budget (`CONNECTION_READ_TIMEOUT`) is spent from the moment it was accepted, the flow's deadline
stays the outer bound, and the pending list is capped (`MAX_PENDING_CONNECTIONS`) so a flood of
connections cannot grow the server's share of the process's file descriptors without bound — the
oldest pending connection makes way for the newest when the cap is reached. One pass takes on at
most `MAX_ACCEPTS_PER_PASS` connections, so a callback queued behind N connections is served
within `ceil((N + 1) / MAX_ACCEPTS_PER_PASS) + 1` poll intervals (`ACCEPT_POLL_INTERVAL`, 25 ms).
The
incremental parse keeps the `MAX_REQUEST_BYTES` cap, the `Content-Length` overflow guard, and the
malformed-request refusals.

Verification: `make check` green, including the new
`music_kit_auth::tests::authorize_serves_the_callback_after_repeated_stalled_connections`, which
holds ten stalled connections open and then sends the browser's callback. Before the fix it failed
with "timed out waiting for the Apple Music sign-in callback after 600ms" — the callback was never
served — and after the fix the callback is served in milliseconds. The new bounds are pinned too:
`a_stalled_connection_is_dropped_when_its_budget_is_spent` (the stalled connection is closed when
its own budget is spent, not when the flow gives up),
`the_pending_list_is_capped_and_the_oldest_connection_makes_way` (the pending list never grows
past `MAX_PENDING_CONNECTIONS`, and the oldest connection makes way for the newest) and
`one_pass_takes_on_a_bounded_number_of_connections` (one pass takes on at most
`MAX_ACCEPTS_PER_PASS` connections). Each of the three fails when the behavior it names is
removed from the server.

**Validation gap:** unclear-invariant — the existing test covered only a single stalled connection, which the sequential server already handled, so the guarantee the code was meant to keep (the callback is served however many connections are held open) had to be reconstructed before the repeated case could be pinned (closest fit).

### The preview-URL resolver guard is bypassed when an HTTP(S) proxy is configured (found by security 2026-10-09, fixed 2026-10-09)

Symptom: the 2026-10-09 fix added `PublicAddressResolver` (`src/audio.rs`) so a
preview URL whose hostname resolves to an internal address is refused. That
guard only runs when `ureq` resolves the target locally. `http::config_with_timeout`
builds on `ureq::Agent::config_builder()`, which starts from `Config::default()`
and therefore takes `proxy: Proxy::try_from_env()` — `ALL_PROXY`, `HTTPS_PROXY`,
`HTTP_PROXY`, or their lowercase spellings. For an HTTP/HTTPS CONNECT proxy
(not SOCKS) `ureq` does not resolve the target: `run.rs` passes
`agent.resolver.empty()` for the target and `connect.rs` sends the target
hostname to the proxy. So with any of those variables set and the preview host
not matching `NO_PROXY`, `PublicAddressResolver` is never asked about the
preview host, and a hostile or compromised Apple Music reply whose
`attributes.previews[].url` names an internal hostname (`metadata.google.internal`,
`ip6-localhost`, or an attacker-controlled name resolving internally) is fetched
by the proxy instead of being refused. The result is blind SSRF (GET only, no
redirects) against hosts the proxy can reach.

Source: the `attributes.previews[].url` string in the Apple Music REST response
(`src/apple_music/rest.rs`, `Resource::preview_url`).

Path: `Resource::preview_url` -> `Song.preview_url` -> `ui/mod.rs`
`TrackSelected` -> `AppleMusicService::play_track` -> `ensure_preview_url_is_valid`
-> `preview_url_problem` (host-text only, so a non-literal name passes) ->
`audio.play` -> worker `download_and_decode` -> `preview_agent()` ->
`agent.get(url).call()` (`src/audio.rs`). `config_with_timeout` (`src/http.rs`)
-> `ureq::Agent::config_builder()` -> `ureq` `Config::default()` sets
`proxy: Proxy::try_from_env()`.

How to reproduce:
- Code path: `src/http.rs` (`config_with_timeout`) -> `ureq::Agent::config_builder()`
  -> `ureq` `Config::default()` (`proxy: Proxy::try_from_env()`). With a proxy
  configured, `ureq` `run.rs` takes the `agent.resolver.empty()` branch for the
  target, and `unversioned/transport/connect.rs` resolves only the proxy before
  sending the target hostname to it.
- Manual: run with `HTTPS_PROXY=http://127.0.0.1:<port>` and a proxy listening
  there; have the REST stub (or a MITM) return
  `attributes.previews[0].url = "https://internal.example/preview.m4a"`. The
  proxy receives `CONNECT internal.example:443`, and `PublicAddressResolver` is
  never consulted. Without the proxy variable the same URL is refused with
  `HostNotFound` (the existing
  `audio::tests::the_preview_agent_refuses_a_hostname_that_resolves_to_loopback`
  covers the direct path).

Suspected cause: the guard is attached to `ureq`'s resolver, which only sees the
target on a direct connection (or a SOCKS proxy, where `resolve_target()` is
true). An HTTP/HTTPS proxy resolves the target itself, so a resolver-level guard
cannot hold under it.

Fix: the production preview agent is built on a configuration with `proxy(None)`
(`preview_agent_with_timeout`, `src/audio.rs`), so `ureq` always resolves the
target locally and `PublicAddressResolver` always sees it; an environment proxy
no longer routes the preview fetch. `src/http.rs` grew
`config_builder_with_timeout`, so the guarded agent shares the REST agent's
global timeout, zero-redirect, and status-as-error choices while overriding the
proxy. The REST agent keeps `Proxy::try_from_env()`, so library browsing still
honors a proxy; only preview playback goes direct. This closes the guard bypass
completely rather than pre-resolving the target locally, which would leave the
proxy free to resolve it differently (DNS rebinding).

Verification: `make check` green, including the new
`audio::tests::the_preview_agent_ignores_a_configured_proxy`, which spawns a
child test process with `HTTPS_PROXY`/`ALL_PROXY` set; the child asserts the
REST agent's config sees the proxy while the preview agent's config has none.
The pre-fix code fails that child assertion (`preview_agent` inherited the
environment proxy).

**Validation gap:** no-fake — the suite had no proxy-configured environment, so confirming the bypass needed a child-process harness (a shim) that sets `HTTPS_PROXY`/`ALL_PROXY` and reads the built agent's config.

### The preview-URL guard never resolves the host, so a hostname that resolves to an internal address bypasses the SSRF check (found by security 2026-10-09, fixed 2026-10-09)

Symptom: `preview_url_problem` (`src/apple_music.rs`) rejected internal
*address literals* and the legacy numeric IPv4 spellings, but it only inspected
the URL's host text — it never resolved the name, so any non-literal hostname
passed. A hostile or compromised Apple Music API reply whose
`attributes.previews[].url` named a hostname that resolves to an internal
address (a cloud metadata name such as `metadata.google.internal`, an
`/etc/hosts` alias such as `ip6-localhost` -> `::1`, or an attacker-controlled
name whose DNS answers `127.0.0.1`) was accepted and fetched by the audio
worker.

Source: the `attributes.previews[].url` string in the Apple Music REST response
(`src/apple_music/rest.rs`, `Resource::preview_url`).

Path: `Resource::preview_url` -> `Song.preview_url` -> `ui/mod.rs`
`TrackSelected` -> `AppleMusicService::play_track` -> `ensure_preview_url_is_valid`
-> `preview_url_problem` (one-time host-text check) -> `audio.play` -> worker
`download_and_decode` -> `agent.get(url).call()` (`src/audio.rs`), which
resolved the name and connected.

Fix: the production preview agent now resolves through `PublicAddressResolver`
(`src/audio.rs`), a `ureq::unversioned::resolver::Resolver` that wraps
`DefaultResolver` and returns `ureq::Error::HostNotFound` when any address the
name resolves to is internal, using `ip_is_internal` shared out of
`apple_music`. The connector therefore connects only to an address the guard
approved, so on the direct path (no proxy configured, the app's default) both
the static internally-resolving name and the rebinding variant are refused
before a connection. The REST agent and the test agents
keep the default resolver, so their deliberate loopback fetches are unaffected.
`src/http.rs` grew `config_with_timeout` so the guarded agent shares the same
transport config as `agent_with_timeout`.

Verification: `make check` green, including the new
`audio::tests::the_preview_agent_refuses_a_hostname_that_resolves_to_loopback`,
which asserts the guarded agent reports `HostNotFound` for
`http://localhost:<port>/preview.m4a` while the unguarded agent reaches the
connect stage — so the refusal comes from the resolver, not from `localhost`
failing to resolve.

### A failed preview start leaves shared state claiming playback (found by improve 2026-10-09, fixed 2026-10-09)

Symptom: `AppleMusicService::play_track` commits the shared-state transition
(`current_track` and `is_playing = true`) before it hands the preview URL to
the injected `audio::AudioOutput`. When the backend rejects the request the
error propagates, but `AppState::is_playing` stays `true`, so the Now Playing
bar and the Play/Pause button report playback that never started. In
production `RodioOutput::play` only sends a worker command, so the window is
narrow, but the seam is public and a real backend can report a rejection.

How to reproduce: play a track with a preview URL through a failing
`AudioOutput` (`test_support::FailingAudio`); the returned error is "the audio
device is gone" while `AppState::is_playing` is still `true`.

Fix: on an audio failure, re-lock the shared state and clear `is_playing`
(leaving `current_track` set, as `stop` does) before returning the error. The
rollback re-checks the play's `is_current` guard under the lock, so a play a
newer one has superseded leaves the newer play's flag alone. `play_track`'s
`is_current` parameter is now `Fn` rather than `FnOnce` so it can be consulted
at both points.

### A browse reply is stored after a newer request was issued, so a navigation shows the previous level's rows (found by robustness 2026-10-08, fixed 2026-10-08)

Symptom: `loading::fetch_into` checks `RequestGeneration::is_current()` inside
the `Task::perform` completion closure, which iced runs on its executor thread,
and then returns the `*Loaded`/`*LoadFailed` message. The message carries no
generation, and `update` runs on the UI thread. If the user presses an artist
row (or the Retry button) after that check but before `update` processes the
reply, `update` first runs the navigation arm — `albums.clear()` then
`albums.begin_fetch()`, which bumps the generation and shows "Loading…" — and
then runs the stale `Message::AlbumsLoaded`, whose arm calls
`player.albums.store(albums)` with the *previous* artist's albums. The panel
shows the wrong artist's albums (and a press on one fetches that album's
songs, not the selected artist's) until the newer fetch lands or its 30s
timeout expires. The same window affects `ArtistsLoaded`/`SongsLoaded` and the
three `*LoadFailed` arms, which store a stale error over a newer in-flight
fetch.

How to reproduce:
- Code path: the completion closure's `generation.is_current()` check is in
  `fetch_into` (`src/ui/loading.rs:247`); the `loaded`/`failed` closures build
  the messages there. The six variants (`src/ui/mod.rs:103-112`) carry no
  generation, and their `update` arms (`src/ui/mod.rs:538-560`) call
  `store`/`fail` unconditionally. The navigation and Retry arms call `clear()`
  then `begin_fetch()` (`src/ui/mod.rs:470`, `:496`, `:526`), bumping the
  counter after a reply may already have passed its `is_current` check.
- Deterministic test: drive a `fetch_into` task to completion to obtain its
  `ArtistsLoaded` message; then call `update` with `Message::LoadArtists`
  (which clears the list and issues a newer generation); then call `update`
  with the saved message. The stale message repopulates the list and clears
  `loading`; the fixed code must reject it.

Suspected cause: the generation guard is evaluated at reply time on the
executor thread, not at store time on the UI thread, and the reply does not
carry its generation, so the check cannot be re-evaluated once the message is
dispatched.

**Fix:** `fetch_into` now wraps each `*Loaded`/`*LoadFailed` reply in a new
`Message::BrowseReply { issued, reply }` that carries the producing
`RequestGeneration`'s `issued` value, and `BrowseList::is_current(issued)`
re-checks that stamp against the target list on the UI thread. The new
`BrowseReply` arm in `update` dispatches the inner message only when the stamp
is still current and otherwise returns `Task::none()`, so a reply that a
navigation or Retry superseded after the executor-thread check is dropped
instead of overwriting the newer list (or its error). Wrapping the existing
replies avoided changing the six `*Loaded`/`*LoadFailed` variants and their
construction sites.

**Validation gap:** unclear-invariant — the existing stale-reply tests exercised only the executor-thread check, so the store-time invariant had to be reconstructed before the UI-thread window could be pinned (closest fit).

### The boot-time artists fetch races the async sign-in, so a signed-in user sees the sample library (found by robustness 2026-10-08, fixed 2026-10-08)

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

**Fix:** `AppleMusicService` now holds a `tokio::sync::Notify` (`session_ready`)
that `authenticate_with` fires after it stores a session, and exposes
`wait_for_session`, which resolves once a session is stored (immediately when
one is already there). `boot` adds `Task::perform(service.wait_for_session(),
|()| Message::LoadArtists)`, so the artists fetch runs again when the background
sign-in completes and replaces the sample list with the signed-in library. The
signal only re-issues the fetch; it does not yet make the sample fallback and
the signed-in library visually distinguishable.

**Validation gap:** real-run-needed — the user-visible symptom (a signed-in
user still seeing the sample artists) has no offline reproduction, so
confirming it required running the app with a valid developer token and
completing the browser sign-in; the regression tests stand in a session through
the injectable sign-in seam instead.

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

**Validation gap:** none — at fix time the reflow was only observable by running
the GUI and diffing screenshots; the shared `src/ui/layout_capture.rs` harness now
lays the root `view` out in both playback states and
`transport_row_does_not_reflow_when_the_play_label_changes` asserts the Stop and Repeat labels
keep their absolute x.

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
