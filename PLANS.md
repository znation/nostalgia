# Plans

## Planned

_None yet._

### Restore the last-played track at launch (stopped), persisted in the settings file (planned 2026-10-29 by the plan role)

**Goal.** Restarting Nostalgia today opens an empty Now Playing bar: the current track is
deliberately not persisted (`docs/invariants.md` `## Settings persistence` calls this out),
so a user who quits mid-album loses not only the position but the track itself, and Play
does nothing useful until they browse back. Winamp remembers its track across a restart;
Nostalgia can match the useful part cheaply, because the whole settings seam already
exists: `persist_settings_if_changed` (`src/ui/player.rs`) snapshots `AppState` through
`Settings::from_state` after every message, and `main` (`src/main.rs`) applies the loaded
snapshot to the initial state before the first view. Persist the current track id and
restore it stopped — not playing, not paused, elapsed blank — exactly the state Stop
already produces (`AppState::stop`), so no view changes at all and a request that touches
nothing visible needs no screenshot comparison.

**Approach.** The track id is already an `AppState` field (`AppState::current_track`, a
`String` id with `None` meaning no track), so it rides the existing snapshot and restore
paths without any new store or UI-owned accessors — it is the first persisted value that
comes straight from the snapshot rather than a UI-only setter.
- `src/settings.rs`: add `current_track: Option<String>` to `Settings` (default `None`),
  capture it in `from_state` (`state.current_track.clone()`), and restore it in `apply_to`
  (`state.current_track = self.current_track`) — the latter touches only `main`'s boot
  call today, so no mid-session snapshot can blank a track. Extend the `Settings` doc
  comment and the `Default` doc: the restored track sits stopped with a blank elapsed
  readout, and playback position stays out of the file.
- `src/settings/text.rs`: write a `current_track=<id>` line after `window_position`, with
  an empty value when `None` (`current_track=`); parse it by matching the shape of the
  other string-valued keys — empty or missing leaves `None`, a non-empty value is taken
  verbatim (unknown or stale ids are already handled downstream: `now_playing_label`
  falls back to showing the id — see `views/tests/now_playing.rs::
  now_playing_label_falls_back_to_id_when_track_not_in_tracks`).
- `src/ui/mod.rs`: no change — `persist_settings_if_changed` already snapshots from state
  and the comparison only saves changed snapshots. Verify this in review, not with code.
- Tests:
  - `src/settings.rs` inline tests: extend `state_with_every_field_set` (set
    `current_track`) and `to_text_round_trips_every_field` so the line round-trips; add
    `current_track_round_trips_and_defaults_to_none` (`to_text` on a saved track, a
    `from_text` with the header line present, without it, and with an empty value →
    `None`, and a value containing `=` survives `split_once`), and
    `apply_to_restores_the_saved_current_track_stopped` (`Some("song-1")` →
    `state.current_track` set with `is_playing == false && paused == false`, and a
    missing-file boot — `Settings::default().apply_to` — leaves `None`).
  - `src/ui/tests/settings_tests.rs`: `saved_current_track_survives_the_persist_round_trip`
    — build a state with a track loaded and stopped, run the boot path end to end
    (`from_state` → `to_text` → `from_text` → `apply_to` on `AppState::default`) and
    assert the track id came back with the transport stopped; alongside the existing
    `player.adopt_saved_settings` round-trip tests, so the UI-owned fields' coverage is
    not disturbed.
- Docs:
  - `docs/usage.md`: in the settings paragraph (the list that begins "The volume, balance,
    Repeat and Shuffle flags…"), add the last-played track to the saved list and one
    sentence: it is restored at the next launch stopped — the Now Playing bar shows it,
    Play starts it again from the beginning, and the position within it is not
    remembered.
  - `docs/invariants.md` `## Settings persistence`: add the last-played track to the
    guarantee's value list, note the restore is a plain `apply_to` field (the id is held
    in `AppState`, unlike the UI-owned values), keep the missing/empty-line → none rule in
    the fallback sentences, and drop `the current track` from the closing "deliberately
    not persisted" list (which keeps playback position, browse levels below the top list,
    and the search box).

**Files touched.** `src/settings.rs`, `src/settings/text.rs`,
`src/ui/tests/settings_tests.rs`, `docs/usage.md`, `docs/invariants.md`.

**Acceptance criteria.**
- With a track loaded (playing, paused, or stopped) and the player quit, a relaunch shows
  that track's title and artist in the Now Playing bar and window title, with the transport
  stopped and a blank elapsed readout; Play loads and plays it as Stop-then-Play does
  today.
- With no track loaded, with no settings file, with a missing or empty `current_track`
  line, or with a stale id no longer in the library, launch behaves exactly as today:
  empty Now Playing bar, and a stale-but-present id degrades only through the existing
  id-fallback label text.
- The flag never affects playback state mid-run: only the boot `apply_to` restores it, and
  every interaction follows the current rules (a track change or stop rewrites the line on
  the next message's save; the snapshot comparison keeps unchanged states from
  writing).
- `make check` passes.

## Done
