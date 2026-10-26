# Plans

Planned features, written by the plan loop and implemented by the feature loop.
Each plan: goal, approach, files touched, acceptance criteria. Move finished plans to Done.

> **Steward drift note (2026-10-06; condensed 2026-10-23 by the steward role).** This note
> tracked the initial prompt's two commitments — a "very close if not pixel-perfect" classic
> Winamp UI and Apple Music as the library — while the build caught up. Both are now reflected
> in the shipped app and the README's Usage section: the Winamp chrome (base-skin palette,
> bevel layer, custom title bar, title bands, Now Playing LCD, browse selection and row
> styling, compact and mini transport strips) and the Apple Music library (REST browse and
> search behind the `AppleMusicService` seam with the sample library as fallback, and preview
> playback through the audio seam). The per-landing detail this note once carried is retired
> here; `PLANS-DONE.md` holds the individual entries, and no current brief-vs-build drift is
> known. Standing lesson: the chrome arrived piecemeal, so later changes should reuse the
> existing vocabulary — `bevel::lcd_well`, `bevel::raised_panel`, and the shared style
> helpers — rather than inventing new chrome. Record any new drift here rather than appending
> another note.

## Planned

_None yet._

### Numpad 2/4/5/6/7/8/9 drive the transport and volume, like Winamp (planned 2026-10-24 by the plan role)

**Goal.** Classic Winamp binds the numeric keypad (with NumLock on): 4 previous, 5 play, 6 next,
7 rewind 5 s, 9 fast-forward 5 s, 8 volume up, 2 volume down. Nostalgia ignores keypad digits:
`shortcuts::message_for` drops the `location` field of `Event::KeyPressed`, and `shortcut` binds
no digit. Map those seven keypad digits to the messages the existing keys send (Z/X/B and the
arrows), so they follow those messages' rules exactly. Top-row digits (`Location::Standard`)
stay unbound. Keypad 0, 1, 3 (Winamp's open file and jump 10 songs) and the keypad operators
are out of scope and stay unbound. With NumLock off, the keypad sends
`Key::Named(ArrowLeft…)` and still goes through the arrow bindings as it does today. This plan
is independent of the waiting song-list footer plan; the two share only `README.md` and
`docs/invariants.md`, in different paragraphs and sections.

**Approach.**
- `src/ui/shortcuts.rs`: add `fn numpad_shortcut(key: &Key, location: Location, modifiers:
  Modifiers) -> Option<Message>` (import `iced::keyboard::Location`). It returns `None` unless
  `location == Location::Numpad` and none of Control, Alt, or Logo is held (Shift is allowed,
  as for the letters). Otherwise it matches `Key::Character` `"4"` → `PreviousTrack`, `"5"` →
  `Play`, `"6"` → `NextTrack`, `"7"` → `SeekBackward`, `"9"` → `SeekForward`, `"8"` →
  `VolumeUp`, `"2"` → `VolumeDown`, and anything else → `None`. Give it a doc comment listing the
  bindings, the unbound keypad keys, and the NumLock-off note.
- `message_for`: destructure `location` too, and return
  `numpad_shortcut(&key, location, modifiers).or_else(|| shortcut(&key, modifiers))`. Leave
  `shortcut`'s signature alone, because its direct tests call it. Add a sentence on the keypad
  digits to the doc comments of `message_for` and `shortcut`.
- `iced::keyboard::listen` already skips events a focused widget captured, so a digit typed into
  `Search library` does not reach the mapper, the same as letters. Do not change
  `src/ui/timers.rs`.

**Files touched.** `src/ui/shortcuts.rs` (code and its `tests` module), `docs/invariants.md`,
`README.md`.

**Acceptance criteria.**
- In the `shortcuts.rs` `tests` module, add a helper `numpad_pressed(key, modifiers)` shaped like
  `key_pressed`, with `location: Location::Numpad`, and these tests:
  - `message_for_maps_the_numpad_transport_digits`: with `Modifiers::NONE` and with
    `Modifiers::SHIFT`, keypad `"4"`, `"5"`, `"6"`, `"7"`, `"9"`, `"8"`, `"2"` map to
    `PreviousTrack`, `Play`, `NextTrack`, `SeekBackward`, `SeekForward`, `VolumeUp`, and
    `VolumeDown`.
  - `message_for_leaves_other_digits_unbound`: keypad `"0"`, `"1"`, `"3"`, and `"+"` map to
    `None`, and each of `"2"` and `"4"` through `"9"` pressed through `key_pressed` (Standard
    location) maps to `None`.
  - `message_for_ignores_numpad_chords`: keypad `"4"` with `Modifiers::CTRL`, `Modifiers::ALT`,
    and `Modifiers::LOGO` maps to `None`.
  - `message_for_maps_numpad_arrows_like_the_arrow_keys`: `Key::Named(ArrowLeft)` and
    `Key::Named(ArrowUp)` through `numpad_pressed` map to `SeekBackward` and `VolumeUp`.
- Existing shortcut tests pass unchanged.
- `docs/invariants.md`: add a `## Numpad keys` section after `## List edge jumps` (Guarantee /
  Enforced by / Pinned by / History, the same shape as `## Media keys`). It states the seven
  keypad bindings, that they send the same messages as Z/X/B and the arrows, that top-row
  digits, keypad 0/1/3, and Ctrl/Alt/Logo chords stay unbound, and the NumLock-off arrow
  behaviour. Name `numpad_shortcut` and `message_for` under Enforced by and the four new tests
  under Pinned by.
- README Usage: after the sentence ending "seek five seconds back or forward.", add one sentence:
  with NumLock on, keypad 4, 5, and 6 play the previous, current, and next song, 7 and 9 seek
  five seconds back or forward, and 8 and 2 nudge the volume, like Winamp. Do not touch the
  brief or status blocks.
- `make check` passes.

### Declare the project's deterministic check (planned 2026-10-25 by the plan role)

**Declares the project check**

**Goal.** Tumwater's doctor reports the `no-check` alert because the project declares no
deterministic check, so merges run without the safety gate. The project already has one: the
`Makefile`'s `check` target (`fmt lint docs test test-one-guard-test`), which the README's
Development section names as the landing gate and `.github/workflows/ci.yml` runs on every push.
Declare that exact command to tumwater.

**Approach.**
- Command: `make check`, run from the project root (it needs the ALSA headers the README's
  Development section names).
- Route: `check.command`. There is no `package.json` at the project root or above it (it is a
  Cargo crate), so the package.json `test`/`typecheck`/`build` script route does not apply. Set it
  from the team root with `tumwater config set check.command "make check"` (run in
  `/home/zach/nostalgia`; it writes `tumwater.json`, which roles must not edit, so this step is
  the user's or tumwater's own).
- Change no source, `Makefile`, or CI file: the gate already exists.

**Files touched.** `tumwater.json` (via `tumwater config set` only); nothing in the crate.

**Acceptance criteria.**
- `tumwater config get check.command` prints `make check`.
- `tumwater doctor` shows its `check` row as ok, and the `no-check` alert is gone.
- `make check` still exits 0 on main.

## Done
