use super::*;

#[test]
fn dragging_balance_into_the_middle_snaps_to_center() {
    let (mut player, state) = test_player();
    seed_current_title(&mut player, &state, "Tune");
    let _ = update(&mut player, Message::BalanceChange(-0.4));
    let _ = update(&mut player, Message::BalanceChange(-0.05));
    assert_eq!(state.blocking_lock().balance(), 0.0);
    assert!(root_view_draws(&player, "Balance: Center"));
}

// A real drag sends many values; continuing inward through the zone stays
// snapped, while moving outward again leaves center.
#[test]
fn a_continuous_inward_balance_drag_stays_snapped() {
    let (mut player, state) = test_player();
    let _ = update(&mut player, Message::BalanceChange(-0.4));
    for v in [-0.05, -0.04, -0.02, 0.01] {
        let _ = update(&mut player, Message::BalanceChange(v));
        assert_eq!(state.blocking_lock().balance(), 0.0, "at {v}");
    }
    let _ = update(&mut player, Message::BalanceChange(0.06));
    assert_eq!(state.blocking_lock().balance(), 0.06);
    let _ = update(&mut player, Message::MixerReleased);
}

// After a release, a lone step out of center (an arrow key) is not snapped.
#[test]
fn a_balance_step_after_release_leaves_center() {
    let (mut player, state) = test_player();
    let _ = update(&mut player, Message::BalanceChange(-0.4));
    let _ = update(&mut player, Message::BalanceChange(-0.05));
    let _ = update(&mut player, Message::MixerReleased);
    let _ = update(&mut player, Message::BalanceChange(0.01));
    assert_eq!(state.blocking_lock().balance(), 0.01);
}
