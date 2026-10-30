# Plans

## Planned

_None yet._

### Name the restored last-played track at launch, like Winamp's remembered title (planned 2026-10-29 by the plan role)

**Goal.** The finished restore plan ("Restore the last-played track at launch (stopped)",
PLANS-DONE.md) persists the current track id and restores it stopped, but the launch starts with
an empty `known_tracks` index, so the relaunched player degrades to the raw id: the Now Playing
bar renders the id string itself (`views/tests/now_playing.rs::
now_playing_label_falls_back_to_id_when_track_not_in_tracks`), the desktop window title falls
back to plain `nostalgia` (`WinampPlayer::window_title`), and the File info gate refuses to open
— until the user replays or re-browses the song. Resolve the saved id once at launch and record
its known-track entry, and again once a sign-in completes (a saved Apple Music library id can
only be looked up with a session); nothing plays, loads, or changes transport state. In Winamp's
terms the restarted playlist already knows the title text of its remembered track; this makes
Nostalgia's bar name that track instead of its opaque id.

**Approach.** A read-only service lookup answers what a song id names (offline sample first,
REST by-id once signed in), a boot message runs it, and a reply handler records the resolved
track into `known_tracks` exactly as `handlers::playback::track_selected` does — one mapping,
one insertion site, and a guard so a late answer can never overwrite a track the user picked
meanwhile.

- `src/apple_music/rest.rs`: add `pub fn get_library_song(&self, session:
  &MusicKitSession, id: &str) -> Result<Option<Song>, AppleMusicError>`: one GET at
  `{API_BASE}/me/library/songs/<id>` (encode the id through
  `paging::encode_path_segment`, the same helper the by-id playlist and album calls use),
  fetched through `RestLibrary::fetch` with the single value mapped by the same
  `resource.into_song("")` call `get_all_songs` maps library songs with (a library song carries
  its album name in attributes, not an album id, so `album_id` stays empty, exactly as the whole
  collection answers); one song in `data` returns `Some`, an empty `data` returns `Ok(None)`, a
  transport or parse failure (a wrong id's 404 among them) surfaces as the `Err` the caller
  folds to `None`.
- `src/apple_music/rest/tests/library.rs`, in the existing `StubTransport::returning` /
  `library_over(&stub)` style: `get_library_song_maps_the_single_resource` (the URL is pinned
  to `…/v1/me/library/songs/<id>` and the mapped song equals `rest_song()`), and
  `get_library_song_reports_none_for_an_empty_collection` (`{"data":[]}` → `Ok(None)`);
  extend the shared fixture body only if the single-resource wrapper differs — elsewhere the
  library collection responses are the same envelope.
- `src/apple_music/browse.rs`: add `pub async fn resolve_song(&self, id: &str) -> Option<Song>`.
  Structure: a blank or control-carrying `id` (checked with the stem of
  `validate::ensure_id_is_valid`, catching its rejection as `None`) resolves to `None`; then the
  sample library answers first — a scan of `sample::sample_all_songs()` for `s.id == id`
  (`"song-N"` sample ids and Apple Music ids never collide, so one rule covers both regimes and
  an offline launch resolves immediately, with no session and no network); then, when
  `self.session()` is `Some`, `off_thread(move || rest.get_library_song(&session, &id))` awaited
  and folded with `.ok().flatten()` (a failed or not-found request resolves to `None` — a name
  the launch cannot fetch just keeps today's id fallback; resolution never panics, never
  returns a song for another id, and never blocks iced's executor, mirroring `browse`'s
  off-thread rule); with no session and no sample match it returns `None`. Document the
  sample-first ordering and the swallow-the-error rule in the doc comment: this is a read-only
  courtesy lookup, not a browse error surface.
- `src/apple_music/tests/browse_rest.rs`: `resolve_song_uses_the_rest_library_when_signed_in`
  (a non-sample id through `signed_in_service(&stub)`, REST song in → `Some(rest_song())`, URL
  pinned), `resolve_song_falls_back_to_the_sample_library_without_a_session` (id `song-2` → the
  sample song, the stub transport records **zero** calls),
  `resolve_song_answers_a_sample_id_before_the_rest_library_when_signed_in` (id `song-1` with a
  session stored → the sample song, the stub records zero calls),
  `resolve_song_returns_none_when_the_request_fails` (`StubTransport::failing` → `None`, not an
  error), and `resolve_song_rejects_a_blank_or_control_id_as_none` (blank and an id with a
  terminal control character → `None`, no calls).
- `src/ui/message.rs`: add `ResolveCurrentTrack` (no payload; zips a resolution read off the
  current state) and `CurrentTrackResolved { id: String, song: Option<Song> }` (the reply,
  carrying the id it was issued for so the handler can check the state still points at it) —
  `Song` is already imported there for the songs-loaded variants.
- `src/ui/handlers/playback.rs`: two handlers beside `track_selected`.
  - `pub(in crate::ui) fn resolve_current_track(player: &mut WinampPlayer) -> Task<Message>`:
    read `player.state.blocking_lock().current_track.clone()`; `None` → `Task::none()`; an id
    already in `player.known_tracks` → `Task::none()` (this `Some`-then-guard keeps
    `session_ready`'s re-issues free once the track resolved, and after Replay it keeps the
    `track_selected` entry with its playlist number); otherwise clone the service (a clone
    shares the session handle, like the fetch handlers) and return `Task::perform(
    service.resolve_song(&id), move |song| Message::CurrentTrackResolved { id, song })`.
  - `pub(in crate::ui) fn current_track_resolved(player: &mut WinampPlayer, id: String, song:
    Option<Song>)`: `song` is `None`, or the id does not still equal
    `player.state.blocking_lock().current_track` (re-check under the lock, then drop it —
    a user who picked another song in the seconds before the answer must not be answered
    retroactively and a resolution can never overwrite that new selection's
    `track_selected` entry), → return without change. Otherwise record the entry into
    `player.known_tracks` with the **same field mapping** `track_selected`'s
    `known_tracks.insert` holds today — `number` empty, since the restored track's row position
    in whatever past list is not persisted — and reset `player.marquee_offset = 0` and
    `player.title_drag = None`, the same fresh-line reset `track_selected` applies: the bar moves
    from a short id line to a long title line at the next message, and
    `WinampPlayer::needs_marquee` re-evaluates it on that rebuild. Never touch the transport,
    the elapsed readout, `current_track`, or the play path: the restored track stays stopped.
    If the sibling plan **"Show a selected song row's info in the File info panel, like
    Winamp's playlist editor"** has already landed, its `views::KnownTrack::from_song`
    constructor is the mapping — build the entry with
    `views::KnownTrack::from_song(&song, String::new())` instead of repeating the literal, and
    that constructor is then this site's only mapping (after both land, the mapping must exist
    exactly once, in `KnownTrack`); if it has not landed yet, write the literal exactly as
    `track_selected` holds it today and note the constructor pending — the two entries only
    share the constructor definition, never a change of shape, so either order merges cleanly.
- `src/ui/mod.rs`: in `boot`'s batch add `Task::done(Message::ResolveCurrentTrack)` next to
  `Task::done(Message::LoadArtists)`, and in the `update` dispatch route both messages to the
  two handlers above. In `handlers::browse::session_ready`, append
  `handlers::playback::resolve_current_track(player)` to the returned `Task::batch` (its guard
  makes every later re-issue free; a saved Apple Music id stays unresolved exactly until the
  session's library answers it).
- Tests (`src/ui/tests/readouts_tests.rs`, beside the `known_tracks_record_the_played_songs_*`
  tests, in their style, driving the player built the recording helpers build — the offline
  service resolves through the sample library, so no stub is needed):
  - `current_track_resolved_records_the_restored_track`: state current track `song-1`, handler
    with the sample song → the entry's title, artist, album, year, genre, formatted length, and
    empty `number` match the sample song, `is_playing` and `paused` stay false, and the state's
    `current_track` is unchanged.
  - `current_track_resolved_skips_a_stale_or_failed_resolution`: resolution `None` → no entry;
    a resolution for an id that is not the current track → no entry and the current track's own
    recorded entry (inserted through `track_selected` first) keeps its number prefix.
  - `resolve_current_track_skips_the_work_an_already_known_track_needs`: a player whose current
    id is already in `known_tracks` returns no scheduled work (any of the no-work assertions
    the buffer/scroll scheduling tests use serve the style), and one with no current track
    likewise schedules nothing.
- Docs:
  - `docs/usage.md`, settings paragraph: after “The last-played track is restored at the next
    launch stopped: the Now Playing bar shows it”, say the bar names it by title and artist
    once the launch can find the song — immediately from the built-in sample library when the
    saved id is a sample-library id, and after signing in from your Apple Music library
    otherwise — while the saved id (as today) shows until then; nothing plays.
  - `docs/invariants.md` `## Settings persistence`: in the last-played-track sentences, after
    “the boot `apply_to` restores the track stopped…”, add: the restored id is resolved into a
    `known_tracks` entry at launch (`Message::ResolveCurrentTrack` →
    `handlers::playback::resolve_current_track` and `current_track_resolved`;
    `AppleMusicService::resolve_song` answers a sample-library id immediately and an Apple
    Music id once a session is stored, through `rest::RestLibrary::get_library_song`; a failed
    fetch, an unknown id, or a track resolved another way leaves the state untouched and keeps
    the id-fallback label), and the entry records no playlist `number` and resets the marquee
    like `track_selected`'s does. The “stale id no longer in the library” sentence stays
    accurate and unamended.

No new visible layout exists: the bar, marquee, and window title render the restored track
exactly through the playing-track paths they already use (see `docs/screenshots/player.png` and
`winamp-screenshots/winamp-2.05-main.jpg` for that shape), so nothing recolors and no chrome or
screenshot comparison is needed — the change only swaps which string the existing renderer
resolves.

**Files touched.** `src/apple_music/rest.rs`, `src/apple_music/rest/tests/library.rs`,
`src/apple_music/browse.rs`, `src/apple_music/tests/browse_rest.rs`, `src/ui/message.rs`,
`src/ui/handlers/playback.rs`, `src/ui/handlers/browse.rs`, `src/ui/mod.rs`,
`src/ui/tests/readouts_tests.rs`, `docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**
- With a last-played track saved (playing, paused, or stopped at quit) whose id is a
  sample-library id, a relaunch names that track — title in the bar (scrolling when long),
  artist line present, format readout per the session rules — in the Now Playing bar and in the
  desktop window title, stopped, with a blank elapsed readout, without signing in and without
  any network call.
- With a saved Apple Music id, the relaunched bar falls back to the id until the user signs in,
  and once the session lands the bar and window title switch to the song's title and artist
  with no user action and no playback; with the sign-out path (no session this run) the id
  fallback persists with no error shown.
- A track the user plays while a resolution is in flight keeps its own `track_selected` entry
  (number prefix included) — a late resolution is dropped when the current track moved, and one
  for an already-known id never reschedules.
- An id that is neither in the sample library nor the signed-in library (or whose fetch fails,
  including offline with a session) resolves to nothing: no entry, no visible error, and the
  existing id-fallback bar text stands.
- `make check` passes.

## Done
