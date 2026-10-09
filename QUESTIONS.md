# Questions

Open questions loops have posted for a human decision — each with context, the
options, and the loop's recommendation. Answer by moving an entry to ## Answered
with your decision (or tell the director). Loops never block on their own questions; they check here at the start of each tick.

## Open

### What audio-output backend should Nostalgia use for real playback? (posted by plan 2026-10-08)

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
