use super::*;

#[test]
fn repeat_label_mirrors_repeat_state() {
    assert_eq!(repeat_label(true), "Repeat: On");
    assert_eq!(repeat_label(false), "Repeat: Off");
}

#[test]
fn shuffle_label_mirrors_shuffle_state() {
    assert_eq!(shuffle_label(true), "Shuffle: On");
    assert_eq!(shuffle_label(false), "Shuffle: Off");
}

// The `view_*` builders are the code that runs on every frame, and no
// other test reaches them: the row tests stop at the (title, label,
// message) tuples and the `update` tests stop before the view layer, so
// a regression that made a builder panic — a bad slider range, an
// out-of-bounds index in the `scrollable_list` loop — would take the
// window down on every refresh with no test catching it. iced `Element`s
// expose no tree introspection, so the observable contract here is that
// each builder constructs its widget tree without panicking over the
// input space the app actually produces: every browse view over both the
// loaded library and the pre-load empty buffer, both play states, the
// volume endpoints the update arm can store, and both repeat states.

// The label-reflow test above pins only that every transport button is
// fixed-width and that its width is independent of the play, repeat, and
// shuffle state; a zeroed, widened, or otherwise wrong width literal would
// clear it while the glyph clipped or the row moved. Pin the face widths in
// row order: Previous 34, Play/Pause 30, Stop 30, Next 34, Repeat 104,
// Shuffle 112. Each is the width of that button's widest face, so a
// deliberate glyph or width change must update this pin.
#[test]
fn transport_buttons_keep_their_measured_face_widths() {
    let widths: Vec<Length> = transport_buttons(false, false, false)
        .iter()
        .map(|button| button.as_widget().size().width)
        .collect();
    assert_eq!(
        widths,
        [
            Length::Fixed(34.0),
            Length::Fixed(30.0),
            Length::Fixed(30.0),
            Length::Fixed(34.0),
            Length::Fixed(104.0),
            Length::Fixed(112.0),
        ]
    );
}

// The four glyph buttons' shared spec, in the mockup's order: Previous,
// Play/Pause, Stop, Next. Pin each glyph, tooltip word, width, and message so
// a swapped face or reordered pair fails here.
#[test]
fn transport_glyphs_match_the_mockup_order_and_widths() {
    let stopped = transport_glyphs(false);
    let glyphs: Vec<&str> = stopped.iter().map(|(glyph, _, _, _)| *glyph).collect();
    assert_eq!(glyphs, ["◀◀", "▶", "■", "▶▶"]);
    let words: Vec<&str> = stopped.iter().map(|(_, word, _, _)| *word).collect();
    assert_eq!(words, ["Previous", "Play", "Stop", "Next"]);
    let widths: Vec<f32> = stopped.iter().map(|(_, _, width, _)| *width).collect();
    assert_eq!(widths, [34.0, 30.0, 30.0, 34.0]);
    assert!(matches!(stopped[0].3, Message::PreviousTrack));
    assert!(matches!(stopped[1].3, Message::PlayPause));
    assert!(matches!(stopped[2].3, Message::Stop));
    assert!(matches!(stopped[3].3, Message::NextTrack));

    let playing = transport_glyphs(true);
    let playing_glyphs: Vec<&str> = playing.iter().map(|(glyph, _, _, _)| *glyph).collect();
    assert_eq!(playing_glyphs, ["◀◀", "II", "■", "▶▶"]);
    let playing_words: Vec<&str> = playing.iter().map(|(_, word, _, _)| *word).collect();
    assert_eq!(playing_words, ["Previous", "Pause", "Stop", "Next"]);
    let playing_widths: Vec<f32> = playing.iter().map(|(_, _, width, _)| *width).collect();
    assert_eq!(
        playing_widths, widths,
        "the glyph swap must not resize a button"
    );
}

#[test]
fn transport_controls_construct_for_both_play_states_volume_endpoints_and_repeat_states() {
    // `view()` passes the shared state's `is_playing`, clamped `volume`,
    // `balance`, `repeat`, and `shuffle`, plus the player's EQ-panel and
    // library-panel visibility, straight through, so build the transport row
    // for every value the update arms can store, in every play, repeat,
    // shuffle, and panel-visibility state.
    for volume in [0.0, 0.5, 1.0] {
        for balance in [-1.0, 0.0, 1.0] {
            for is_playing in [false, true] {
                for repeat in [false, true] {
                    for shuffle in [false, true] {
                        for eq_panel_visible in [false, true] {
                            for library_panel_visible in [false, true] {
                                let _controls = view_transport_controls(
                                    is_playing,
                                    volume,
                                    balance,
                                    repeat,
                                    shuffle,
                                    eq_panel_visible,
                                    library_panel_visible,
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn eq_panel_toggle_targets_the_visibility_flag() {
    let (label, width, message) = eq_panel_toggle();

    assert_eq!(label, "EQ");
    assert_eq!(width, EQ_PANEL_BUTTON_WIDTH);
    assert_eq!(width, 30.0);
    assert!(
        matches!(message, Message::ToggleEqualizerPanel),
        "the EQ button must toggle the equalizer panel, not the equalizer's on/off state"
    );
}

#[test]
fn library_panel_toggle_targets_the_visibility_flag() {
    let (label, width, message) = library_panel_toggle();

    assert_eq!(label, "PL");
    assert_eq!(width, LIBRARY_PANEL_BUTTON_WIDTH);
    assert_eq!(width, 30.0);
    assert!(
        matches!(message, Message::ToggleLibraryPanel),
        "the PL button must toggle the library section, not any playback or browse state"
    );
}

#[test]
fn volume_slider_spec_matches_the_state_clamp_and_pins_its_granularity() {
    assert_slider_range_matches_clamp(clamp_volume, VOLUME_MIN, VOLUME_MAX, VOLUME_STEP);

    // One-percent drag granularity, pinned so the step cannot silently
    // coarsen while the endpoints stay put.
    assert_eq!(VOLUME_STEP, 0.01);
}

/// Asserts the vertical direction a wheel mapping produces, without needing
/// `Message` to implement equality.
fn assert_wheel_direction(message: Message, expected: f32) {
    match message {
        Message::VolumeWheel(direction) => assert_eq!(direction, expected),
        other => panic!("expected a volume wheel message, got {other:?}"),
    }
}

// Both wheel delta kinds carry their vertical direction through as a sign:
// positive scrolls up (raise), negative scrolls down (lower), and the
// magnitude is not a step count. Line and pixel deltas map alike.
#[test]
fn volume_wheel_message_maps_a_vertical_roll_to_a_direction() {
    assert_wheel_direction(
        volume_wheel_message(iced::mouse::ScrollDelta::Lines { x: 0.0, y: 3.0 }),
        1.0,
    );
    assert_wheel_direction(
        volume_wheel_message(iced::mouse::ScrollDelta::Lines { x: 0.0, y: -2.0 }),
        -1.0,
    );
    assert_wheel_direction(
        volume_wheel_message(iced::mouse::ScrollDelta::Pixels { x: 0.0, y: 120.0 }),
        1.0,
    );
    assert_wheel_direction(
        volume_wheel_message(iced::mouse::ScrollDelta::Pixels { x: 0.0, y: -45.0 }),
        -1.0,
    );
}

// A horizontal-only roll has no vertical delta, so it maps to the no-op
// direction rather than borrowing the horizontal sign.
#[test]
fn volume_wheel_message_zeroes_a_horizontal_only_roll() {
    assert_wheel_direction(
        volume_wheel_message(iced::mouse::ScrollDelta::Lines { x: 4.0, y: 0.0 }),
        0.0,
    );
    assert_wheel_direction(
        volume_wheel_message(iced::mouse::ScrollDelta::Pixels { x: 30.0, y: 0.0 }),
        0.0,
    );
}

// The balance twin of the volume spec: the balance slider's range must
// equal the range `set_balance`/`clamp_balance` store, so a drag to either
// end lands exactly on the stored extreme, and its step is pinned so it
// cannot silently coarsen.
#[test]
fn balance_slider_spec_matches_the_state_clamp_and_pins_its_granularity() {
    assert_slider_range_matches_clamp(clamp_balance, BALANCE_MIN, BALANCE_MAX, BALANCE_STEP);

    assert_eq!(BALANCE_STEP, 0.01);
}

// The transport row is rebuilt every frame, and the Play/Pause, Repeat,
// and Shuffle labels change with state. A button that sizes to its text
// resizes when its label changes, reflowing every widget after it in the
// row (the qa reproduction measured a 14-15px shift when Play swapped to
// the wider Pause). Pin that every transport button requests a fixed width
// and that the widths do not depend on the play, repeat, or shuffle state,
// so a label swap cannot move the row.
#[test]
fn transport_buttons_keep_a_fixed_width_across_label_changes() {
    let stopped = transport_buttons(false, false, false);
    let playing = transport_buttons(true, false, false);
    let repeat_on = transport_buttons(false, true, false);
    let shuffle_on = transport_buttons(false, false, true);

    for (index, button) in stopped.iter().enumerate() {
        assert!(
            matches!(button.as_widget().size().width, Length::Fixed(_)),
            "transport button {index} is not fixed-width"
        );
    }
    for (a, b) in stopped.iter().zip(playing.iter()) {
        assert_eq!(a.as_widget().size().width, b.as_widget().size().width);
    }
    for (a, b) in stopped.iter().zip(repeat_on.iter()) {
        assert_eq!(a.as_widget().size().width, b.as_widget().size().width);
    }
    for (a, b) in stopped.iter().zip(shuffle_on.iter()) {
        assert_eq!(a.as_widget().size().width, b.as_widget().size().width);
    }
}

#[test]
fn play_pause_glyph_and_word_mirror_playing_state() {
    assert_eq!(play_pause_glyph(true), "II");
    assert_eq!(play_pause_glyph(false), "▶");
    assert_eq!(play_pause_word(true), "Pause");
    assert_eq!(play_pause_word(false), "Play");
}
