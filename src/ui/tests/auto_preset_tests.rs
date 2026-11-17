use super::*;

/// Asserts the shared state holds `preset`'s preamp and band gains.
fn assert_curve_applied(state: &AppState, preset: crate::equalizer::Preset) {
    assert_eq!(state.eq_preamp(), preset.preamp);
    assert_eq!(state.eq_bands(), preset.bands);
}

// The AUTO button flips only its own flag: the stored curve, preset, and
// equalizer on/off state stay put, and the toggle schedules no backend work.
#[test]
fn toggle_eq_auto_flips_only_the_auto_flag() {
    let (mut player, state) = test_player();
    let preset = rock_preset();
    let _ = update(
        &mut player,
        Message::EqPresetSelected(PresetChoice::BuiltIn(preset)),
    );

    assert_message_schedules_no_work(&mut player, Message::ToggleEqAuto);
    {
        let state = state.blocking_lock();
        assert!(state.eq_auto);
        assert!(!state.eq_enabled);
        assert_eq!(state.eq_preset(), Some(preset));
        assert_curve_applied(&state, preset);
    }

    assert_message_schedules_no_work(&mut player, Message::ToggleEqAuto);
    let state = state.blocking_lock();
    assert!(!state.eq_auto);
    assert_eq!(state.eq_preset(), Some(preset));
    assert_eq!(state.eq_bands(), preset.bands);
}

// AUTO tracks the selected song's primary genre through the same
// mutate-and-forward path as a pick-list pick: a trimmed, case-insensitive
// match applies the whole preset and reaches the audio backend, while an
// unmatched genre or AUTO off leaves the previous Rock curve and its
// forwarded calls alone.
#[test]
fn auto_on_applies_a_genre_matched_preset_on_track_selection() {
    let (recording, mut player, state) = player_with_recording();
    let mut songs = stepping_songs();
    songs[0].genre = "  rOcK  ".to_string();
    songs[1].genre = "Alternative".to_string();
    songs[2].genre = "Vocal".to_string();
    let _ = update(&mut player, Message::SongsLoaded(songs));

    let vocal = *crate::equalizer::PRESETS
        .iter()
        .find(|preset| preset.name == "Vocal")
        .unwrap();
    let _ = update(
        &mut player,
        Message::EqPresetSelected(PresetChoice::BuiltIn(vocal)),
    );
    state.blocking_lock().eq_auto = true;

    let before = recording.calls().len();
    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 0 });
    let rock = rock_preset();
    {
        let state = state.blocking_lock();
        assert_eq!(state.eq_preset(), Some(rock));
        assert_curve_applied(&state, rock);
    }
    let calls = recording.calls();
    assert_eq!(
        calls.len(),
        before + 1,
        "the matched preset is forwarded once"
    );
    match calls.last() {
        Some(AudioCall::SetEqualizer(snapshot)) => {
            assert_eq!(snapshot.preamp_db, rock.preamp);
            assert_eq!(snapshot.bands_db, rock.bands);
        }
        other => panic!("expected the Rock curve to reach the backend, got {other:?}"),
    }

    // An unmatched genre changes neither the stored curve nor the backend.
    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 1 });
    assert_eq!(state.blocking_lock().eq_preset(), Some(rock));
    assert_eq!(recording.calls().len(), calls.len());

    // With AUTO off, even a genre that names another preset leaves the curve
    // and the backend alone.
    state.blocking_lock().eq_auto = false;
    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 2 });
    let state = state.blocking_lock();
    assert_eq!(state.eq_preset(), Some(rock));
    assert_eq!(state.eq_bands(), rock.bands);
    assert_eq!(recording.calls().len(), calls.len());
}
