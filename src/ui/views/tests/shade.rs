use super::*;

// The main row and both compact strips must share one spec: same glyphs in the
// same order, same fixed widths, same messages. Pin the compact projection
// against `transport_glyphs`.
#[test]
fn main_transport_and_shade_strip_share_glyphs_order_and_widths() {
    let full = transport_glyphs();
    let shade = mini_transport_controls();
    let full_spec: Vec<(&str, f32)> = full
        .iter()
        .map(|(glyph, _, width, _)| (*glyph, *width))
        .collect();
    let shade_spec: Vec<(&str, f32)> = shade
        .iter()
        .map(|(glyph, width, _)| (*glyph, *width))
        .collect();
    assert_eq!(full_spec, shade_spec);
    for ((_, _, _, full_message), (_, _, shade_message)) in full.iter().zip(shade.iter()) {
        assert_eq!(
            std::mem::discriminant(full_message),
            std::mem::discriminant(shade_message)
        );
    }
}

// The five compact buttons must emit the same messages as their full
// transport-row twins, and each keeps a fixed width so the row cannot reflow.
// Pin both, in row order.
#[test]
fn shade_controls_map_to_transport_messages_and_keep_fixed_widths() {
    let controls = mini_transport_controls();

    let labels: Vec<&str> = controls.iter().map(|(label, _, _)| *label).collect();
    assert_eq!(labels, vec!["◀◀", "▶", "II", "■", "▶▶"]);
    let widths: Vec<f32> = controls.iter().map(|(_, width, _)| *width).collect();
    assert_eq!(widths, vec![34.0, 30.0, 30.0, 30.0, 34.0]);

    let messages: Vec<&Message> = controls.iter().map(|(_, _, message)| message).collect();
    assert!(matches!(messages[0], Message::PreviousTrack));
    assert!(matches!(messages[1], Message::Play));
    assert!(matches!(messages[2], Message::Pause));
    assert!(matches!(messages[3], Message::Stop));
    assert!(matches!(messages[4], Message::NextTrack));
}

// The rolled-up window is exactly the title bar plus the strip; both the
// view and `resize_to_shade` read `SHADED_HEIGHT`, so pin its composition and
// its positive, fixed size.
#[test]
fn shade_height_pins_the_rolled_up_window() {
    assert_eq!(SHADE_STRIP_HEIGHT, 28.0);
    assert_eq!(SHADED_HEIGHT, super::TITLE_BAR_HEIGHT + SHADE_STRIP_HEIGHT);
}

// Laying the strip out with the headless renderer resolves its fixed height
// for both playback states, so a browser or layout change cannot shrink the
// rolled-up window to an empty band.
#[test]
fn view_shade_strip_keeps_its_fixed_height_for_both_playback_states() {
    let renderer = layout_capture::headless_renderer();

    for mode in [
        TransportMode::Stopped,
        TransportMode::Paused,
        TransportMode::Playing,
    ] {
        let mut element = view_shade_strip(
            mode,
            0.5,
            Cow::Borrowed("00:53"),
            Cow::Borrowed("3:30"),
            None,
            None,
        );
        let mut tree = Tree::new(element.as_widget());
        let limits = Limits::new(Size::ZERO, Size::new(1000.0, 1000.0));
        let node = element
            .as_widget_mut()
            .layout(&mut tree, &renderer, &limits);
        assert_eq!(
            node.size().height,
            SHADE_STRIP_HEIGHT,
            "the strip must keep its fixed height in {mode:?} mode"
        );
    }
}

// The rolled-up window restores like the classic Winamp shade: a double-click
// anywhere on the strip's blank chrome publishes one `ToggleWindowShade`, while
// a single click stays inert. The center point lands in the fill spacer between
// the readout and the right-edge volume slider, so the press does not hit a
// control.
#[test]
fn shade_strip_double_click_restores_the_window() {
    let mut renderer = layout_capture::headless_renderer();
    let mut element = view_shade_strip(
        TransportMode::Stopped,
        0.5,
        Cow::Borrowed("00:53"),
        Cow::Borrowed("3:30"),
        None,
        None,
    );
    let capture = layout_capture::capture_text(&mut element, &renderer);
    let center = iced::Point::new(capture.size().width / 2.0, capture.size().height / 2.0);
    let single_click = [
        iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
        iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
            iced::mouse::Button::Left,
        )),
    ];
    let messages = layout_capture::publish_at(element, &mut renderer, &single_click, center);
    assert!(
        !messages
            .iter()
            .any(|message| matches!(message, Message::ToggleWindowShade)),
        "a single click on the strip must not unshade: {messages:?}"
    );

    let double_click = [
        iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
        iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
            iced::mouse::Button::Left,
        )),
        iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
        iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
            iced::mouse::Button::Left,
        )),
    ];
    let messages = layout_capture::publish_at(
        view_shade_strip(
            TransportMode::Stopped,
            0.5,
            Cow::Borrowed("00:53"),
            Cow::Borrowed("3:30"),
            None,
            None,
        ),
        &mut renderer,
        &double_click,
        center,
    );
    let toggles = messages
        .iter()
        .filter(|message| matches!(message, Message::ToggleWindowShade))
        .count();
    assert_eq!(
        toggles, 1,
        "a double-click on the strip must publish exactly one ToggleWindowShade: {messages:?}"
    );
}

// The strip's own controls keep the presses that a double-click would
// otherwise turn into a restore: the stopped readout, whose time toggle is
// inert, the Play button, and both sliders. Each double-click publishes its
// control's message and none publishes `ToggleWindowShade`.
#[test]
fn shade_strip_double_clicks_on_controls_do_not_restore_the_window() {
    #[derive(Default)]
    struct CapturedControls {
        seek: Option<Rectangle>,
        volume: Option<Rectangle>,
    }
    impl Operation for CapturedControls {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }
        fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
            if id == Some(&Id::new(MINI_SEEK_ID)) {
                self.seek = Some(bounds);
            } else if id == Some(&Id::new(MINI_VOLUME_ID)) {
                self.volume = Some(bounds);
            }
        }
    }

    let double_click = [
        iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
        iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
            iced::mouse::Button::Left,
        )),
        iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
        iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
            iced::mouse::Button::Left,
        )),
    ];

    let strip = || {
        view_shade_strip(
            TransportMode::Stopped,
            0.5,
            Cow::Borrowed("00:53"),
            Cow::Borrowed("3:30"),
            Some(Duration::from_secs(10)),
            Some(Duration::from_secs(30)),
        )
    };
    let mut renderer = layout_capture::headless_renderer();
    let mut element = strip();
    let capture = layout_capture::capture_text(&mut element, &renderer);
    let readout = capture.find("00:53/3:30");
    let play = capture.find("▶");
    let mut tree = Tree::new(element.as_widget());
    let limits = Limits::new(Size::ZERO, Size::new(1000.0, 1000.0));
    let node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &limits);
    let mut controls = CapturedControls::default();
    element
        .as_widget_mut()
        .operate(&mut tree, Layout::new(&node), &renderer, &mut controls);
    let seek = controls.seek.expect("the seek slider is drawn");
    let volume = controls.volume.expect("the volume slider is drawn");

    for (control, point) in [
        (
            "stopped readout",
            iced::Point::new(readout.center_x(), readout.center_y()),
        ),
        (
            "play button",
            iced::Point::new(play.center_x(), play.center_y()),
        ),
        (
            "seek slider",
            iced::Point::new(seek.x + seek.width * 0.6, seek.center_y()),
        ),
        (
            "volume slider",
            iced::Point::new(volume.x + volume.width * 0.25, volume.center_y()),
        ),
    ] {
        let messages = layout_capture::publish_at(strip(), &mut renderer, &double_click, point);
        assert!(
            !messages
                .iter()
                .any(|message| matches!(message, Message::ToggleWindowShade)),
            "a double-click on the {control} must not unshade: {messages:?}"
        );
        let keeps_its_behavior = match control {
            "stopped readout" => messages
                .iter()
                .all(|message| matches!(message, Message::Ignored)),
            "play button" => messages
                .iter()
                .any(|message| matches!(message, Message::Play)),
            "seek slider" => messages
                .iter()
                .any(|message| matches!(message, Message::SeekPreviewChanged(_))),
            "volume slider" => messages
                .iter()
                .any(|message| matches!(message, Message::VolumeChange(_))),
            _ => unreachable!(),
        };
        assert!(
            keeps_its_behavior,
            "the {control} must keep its own press behavior: {messages:?}"
        );
    }
}

// The rolled-up strip's seek slider is the Now Playing bar's own control: it
// appears only when the backend reports both a position and a non-zero
// preview length, sits between the readout and the right-edge volume slider,
// and blanks without disturbing the row's fill layout otherwise.
#[test]
fn shade_strip_shows_a_seek_slider_only_when_position_and_length_are_known() {
    #[derive(Default)]
    struct CapturedControls {
        seek: Option<Rectangle>,
        volume: Option<Rectangle>,
    }
    impl Operation for CapturedControls {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }
        fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
            if id == Some(&Id::new(MINI_SEEK_ID)) {
                self.seek = Some(bounds);
            } else if id == Some(&Id::new(MINI_VOLUME_ID)) {
                self.volume = Some(bounds);
            }
        }
    }

    for (position, duration, expected) in [
        (
            Some(Duration::from_secs(10)),
            Some(Duration::from_secs(30)),
            true,
        ),
        (None, Some(Duration::from_secs(30)), false),
        (Some(Duration::from_secs(10)), None, false),
        (Some(Duration::from_secs(10)), Some(Duration::ZERO), false),
    ] {
        let renderer = layout_capture::headless_renderer();
        let mut element = view_shade_strip(
            TransportMode::Stopped,
            0.5,
            Cow::Borrowed("00:53"),
            Cow::Borrowed("3:30"),
            position,
            duration,
        );
        let capture = layout_capture::capture_text(&mut element, &renderer);
        let mut tree = Tree::new(element.as_widget());
        let limits = Limits::new(Size::ZERO, Size::new(1000.0, 1000.0));
        let node = element
            .as_widget_mut()
            .layout(&mut tree, &renderer, &limits);
        let mut controls = CapturedControls::default();
        element
            .as_widget_mut()
            .operate(&mut tree, Layout::new(&node), &renderer, &mut controls);

        let volume = controls
            .volume
            .unwrap_or_else(|| panic!("missing volume slider for {position:?}/{duration:?}"));
        assert_eq!(volume.width, MINI_VOLUME_WIDTH);
        assert!(
            (volume.x + volume.width - (node.size().width - 6.0)).abs() < 0.5,
            "the volume slider must stay at the right edge: {volume:?} in {:?}",
            node.size()
        );
        match controls.seek {
            Some(seek) if expected => {
                let readout = capture.find("00:53/3:30");
                assert!(
                    readout.x + readout.width <= seek.x + 0.5,
                    "the seek slider must follow the readout: {readout:?} then {seek:?}"
                );
                assert!(
                    seek.x + seek.width <= volume.x + 0.5,
                    "the seek slider must precede the volume slider: {seek:?} then {volume:?}"
                );
            }
            None if !expected => {}
            other => {
                panic!("seek container presence wrong for {position:?}/{duration:?}: {other:?}")
            }
        }
    }
}

/// Lays a compact strip out with the headless tiny-skia renderer and records
/// every text fragment plus the bounds of the container tagged
/// [`MINI_VOLUME_ID`], so a test can see the time readout and the volume
/// slider's place in the real laid-out row.
fn captured_mini_strip(
    mut element: Element<'_, Message>,
) -> (Vec<String>, Option<Rectangle>, Size) {
    #[derive(Default)]
    struct CapturedContainer {
        slider: Option<Rectangle>,
    }
    impl Operation for CapturedContainer {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }
        fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
            if id == Some(&Id::new(MINI_VOLUME_ID)) {
                self.slider = Some(bounds);
            }
        }
    }

    let renderer = layout_capture::headless_renderer();
    let capture = layout_capture::capture_text(&mut element, &renderer);
    let texts = capture.texts().map(str::to_string).collect();

    let mut tree = Tree::new(element.as_widget());
    let limits = Limits::new(Size::ZERO, Size::new(1000.0, 1000.0));
    let node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &limits);
    let mut captured = CapturedContainer::default();
    element
        .as_widget_mut()
        .operate(&mut tree, Layout::new(&node), &renderer, &mut captured);
    (texts, captured.slider, capture.size())
}

fn captured_shade_strip(elapsed: &str, total: &str) -> (Vec<String>, Option<Rectangle>, Size) {
    captured_mini_strip(view_shade_strip(
        TransportMode::Stopped,
        0.5,
        Cow::Borrowed(elapsed),
        Cow::Borrowed(total),
        None,
        None,
    ))
}

fn captured_library_strip(elapsed: &str, total: &str) -> (Vec<String>, Option<Rectangle>, Size) {
    captured_mini_strip(view_library_strip(
        TransportMode::Stopped,
        0.5,
        Cow::Borrowed(elapsed),
        Cow::Borrowed(total),
    ))
}

/// Asserts a captured compact strip draws the `{time}/3:30` readout and seats
/// its [`MINI_VOLUME_WIDTH`] volume slider 6px in from the right edge; `edge`
/// names the strip in the failure message.
fn assert_readout_and_right_edge_volume(
    texts: &[String],
    slider: Option<Rectangle>,
    size: Size,
    time: &str,
    edge: &str,
) {
    let expected = format!("{time}/3:30");
    assert!(
        texts.iter().any(|text| text == &expected),
        "missing {expected} readout: {texts:?}"
    );
    let slider = slider.unwrap_or_else(|| panic!("missing volume slider: {texts:?}"));
    assert_eq!(slider.width, MINI_VOLUME_WIDTH);
    assert!(
        (slider.x + slider.width - (size.width - 6.0)).abs() < 0.5,
        "slider must sit at the {edge}'s right edge: {slider:?} in {size:?}"
    );
}

// The compact readout joins the elapsed (or remaining) value to the track's
// total, and keeps the single plain placeholder when no total is known.
#[test]
fn mini_transport_readout_appends_the_known_total() {
    for (elapsed, total, expected) in [
        ("00:53", "3:30", "00:53/3:30"),
        ("-02:37", "3:30", "-02:37/3:30"),
        ("00:53", "--:--", "00:53"),
        ("--:--", "--:--", "--:--"),
    ] {
        let readout = mini_transport_readout(Cow::Borrowed(elapsed), Cow::Borrowed(total));
        assert_eq!(readout, expected, "{elapsed:?} over {total:?}");
    }
}

// Both compact strips must wire the shared guard up, not just expose it: a
// real left press at the readout's bounds publishes exactly one
// `ToggleTimeDisplay` while playing or paused, and none while stopped.
#[test]
fn compact_readout_toggles_time_display_only_while_not_stopped() {
    let mut renderer = layout_capture::headless_renderer();
    for mode in [
        TransportMode::Playing,
        TransportMode::Paused,
        TransportMode::Stopped,
    ] {
        for strip in ["shade", "library"] {
            let mut element: iced::Element<'_, Message, iced::Theme, iced::Renderer> = match strip {
                "shade" => view_shade_strip(
                    mode,
                    0.5,
                    Cow::Borrowed("00:53"),
                    Cow::Borrowed("3:30"),
                    None,
                    None,
                ),
                _ => view_library_strip(mode, 0.5, Cow::Borrowed("00:53"), Cow::Borrowed("3:30")),
            };
            let capture = layout_capture::capture_text(&mut element, &renderer);
            let readout = capture.find("00:53/3:30");
            let messages = layout_capture::publish_at(
                element,
                &mut renderer,
                &[iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
                    iced::mouse::Button::Left,
                ))],
                iced::Point::new(readout.center_x(), readout.center_y()),
            );
            let expected = mode != TransportMode::Stopped;
            let toggles = messages
                .iter()
                .filter(|message| matches!(message, Message::ToggleTimeDisplay))
                .count();
            assert_eq!(
                toggles,
                usize::from(expected),
                "a press on the {mode:?} {strip} readout must {} publish ToggleTimeDisplay",
                if expected { "still" } else { "not" }
            );
            if !expected {
                assert!(
                    messages.is_empty(),
                    "the stopped {strip} readout published {messages:?}"
                );
            }
        }
    }
}

// The strip's two non-button controls must actually reach the laid-out row:
// the time readout the caller gives it (elapsed or remaining) and the volume
// slider. The fill spacer pushes the slider to the right edge, so it does not
// sit immediately after the readout on the left.
#[test]
fn shade_strip_draws_the_time_readout_and_puts_the_volume_slider_at_the_right_edge() {
    for time in ["00:53", "-02:37"] {
        let (texts, slider, size) = captured_shade_strip(time, "3:30");
        assert_readout_and_right_edge_volume(&texts, slider, size, time, "strip");
    }
}

// The strip's volume slider must read the shared volume range and write
// `Message::VolumeChange`, the same contract as the full transport row's
// volume slider.
#[test]
fn shade_volume_slider_spec_matches_the_shared_volume_range_and_message() {
    let (min, max, step, width, on_change) = mini_volume_spec();
    assert_eq!((min, max, step), (VOLUME_MIN, VOLUME_MAX, VOLUME_STEP));
    assert_eq!(width, MINI_VOLUME_WIDTH);
    assert_eq!(MINI_VOLUME_WIDTH, 80.0);
    assert!(matches!(
        on_change(0.5),
        Message::VolumeChange(value) if value == 0.5
    ));
}

// The library footer is a fixed-height strip, so adding it to the normal root
// column cannot let the browse content squeeze it or stretch it.
#[test]
fn view_library_strip_keeps_its_fixed_height() {
    let renderer = layout_capture::headless_renderer();

    for mode in [
        TransportMode::Stopped,
        TransportMode::Paused,
        TransportMode::Playing,
    ] {
        let mut element =
            view_library_strip(mode, 0.5, Cow::Borrowed("00:53"), Cow::Borrowed("3:30"));
        let mut tree = Tree::new(element.as_widget());
        let limits = Limits::new(Size::ZERO, Size::new(1000.0, 1000.0));
        let node = element
            .as_widget_mut()
            .layout(&mut tree, &renderer, &limits);
        assert_eq!(
            node.size().height,
            MINI_TRANSPORT_HEIGHT,
            "the library footer must keep its fixed height in {mode:?} mode"
        );
    }
}

// The footer's five glyphs, time readout, and right-edge volume slider must
// actually reach the laid-out row, in the same order and geometry as the
// shade strip's.
#[test]
fn library_strip_draws_the_five_controls_time_and_right_edge_volume() {
    for time in ["00:53", "-02:37"] {
        let (texts, slider, size) = captured_library_strip(time, "3:30");
        let glyphs: Vec<&str> = texts
            .iter()
            .map(String::as_str)
            .filter(|text| ["◀◀", "▶", "II", "■", "▶▶"].contains(text))
            .collect();
        assert_eq!(glyphs, vec!["◀◀", "▶", "II", "■", "▶▶"]);
        assert_readout_and_right_edge_volume(&texts, slider, size, time, "footer");
    }
}

// The row draws both Play and Pause in every playback state: neither face is
// chosen at build time, so the compact strip keeps the same five glyphs while
// stopped, playing, and paused.
#[test]
fn transport_controls_draw_play_and_pause_in_every_playback_state() {
    for mode in [
        TransportMode::Stopped,
        TransportMode::Playing,
        TransportMode::Paused,
    ] {
        let (texts, _, _) = captured_mini_strip(view_shade_strip(
            mode,
            0.5,
            Cow::Borrowed("00:53"),
            Cow::Borrowed("3:30"),
            None,
            None,
        ));
        for glyph in ["▶", "II"] {
            assert!(
                texts.iter().any(|text| text == glyph),
                "the {glyph:?} button must be drawn in {mode:?} mode: {texts:?}"
            );
        }
    }
}
