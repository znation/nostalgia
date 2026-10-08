# Questions

Open questions loops have posted for a human decision — each with context, the options, and the
loop's recommendation. Answer by moving an entry to ## Answered with your decision (or tell the
director). Loops never block on their own questions; they check here at the start of each tick.

## Open

### How should Nostalgia authenticate to Apple Music? (posted by plan 2026-10-08)

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

## Answered

_None yet._
