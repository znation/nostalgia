# Plans

## Planned

_None yet._

### Clamp the restored window position to the monitor the window opens on, so a saved off-screen point cannot hide the player (planned 2026-10-30 by the plan role)

**Goal.** `docs/usage.md`'s settings paragraph tells a user whose window opened off-screen
that they must "delete the `window_position` line to reset an off-screen window": the saved
screen position is restored verbatim with no bound (`launch_position` maps the saved point to
`iced::window::Position::Specific`), and `docs/invariants.md`'s Settings persistence section
pins that as "a set value is passed to `.position(...)` through `launch_position` without
clamping to monitors (iced cannot query them before the window opens)". iced 0.14 can clamp
at open time after all: `iced::window::Position::SpecificWith(fn(Size, Size) -> Point)` hands
the opening window its own logical size and its monitor's logical resolution, and the window
system open path applies the returned point relative to that monitor's origin — so an
unbounded saved point can be brought back on screen before the window exists. This entry
clamps the restored point by that rule, with the clamp's exact arithmetic, its fallbacks,
and the doc anchors pinned here so nothing is left open.

**Ground truth verified this run (2026-10-30).** `src/ui/handlers/window.rs::launch_position`
maps `Some((x, y))` to `iced::window::Position::Specific(iced::Point::new(x, y))` and `None`
to `Position::Default`; its doc comment still claims iced cannot query monitors before open,
which is stale ground truth now — but the API is a `fn` pointer, so the saved coordinates
cannot ride in a closure and must reach the function another way (a one-write-once
`std::sync::OnceLock` set in `launch_position` before the pointer is handed over). The window
system applies `settings.position` in `iced_winit-0.14.1/src/conversion.rs::position` against
the primary monitor: `Position::SpecificWith(to_position)` calls `to_position(window_size,
monitor_logical_resolution)` and treats the returned `Point` as an offset from
`monitor.position()` (falling back to the window system's own default when no monitor is
reported — the same fallback `Position::Default` gets). The saved position itself is logical
(`window_moved` stores the `iced::Point` the `Moved` event delivers, written by the
persistence machinery as `window_position=<x>,<y>` in `src/settings.rs`), so all three sizes
the clamp sees are logical and share one coordinate space. The restore path's own tests sit
in `src/ui/tests/settings_tests.rs` (`launch_position_opens_at_the_saved_point_or_the_default`),
and the persistence guarantee's doc home is the `## Settings persistence` section of
`docs/invariants.md` (the sentence naming `launch_position`). A grep of `PLANS-DONE.md` and
`BUGS-FIXED.md` for `clamp`, `monitor`, and `off-screen` shows the landed window-position
feature ("Remember the window's screen position across launches") has no clamp coverage —
its entry and the invariant both record the unclamped behavior as a limitation. Wayland is
unaffected by design there: it reports no window moves, writes no `window_position` line,
and keeps opening at the window system's default.

**Approach.**

1. `src/ui/handlers/window.rs`: add a module-private `static SAVED_LAUNCH_POSITION:
   std::sync::OnceLock<(f32, f32)>` and a pure helper `fn clamp_into_monitor(point:
   iced::Point, window: iced::Size, monitor: iced::Size) -> iced::Point`, with the rule
   pinned exactly: clamp each axis independently — `point.x.clamp(0.0, (monitor.width -
   window.width).max(0.0))`, the same for `y` — so the window opens fully inside the monitor
   it opens on, and a monitor smaller than the window leaves the point at `0` on that axis.
   Rewrite `launch_position`'s saved arm to store the point in the `OnceLock` (`set`; the
   first write wins, and `init_ui` calls this once at boot) and return
   `iced::window::Position::SpecificWith(clamped_saved_position)`, where
   `clamped_saved_position(window, monitor)` reads the `OnceLock` through
   `clamp_into_monitor` (an unset lock — only a test's default path — clamps
   `Point::ORIGIN`). Keep the `None` arm as `Position::Default`.
2. `src/ui/handlers/window.rs` doc comment: replace the "the position is not clamped to the
   monitors, since iced cannot query them before the window opens" sentence with the new
   truth — the clamp happens inside the window system's open path (`Position::SpecificWith`
   consults the monitor at creation), anchored to the monitor the window opens on, and the
   launch monitor's origin is not queried, which is the pinned trade-off below.
3. `docs/usage.md` settings paragraph: replace "The window reopens at its saved position
   without checking that a monitor is still there; delete the `window_position` line to
   reset an off-screen window." with the clamped behavior: a saved position now opens
   clamped inside the monitor the window opens on, so a point saved for a monitor that is
   no longer there (or for a monitor smaller than the player) reopens on screen at the
   nearest edge instead of hiding the window; a point carried over from another monitor is
   re-anchored to the monitor the window opens on. Keep the Wayland sentence unchanged.
4. `docs/invariants.md` `## Settings persistence`: replace the "a set value is passed to
   `.position(...)` through `launch_position` without clamping to monitors" sentence with
   the new guarantee (clamped at open time by `clamp_into_monitor`, per the rule in
   approach item 1), enforced by `launch_position` and pinned by the tests below.

Nothing in the window's chrome, layout, size, or rendering changes — this only bounds the
point handed to the window system at open, so no `winamp-screenshots/` comparison applies.

**Acceptance criteria.**

- `src/ui/tests/settings_tests.rs`: `launch_position(Some((10.0, 20.0)))` now matches
  `iced::window::Position::SpecificWith(_)` and `launch_position(None)` still matches
  `iced::window::Position::Default` (the renamed twin of
  `launch_position_opens_at_the_saved_point_or_the_default`).
- New tests beside it pin `clamp_into_monitor`'s rule: an in-bounds point returns
  unchanged; a point beyond the monitor's right or bottom edge is clamped to `monitor
  extent - window extent` while its other axis passes through independently; a negative
  coordinate clamps to `0`; a monitor smaller than the window clamps both axes to `0`.
- `make check` passes.

**Size.** One `OnceLock`, one pure clamp helper (its rule is four `clamp` expressions),
one enum-arm swap and one doc comment in `src/ui/handlers/window.rs`, two doc anchors —
roughly sixty lines including the five tests. Out of scope: any monitoring of unplugged
monitors after the launch, any per-monitor origin correction (iced 0.14 has no monitor
list with origins — `iced_runtime-0.14.0/src/window.rs` exposes only `monitor_size`),
any change to what `window_moved` stores or what Wayland does, and anything in the UI
chrome.

## Done
