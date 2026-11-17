use super::*;

// A plain roll over the balance slider moves the balance instead of the
// volume, in both directions.
#[test]
fn wheel_over_the_balance_slider_nudges_the_balance_not_the_volume() {
    let (player, _state) = test_player();
    let slider = captured_root_view_container(&player, views::BALANCE_SLIDER_ID)
        .expect("the balance slider is laid out");

    for (y, expected) in [(1.0, 1.0), (-1.0, -1.0)] {
        let messages = root_wheel_messages(&player, slider, y);
        let wheels: Vec<&f32> = messages
            .iter()
            .filter_map(|message| match message {
                Message::BalanceWheel(direction) => Some(direction),
                _ => None,
            })
            .collect();
        assert_eq!(
            wheels.len(),
            1,
            "exactly one balance roll expected: {messages:?}"
        );
        assert_eq!(*wheels[0], expected);
        assert!(
            !messages
                .iter()
                .any(|message| matches!(message, Message::VolumeWheel(_))),
            "the roll must not nudge the volume: {messages:?}"
        );
    }
}

// A plain roll over the equalizer's preamp or any band moves that slider
// instead of the volume.
#[test]
fn wheel_over_the_preamp_and_every_band_nudges_that_slider_not_the_volume() {
    let (mut player, _state) = test_player();
    player.eq_panel_visible = true;

    let preamp = captured_root_view_container(&player, views::EQ_PREAMP_SLIDER_ID)
        .expect("the preamp slider is laid out");
    // The slider itself is 16px wide at the column's left, narrower than the
    // band label that sets the column's width, so aim at the slider's centre
    // rather than the container's.
    let slider_area = |bounds: iced::Rectangle| iced::Rectangle {
        width: 16.0,
        height: 100.0,
        ..bounds
    };
    let messages = root_wheel_messages(&player, slider_area(preamp), 1.0);
    let preamp_wheels: Vec<&f32> = messages
        .iter()
        .filter_map(|message| match message {
            Message::EqPreampWheel(direction) => Some(direction),
            _ => None,
        })
        .collect();
    assert_eq!(
        preamp_wheels.len(),
        1,
        "exactly one preamp roll expected: {messages:?}"
    );
    assert_eq!(*preamp_wheels[0], 1.0);
    assert!(
        !messages
            .iter()
            .any(|message| matches!(message, Message::VolumeWheel(_))),
        "the preamp roll must not nudge the volume: {messages:?}"
    );

    for index in 0..BAND_COUNT {
        let band = captured_root_view_container(&player, &views::eq_band_slider_id(index))
            .unwrap_or_else(|| panic!("band {index} is laid out"));
        let messages = root_wheel_messages(&player, slider_area(band), 1.0);
        let band_wheels: Vec<(usize, f32)> = messages
            .iter()
            .filter_map(|message| match message {
                Message::EqBandWheel(moved, direction) => Some((*moved, *direction)),
                _ => None,
            })
            .collect();
        assert_eq!(
            band_wheels,
            vec![(index, 1.0)],
            "only band {index} moves: {messages:?}"
        );
        assert!(
            !messages
                .iter()
                .any(|message| matches!(message, Message::VolumeWheel(_))),
            "the band roll must not nudge the volume: {messages:?}"
        );
    }
}
