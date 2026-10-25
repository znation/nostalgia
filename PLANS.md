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

### Snap the balance slider to center when a drag moves into the middle zone (planned 2026-10-24 by the plan role)

**Goal.** Make it easy to set the balance back to exactly center. In the classic player the
balance slider has a center detent. Nostalgia's balance slider moves in 1% steps
(`views::BALANCE_STEP`), so getting back to an exact `0.0` (`Balance: Center`, the green center
tint) takes pixel-precise dragging. Add a detent: a balance-slider change that moves toward
center and lands within `0.1` of it stores exactly `0.0`. A change that moves away from center
is never snapped. Without that rule the slider's own arrow keys could never leave center,
because a 1% step out would snap straight back.

**Approach.**
- `src/state.rs`: next to `clamp_balance`, add
  `pub const BALANCE_CENTER_SNAP: f32 = 0.1;` and
  `pub fn snap_balance_to_center(current: f32, requested: f32) -> f32`. The function clamps
  `requested` with `clamp_balance` (so NaN becomes `0.0`). It returns `0.0` when the clamped
  value's magnitude is below `BALANCE_CENTER_SNAP` **and** below `current.abs()`. Otherwise it
  returns the clamped value. Give it a doc comment that states the inward-only rule and why.
  Leave `AppState::set_balance` unchanged, so settings restore (`adopt_saved_settings`) still
  stores a saved off-center value as-is.
- `src/ui/handlers/mixer.rs` `balance_change`: replace the closure with
  `|state| state.set_balance(crate::state::snap_balance_to_center(state.balance(), balance))`.
  `mutate_balance` already forwards `state.balance()` to `set_output_balance`, so the audio
  backend gets the snapped value. The LCD readout (`mixer_readout_text`) and the groove tint
  (`style::balance_level_color`) also read the stored value, so they show `Balance: Center` and
  the center colour with no further change. Update the doc comment to mention the detent.
- No new message, no view change, and no settings-format change.

**Files touched.** `src/state.rs` (the function, the constant, unit tests), `src/ui/handlers/mixer.rs`,
`src/ui/tests.rs` (one update-level test), `docs/invariants.md`, `README.md` (Usage only).

**Acceptance criteria.**
- Unit tests in `src/state.rs`'s `tests` module, named
  `snap_balance_to_center_snaps_an_inward_move_near_center` and
  `snap_balance_to_center_never_snaps_an_outward_move`:
  - `(0.3, 0.05)` → `0.0`, and `(-0.3, -0.09)` → `0.0`.
  - `(0.3, 0.1)` → `0.1`, since the boundary is not snapped.
  - `(0.0, 0.01)` → `0.01`, and `(0.0, -0.05)` → `-0.05`.
  - `(0.05, 0.06)` → `0.06`.
  - `(0.5, f32::NAN)` → `0.0`, and `(0.5, 2.0)` → `1.0`.
- `src/ui/tests.rs`: add `dragging_balance_into_the_middle_snaps_to_center`. Use `test_player`
  and `seed_current_title`, then send `Message::BalanceChange(-0.4)`, then
  `Message::BalanceChange(-0.05)`. Assert `state.blocking_lock().balance() == 0.0` and
  `root_view_draws(&player, "Balance: Center")`. The existing `balance_change_clamps_value_before_storing`,
  `dragging_balance_shows_the_side`, and the `assert_message_schedules_no_work` balance cases
  still pass unchanged. Each one starts from center, so its moves are outward.
- `docs/invariants.md`: add a `## Balance center detent` section after `## Mixer drag readout`.
  Its guarantee is the inward-only snap below `BALANCE_CENTER_SNAP`, applied only to
  `BalanceChange` and not to settings restore. It is enforced by
  `state::snap_balance_to_center` and `handlers::mixer::balance_change`, and pinned by the
  tests above.
- README Usage: after "The balance slider sets its stereo balance.", add a sentence saying
  that a drag moving back toward the middle snaps to exact center within 10% of it. Do not
  touch the brief or status blocks.
- `make check` passes.

## Done
