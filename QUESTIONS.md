# Questions

Open questions loops have posted for a human decision — each with context, the options, and the
loop's recommendation. Answer by moving an entry to ## Answered with your decision (or tell the
director). Loops never block on their own questions; they check here at the start of each tick.

## Open

### What should the transport row's Eject button do when it is added? (posted by bugfix 2026-10-30)

**Context.** The base skin's transport row carries an Eject button between Next and the
Shuffle/Repeat buttons (`winamp-screenshots/`'s 2.95 capture and the repo's own mockup,
`docs/screenshots/player.png`, both show it), and the open BUGS.md entry "The transport row
omits the base skin's Eject button" tracks the missing control. The sibling half of that entry
(the LED-marked Repeat/Shuffle toggles) was fixed 2026-10-30, but the Eject button itself needs
a behavior before a control can ship: in Winamp 2.x, Eject opens an Add File(s) dialog, and this
app plays the Apple Music library and the built-in sample library only — nothing on the machine
opens behind it today.

**Options.**

1. Add the button only when a behavior exists (for instance an Add-files/import surface) —
   until then the bug entry stays open and the row keeps its current order. Recommended: the
   one that never ships a control that lies.
2. Wire Eject to `Message::ToggleLibraryPanel`, the message the `PL` button publishes: the
   button sits one place closer to Winamp's row order, at the cost of a second control for the
   same thing.
3. Add it as a visual-only button that publishes nothing, filling the row position until a
   behavior is decided.

**Recommendation.** Option 1. Duplicate (2) and no-op (3) both make a control claim something
it does not do; the reference-matching passes have already shown the row's order can be
completed later without disturbing anything else. Moving this question to `## Answered` with
the choice unblocks the bug entry's fix directly.

### What should the planner plan next — the parked directions or the browse-selection keys? (posted by plan 2026-10-29)

**Context.** The Winamp-faithful surface is delivered and pinned: the OPTIONS menu rows, the
footers (counts and totals per view), the remove/clear row operations, Sort/Reverse/Shuffle,
the row moves (Alt+Up/Down, with the Alt+Shift+Up/Down edge move waiting in PLANS.md), both
jump panels, the File info panel, the scale sliders and readouts, search matching over every
displayed field, the shade strip, the mini transport, and settings persistence — each maps to
an inventoried reference capture and is planned or landed (`PLANS-DONE.md`, `docs/usage.md`).
What is still open is a set of directions earlier planner runs deliberately parked for a user
decision, plus one gap found this run. Every option below is grounded on the current code and
is plannable once chosen; the planner wants one (or an ordering) rather than a guess.

One grounded gap first: the browse list has no keyboard way to move its selection bar. The
classic playlist editor moves the selection with Up/Down; in Nostalgia those keys nudge volume
(`shortcut()` in `src/ui/shortcuts.rs`, the plain-arrow volume arms) and the row move takes
Alt+Up/Down, so the selection itself moves only by mouse press (`BrowseList::select_row`).
Binding new keys takes a taste call, so it is asked here rather than guessed at.

**Options.** Each is plannable in an implementation-sized run, named with its rough size.

1. **Surface REST transport errors, starting with 401-vs-other (small; service seam).** Today
   signed-in queries fall back silently to the sample library (`docs/invariants.md` `## REST`
   transport section), so a stale or revoked session looks like an offline library. Plan:
   classify the transport's errors at the `AppleMusicService` seam and surface a 401 (and
   friends) in the UI the way other failures surface, keeping the failure fallback the same.
   Recommended as the first run's plan: smallest, seam-contained, no new dependency.
2. **Keyboard selection movement for the browse list (small; shortcuts + handlers).** Give the
   moving selection a chord: `Ctrl+Up`/`Ctrl+Down` are free today (only Ctrl+F/Z/B/R/W/J/A/T/V
   and Ctrl+Shift+Delete are claimed; Ctrl+Up/Down bind nothing), while plain Shift+arrows
   already fall through to volume. It moves the selection bar and reveals the moved row exactly
   like the row moves do, with the same no-op rules. Requires no view or chrome change.
3. **Oscilloscope visualizer modes (medium; LCD + player).** The LCD visualizer currently has
   one drawn mode behind ToggleVisualizer; Winamp 2.x cycles spectrum/oscilloscope/off on a
   click. A mode state, a cycling press (the strip's click already publishes the toggle
   message), and tests would extend the landed groundwork.
4. **Album artwork decode.** The Song/album types have no artwork field today; decoding cover
   art into the File info panel (or Now Playing bar) is the single most visible addition from
   Winamp's neighbours, though the Winamp 2.x base skin shows no artwork itself — see the note.
5. **Album ordering (from the earlier parked list).** Decide what ordering the All Albums list
   should use beyond the current library order if anything more than the status quo is wanted.
   Low value until someone names the wanted order, so it is slowest of the UI options.
6. **Position-within-track restore (large; audio seam).** Remember and restore the playing
   position across relaunch; `docs/usage.md` explicitly notes the position is not remembered.
   Largest option, touches the audio seam's state, and its UX value is limited by preview
   lengths (roughly 30s), so it is recommended to leave it parked longest.

Mouse-drag row reorder and Double-size stay rejected and are not offered. If you pick nothing,
the planner will keep its current stance: refuse to guess these surfaces, keep `## Planned`
stocked only with asked-for work, and surface a short nothing-to-do instead.

**Recommendation.** Pick option 1 now (smallest seam-contained improvement, no taste call),
option 2 second (closes the browse-selection keyboard gap with the free Ctrl+Up/Ctrl+Down
chords), and answer options 3–6 only if you want them built: 3 is a bounded visualizer pass,
4 and 6 are the two genuinely large items, and 5 needs the intended order named by you to
become plannable. A reply moving this entry to `## Answered` with your ordering is all the
planner needs; every option then becomes a plan in the normal queue.


## Answered

### How should Nostalgia authenticate to Apple Music? (posted by plan 2026-10-08)

**Decision (2026-10-08): Option 2 — MusicKit authorization flow.** Nostalgia
obtains the MusicKit *user token* through the MusicKit JS authorization flow
instead of asking the user to paste it. The concrete mechanism is recorded in
PLANS.md's "Add the MusicKit loopback authorization module": MusicKit JS is
served from a loopback page opened in the system browser (no webview
dependency), and the *developer token* still comes from
`APPLE_MUSIC_DEVELOPER_TOKEN`, because signing the ES256 developer token is an
owner-side secret. The sibling PLANS.md entry "Wire the MusicKit session into
`AppleMusicService`" stores the result. The **related choice** below (an HTTP
client for the REST calls) is not needed by the auth flow itself and remains as
recommended (`ureq`); the eventual REST-integration plan settles it.

**Context.** `AppleMusicService` in `src/apple_music.rs` answers every browse
query (`get_favorite_artists`, `get_albums_by_artist`, `get_songs_from_album`)
from `sample_library()`, and `play_track` only mutates shared state; its token
field is unset. The project's reason to exist is "using Apple Music as the
music library", so this is the largest remaining gap — but the auth path is a
product/architecture call, and no plan should guess it.

**Options.**

1. **Pasted tokens (recommended).** Read a MusicKit *developer token* (a JWT
   signed with an Apple Music private key) and a *user token* from environment
   variables (e.g. `APPLE_MUSIC_DEVELOPER_TOKEN`, `APPLE_MUSIC_USER_TOKEN`) and
   call the REST API at `api.music.apple.com` directly. No new UI; the user
   obtains both tokens themselves. Keeps the existing narrow seam, is testable
   with a stub, and works on Linux.
2. **MusicKit authorization flow.** Add a sign-in flow (MusicKit JS in a
   webview, or the native MusicKit framework on macOS) to obtain the user
   token. Closer to "just works", but adds a webview/native dependency and a
   per-platform path; much larger.
3. **Keep the sample library.** Defer real integration indefinitely.

**Recommendation.** Option 1: it keeps the `AppleMusicService` seam, needs no
new UI, and unblocks the integration plan; token acquisition can be improved
later behind the same seam.

**Related choice.** Any option needs an HTTP client, and the project's
principles prefer the standard library, which has none. Recommend the smallest
blocking client (`ureq`) behind the service seam; `reqwest`/`tokio` is the
alternative if the async runtime is wanted.

### What audio-output backend should Nostalgia use for real playback? (posted by plan 2026-10-08)

**Decision (2026-10-08): Option 1 — a Rust audio-output crate behind the
service seam.** Nostalgia will play audio through a Rust audio-output crate
(`rodio`) hidden behind the existing `AppleMusicService` seam, which hands it a
playable asset URL. The playable asset is the Apple Music **preview**
(`attributes.previews[0].url`, a short M4A) that the browse response already
carries; full DRM-protected tracks stay out of scope because the REST
developer/user tokens cannot decrypt them, and native `AVPlayer` (Option 2) is
deferred until preview playback has landed. The backend is injected into the
service, so a recording stub stands in under test and a silent stub stands in
when no audio device opens. Routing: PLANS.md "Carry the Apple Music preview
URL on `Song`" (the model/REST half) and "Add the audio-output seam and a
rodio backend" (the playback half), in that order.

**Context.** Browse is real: `AppleMusicService` answers `get_favorite_artists`,
`get_albums_by_artist`, and `get_songs_from_album` from the Apple Music REST
client (`src/apple_music/rest.rs`) when a MusicKit session is stored, and from
`sample_library()` otherwise. Playback is still a stub: `play_track` records
the selected track and sets `is_playing`, `pause` clears the flag, and
`next_track`/`previous_track` only print (`src/apple_music.rs`). The crate has
no audio dependency (`Cargo.toml`), so nothing produces sound today. The
project's reason to exist is Apple Music as the library, which makes audio
output the largest remaining gap — but the backend is a dependency and
platform call, so no plan should guess it.

**Options.**

1. **A Rust audio-output crate behind the service seam (recommended).** Add a
   backend such as `rodio` (or `cpal` plus a decoder) and have the service
   hand it a playable asset URL, keeping `AppleMusicService` the narrow seam a
   test stub can stand behind. Cross-platform, so Linux works; costs one
   dependency, which PRINCIPLES' "prefer the standard library" cannot avoid
   (std has no audio).
2. **Native Apple-platform audio behind the same seam.** Use MusicKit /
   `AVPlayer` on macOS and keep the Linux build on the stub. Closest to
   full-track playback, but platform-specific and much larger.
3. **Defer playback.** Keep the stub and keep building UI/other features.

**Recommendation.** Option 1, scoped to whichever playable asset the API can
actually supply (the implementer must confirm that before wiring it). It lands
behind the existing seam, is testable with a stub, and needs no new UI.

**Related.** This is independent of the planned "Add a Winamp Shuffle toggle
that randomizes Next", which changes only which song Next selects.

### Who runs `tumwater config set check.command "make check"`? (posted by steward 2026-10-25)

**Context.** PLANS.md "Declare the project's deterministic check" needs `tumwater.json` changed,
which no role may edit, so a feature role that claims the entry cannot finish it and it sits at
the front of the queue's tail. The `Makefile` `check` target already exists and CI runs it.

**Options.** (1) You run `tumwater config set check.command "make check"` in `/home/zach/nostalgia`,
then move the entry to Done. (2) Decline: remove the entry and accept the `no-check` doctor alert.

**Recommendation.** Option 1; it is a one-line command and closes the alert.

**Answered 2026-10-25 by the user:**

Done. I get this:
```
nostalgia on  main [⇡] is 📦 v0.1.0 via 🦀 v1.95.0 ❯ tumwater config set check.command "make check"
check.command is already "make check", so tumwater.json was not written
```
