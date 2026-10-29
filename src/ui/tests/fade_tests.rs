//! Shift+V's fade-out and stop: the stepped volume, the closing stop, and the
//! interruptions. Plain tests (not `#[tokio::test]`): the handlers take the
//! state lock with `blocking_lock`, which panics inside a runtime.

use super::*;

fn playing_with_recording() -> (Arc<RecordingAudio>, WinampPlayer, Arc<Mutex<AppState>>) {
    let (recording, player, state) = player_with_recording();
    {
        let mut state = state.blocking_lock();
        state.current_track = Some("song-1".to_string());
        state.play();
    }
    (recording, player, state)
}

fn tick(player: &mut WinampPlayer) -> Task<Message> {
    update(player, Message::MarqueeTick)
}

fn volumes(recording: &RecordingAudio) -> Vec<f32> {
    recording
        .calls()
        .into_iter()
        .filter_map(|call| match call {
            AudioCall::SetVolume(volume) => Some(volume),
            _ => None,
        })
        .collect()
}

#[test]
fn a_full_fade_steps_the_volume_down_then_stops_and_restores_it() {
    let (recording, mut player, state) = playing_with_recording();
    let saved = state.blocking_lock().volume();

    let _ = update(&mut player, Message::FadeOutAndStop);
    assert!(recording.calls().is_empty());
    assert!(player.needs_lcd_tick());

    for _ in 0..7 {
        let _ = tick(&mut player);
    }
    let steps = volumes(&recording);
    assert_eq!(steps.len(), 7);
    for (index, step) in steps.iter().enumerate() {
        let expected = saved * (7 - index) as f32 / 8.0;
        assert!((step - expected).abs() < 1e-6, "{step} vs {expected}");
    }
    assert!(steps.windows(2).all(|pair| pair[1] < pair[0]));

    let task = tick(&mut player);
    futures::executor::block_on(assert_transport_settled(task, "fade stop"));
    let calls = recording.calls();
    assert_eq!(calls[7], AudioCall::Stop);
    assert_eq!(calls[8], AudioCall::SetVolume(saved));
    assert_eq!(calls.len(), 9);
    let state = state.blocking_lock();
    assert!(!state.is_playing);
    assert_eq!(state.volume(), saved);
    drop(state);
    assert_eq!(player.needs_lcd_tick(), player.needs_marquee());
}

#[test]
fn fade_out_while_not_playing_acts_as_stop() {
    let (recording, mut player, state) = playing_with_recording();
    state.blocking_lock().is_playing = false;

    let task = update(&mut player, Message::FadeOutAndStop);
    futures::executor::block_on(assert_transport_settled(task, "stop"));
    assert_eq!(player.fade_out_ticks, 0);
    assert_eq!(recording.calls(), vec![AudioCall::Stop]);
}

#[test]
fn a_second_fade_press_does_not_restart_the_countdown() {
    let (_recording, mut player, _state) = playing_with_recording();
    let _ = update(&mut player, Message::FadeOutAndStop);
    let _ = tick(&mut player);
    let _ = tick(&mut player);
    let before = player.fade_out_ticks;
    let _ = update(&mut player, Message::FadeOutAndStop);
    assert_eq!(player.fade_out_ticks, before);
}

#[test]
fn play_mid_fade_restores_the_volume_and_cancels_the_rest() {
    let (recording, mut player, state) = playing_with_recording();
    let saved = state.blocking_lock().volume();
    let _ = update(&mut player, Message::FadeOutAndStop);
    let _ = tick(&mut player);
    let _ = update(&mut player, Message::Play);
    assert_eq!(player.fade_out_ticks, 0);
    assert_eq!(volumes(&recording).last(), Some(&saved));

    let before = recording.calls().len();
    for _ in 0..8 {
        let _ = tick(&mut player);
    }
    assert_eq!(recording.calls().len(), before);
}

#[test]
fn a_track_selection_mid_fade_restores_the_volume() {
    let (recording, mut player, state) = playing_with_recording();
    player.songs.items = stepping_songs();
    let saved = state.blocking_lock().volume();
    let _ = update(&mut player, Message::FadeOutAndStop);
    let _ = tick(&mut player);
    let epoch = player.songs.epoch;
    let _ = update(&mut player, Message::TrackSelected { epoch, index: 0 });
    assert_eq!(player.fade_out_ticks, 0);
    assert_eq!(volumes(&recording).last(), Some(&saved));
}

#[test]
fn stop_mid_fade_stops_and_then_restores_the_volume() {
    let (recording, mut player, state) = playing_with_recording();
    let saved = state.blocking_lock().volume();
    let _ = update(&mut player, Message::FadeOutAndStop);
    let _ = tick(&mut player);
    let task = update(&mut player, Message::Stop);
    futures::executor::block_on(assert_transport_settled(task, "stop"));
    assert_eq!(player.fade_out_ticks, 0);
    let calls = recording.calls();
    assert_eq!(calls[calls.len() - 2], AudioCall::Stop);
    assert_eq!(calls[calls.len() - 1], AudioCall::SetVolume(saved));
}

#[test]
fn a_full_fade_leaves_the_saved_settings_volume_alone() {
    let (_recording, mut player, state) = playing_with_recording();
    let before = Settings::from_state(&state.blocking_lock());
    let _ = update(&mut player, Message::FadeOutAndStop);
    for _ in 0..7 {
        let _ = tick(&mut player);
    }
    let task = tick(&mut player);
    futures::executor::block_on(assert_transport_settled(task, "fade stop"));
    assert_eq!(Settings::from_state(&state.blocking_lock()), before);
}

fn hold_shift(player: &mut WinampPlayer, shift: bool) {
    let _ = update(
        player,
        Message::ModifiersHeld {
            control: false,
            shift,
        },
    );
}

#[test]
fn shift_click_stop_starts_the_fade() {
    let (recording, mut player, state) = playing_with_recording();
    let saved = state.blocking_lock().volume();
    hold_shift(&mut player, true);

    let _ = update(&mut player, Message::StopButton);
    assert!(recording.calls().is_empty());
    assert_eq!(player.fade_out_ticks, crate::ui::player::FADE_OUT_TICKS);

    for _ in 0..7 {
        let _ = tick(&mut player);
    }
    let steps = volumes(&recording);
    assert_eq!(steps.len(), 7);
    for (index, step) in steps.iter().enumerate() {
        let expected = saved * (7 - index) as f32 / 8.0;
        assert!((step - expected).abs() < 1e-6, "{step} vs {expected}");
    }
    let task = tick(&mut player);
    futures::executor::block_on(assert_transport_settled(task, "fade stop"));
    let calls = recording.calls();
    assert_eq!(calls[7], AudioCall::Stop);
    assert_eq!(calls[8], AudioCall::SetVolume(saved));
}

#[test]
fn plain_stop_click_stops_at_once() {
    let (recording, mut player, _state) = playing_with_recording();
    assert!(!player.shift_held);

    let task = update(&mut player, Message::StopButton);

    futures::executor::block_on(assert_transport_settled(task, "stop"));
    assert!(recording.calls().contains(&AudioCall::Stop));
    assert_eq!(player.fade_out_ticks, 0);
}

// The options menu's `Stop with fadeout` row reuses `Message::FadeOutAndStop`:
// choosing it (by mouse or by keyboard) closes the menu and then fades like
// Shift+V while a preview plays, or acts as Stop while paused or stopped.
#[test]
fn options_stop_with_fadeout_row_fades_or_stops_and_closes() {
    let row_index = OptionsRow::ALL
        .iter()
        .position(|row| matches!(row, OptionsRow::FadeOutAndStop))
        .unwrap();
    for keyboard in [false, true] {
        // While a preview plays: fade like Shift+V, with no backend call on
        // the press and no change to the playing state.
        let (recording, mut player, _state) = playing_with_recording();
        player.options_visible = true;
        if keyboard {
            player.options_selected = row_index;
        }
        let task = update(
            &mut player,
            if keyboard {
                Message::OptionsActivate
            } else {
                OptionsRow::FadeOutAndStop.message()
            },
        );
        assert!(!player.options_visible, "the row must close the menu");
        assert_eq!(player.fade_out_ticks, crate::ui::player::FADE_OUT_TICKS);
        assert!(recording.calls().is_empty());
        assert_no_task(task);

        // While paused: stop at once, with no countdown.
        let (recording, mut player, state) = playing_with_recording();
        state.blocking_lock().is_playing = false;
        player.options_visible = true;
        if keyboard {
            player.options_selected = row_index;
        }
        let task = update(
            &mut player,
            if keyboard {
                Message::OptionsActivate
            } else {
                OptionsRow::FadeOutAndStop.message()
            },
        );
        futures::executor::block_on(assert_transport_settled(task, "stop"));
        assert!(!player.options_visible, "the row must close the menu");
        assert_eq!(player.fade_out_ticks, 0);
        assert_eq!(recording.calls(), vec![AudioCall::Stop]);
    }
}

#[test]
fn shift_release_restores_a_plain_stop_click() {
    let (recording, mut player, _state) = playing_with_recording();
    hold_shift(&mut player, true);
    hold_shift(&mut player, false);

    let task = update(&mut player, Message::StopButton);

    futures::executor::block_on(assert_transport_settled(task, "stop"));
    assert!(recording.calls().contains(&AudioCall::Stop));
    assert_eq!(player.fade_out_ticks, 0);
}
