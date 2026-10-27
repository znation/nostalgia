use super::*;

// A roll over a drawn seek slider seeks by sign: wheel-up forward, wheel-down
// back, for both delta kinds.
#[test]
fn seek_wheel_message_maps_a_vertical_roll_to_a_direction() {
    use iced::mouse::ScrollDelta::{Lines, Pixels};
    assert!(matches!(
        seek_wheel_message(Lines { x: 0.0, y: 3.0 }),
        Message::SeekForward
    ));
    assert!(matches!(
        seek_wheel_message(Pixels { x: 0.0, y: 120.0 }),
        Message::SeekForward
    ));
    assert!(matches!(
        seek_wheel_message(Lines { x: 0.0, y: -2.0 }),
        Message::SeekBackward
    ));
    assert!(matches!(
        seek_wheel_message(Pixels { x: 0.0, y: -45.0 }),
        Message::SeekBackward
    ));
}

#[test]
fn seek_wheel_message_ignores_a_horizontal_only_roll() {
    use iced::mouse::ScrollDelta::{Lines, Pixels};
    for delta in [
        Lines { x: 4.0, y: 0.0 },
        Pixels { x: 30.0, y: 0.0 },
        Lines {
            x: 0.0,
            y: f32::NAN,
        },
    ] {
        assert!(matches!(seek_wheel_message(delta), Message::Ignored));
    }
}

// An infinite roll is comparable, so it seeks by sign like any other; only
// NaN (incomparable) is ignored.
#[test]
fn seek_wheel_message_gives_an_infinite_roll_its_signs_direction() {
    use iced::mouse::ScrollDelta::{Lines, Pixels};
    assert!(matches!(
        seek_wheel_message(Lines {
            x: 0.0,
            y: f32::INFINITY
        }),
        Message::SeekForward
    ));
    assert!(matches!(
        seek_wheel_message(Pixels {
            x: 0.0,
            y: f32::NEG_INFINITY
        }),
        Message::SeekBackward
    ));
}

// The shade strip's drawn seek slider seeks on a wheel roll, and the roll
// does not also reach the volume.
#[test]
fn wheel_over_the_shade_seek_slider_seeks() {
    #[derive(Default)]
    struct CapturedSeek(Option<Rectangle>);
    impl Operation for CapturedSeek {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }
        fn container(&mut self, id: Option<&Id>, bounds: Rectangle) {
            if id == Some(&Id::new(MINI_SEEK_ID)) {
                self.0 = Some(bounds);
            }
        }
    }

    let strip = || {
        view_shade_strip(
            TransportMode::Playing,
            true,
            0.5,
            Cow::Borrowed("00:10"),
            Cow::Borrowed("0:30"),
            Some(Duration::from_secs(10)),
            Some(Duration::from_secs(30)),
        )
    };
    let mut renderer = layout_capture::headless_renderer();
    let mut element = strip();
    let mut captured = CapturedSeek::default();
    layout_capture::operate(&mut element, &renderer, &mut captured);
    let seek = captured.0.expect("the seek slider is drawn");
    let point = iced::Point::new(seek.center_x(), seek.center_y());

    for (y, expect_forward) in [(1.0, true), (-1.0, false)] {
        let wheel = [iced::Event::Mouse(iced::mouse::Event::WheelScrolled {
            delta: iced::mouse::ScrollDelta::Lines { x: 0.0, y },
        })];
        let messages = layout_capture::publish_at(strip(), &mut renderer, &wheel, point);
        let seeks: Vec<_> = messages
            .iter()
            .filter(|m| matches!(m, Message::SeekForward | Message::SeekBackward))
            .collect();
        assert_eq!(seeks.len(), 1, "exactly one seek expected: {messages:?}");
        assert_eq!(matches!(seeks[0], Message::SeekForward), expect_forward);
        assert!(
            !messages
                .iter()
                .any(|m| matches!(m, Message::VolumeWheel(_))),
            "the roll must not nudge the volume: {messages:?}"
        );
    }
}
