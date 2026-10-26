use super::*;

// The slider specs are literals iced exposes no way to read back, so they
// are named constants the builders consume and these tests pin. The
// important one is the tie to the shared-state clamp: each slider's range
// must equal the range its state clamp stores. A slider bound the clamp
// moves means the thumb cannot reach the stored extreme; a bound the clamp
// accepts but the slider cannot reach leaves part of the scale unusable.

// A left press on a dB scale label must publish the set-all message with that
// label's gain, and nothing else.
#[test]
fn pressing_a_db_scale_label_sets_every_band_to_its_gain() {
    for (label, gain) in [
        ("+12 DB", GAIN_MAX_DB),
        ("0 DB", 0.0),
        ("-12 DB", GAIN_MIN_DB),
    ] {
        let mut renderer = layout_capture::headless_renderer();
        let mut element = view_equalizer(true, 0.0, &[0.0; BAND_COUNT], None);
        let capture = layout_capture::capture_text(&mut element, &renderer);
        let text = capture.find(label);
        let messages = layout_capture::publish_at(
            element,
            &mut renderer,
            &[iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
                iced::mouse::Button::Left,
            ))],
            iced::Point::new(text.center_x(), text.center_y()),
        );
        let sent: Vec<f32> = messages
            .iter()
            .filter_map(|message| match message {
                Message::EqBandsSetAll(g) => Some(*g),
                _ => None,
            })
            .collect();
        assert_eq!(sent, vec![gain], "a press on {label}: {messages:?}");
    }
}

// The equalizer's wheel messages carry the same vertical sign as the volume
// wheel and name the slider they came from: the preamp, or the exact band
// index, including a no-op zero roll.
#[test]
fn equalizer_wheel_messages_map_the_roll_direction_and_band() {
    use iced::mouse::ScrollDelta::{Lines, Pixels};

    for (delta, expected) in [
        (Lines { x: 0.0, y: 2.0 }, 1.0),
        (Pixels { x: 0.0, y: -90.0 }, -1.0),
        (Lines { x: 7.0, y: 0.0 }, 0.0),
        (
            Pixels {
                x: 0.0,
                y: f32::NAN,
            },
            0.0,
        ),
    ] {
        match eq_preamp_wheel_message(delta) {
            Message::EqPreampWheel(direction) => assert_eq!(direction, expected),
            other => panic!("expected a preamp wheel message, got {other:?}"),
        }
        match eq_band_wheel_message(3, delta) {
            Message::EqBandWheel(index, direction) => {
                assert_eq!(index, 3);
                assert_eq!(direction, expected);
            }
            other => panic!("expected a band wheel message, got {other:?}"),
        }
    }
}

// The mockup draws the preamp as a vertical slider in the band row with a dB
// scale beside it, not as a horizontal slider above the curve. Lay out the
// real panel and pin the scale's top-to-bottom order, the preamp label's
// baseline sharing the band labels' row, and the scale sitting left of the
// first band.
#[test]
fn equalizer_preamp_and_db_scale_sit_left_of_the_band_sliders() {
    let bands = [0.0; BAND_COUNT];
    let mut element = view_equalizer(false, 0.0, &bands, None);
    let capture = layout_capture::capture_text(&mut element, &layout_capture::headless_renderer());

    let plus_twelve = capture.find("+12 DB");
    let zero = capture.find("0 DB");
    let minus_twelve = capture.find("-12 DB");
    assert!(
        plus_twelve.y < zero.y && zero.y < minus_twelve.y,
        "the dB scale reads top to bottom: +12 {:?}, 0 {:?}, -12 {:?}",
        plus_twelve.y,
        zero.y,
        minus_twelve.y
    );
    assert!(
        minus_twelve.y - plus_twelve.y <= EQ_SLIDER_HEIGHT,
        "the dB scale fits one slider's height: +12 {:?}, -12 {:?}",
        plus_twelve.y,
        minus_twelve.y
    );

    let preamp = capture.find("Preamp");
    let first_frequency = capture.find(BAND_FREQUENCIES[0]);
    assert!(
        (preamp.y - first_frequency.y).abs() <= 0.5,
        "the preamp label sits in the band row: {preamp:?} vs {first_frequency:?}"
    );

    for scale in [plus_twelve, zero, minus_twelve] {
        assert!(
            scale.x + scale.width <= first_frequency.x,
            "the dB scale sits left of the first band: {scale:?} vs {first_frequency:?}"
        );
    }

    // The curve now shares the band row's left edge instead of starting over
    // the preamp and scale, so its first column must sit on the first band.
    let mut tree = Tree::new(element.as_widget());
    let renderer = layout_capture::headless_renderer();
    let node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &layout_capture::limits());
    let mut containers = CapturedContainers::default();
    element
        .as_widget_mut()
        .operate(&mut tree, Layout::new(&node), &renderer, &mut containers);
    let first_segment = containers
        .0
        .iter()
        .filter(|bounds| bounds.height == RESPONSE_CURVE_LINE_HEIGHT)
        .min_by(|left, right| left.x.total_cmp(&right.x))
        .copied()
        .expect("the curve draws a segment");
    assert!(
        (first_segment.x - first_frequency.x).abs() <= 0.5,
        "the curve starts at the first band: {first_segment:?} vs {first_frequency:?}"
    );
}

// The ten band labels are the equalizer's only frequency readout: when a
// pair touches, two frequencies read as one number (`1703106001K`). This was
// confirmed only from a captured window, so lay the panel out with the
// headless tiny-skia renderer — which measures text for real — and require a
// visible gap between every pair of neighboring labels.
#[test]
fn equalizer_band_labels_leave_a_gap_between_frequencies() {
    let bands = [0.0; BAND_COUNT];
    let mut element = view_equalizer(false, 0.0, &bands, None);
    let capture = layout_capture::capture_text(&mut element, &layout_capture::headless_renderer());

    let bounds: Vec<(&str, Rectangle)> = BAND_FREQUENCIES
        .iter()
        .map(|frequency| (*frequency, capture.find(frequency)))
        .collect();
    for pair in bounds.windows(2) {
        let (left_label, left) = pair[0];
        let (right_label, right) = pair[1];
        assert!(
            left.x + left.width + 1.0 <= right.x,
            "{left_label} and {right_label} labels touch: {:?}",
            capture.fragments()
        );
    }
}

// The response curve is the panel's only readout of the whole stored shape,
// so its pure offset helper must trace the same gains the sliders show: flat
// in the middle, the extremes at the plot's edges, and linear between the ten
// bands. Pin the exact values, the endpoint correspondence, and the two
// degenerate column counts.
#[test]
fn response_curve_offsets_follow_the_band_gains() {
    assert!(
        response_curve_offsets(&[0.0; BAND_COUNT], 0).is_empty(),
        "no columns means no segments"
    );
    assert_eq!(
        response_curve_offsets(&[0.0; BAND_COUNT], 1),
        vec![0.5],
        "one column sits on the first band instead of dividing by zero"
    );

    let flat = response_curve_offsets(&[0.0; BAND_COUNT], RESPONSE_CURVE_COLUMNS);
    assert_eq!(flat.len(), RESPONSE_CURVE_COLUMNS);
    assert!(
        flat.iter().all(|offset| *offset == 0.5),
        "a flat curve is centered: {flat:?}"
    );
    assert!(
        response_curve_offsets(&[GAIN_MIN_DB; BAND_COUNT], RESPONSE_CURVE_COLUMNS)
            .iter()
            .all(|offset| *offset == 0.0),
        "all-min gains sit on the plot's bottom edge"
    );
    assert!(
        response_curve_offsets(&[GAIN_MAX_DB; BAND_COUNT], RESPONSE_CURVE_COLUMNS)
            .iter()
            .all(|offset| *offset == 1.0),
        "all-max gains sit on the plot's top edge"
    );

    let mut ramp = [0.0; BAND_COUNT];
    ramp[0] = GAIN_MIN_DB;
    ramp[BAND_COUNT - 1] = GAIN_MAX_DB;
    let endpoints = response_curve_offsets(&ramp, RESPONSE_CURVE_COLUMNS);
    assert_eq!(
        endpoints.first().copied(),
        Some((clamp_gain(GAIN_MIN_DB) - GAIN_MIN_DB) / (GAIN_MAX_DB - GAIN_MIN_DB))
    );
    assert_eq!(
        endpoints.last().copied(),
        Some((clamp_gain(GAIN_MAX_DB) - GAIN_MIN_DB) / (GAIN_MAX_DB - GAIN_MIN_DB))
    );

    let three = response_curve_offsets(&ramp, 3);
    assert_eq!(
        three,
        vec![0.0, 0.5, 1.0],
        "columns interpolate between bands"
    );
}

// Lay the curve row out under the null renderer: it must keep its fixed
// height for any curve and draw exactly one green segment per column, first
// column at the bottom and last at the top for a rising curve. The zero-offset
// first column is the one iced would silently drop if it were `Fixed(0.0)`.
#[test]
fn response_curve_row_keeps_its_height_and_draws_one_segment_per_column() {
    let mut bands = [0.0; BAND_COUNT];
    bands[0] = GAIN_MIN_DB;
    bands[BAND_COUNT - 1] = GAIN_MAX_DB;
    let mut element: iced::Element<'_, Message, iced::Theme, ()> = response_curve_row(&bands);
    let mut tree = Tree::new(element.as_widget());
    let node = element
        .as_widget_mut()
        .layout(&mut tree, &(), &layout_capture::limits());
    assert_eq!(node.bounds().height, RESPONSE_CURVE_HEIGHT);

    let mut captured = CapturedContainers::default();
    element
        .as_widget_mut()
        .operate(&mut tree, Layout::new(&node), &(), &mut captured);
    let segments: Vec<Rectangle> = captured
        .0
        .iter()
        .filter(|bounds| bounds.height == RESPONSE_CURVE_LINE_HEIGHT)
        .copied()
        .collect();
    assert_eq!(
        segments.len(),
        RESPONSE_CURVE_COLUMNS,
        "one segment per column, including the zero-offset first column"
    );
    let first = segments[0];
    let last = segments[segments.len() - 1];
    assert_eq!(
        first.y + first.height,
        RESPONSE_CURVE_HEIGHT,
        "the first band's minimum gain draws on the bottom edge: {first:?}"
    );
    assert_eq!(
        last.y, 0.0,
        "the last band's maximum gain draws on the top edge: {last:?}"
    );

    // The mockup's flat preset: every segment stays on the plot's midline.
    let mut flat_element: iced::Element<'_, Message, iced::Theme, ()> =
        response_curve_row(&[0.0; BAND_COUNT]);
    let mut flat_tree = Tree::new(flat_element.as_widget());
    let flat_node =
        flat_element
            .as_widget_mut()
            .layout(&mut flat_tree, &(), &layout_capture::limits());
    let mut flat_captured = CapturedContainers::default();
    flat_element.as_widget_mut().operate(
        &mut flat_tree,
        Layout::new(&flat_node),
        &(),
        &mut flat_captured,
    );
    let flat_segments: Vec<Rectangle> = flat_captured
        .0
        .iter()
        .filter(|bounds| bounds.height == RESPONSE_CURVE_LINE_HEIGHT)
        .copied()
        .collect();
    assert_eq!(flat_segments.len(), RESPONSE_CURVE_COLUMNS);
    let midline = (RESPONSE_CURVE_HEIGHT - RESPONSE_CURVE_LINE_HEIGHT) / 2.0;
    assert!(
        flat_segments.iter().all(|segment| segment.y == midline),
        "a flat curve draws a centered line: {flat_segments:?}"
    );
}

#[test]
fn preamp_slider_style_matches_the_band_tint_for_its_gain() {
    use iced::Background;
    use iced::widget::slider;
    for gain in [-12.0, -6.0, 0.0, 6.0, 12.0, 20.0, f32::NAN] {
        for status in [
            slider::Status::Active,
            slider::Status::Hovered,
            slider::Status::Dragged,
        ] {
            let preamp = preamp_slider_style(gain, status);
            let band = style::band_slider_style(gain, status);
            assert_eq!(preamp.rail.backgrounds, band.rail.backgrounds);
            assert_eq!(preamp.rail.width, band.rail.width);
            assert_eq!(preamp.rail.border, band.rail.border);
            assert_eq!(preamp.handle, band.handle);
        }
    }
    let low = preamp_slider_style(-12.0, slider::Status::Active);
    assert_eq!(
        low.rail.backgrounds.0,
        Background::Color(style::band_gain_color(-12.0))
    );
    assert_ne!(
        low.rail.backgrounds.0,
        style::chrome_slider_style(slider::Status::Active)
            .rail
            .backgrounds
            .0
    );
}

#[test]
fn eq_enabled_label_mirrors_enabled_state() {
    assert_eq!(eq_enabled_label(true), "EQ: On");
    assert_eq!(eq_enabled_label(false), "EQ: Off");
}

// Classic Winamp names a hand-tuned curve `Custom` rather than showing an
// empty pick list. Lay out the real panel for both states and compare its
// closed pick list pixel for pixel with the same header row rendered for the
// expected label: the helper alone cannot prove the panel uses it, and iced
// paints the closed label straight through the renderer rather than as a text
// operation.
#[test]
fn equalizer_pick_list_shows_custom_for_a_hand_tuned_curve() {
    assert_eq!(preset_pick_list_label(None), "Custom");
    assert_eq!(preset_pick_list_label(None), CUSTOM_PRESET_LABEL);
    assert_eq!(preset_pick_list_label(Some(PRESETS[0])), PRESETS[0].name);
    assert_ne!(
        preset_pick_list_label(Some(PRESETS[0])),
        CUSTOM_PRESET_LABEL
    );

    for preset in [None, Some(PRESETS[0])] {
        let mut renderer = layout_capture::headless_renderer();
        let mut element = view_equalizer(false, 0.0, &[0.0; BAND_COUNT], preset);
        let (capture, pixels) = layout_capture::capture_text_and_pixels(
            &mut element,
            &mut renderer,
            &iced::Theme::Dark,
        );
        let (expected, expected_pixels) =
            reference_header_row(preset, preset_pick_list_label(preset));
        let button = capture.find("EQ: Off");
        let expected_button = expected.find("EQ: Off");
        // Skip the panel's top bevel row and the row's fractional bottom
        // edge, then compare the rest of the header row, including the closed
        // pick list, pixel for pixel.
        let region = iced::Rectangle {
            x: expected_button.x,
            y: 1.0,
            width: expected.size().width.floor() - expected_button.x,
            height: expected.size().height.floor() - 1.0,
        };
        let panel_region = iced::Rectangle {
            x: region.x + button.x - expected_button.x,
            y: region.y + button.y - expected_button.y,
            ..region
        };
        assert_eq!(
            pixels.region(panel_region),
            expected_pixels.region(region),
            "the panel must draw {preset:?}'s closed pick-list label"
        );

        if preset.is_none() {
            let (_, none_pixels) = reference_header_row(preset, "(none)");
            assert_ne!(
                pixels.region(panel_region),
                none_pixels.region(region),
                "the hand-tuned curve must not draw the old empty-list placeholder"
            );
        }
    }
}

/// Renders the header row `view_equalizer` builds — the EQ button, its spacer,
/// and the closed preset pick list — so its pixels can be compared with the
/// real panel's row for a given placeholder.
fn reference_header_row(
    selected: Option<crate::equalizer::Preset>,
    placeholder: &str,
) -> (layout_capture::TextCapture, layout_capture::PixelCapture) {
    let mut element: iced::Element<'static, Message> = iced::widget::Row::new()
        .push(labeled_button(
            eq_enabled_label(false),
            Message::ToggleEqualizer,
        ))
        .push(spacer(8.0))
        .push(
            iced::widget::PickList::new(PRESETS.as_slice(), selected, Message::EqPresetSelected)
                .placeholder(placeholder)
                .text_size(12)
                .style(|_theme, status| style::chrome_pick_list_style(status))
                .menu_style(|_theme| style::preset_menu_style()),
        )
        .into();
    let mut renderer = layout_capture::headless_renderer();
    layout_capture::capture_text_and_pixels(&mut element, &mut renderer, &iced::Theme::Dark)
}

#[test]
fn equalizer_panel_constructs_for_both_states_and_gain_endpoints() {
    // `view()` passes the shared EQ flag, preamp, and band array straight
    // through, so build the panel for both on/off states and every gain
    // the update arms can store: the range endpoints and flat. iced
    // `Element`s expose no tree introspection, so the observable contract
    // is that the builder constructs its ten-slider tree without
    // panicking over the input space the app produces.
    for enabled in [false, true] {
        for gain in [GAIN_MIN_DB, 0.0, GAIN_MAX_DB] {
            for preset in [None, Some(PRESETS[0])] {
                let bands = [gain; BAND_COUNT];
                let _panel = view_equalizer(enabled, gain, &bands, preset);
            }
        }
    }
}

// The equalizer twin: every preamp and band slider spans
// `GAIN_MIN_DB..=GAIN_MAX_DB` and must equal the range `clamp_gain`
// stores, so a drag to either end lands exactly on the stored extreme.
#[test]
fn equalizer_slider_spec_matches_the_gain_clamp_and_steps_whole_decibels() {
    assert_slider_range_matches_clamp(clamp_gain, GAIN_MIN_DB, GAIN_MAX_DB, EQ_STEP);

    assert_eq!(EQ_STEP, 1.0);
}
