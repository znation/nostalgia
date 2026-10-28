//! The LCD's timed `Volume: N%` readout after a key or wheel volume nudge.

use super::*;

fn nudged(message: Message) -> (WinampPlayer, Arc<Mutex<AppState>>) {
    let (mut player, state) = test_player();
    seed_current_title(&mut player, &state, "Tune");
    let _ = update(&mut player, message);
    (player, state)
}

fn level(state: &Arc<Mutex<AppState>>) -> u32 {
    (state.blocking_lock().volume() * 100.0).round() as u32
}

#[test]
fn a_key_nudge_shows_the_level_in_the_title_s_place() {
    for message in [Message::VolumeUp, Message::VolumeDown] {
        let (player, state) = nudged(message);
        assert!(root_view_draws(
            &player,
            &format!("Volume: {}%", level(&state))
        ));
        assert!(!root_view_draws(&player, "Tune"));
        assert!(player.needs_lcd_tick());
    }
}

#[test]
fn the_readout_lasts_four_ticks_then_the_title_returns() {
    let (mut player, state) = nudged(Message::VolumeUp);
    let readout = format!("Volume: {}%", level(&state));
    for _ in 0..3 {
        let _ = update(&mut player, Message::MarqueeTick);
    }
    assert!(root_view_draws(&player, &readout));
    let _ = update(&mut player, Message::MarqueeTick);
    assert!(root_view_draws(&player, "Tune"));
    assert!(!root_view_draws(&player, &readout));
    assert_eq!(player.volume_flash_ticks, 0);
    assert_eq!(player.needs_lcd_tick(), player.needs_marquee());
}

#[test]
fn the_flash_pauses_a_scrolling_title() {
    let (mut player, state) = test_player();
    seed_current_title(&mut player, &state, &"Long title ".repeat(8));
    assert!(player.needs_marquee());
    let _ = update(&mut player, Message::VolumeUp);
    let start = player.marquee_offset;
    for _ in 0..4 {
        let _ = update(&mut player, Message::MarqueeTick);
    }
    assert_eq!(player.marquee_offset, start);
    let _ = update(&mut player, Message::MarqueeTick);
    assert_eq!(player.marquee_offset, start.wrapping_add(1));
}

#[test]
fn the_wheel_flashes_but_a_zero_or_nan_roll_does_not() {
    let (player, _) = nudged(Message::VolumeWheel(1.0));
    assert!(player.volume_flash_ticks > 0);
    for direction in [0.0, f32::NAN] {
        let (player, _) = nudged(Message::VolumeWheel(direction));
        assert_eq!(player.volume_flash_ticks, 0);
    }
}

#[test]
fn releasing_a_mixer_key_keeps_the_readout() {
    let (mut player, state) = nudged(Message::VolumeUp);
    let _ = update(&mut player, Message::MixerReleased);
    assert!(root_view_draws(
        &player,
        &format!("Volume: {}%", level(&state))
    ));
}

#[test]
fn a_balance_or_preamp_roll_clears_the_flash() {
    for message in [Message::BalanceWheel(1.0), Message::EqPreampWheel(1.0)] {
        let (mut player, _) = nudged(Message::VolumeUp);
        let _ = update(&mut player, message);
        assert_eq!(player.volume_flash_ticks, 0);
        assert!(root_view_draws(&player, "Tune"));
    }
}

#[test]
fn a_drag_readout_takes_precedence_over_the_flash() {
    let (mut player, _) = nudged(Message::VolumeUp);
    player.mixer_drag = Some(views::MixerDrag::Balance);
    assert!(root_view_draws(&player, "Balance:"));
    assert!(!root_view_draws(&player, "Volume:"));
}
