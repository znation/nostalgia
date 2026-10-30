use super::*;

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
// Previous 34, Play 30, Pause 30, Stop 30, Next 34, Repeat 44, Shuffle 92.
// Each is the width of that button's widest face — for Repeat and Shuffle
// that face now carries the LED square and the short engraved label (set
// when the toggles traded their `On/Off` text for the base skin's LED
// squares) — so a deliberate glyph or width change must update this pin.
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
            Length::Fixed(44.0),
            Length::Fixed(92.0),
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

// The Repeat and Shuffle toggles do not sink at rest in either state: the
// base skin marks their on/off state on the square LED each face carries,
// and its chrome stays raised whether the flag is on or off — the groove
// sinks only while the mouse holds the button down. Sample each whole face
// through the pixel harness: at rest the face must read the raised metal in
// both states, never the pressed face.
#[test]
fn repeat_and_shuffle_toggles_stay_raised_in_either_state() {
    let raised_face = |mut element: Element<'_, Message, iced::Theme, iced::Renderer>| {
        let mut renderer = layout_capture::headless_renderer();
        let (text, pixels) = layout_capture::capture_text_and_pixels(
            &mut element,
            &mut renderer,
            &iced::Theme::Dark,
        );
        let size = text.size();
        let region = pixels.region(Rectangle::new(iced::Point::new(0.0, 0.0), size));
        (
            region_has_color(&region, theme::BUTTON_FACE),
            region_has_color(&region, theme::BUTTON_FACE_PRESSED),
        )
    };
    assert_eq!(
        raised_face(repeat_toggle(true).into()),
        (true, false),
        "Repeat on must stay raised"
    );
    assert_eq!(
        raised_face(repeat_toggle(false).into()),
        (true, false),
        "Repeat off must stay raised"
    );
    assert_eq!(
        raised_face(shuffle_toggle(true).into()),
        (true, false),
        "Shuffle on must stay raised"
    );
    assert_eq!(
        raised_face(shuffle_toggle(false).into()),
        (true, false),
        "Shuffle off must stay raised"
    );
}

// The base skin marks Repeat and Shuffle with a small square LED on each
// button face: `docs/screenshots/transport.png` reads (0,232,0) inside the
// lit repeat LED in its Repeat-on panel, and `docs/screenshots/player.png`
// reads (16,20,15) inside the resting Shuffle button's LED, so the state
// lives in the LED, not the label or the chrome. Sample both toggles through
// the pixel harness: the on button must paint the lit square and the off
// button the dark one, and neither may show the other's LED color. The two
// colors are written inline so this test exercises the pixels themselves
// rather than constants that could drift with the style code.
#[test]
fn repeat_and_shuffle_paint_their_led_square_with_their_state() {
    let led_on = iced::Color::from_rgb8(0, 232, 0);
    let led_off = iced::Color::from_rgb8(16, 20, 15);
    let leds = |mut element: Element<'_, Message, iced::Theme, iced::Renderer>| {
        let mut renderer = layout_capture::headless_renderer();
        let (text, pixels) = layout_capture::capture_text_and_pixels(
            &mut element,
            &mut renderer,
            &iced::Theme::Dark,
        );
        let size = text.size();
        pixels.region(Rectangle::new(iced::Point::new(0.0, 0.0), size))
    };
    assert!(
        region_has_color(&leds(repeat_toggle(true).into()), led_on),
        "the Repeat toggle must paint its LED square lit while Repeat is on"
    );
    assert!(
        region_has_color(&leds(repeat_toggle(false).into()), led_off),
        "the Repeat toggle must paint its LED square dark while Repeat is off"
    );
    assert!(
        !region_has_color(&leds(repeat_toggle(false).into()), led_on),
        "the Repeat toggle's LED square must not read lit while Repeat is off"
    );
    assert!(
        region_has_color(&leds(shuffle_toggle(true).into()), led_on),
        "the Shuffle toggle must paint its LED square lit while Shuffle is on"
    );
    assert!(
        region_has_color(&leds(shuffle_toggle(false).into()), led_off),
        "the Shuffle toggle must paint its LED square dark while Shuffle is off"
    );
    assert!(
        !region_has_color(&leds(shuffle_toggle(false).into()), led_on),
        "the Shuffle toggle's LED square must not read lit while Shuffle is off"
    );
}

// The base skin's transport buttons are light metal with the glyph engraved dark:
// `winamp-2.x-stack.png` reads (189,198,198)/(173,181,181) faces against near-black
// glyphs, and the mockups agree — `docs/screenshots/player.png` reads (185,196,209)
// against (57,57,90). Sample the resting face through the pixel harness: it must read
// that light metal blue-gray, not the dark gray tile the old palette produced.
#[test]
fn transport_button_face_renders_light_metal() {
    let mut renderer = layout_capture::headless_renderer();
    let mut element: Element<'_, Message, iced::Theme, iced::Renderer> =
        repeat_toggle(false).into();
    let (text, pixels) =
        layout_capture::capture_text_and_pixels(&mut element, &mut renderer, &iced::Theme::Dark);
    // The left padding strip, clear of the 1px border and the glyphs: antialiased
    // text blends the label color into the face and can land on the other face's
    // color (see the sink test above).
    let size = text.size();
    let face = Rectangle::new(
        iced::Point::new(2.0, 2.0),
        Size::new(4.0, size.height - 4.0),
    );
    assert!(
        pixels.max_red_in(face) >= 150,
        "the transport button's resting face must read the light metal blue-gray, not a dark gray tile"
    );
}

// The Repeat face is icon-only, so the pulse of the value is the loop glyph:
// a font missing from the headless set measures no advance (`⟳` measured 0.0
// and painted nothing) and would render an empty face between the LED square
// and the pressed chrome. Measure the laid-out label: it must have a real
// advance and actually paint inside its bounds.
#[test]
fn repeat_face_glyph_measures_and_paints_the_loop_icon() {
    let mut renderer = layout_capture::headless_renderer();
    let mut element: Element<'_, Message, iced::Theme, iced::Renderer> =
        repeat_toggle(false).into();
    let capture = layout_capture::capture_text(&mut element, &renderer);
    let bounds = capture.find("↻");
    assert!(
        bounds.width >= 6.0,
        "the repeat loop glyph must lay out with a real advance, got width {}",
        bounds.width
    );
    let (_text, pixels) =
        layout_capture::capture_text_and_pixels(&mut element, &mut renderer, &iced::Theme::Dark);
    let size = capture.size();
    let region = pixels.region(iced::Rectangle::new(iced::Point::new(0.0, 0.0), size));
    let face = [
        theme::BUTTON_FACE.r as u8,
        theme::BUTTON_FACE.g as u8,
        theme::BUTTON_FACE.b as u8,
        255,
    ];
    assert!(
        region.len() >= 4,
        "the repeat face must lay out to a measurable area"
    );
    // The glyph bounds sit at the face's center, clear of the bevels, so
    // pixels that are neither the metal face nor the LED square nor the bevel
    // colors are glyph ink (including antialiased blends).
    let bevel = [
        theme::BUTTON_EDGE_LIGHT.r as u8,
        theme::BUTTON_EDGE_LIGHT.g as u8,
        theme::BUTTON_EDGE_LIGHT.b as u8,
        255,
    ];
    let led = [
        theme::LED_OFF.r as u8,
        theme::LED_OFF.g as u8,
        theme::LED_OFF.b as u8,
        255,
    ];
    let press = [
        theme::BUTTON_FACE_PRESSED.r as u8,
        theme::BUTTON_FACE_PRESSED.g as u8,
        theme::BUTTON_FACE_PRESSED.b as u8,
        255,
    ];
    let count = region
        .chunks_exact(4)
        .filter(|p| **p != face && **p != bevel && **p != led && **p != press)
        .count();
    assert!(
        count >= 4,
        "the repeat loop glyph must paint inside its bounds, {count} ink pixels of {}",
        region.len() / 4
    );
}

// The pinned face width is only honest if the engraved label beside the LED
// square actually fits inside it: a label wider than the pin minus the
// padding, the LED square, and the gap wraps or clips, and the whole point
// of the pin is that the face cannot reflow the row. Measure the laid-out
// label: it must stay on one line inside the budget.
#[test]
fn shuffle_led_toggle_label_fits_its_pinned_face() {
    let renderer = layout_capture::headless_renderer();
    let mut element: Element<'_, Message, iced::Theme, iced::Renderer> =
        shuffle_toggle(false).into();
    let capture = layout_capture::capture_text(&mut element, &renderer);
    let bounds = capture.find("SHUFFLE");
    // The budget is the [`SHUFFLE_WIDTH`] pin minus the button's default
    // 10px side padding and the LED square and its gap.
    assert!(
        bounds.width <= 58.0,
        "the SHUFFLE label must fit the pinned face, got width {}",
        bounds.width
    );
    assert!(
        bounds.height < 30.0,
        "the SHUFFLE label must stay on one line, got height {}",
        bounds.height
    );
}

// Part 2/4 of the pixel-font split gave the row's five glyph buttons the
// vendored Tiny5 shape face and their hover tooltips' words the Silkscreen
// letterform face at the on-grid 16px word size (12 left the system sans).
// A fixed-width button reports its content box rather than the text's own
// advance, so the faces are pinned through the same widgets shrunk to them:
// each probe carries exactly the `Text` the row's builders hand iced. The
// widths are the probed Tiny5/Silkscreen advances part 1/4 recorded (▶ 8.0
// and ■ 10.0 at 16; the glyph button colors were pinned by the part 1/4 seam
// test), plus the engraved `SHUFFLE`	Run at 76.0; a lost registration or a
// swapped family moves them.
#[test]
fn transport_glyph_buttons_draw_the_vendored_shape_face() {
    let renderer = layout_capture::headless_renderer();
    let spec = transport_glyphs().map(|(glyph, _, _, message)| (glyph, message));
    let widths = spec
        .into_iter()
        .map(|(glyph, message)| {
            let mut button: iced::Element<'_, Message, iced::Theme, iced::Renderer> =
                iced::widget::Button::new(iced::widget::Text::new(glyph).font(theme::shape_font()))
                    .on_press(message)
                    .into();
            let capture = layout_capture::capture_text(&mut button, &renderer);
            let fragment = capture.find(glyph);
            fragment.width
        })
        .collect::<Vec<_>>();
    assert_eq!(widths, [16.0, 8.0, 8.0, 10.0, 16.0]);
}

#[test]
fn transport_tooltip_words_and_shuffle_label_draw_the_silkscreen_face() {
    let renderer = layout_capture::headless_renderer();
    let words = transport_glyphs().map(|(_, word, _, _)| word.to_string());
    let mut labels: Vec<&str> = words.iter().map(String::as_str).collect();
    labels.push("SHUFFLE");
    let widths: Vec<f32> = labels
        .iter()
        .map(|word| {
            let mut element: iced::Element<'_, (), iced::Theme, iced::Renderer> =
                iced::widget::Text::new(*word)
                    .size(16.0)
                    .font(theme::ui_font())
                    .into();
            let capture = layout_capture::capture_text(&mut element, &renderer);
            let fragment = capture.find(word);
            fragment.width
        })
        .collect::<Vec<_>>();
    // The bounds surface f32 rounding on the way out of the layout replay, so
    // the pins compare each width within a hundredth of a pixel.
    let expected = [90.0, 48.0, 58.0, 46.0, 48.0, 76.0];
    for ((word, measured), face) in labels.iter().zip(widths.iter()).zip(expected) {
        assert!(
            (*measured - face).abs() < 0.01,
            "the tooltip words and Shuffle label must draw in the vendored \
             Silkscreen face: {word} measured {measured}, Silkscreen pins {face}"
        );
    }
}
