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
