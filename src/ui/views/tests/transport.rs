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
// fixed-width and that its width is independent of the repeat and shuffle
// state; a zeroed, widened, or otherwise wrong width literal would clear it
// while the glyph clipped or the row moved. Pin the face widths in row order:
// Previous 34, Play 30, Pause 30, Stop 30, Next 34, Repeat 104, Shuffle 112.
// Each is the width of that button's widest face, so a deliberate glyph or
// width change must update this pin.
#[test]
fn transport_buttons_keep_their_measured_face_widths() {
    let widths: Vec<Length> = transport_buttons(false, false)
        .iter()
        .map(|button| button.as_widget().size().width)
        .collect();
    assert_eq!(
        widths,
        [
            Length::Fixed(34.0),
            Length::Fixed(30.0),
            Length::Fixed(30.0),
            Length::Fixed(30.0),
            Length::Fixed(34.0),
            Length::Fixed(104.0),
            Length::Fixed(112.0),
        ]
    );
}

// The five glyph buttons' shared spec, in the mockup's order: Previous, Play,
// Pause, Stop, Next. Pin each glyph, tooltip word, width, and message so a
// swapped face or reordered pair fails here.
#[test]
fn transport_glyphs_match_the_mockup_order_and_widths() {
    let glyphs = transport_glyphs();
    let labels: Vec<&str> = glyphs.iter().map(|(glyph, _, _, _)| *glyph).collect();
    assert_eq!(labels, ["◀◀", "▶", "II", "■", "▶▶"]);
    let words: Vec<&str> = glyphs.iter().map(|(_, word, _, _)| *word).collect();
    assert_eq!(words, ["Previous", "Play", "Pause", "Stop", "Next"]);
    let widths: Vec<f32> = glyphs.iter().map(|(_, _, width, _)| *width).collect();
    assert_eq!(widths, [34.0, 30.0, 30.0, 30.0, 34.0]);
    assert!(matches!(glyphs[0].3, Message::PreviousButton));
    assert!(matches!(glyphs[1].3, Message::Play));
    assert!(matches!(glyphs[2].3, Message::Pause));
    assert!(matches!(glyphs[3].3, Message::StopButton));
    assert!(matches!(glyphs[4].3, Message::NextButton));
}

#[test]
fn transport_controls_construct_for_volume_endpoints_and_repeat_states() {
    // `view()` passes the shared state's clamped `volume`, `balance`,
    // `repeat`, and `shuffle`, plus the player's EQ-panel and library-panel
    // visibility, straight through, so build the transport row for every
    // value the update arms can store, in every repeat, shuffle, and
    // panel-visibility state.
    for volume in [0.0, 0.5, 1.0] {
        for balance in [-1.0, 0.0, 1.0] {
            for repeat in [false, true] {
                for shuffle in [false, true] {
                    for eq_panel_visible in [false, true] {
                        for library_panel_visible in [false, true] {
                            let _controls = view_transport_controls(
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

// A diagonal roll follows its vertical sign alone, whatever the horizontal
// component says; infinite deltas keep their sign; and NaN, which compares
// false both ways, is the no-op rather than a raise or lower.
#[test]
fn volume_wheel_message_follows_only_the_vertical_sign() {
    use iced::mouse::ScrollDelta::{Lines, Pixels};
    assert_wheel_direction(volume_wheel_message(Lines { x: -9.0, y: 1.0 }), 1.0);
    assert_wheel_direction(volume_wheel_message(Pixels { x: 9.0, y: -1.0 }), -1.0);
    assert_wheel_direction(
        volume_wheel_message(Lines {
            x: 0.0,
            y: f32::INFINITY,
        }),
        1.0,
    );
    assert_wheel_direction(
        volume_wheel_message(Pixels {
            x: 0.0,
            y: f32::NEG_INFINITY,
        }),
        -1.0,
    );
    assert_wheel_direction(
        volume_wheel_message(Lines {
            x: 5.0,
            y: f32::NAN,
        }),
        0.0,
    );
}

// The balance wheel shares the volume wheel's sign mapping: both delta kinds,
// both signs, and a horizontal-only, zero, or NaN roll as the no-op.
#[test]
fn balance_wheel_message_maps_a_vertical_roll_to_a_direction() {
    use iced::mouse::ScrollDelta::{Lines, Pixels};

    fn direction(message: Message) -> f32 {
        match message {
            Message::BalanceWheel(direction) => direction,
            other => panic!("expected a balance wheel message, got {other:?}"),
        }
    }

    for (delta, expected) in [
        (Lines { x: 0.0, y: 3.0 }, 1.0),
        (Lines { x: 0.0, y: -2.0 }, -1.0),
        (Pixels { x: 0.0, y: 120.0 }, 1.0),
        (Pixels { x: 0.0, y: -45.0 }, -1.0),
        (Lines { x: 4.0, y: 0.0 }, 0.0),
        (Pixels { x: 30.0, y: 0.0 }, 0.0),
        (
            Lines {
                x: 5.0,
                y: f32::NAN,
            },
            0.0,
        ),
    ] {
        assert_eq!(direction(balance_wheel_message(delta)), expected);
    }
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

// The transport row is rebuilt every frame, and the Repeat and Shuffle labels
// change with state. A button that sizes to its text resizes when its label
// changes, reflowing every widget after it in the row. Pin that every
// transport button requests a fixed width and that the widths do not depend
// on the repeat or shuffle state, so a label swap cannot move the row.
#[test]
fn transport_buttons_keep_a_fixed_width_across_label_changes() {
    let stopped = transport_buttons(false, false);
    let repeat_on = transport_buttons(true, false);
    let shuffle_on = transport_buttons(false, true);

    for (index, button) in stopped.iter().enumerate() {
        assert!(
            matches!(button.as_widget().size().width, Length::Fixed(_)),
            "transport button {index} is not fixed-width"
        );
    }
    for (a, b) in stopped.iter().zip(repeat_on.iter()) {
        assert_eq!(a.as_widget().size().width, b.as_widget().size().width);
    }
    for (a, b) in stopped.iter().zip(shuffle_on.iter()) {
        assert_eq!(a.as_widget().size().width, b.as_widget().size().width);
    }
}

// Repeat and Shuffle sink while on, like the EQ and PL toggles: the on button draws the pressed
// face at rest and the off button the raised face.
#[test]
fn repeat_and_shuffle_toggles_sink_while_on() {
    let mut renderer = layout_capture::headless_renderer();
    let mut faces = |mut element: Element<'_, Message, iced::Theme, iced::Renderer>| {
        let (text, pixels) = layout_capture::capture_text_and_pixels(
            &mut element,
            &mut renderer,
            &iced::Theme::Dark,
        );
        // Sample the left padding strip, clear of the 1px border and the glyphs: antialiased
        // text blends the label color into the face and can land on the other face's color.
        let size = text.size();
        let region = pixels.region(Rectangle::new(
            iced::Point::new(2.0, 2.0),
            Size::new(4.0, size.height - 4.0),
        ));
        (
            region_has_color(&region, theme::BUTTON_FACE_PRESSED),
            region_has_color(&region, theme::BUTTON_FACE),
        )
    };
    for (name, on, off) in [
        ("Repeat", repeat_toggle(true), repeat_toggle(false)),
        ("Shuffle", shuffle_toggle(true), shuffle_toggle(false)),
    ] {
        assert_eq!(faces(on.into()), (true, false), "{name} on must sink");
        assert_eq!(
            faces(off.into()),
            (false, true),
            "{name} off must stay raised"
        );
    }
}
