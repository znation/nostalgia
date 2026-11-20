use super::*;

// A spectrum reply stores exactly what the backend reported, and a later
// `None` reply clears the visualizer instead of keeping the old bars.
#[test]
fn spectrum_reply_stores_the_bars_and_lets_a_quieter_band_fall() {
    let (mut player, _state) = test_player();
    let epoch = player.position_epoch;
    let spectrum = recording_spectrum();

    let _ = update(
        &mut player,
        Message::Spectrum {
            epoch,
            spectrum: Some(spectrum),
        },
    );
    assert_eq!(player.spectrum, Some(spectrum));
    assert_eq!(player.spectrum_peaks, Some(spectrum));

    // A lower band leaves its peak to fall one step.
    let mut lower = spectrum;
    lower.bands[2] = 0.0;
    let _ = update(
        &mut player,
        Message::Spectrum {
            epoch,
            spectrum: Some(lower),
        },
    );
    assert_eq!(player.spectrum.unwrap().bands[2], 0.25);
    assert_eq!(player.spectrum_peaks.unwrap().bands[2], 0.375);

    let _ = update(
        &mut player,
        Message::Spectrum {
            epoch,
            spectrum: None,
        },
    );
    assert_eq!(player.spectrum, None);
    assert_eq!(player.spectrum_peaks, None);
}

// A reply from before a Stop leaves the stored peaks where they were.
#[test]
fn a_stale_spectrum_reply_leaves_the_peaks_alone() {
    let (mut player, _state) = test_player();
    let held = recording_spectrum();
    player.spectrum_peaks = Some(held);
    let stale = player.position_epoch.wrapping_sub(1);

    let _ = update(
        &mut player,
        Message::Spectrum {
            epoch: stale,
            spectrum: None,
        },
    );
    assert_eq!(player.spectrum_peaks, Some(held));
}

// A spectrum reply is dropped when it was issued before a Stop, or when it
// lands while a transport transition is still reaching the backend: either
// way it could describe the previous player. A reply at the current epoch
// with no transition pending is stored, so the drops above come from the
// guards and not from the message being ignored entirely.
#[test]
fn spectrum_reply_with_a_stale_epoch_or_a_pending_transition_is_dropped() {
    assert_reply_dropped_while_stale_or_pending(
        |epoch, value| Message::Spectrum {
            epoch,
            spectrum: Some(value),
        },
        |player| player.spectrum,
        |player, value| player.spectrum = Some(value),
        Spectrum {
            bands: [0.25; BAND_COUNT],
        },
        recording_spectrum(),
    );
}

// Pause keeps the loaded preview's bars frozen, while Stop clears them
// immediately, like the elapsed readout and the status row's format.
#[test]
fn stop_clears_the_spectrum_while_pause_keeps_it() {
    let (mut player, _current_track) = play_song_1_then_browse_to_album_2();
    let spectrum = recording_spectrum();
    player.spectrum = Some(spectrum);
    player.spectrum_peaks = Some(spectrum);

    let _ = update(&mut player, Message::Pause);
    assert_eq!(player.spectrum, Some(spectrum));
    assert_eq!(player.spectrum_peaks, Some(spectrum));

    let _ = update(&mut player, Message::Stop);
    assert_eq!(player.spectrum, None);
    assert_eq!(player.spectrum_peaks, None);
}
