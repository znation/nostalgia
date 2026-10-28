//! The LCD's timed mixer readout after a key or wheel nudge of the volume, balance, preamp, or a band.

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

fn readout(state: &Arc<Mutex<AppState>>) -> String {
    format!("Volume: {}%", level(state))
}

#[test]
fn a_key_nudge_shows_the_level_in_the_title_s_place() {
    for message in [Message::VolumeUp, Message::VolumeDown] {
        let (player, state) = nudged(message);
        assert!(root_view_draws(&player, &readout(&state)));
        assert!(!root_view_draws(&player, "Tune"));
        assert!(player.needs_lcd_tick());
    }
}

#[test]
fn the_readout_lasts_four_ticks_then_the_title_returns() {
    let (mut player, state) = nudged(Message::VolumeUp);
    let readout = readout(&state);
    for _ in 0..3 {
        let _ = update(&mut player, Message::MarqueeTick);
    }
    assert!(root_view_draws(&player, &readout));
    let _ = update(&mut player, Message::MarqueeTick);
    assert!(root_view_draws(&player, "Tune"));
    assert!(!root_view_draws(&player, &readout));
    assert_eq!(player.mixer_flash_ticks, 0);
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
    assert!(player.mixer_flash_ticks > 0);
    for direction in [0.0, f32::NAN] {
        let (player, _) = nudged(Message::VolumeWheel(direction));
        assert_eq!(player.mixer_flash_ticks, 0);
    }
}

#[test]
fn releasing_a_mixer_key_keeps_the_readout() {
    let (mut player, state) = nudged(Message::VolumeUp);
    let _ = update(&mut player, Message::MixerReleased);
    assert!(root_view_draws(&player, &readout(&state)));
}

#[test]
fn a_slider_roll_flashes_that_sliders_readout() {
    let (state_player, _) = nudged(Message::BalanceWheel(1.0));
    assert_eq!(state_player.mixer_drag, None);
    let cases = [
        (Message::BalanceWheel(1.0), views::MixerDrag::Balance),
        (Message::EqPreampWheel(1.0), views::MixerDrag::EqPreamp),
        (Message::EqBandWheel(0, 1.0), views::MixerDrag::EqBand(0)),
    ];
    for (message, slider) in cases {
        let (mut player, state) = nudged(message);
        let text = {
            let s = state.blocking_lock();
            views::mixer_readout_text(
                slider,
                s.volume(),
                s.balance(),
                s.eq_preamp(),
                &s.eq_bands(),
            )
        };
        assert!(root_view_draws(&player, &text), "{text}");
        assert!(!root_view_draws(&player, "Tune"));
        assert_eq!(player.mixer_drag, None);
        for _ in 0..4 {
            let _ = update(&mut player, Message::MarqueeTick);
        }
        assert!(root_view_draws(&player, "Tune"));
        assert_eq!(player.mixer_flash_ticks, 0);
    }
}

#[test]
fn a_no_op_slider_roll_flashes_nothing() {
    let messages = [
        Message::BalanceWheel(0.0),
        Message::BalanceWheel(f32::NAN),
        Message::EqPreampWheel(0.0),
        Message::EqPreampWheel(f32::NAN),
        Message::EqBandWheel(0, 0.0),
        Message::EqBandWheel(0, f32::NAN),
        Message::EqBandWheel(BAND_COUNT, 1.0),
    ];
    for message in messages {
        let (player, _) = nudged(message);
        assert_eq!(player.mixer_flash_ticks, 0);
    }
}

#[test]
fn a_slider_roll_and_a_volume_nudge_replace_each_other() {
    let (mut player, _) = nudged(Message::VolumeUp);
    let _ = update(&mut player, Message::BalanceWheel(1.0));
    assert!(root_view_draws(&player, "Balance:"));
    assert!(!root_view_draws(&player, "Volume:"));
    let _ = update(&mut player, Message::MarqueeTick);
    let _ = update(&mut player, Message::VolumeUp);
    assert!(root_view_draws(&player, "Volume:"));
    assert!(!root_view_draws(&player, "Balance:"));
    assert_eq!(player.mixer_flash_ticks, 4);
}

#[test]
fn a_drag_readout_takes_precedence_over_the_flash() {
    let (mut player, _) = nudged(Message::VolumeUp);
    player.mixer_drag = Some(views::MixerDrag::Balance);
    assert!(root_view_draws(&player, "Balance:"));
    assert!(!root_view_draws(&player, "Volume:"));
}

#[test]
fn a_no_op_mixer_roll_keeps_the_flash() {
    let rolls = [
        Message::BalanceWheel(0.0),
        Message::BalanceWheel(f32::NAN),
        Message::EqPreampWheel(0.0),
        Message::EqBandWheel(0, f32::NAN),
        Message::EqBandWheel(usize::MAX, 1.0),
    ];
    for message in rolls {
        let (mut player, _) = nudged(Message::VolumeUp);
        let flash = player.mixer_flash_ticks;
        let _ = update(&mut player, message);
        assert_eq!(player.mixer_flash_ticks, flash);
    }
}

#[test]
fn a_band_roll_flashes_the_rolled_band_with_its_new_gain() {
    let (player, state) = nudged(Message::EqBandWheel(BAND_COUNT - 1, 1.0));
    let gain = state.blocking_lock().eq_bands()[BAND_COUNT - 1];
    assert!(gain > 0.0);
    assert!(root_view_draws(&player, "16KHZ"));
    assert!(root_view_draws(&player, &format!("+{gain:.0} DB")));
    assert!(!root_view_draws(&player, "60HZ"));
}

#[test]
fn a_roll_against_a_limit_still_flashes_the_clamped_level() {
    for (limit, direction, text) in [
        (GAIN_MAX_DB, 1.0, "Preamp: +12 DB"),
        (GAIN_MIN_DB, -1.0, "Preamp: -12 DB"),
    ] {
        let (mut player, state) = test_player();
        seed_current_title(&mut player, &state, "Tune");
        state.blocking_lock().set_eq_preamp(limit);
        let _ = update(&mut player, Message::EqPreampWheel(direction));
        assert_eq!(state.blocking_lock().eq_preamp(), limit);
        assert_eq!(player.mixer_flash_ticks, 4);
        assert!(root_view_draws(&player, text), "{text}");
    }
}

#[test]
fn releasing_a_slider_keeps_a_slider_roll_flash() {
    let (mut player, _) = nudged(Message::BalanceWheel(1.0));
    let _ = update(&mut player, Message::MixerReleased);
    assert!(root_view_draws(&player, "Balance:"));
    assert_eq!(player.mixer_flash_ticks, 4);
}
