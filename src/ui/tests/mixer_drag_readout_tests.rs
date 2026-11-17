use super::*;

// Dragging the volume slider swaps the LCD title for the live level, and the
// release brings the title back.
#[test]
fn dragging_volume_shows_the_level_until_release() {
    let (mut player, state) = test_player();
    seed_current_title(&mut player, &state, "Tune");
    assert!(root_view_draws(&player, "Tune"));

    let _ = update(&mut player, Message::VolumeChange(0.4));
    assert_eq!(player.mixer_drag, Some(views::MixerDrag::Volume));
    assert!(root_view_draws(&player, "Volume: 40%"));
    assert!(!root_view_draws(&player, "Tune"));

    let _ = update(&mut player, Message::MixerReleased);
    assert_eq!(player.mixer_drag, None);
    assert!(root_view_draws(&player, "Tune"));
    assert!(!root_view_draws(&player, "Volume: 40%"));
}

#[test]
fn dragging_an_eq_band_shows_its_gain_until_release() {
    let (mut player, state) = test_player();
    seed_current_title(&mut player, &state, "Tune");
    let _ = update(&mut player, Message::EqBandChange(0, 5.0));
    assert_eq!(player.mixer_drag, Some(views::MixerDrag::EqBand(0)));
    assert!(root_view_draws(&player, "EQ: 60HZ +5 DB"));
    assert!(!root_view_draws(&player, "Tune"));
    let _ = update(&mut player, Message::MixerReleased);
    assert_eq!(player.mixer_drag, None);
    assert!(root_view_draws(&player, "Tune"));
}

#[test]
fn dragging_the_preamp_shows_its_gain() {
    let (mut player, state) = test_player();
    seed_current_title(&mut player, &state, "Tune");
    let _ = update(&mut player, Message::EqPreampChange(-3.0));
    assert_eq!(player.mixer_drag, Some(views::MixerDrag::EqPreamp));
    assert!(root_view_draws(&player, "Preamp: -3 DB"));
}

#[test]
fn eq_toggle_and_preset_do_not_show_the_readout() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::ToggleEqualizer);
    assert_eq!(player.mixer_drag, None);
    let _ = update(
        &mut player,
        Message::EqPresetSelected(PresetChoice::BuiltIn(rock_preset())),
    );
    assert_eq!(player.mixer_drag, None);
}

#[test]
fn an_out_of_range_eq_band_shows_no_readout() {
    let (mut player, _state) = test_player();
    let _ = update(
        &mut player,
        Message::EqBandChange(crate::equalizer::BAND_COUNT, 5.0),
    );
    assert_eq!(player.mixer_drag, None);
}

#[test]
fn dragging_balance_shows_the_side() {
    let (mut player, state) = test_player();
    seed_current_title(&mut player, &state, "Tune");
    let _ = update(&mut player, Message::BalanceChange(-0.25));
    assert_eq!(player.mixer_drag, Some(views::MixerDrag::Balance));
    assert!(root_view_draws(&player, "Balance: 25% Left"));
}

// Keys and the wheel change the volume without showing the readout.
#[test]
fn volume_keys_and_wheel_do_not_show_the_readout() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::VolumeUp);
    assert_eq!(player.mixer_drag, None);
    let _ = update(&mut player, Message::VolumeWheel(1.0));
    assert_eq!(player.mixer_drag, None);
}

#[test]
fn the_marquee_pauses_during_a_mixer_drag() {
    let (mut player, state) = test_player();
    seed_current_title(&mut player, &state, "A Title Too Long For The LCD Bar");
    assert!(player.needs_marquee());
    let _ = update(&mut player, Message::VolumeChange(0.5));
    assert!(!player.needs_marquee());
    let _ = update(&mut player, Message::MixerReleased);
    assert!(player.needs_marquee());
}
