use super::*;

#[test]
fn now_playing_label_shows_nothing_with_no_current_track() {
    assert_eq!(now_playing_label(&known_tracks(&[]), None), "Nothing");
}

#[test]
fn now_playing_label_resolves_known_track_to_title() {
    let tracks = known_tracks(&[("song-1", "The Sample Band", "Opening")]);
    assert_eq!(now_playing_label(&tracks, Some("song-1")), "Opening");
}

#[test]
fn now_playing_label_falls_back_to_id_when_track_not_in_tracks() {
    let tracks = known_tracks(&[("song-1", "The Sample Band", "Opening")]);
    assert_eq!(
        now_playing_label(&tracks, Some("no-such-song")),
        "no-such-song"
    );
}

#[test]
fn now_playing_label_falls_back_to_id_when_tracks_is_empty() {
    assert_eq!(
        now_playing_label(&known_tracks(&[]), Some("song-1")),
        "song-1"
    );
}

// The Now Playing bar renders `<artist> - <title>`; the artist resolves
// through the same id→track index as the title. Pin the known case, both
// fallbacks (unknown id and no current track), and — as with the label —
// that the known artist borrows from the caller's map rather than cloning
// per frame.
#[test]
fn now_playing_artist_resolves_known_track_to_artist() {
    let tracks = known_tracks(&[("song-1", "The Sample Band", "Opening")]);
    assert_eq!(
        now_playing_artist(&tracks, Some("song-1")),
        "The Sample Band"
    );
}

#[test]
fn now_playing_artist_is_empty_when_track_not_in_tracks() {
    let tracks = known_tracks(&[("song-1", "The Sample Band", "Opening")]);
    assert_eq!(now_playing_artist(&tracks, Some("no-such-song")), "");
}

#[test]
fn now_playing_artist_is_empty_with_no_current_track() {
    assert_eq!(now_playing_artist(&known_tracks(&[]), None), "");
}

#[test]
fn now_playing_artist_borrows_the_artist() {
    let tracks = known_tracks(&[("song-1", "The Sample Band", "Opening")]);

    let artist = now_playing_artist(&tracks, Some("song-1"));
    assert!(matches!(&artist, Cow::Borrowed(_)));
    assert_eq!(artist.as_ptr(), tracks["song-1"].artist.as_ptr());

    let artist = now_playing_artist(&tracks, None);
    assert!(matches!(&artist, Cow::Borrowed(_)));
}

// The Now Playing bar draws the cached playlist number prefix between its
// caption and the artist. Pin the known case, both empty fallbacks (unknown id
// and no current track), and that the known prefix borrows from the caller's
// map rather than cloning per frame.
#[test]
fn now_playing_number_resolves_known_track_to_its_cached_prefix() {
    let mut tracks = known_tracks(&[("song-1", "The Sample Band", "Opening")]);
    tracks.get_mut("song-1").expect("seed track").number = "1. ".to_string();

    let number = now_playing_number(&tracks, Some("song-1"));
    assert_eq!(number, "1. ");
    assert!(matches!(&number, Cow::Borrowed(_)));
    assert_eq!(number.as_ptr(), tracks["song-1"].number.as_ptr());

    assert_eq!(now_playing_number(&tracks, Some("no-such-song")), "");
    assert_eq!(now_playing_number(&tracks, None), "");
}

#[test]
fn now_playing_time_resolves_known_track_to_cached_label() {
    let mut tracks = known_tracks(&[("song-1", "The Sample Band", "Opening")]);
    tracks.get_mut("song-1").expect("seed track").time = "3:30".to_string();

    let time = now_playing_time(&tracks, Some("song-1"));
    assert_eq!(time, "3:30");
    assert!(matches!(&time, Cow::Borrowed(_)));
    assert_eq!(time.as_ptr(), tracks["song-1"].time.as_ptr());

    let missing = now_playing_time(&tracks, Some("no-such-song"));
    assert_eq!(missing, "--:--");
    assert!(matches!(&missing, Cow::Borrowed(_)));

    let none = now_playing_time(&tracks, None);
    assert_eq!(none, "--:--");
    assert!(matches!(&none, Cow::Borrowed(_)));
}

// The right edge always shows the plain cached total; the backend-reported
// position belongs to the separate left-hand elapsed readout, never to this
// string.
#[test]
fn now_playing_time_returns_the_plain_total_even_with_a_position() {
    let mut tracks = known_tracks(&[("song-1", "The Sample Band", "Opening")]);
    tracks.get_mut("song-1").expect("seed track").time = "3:30".to_string();

    assert_eq!(now_playing_time(&tracks, Some("song-1")), "3:30");
    assert_eq!(
        now_playing_elapsed(
            &tracks,
            Some("song-1"),
            Some(Duration::from_secs(192)),
            TimeDisplay::Elapsed,
        ),
        "03:12"
    );
    assert_eq!(now_playing_time(&tracks, Some("song-1")), "3:30");
}

#[test]
fn now_playing_elapsed_reads_placeholder_zero_or_position() {
    let mut tracks = known_tracks(&[("song-1", "The Sample Band", "Opening")]);
    tracks.get_mut("song-1").expect("seed track").time = "3:30".to_string();

    let unknown = now_playing_elapsed(
        &tracks,
        Some("no-such-song"),
        Some(Duration::from_secs(53)),
        TimeDisplay::Elapsed,
    );
    assert_eq!(unknown, "--:--");
    assert!(matches!(&unknown, Cow::Borrowed(_)));

    let none = now_playing_elapsed(
        &tracks,
        None,
        Some(Duration::from_secs(53)),
        TimeDisplay::Elapsed,
    );
    assert_eq!(none, "--:--");
    assert!(matches!(&none, Cow::Borrowed(_)));

    let zero = now_playing_elapsed(&tracks, Some("song-1"), None, TimeDisplay::Elapsed);
    assert_eq!(zero, "00:00");
    assert!(matches!(&zero, Cow::Borrowed(_)));

    let position = now_playing_elapsed(
        &tracks,
        Some("song-1"),
        Some(Duration::from_secs(53)),
        TimeDisplay::Elapsed,
    );
    assert_eq!(position, "00:53");
    assert!(matches!(&position, Cow::Owned(_)));
}

#[test]
fn format_lcd_elapsed_pads_minutes_and_seconds() {
    assert_eq!(format_lcd_elapsed(0), "00:00");
    assert_eq!(format_lcd_elapsed(53_000), "00:53");
    assert_eq!(format_lcd_elapsed(192_000), "03:12");
    // Minutes stay unbounded, so an hour reads as 60:00 rather than rolling
    // over.
    assert_eq!(format_lcd_elapsed(3_600_000), "60:00");
}

#[test]
fn format_lcd_remaining_prefixes_a_minus_and_pads() {
    assert_eq!(format_lcd_remaining(0), "-00:00");
    assert_eq!(format_lcd_remaining(157_000), "-02:37");
    assert_eq!(format_lcd_remaining(3_600_000), "-60:00");
}

// The remaining mode counts down from the known track's own length, treating a
// missing position as zero; the length and position share milliseconds, so a
// position past the length saturates at -00:00 instead of underflowing. A
// known track with no length has nothing to count down from, and an unknown or
// absent track keeps the placeholder.
#[test]
fn now_playing_elapsed_remaining_counts_down_from_the_total() {
    let mut tracks = known_tracks(&[("song-1", "The Sample Band", "Opening")]);
    tracks.get_mut("song-1").expect("seed track").duration_ms = 210_000;

    assert_eq!(
        now_playing_elapsed(
            &tracks,
            Some("song-1"),
            Some(Duration::from_secs(53)),
            TimeDisplay::Remaining,
        ),
        "-02:37"
    );
    assert_eq!(
        now_playing_elapsed(&tracks, Some("song-1"), None, TimeDisplay::Remaining),
        "-03:30"
    );
    assert_eq!(
        now_playing_elapsed(
            &tracks,
            Some("song-1"),
            Some(Duration::from_secs(300)),
            TimeDisplay::Remaining,
        ),
        "-00:00"
    );

    tracks.get_mut("song-1").expect("seed track").duration_ms = 0;
    let no_length = now_playing_elapsed(
        &tracks,
        Some("song-1"),
        Some(Duration::from_secs(53)),
        TimeDisplay::Remaining,
    );
    assert_eq!(no_length, "--:--");
    assert!(matches!(&no_length, Cow::Borrowed(_)));

    let unknown = now_playing_elapsed(
        &tracks,
        Some("no-such-song"),
        Some(Duration::from_secs(53)),
        TimeDisplay::Remaining,
    );
    assert_eq!(unknown, "--:--");
    assert!(matches!(&unknown, Cow::Borrowed(_)));
}

#[test]
fn format_track_time_shows_placeholders_when_the_source_supplied_no_duration() {
    assert_eq!(format_track_time(0), "--:--");
}

// The marquee window is a pure character window: a line that fits is left
// alone so the common case renders borrowed and still, and an overlong line
// starts at the requested offset, wraps through the separator, and counts
// characters rather than bytes.
#[test]
fn marquee_text_keeps_a_line_that_fits() {
    assert_eq!(marquee_text("Opening", 9, 28), "Opening");
}

#[test]
fn marquee_text_windows_from_the_offset() {
    let text = "The Sample Band - Opening";
    assert_eq!(marquee_text(text, 0, 10), "The Sample");
    assert_eq!(marquee_text(text, 2, 10), "e Sample B");
}

#[test]
fn marquee_text_wraps_through_the_separator_to_the_start() {
    let text = "The Sample Band - Opening";
    assert_eq!(marquee_text(text, 25, 10), "  ***  The");
    assert_eq!(marquee_text(text, 27, 10), "***  The S");
    assert_eq!(marquee_text(text, 57, 10), "  ***  The");
    assert_eq!(marquee_text("Opening", 9, 28), "Opening");
}

#[test]
fn marquee_text_counts_characters_not_bytes() {
    assert_eq!(marquee_text("Café — Ok", 0, 5), "Café ");
}

/// Lays out the indicator strip (with its live `O`, `A`, `I`, and `V` controls)
/// and returns the text fragments it would draw and the width the column
/// occupies.
fn captured_indicator_texts() -> (Vec<String>, f32) {
    let mut element: iced::Element<'_, Message, iced::Theme, ()> =
        lcd_indicator_strip(IndicatorModes::default()).into();
    let capture = layout_capture::capture_text(&mut element, &());
    (
        capture.texts().map(str::to_string).collect(),
        capture.size().width,
    )
}

/// Lays the indicator strip out with the headless renderer for `modes` and
/// returns the text fragments it draws and the pixels it paints.
fn captured_indicator(
    modes: IndicatorModes,
) -> (layout_capture::TextCapture, layout_capture::PixelCapture) {
    let mut renderer = layout_capture::headless_renderer();
    let mut element: iced::Element<'_, Message, iced::Theme, iced::Renderer> =
        lcd_indicator_strip(modes).into();
    layout_capture::capture_text_and_pixels(&mut element, &mut renderer, &iced::Theme::Dark)
}

// Each live letter lights only for its own mode: O for the options menu, A for
// always-on-top, I for File info, and V for the visualizer. Every other case
// stays dim, including the decorative D under every combination.
#[test]
fn indicator_label_color_lights_only_the_active_mode() {
    for label in LCD_INDICATOR_LABELS {
        assert_eq!(
            indicator_label_color(label, IndicatorModes::default()),
            theme::LCD_GREEN_DIM,
            "no mode on must leave {label} dim"
        );
    }

    let all_on = IndicatorModes {
        options_visible: true,
        always_on_top: true,
        file_info_visible: true,
        visualizer_visible: true,
    };
    for label in LCD_INDICATOR_LABELS {
        let expected = if matches!(label, "O" | "A" | "I" | "V") {
            theme::LCD_GREEN
        } else {
            theme::LCD_GREEN_DIM
        };
        assert_eq!(
            indicator_label_color(label, all_on),
            expected,
            "all modes on must leave only the live letters lit: {label}"
        );
    }

    for (modes, lit) in [
        (
            IndicatorModes {
                options_visible: true,
                ..Default::default()
            },
            "O",
        ),
        (
            IndicatorModes {
                always_on_top: true,
                ..Default::default()
            },
            "A",
        ),
        (
            IndicatorModes {
                file_info_visible: true,
                ..Default::default()
            },
            "I",
        ),
        (
            IndicatorModes {
                visualizer_visible: true,
                ..Default::default()
            },
            "V",
        ),
    ] {
        for label in LCD_INDICATOR_LABELS {
            let expected = if label == lit {
                theme::LCD_GREEN
            } else {
                theme::LCD_GREEN_DIM
            };
            assert_eq!(
                indicator_label_color(label, modes),
                expected,
                "only {lit} must light under {modes:?}, not {label}"
            );
        }
    }
}

// The drawn strip really renders the active letters brighter than the dim
// ones, rather than only exposing the right colors: lay it out all-off and
// all-on and compare the green of each letter's pixels.
#[test]
fn lcd_indicator_strip_lights_the_active_letters_and_dims_the_rest() {
    let (dim_capture, dim_pixels) = captured_indicator(IndicatorModes::default());
    for label in LCD_INDICATOR_LABELS {
        let green = dim_pixels.max_green_in(dim_capture.find(label));
        assert!(
            green < 128,
            "every letter must be dim with no mode on: {label} green {green}"
        );
    }

    let (lit_capture, lit_pixels) = captured_indicator(IndicatorModes {
        options_visible: true,
        always_on_top: true,
        file_info_visible: true,
        visualizer_visible: true,
    });
    assert_eq!(
        lit_capture.texts().collect::<Vec<_>>(),
        vec!["O", "A", "I", "D", "V"],
        "lighting the letters must not change their order"
    );
    for lit in ["O", "A", "I", "V"] {
        let lit_green = lit_pixels.max_green_in(lit_capture.find(lit));
        let dim_green = lit_pixels.max_green_in(lit_capture.find("D"));
        assert!(
            lit_green > dim_green + 100,
            "{lit} must be lit and D dim: {lit} green {lit_green}, D green {dim_green}"
        );
    }
}

// The LCD's left edge draws the five classic indicator letters in the
// mockup's order, one per line, at the fixed strip width so the content
// beside it never shifts.
#[test]
fn lcd_indicator_strip_draws_each_label_in_order_at_a_fixed_width() {
    let (texts, width) = captured_indicator_texts();
    assert_eq!(texts, vec!["O", "A", "I", "D", "V"]);
    assert_eq!(width, 12.0, "the strip's fixed width");
}

// All four non-D letters are live: O opens the options menu, A reuses the
// always-on-top toggle, I opens File info, V toggles the LCD visualizer, and D
// publishes nothing.
#[test]
fn lcd_indicator_message_arms_o_a_i_and_v_and_leaves_d_inert() {
    assert!(matches!(
        lcd_indicator_message("O"),
        Some(Message::ToggleOptions)
    ));
    assert!(matches!(
        lcd_indicator_message("A"),
        Some(Message::ToggleAlwaysOnTop)
    ));
    assert!(matches!(
        lcd_indicator_message("I"),
        Some(Message::ToggleFileInfo)
    ));
    assert!(matches!(
        lcd_indicator_message("V"),
        Some(Message::ToggleVisualizer)
    ));
    assert!(
        lcd_indicator_message("D").is_none(),
        "D must stay decorative"
    );
}

// The strip must wire the mapping up, not just expose it: drive a real left
// press at each letter's bounds. O publishes the options toggle, A publishes
// the always-on-top toggle, I publishes the File info toggle, V publishes the
// visualizer toggle, and D publishes nothing.
#[test]
fn lcd_indicator_strip_publishes_options_on_o_always_on_top_on_a_visualizer_on_v_and_file_info_on_i()
 {
    let mut renderer = layout_capture::headless_renderer();
    for label in ["O", "A", "I", "V", "D"] {
        let mut element: iced::Element<'_, Message, iced::Theme, iced::Renderer> =
            lcd_indicator_strip(IndicatorModes::default()).into();
        let capture = layout_capture::capture_text(&mut element, &renderer);
        let letter = capture.find(label);
        let messages = layout_capture::publish_at(
            element,
            &mut renderer,
            &[iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
                iced::mouse::Button::Left,
            ))],
            iced::Point::new(letter.center_x(), letter.center_y()),
        );
        match label {
            "O" => assert!(
                messages
                    .iter()
                    .any(|message| matches!(message, Message::ToggleOptions)),
                "a press on O must publish ToggleOptions: {messages:?}"
            ),
            "A" => assert!(
                messages
                    .iter()
                    .any(|message| matches!(message, Message::ToggleAlwaysOnTop)),
                "a press on A must publish ToggleAlwaysOnTop: {messages:?}"
            ),
            "I" => assert!(
                messages
                    .iter()
                    .any(|message| matches!(message, Message::ToggleFileInfo)),
                "a press on I must publish ToggleFileInfo: {messages:?}"
            ),
            "V" => assert!(
                messages
                    .iter()
                    .any(|message| matches!(message, Message::ToggleVisualizer)),
                "a press on V must publish ToggleVisualizer: {messages:?}"
            ),
            _ => assert!(
                messages.is_empty(),
                "a press on {label} must publish nothing: {messages:?}"
            ),
        }
    }
}

// A left press anywhere on the visualizer strip publishes the same toggle as
// the indicator strip's V, whether the bars are drawn or the spectrum is empty.
#[test]
fn visualizer_row_press_publishes_toggle_visualizer() {
    let mut renderer = layout_capture::headless_renderer();
    for spectrum in [
        None,
        Some(Spectrum {
            bands: [0.5; BAND_COUNT],
        }),
    ] {
        let mut probe: iced::Element<'_, Message, iced::Theme, ()> =
            now_playing_visualizer_row(spectrum, None).into();
        let (_, node) = layout_capture::layout_tree(&mut probe, &());
        let bounds = node.bounds();

        let element: iced::Element<'_, Message, iced::Theme, iced::Renderer> =
            now_playing_visualizer_row(spectrum, None).into();
        let messages = layout_capture::publish_at(
            element,
            &mut renderer,
            &[iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
                iced::mouse::Button::Left,
            ))],
            iced::Point::new(bounds.center_x(), bounds.center_y()),
        );
        assert!(
            matches!(messages.as_slice(), [Message::ToggleVisualizer]),
            "a press on the visualizer row must publish exactly ToggleVisualizer: {messages:?}"
        );
    }
}

// Hiding the visualizer drops the bars from the LCD without clearing the
// stored spectrum, so showing them again draws the current ones.
#[test]
fn visible_spectrum_hides_the_stored_bars_while_the_visualizer_is_off() {
    let spectrum = recording_spectrum();
    assert_eq!(visible_spectrum(Some(spectrum), true), Some(spectrum));
    assert_eq!(visible_spectrum(Some(spectrum), false), None);
    assert_eq!(visible_spectrum(None, true), None);
    assert_eq!(visible_spectrum(None, false), None);
}

// A peak holds its level while the bars fall, drops one step per reply, and
// never sinks below the reported band or zero; odd bands are clamped.
#[test]
fn next_spectrum_peaks_follows_a_rising_band_and_falls_slowly_after_it() {
    let bands = |first: f32, second: f32| {
        let mut bands = [0.0; BAND_COUNT];
        bands[0] = first;
        bands[1] = second;
        Spectrum { bands }
    };
    let first = next_spectrum_peaks(None, Some(bands(0.5, 2.0))).unwrap();
    assert_eq!(first.bands[0], 0.5);
    assert_eq!(first.bands[1], 1.0);
    let lower = next_spectrum_peaks(Some(first), Some(bands(0.0, f32::NAN))).unwrap();
    assert_eq!(lower.bands[0], 0.375);
    assert_eq!(lower.bands[1], 0.875);
    let floor = next_spectrum_peaks(Some(lower), Some(bands(0.4, -1.0))).unwrap();
    assert_eq!(floor.bands[0], 0.4);
    assert_eq!(floor.bands[1], 0.75);
    let low = next_spectrum_peaks(Some(bands(0.1, 0.0)), Some(bands(0.0, 0.0))).unwrap();
    assert_eq!(low.bands[0], 0.0);
    let raised = next_spectrum_peaks(Some(bands(0.2, 0.0)), Some(bands(0.9, 0.0))).unwrap();
    assert_eq!(raised.bands[0], 0.9);
    assert_eq!(next_spectrum_peaks(Some(first), None), None);
}

// A bar jumps up at once, falls a quarter per reply, and clamps odd bands.
#[test]
fn next_spectrum_bars_rise_at_once_and_fall_a_quarter_per_reply() {
    let bands = |first: f32, second: f32| {
        let mut bands = [0.0; BAND_COUNT];
        bands[0] = first;
        bands[1] = second;
        Spectrum { bands }
    };
    let full = next_spectrum_bars(None, Some(bands(1.0, 2.0))).unwrap();
    assert_eq!(full.bands[0], 1.0);
    assert_eq!(full.bands[1], 1.0);
    let lower = next_spectrum_bars(Some(full), Some(bands(0.0, f32::NAN))).unwrap();
    assert_eq!(lower.bands[0], 0.75);
    assert_eq!(lower.bands[1], 0.75);
    let lower = next_spectrum_bars(Some(lower), Some(bands(0.0, -1.0))).unwrap();
    assert_eq!(lower.bands[0], 0.5);
    let raised = next_spectrum_bars(Some(lower), Some(bands(0.9, 0.0))).unwrap();
    assert_eq!(raised.bands[0], 0.9);
    let bar = next_spectrum_bars(Some(full), Some(bands(0.0, 0.0))).unwrap();
    let peak = next_spectrum_peaks(Some(full), Some(bands(0.0, 0.0))).unwrap();
    assert!(bar.bands[0] < peak.bands[0]);
    assert_eq!(next_spectrum_bars(Some(full), None), None);
}

// The marker sits on or above its bar and inside the 24 px strip, and only a
// positive peak draws one.
#[test]
fn visualizer_peak_marker_bottom_sits_on_or_above_its_bar_inside_the_strip() {
    assert_eq!(visualizer_peak_marker_bottom(10.0, 0.0), None);
    assert_eq!(visualizer_peak_marker_bottom(10.0, f32::NAN), None);
    assert_eq!(visualizer_peak_marker_bottom(10.0, 6.0), Some(10.0));
    assert_eq!(visualizer_peak_marker_bottom(10.0, 15.0), Some(15.0));
    assert_eq!(visualizer_peak_marker_bottom(24.0, 24.0), Some(23.0));
}

// The whole bar's content carries the strip as well as the elapsed, title,
// and total readouts, so a refactor cannot drop the strip while its builder
// alone stays tested. The content is laid out directly because the concrete
// `lcd_well` wrapper ties `view_now_playing` to iced's fallback renderer.
// Every letter sits left of the readouts, in order down the left edge, and
// the total keeps its right-edge place beside them. The null renderer gives
// text no size, so the letters' order is read from the draw sequence and
// their `x` positions from the row layout, not from their zero-height bounds.
#[test]
fn now_playing_content_draws_the_indicator_strip_beside_the_title() {
    let mut element: iced::Element<'_, Message, iced::Theme, ()> = now_playing_content(
        Cow::Borrowed("00:53"),
        Cow::Borrowed("Opening"),
        Cow::Borrowed("Band"),
        Cow::Borrowed(""),
        Cow::Borrowed("3:30"),
        0,
        None,
        None,
        None,
        TransportMode::Playing,
        true,
        None,
        IndicatorModes::default(),
    )
    .into();
    let capture = layout_capture::capture_text(&mut element, &());

    assert_eq!(
        capture.texts().take(6).collect::<Vec<_>>(),
        vec![
            "O",
            "A",
            "I",
            "D",
            "V",
            transport_glyph(TransportMode::Playing)
        ]
    );
    let find = |text: &str| capture.find(text);
    let elapsed = find("00:53");
    let title = find("Opening");
    let total = find("3:30");

    for label in ["O", "A", "I", "D", "V"] {
        let letter = find(label);
        assert!(
            letter.x < elapsed.x && letter.x < title.x && letter.x < total.x,
            "indicator {label} must sit left of the readouts: {letter:?}"
        );
    }

    assert!(
        (total.x + total.width - capture.size().width).abs() < 0.5,
        "the total must stay at the right edge: {total:?} in {:?}",
        capture.size()
    );
}

// Each transport state draws its own glyph in its documented color: green for
// playing and paused, red for stopped. Pin the exact strings and colors so a
// swap cannot ship unnoticed.
#[test]
fn transport_glyph_names_and_colors_each_mode() {
    assert_eq!(transport_glyph(TransportMode::Playing), "▶");
    assert_eq!(transport_glyph(TransportMode::Paused), "II");
    assert_eq!(transport_glyph(TransportMode::Stopped), "■");

    assert_eq!(
        transport_glyph_color(TransportMode::Playing),
        theme::LCD_GREEN
    );
    assert_eq!(
        transport_glyph_color(TransportMode::Paused),
        theme::LCD_GREEN
    );
    assert_eq!(
        transport_glyph_color(TransportMode::Stopped),
        theme::LCD_RED
    );
}

// Only the paused readout's hidden phase and a stopped readout with a real
// time are transparent: a paused visible phase, both playing phases, and the
// stopped `--:--` placeholder stay green. Pin the exact transparent color so a
// refactor cannot blank the readout by hiding the whole row.
#[test]
fn elapsed_readout_color_blinks_when_paused_and_clears_when_stopped() {
    let transparent = Color::from_rgba(0.0, 0.0, 0.0, 0.0);
    let elapsed = "00:53";

    assert_eq!(
        elapsed_readout_color(TransportMode::Paused, false, elapsed),
        transparent
    );
    assert_eq!(
        elapsed_readout_color(TransportMode::Paused, true, elapsed),
        theme::LCD_GREEN
    );
    assert_eq!(
        elapsed_readout_color(TransportMode::Playing, false, elapsed),
        theme::LCD_GREEN
    );
    assert_eq!(
        elapsed_readout_color(TransportMode::Playing, true, elapsed),
        theme::LCD_GREEN
    );
    assert_eq!(
        elapsed_readout_color(TransportMode::Stopped, false, elapsed),
        transparent
    );
    assert_eq!(
        elapsed_readout_color(TransportMode::Stopped, true, elapsed),
        transparent
    );
}

// The `--:--` placeholder says there is no track or no known time, so a
// stopped player must keep showing it instead of clearing it like a real
// elapsed value. The paused hidden phase still blinks it, as before.
#[test]
fn elapsed_readout_color_keeps_the_placeholder_visible_while_stopped() {
    let transparent = Color::from_rgba(0.0, 0.0, 0.0, 0.0);

    assert_eq!(
        elapsed_readout_color(TransportMode::Stopped, false, "--:--"),
        theme::LCD_GREEN
    );
    assert_eq!(
        elapsed_readout_color(TransportMode::Stopped, true, "--:--"),
        theme::LCD_GREEN
    );
    assert_eq!(
        elapsed_readout_color(TransportMode::Paused, false, "--:--"),
        transparent
    );
}

// The stopped readout must not react to a click, so it cannot switch the
// persisted elapsed/remaining mode while it clears the time (the `--:--`
// placeholder aside); the playing and paused readouts still toggle it.
#[test]
fn elapsed_readout_is_clickable_only_while_not_stopped() {
    assert!(elapsed_readout_clickable(TransportMode::Playing));
    assert!(elapsed_readout_clickable(TransportMode::Paused));
    assert!(!elapsed_readout_clickable(TransportMode::Stopped));
}

// The row must wire that predicate up, not just expose it: drive a real left
// press at the elapsed text's bounds and read the messages it publishes. A
// stopped press must publish none, while playing and paused publish
// `ToggleTimeDisplay`.
#[test]
fn elapsed_readout_toggles_only_while_not_stopped() {
    let mut renderer = layout_capture::headless_renderer();
    for (mode, expected) in [
        (TransportMode::Playing, true),
        (TransportMode::Paused, true),
        (TransportMode::Stopped, false),
    ] {
        let mut element: iced::Element<'_, Message, iced::Theme, iced::Renderer> = now_playing_row(
            Cow::Borrowed("00:53"),
            Cow::Borrowed("Opening"),
            Cow::Borrowed("Band"),
            Cow::Borrowed(""),
            Cow::Borrowed("3:30"),
            0,
            mode,
            true,
            None,
        )
        .into();
        let capture = layout_capture::capture_text(&mut element, &renderer);
        let elapsed = capture.find("00:53");
        let messages = layout_capture::publish_at(
            element,
            &mut renderer,
            &[iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
                iced::mouse::Button::Left,
            ))],
            iced::Point::new(elapsed.center_x(), elapsed.center_y()),
        );
        let toggles = messages
            .iter()
            .filter(|message| matches!(message, Message::ToggleTimeDisplay))
            .count();
        assert_eq!(
            toggles,
            usize::from(expected),
            "a press on the {mode:?} readout must {} publish ToggleTimeDisplay",
            if expected { "still" } else { "not" }
        );
        if !expected {
            assert!(
                messages.is_empty(),
                "the stopped readout published {messages:?}"
            );
        }
    }
}

/// Lays out the sample "00:53 / Opening / Band" row in `mode`, with the
/// elapsed readout's blink phase `elapsed_visible`, and captures its text.
fn captured_sample_row(mode: TransportMode, elapsed_visible: bool) -> layout_capture::TextCapture {
    let mut element: iced::Element<'_, Message, iced::Theme, ()> = now_playing_row(
        Cow::Borrowed("00:53"),
        Cow::Borrowed("Opening"),
        Cow::Borrowed("Band"),
        Cow::Borrowed(""),
        Cow::Borrowed("3:30"),
        0,
        mode,
        elapsed_visible,
        None,
    )
    .into();
    layout_capture::capture_text(&mut element, &())
}

// The hidden blink phase draws the elapsed text transparent instead of
// removing it, so the text keeps the same bounds and the title, total, and
// status row stay in place.
#[test]
fn now_playing_row_keeps_the_elapsed_bounds_when_the_blink_hides_it() {
    let visible = captured_sample_row(TransportMode::Paused, true);
    let hidden = captured_sample_row(TransportMode::Paused, false);

    assert_eq!(visible.find("00:53"), hidden.find("00:53"));
    assert_eq!(
        visible.texts().collect::<Vec<_>>(),
        hidden.texts().collect::<Vec<_>>()
    );
}

// The glyph sits in a fixed-width slot, so switching from `▶` to `II` or `■`
// must not move the large elapsed readout (or, through it, the title line).
// A stopped readout with a real time is drawn transparent but still laid out
// at the same bounds, so its equal-x assertion pins that stopping does not
// move it.
#[test]
fn now_playing_row_keeps_the_elapsed_readout_in_place_across_transport_modes() {
    let mut elapsed_x: Option<f32> = None;
    for mode in [
        TransportMode::Playing,
        TransportMode::Paused,
        TransportMode::Stopped,
    ] {
        let capture = captured_sample_row(mode, true);
        let glyph = capture.find(transport_glyph(mode));
        let elapsed = capture.find("00:53");
        assert!(
            glyph.x < elapsed.x,
            "the glyph must draw before the elapsed readout in {mode:?}: {glyph:?} {elapsed:?}"
        );
        match elapsed_x {
            Some(expected) => assert!(
                (elapsed.x - expected).abs() < 0.5,
                "the elapsed readout moved in {mode:?}: {elapsed:?} vs x={expected}"
            ),
            None => elapsed_x = Some(elapsed.x),
        }
    }
}

// The bar's own measure of the line: the playlist number prefix's characters
// plus the label's, the artist's, and the `" - "` separator between them. The
// view windows the line when the measure exceeds the LCD's budget, and the
// marquee timer runs only while it does — so a measure that disagreed with the
// view's would leave the timer scrolling a line the bar shows whole, or idle
// while the bar windows one. The prefix counts, since the view draws it on the
// same line.
#[test]
fn display_len_counts_the_number_the_label_the_artist_and_the_separator() {
    assert_eq!(display_len("Opening", "", ""), 7);
    assert_eq!(display_len("Opening", "The Sample Band", ""), 25);
    // The prefix adds its own characters (`"12. "` is four).
    assert_eq!(display_len("Opening", "The Sample Band", "1. "), 28);
    assert_eq!(display_len("Opening", "", "12. "), 11);
    // The separator is three characters, and a non-ASCII artist counts
    // characters rather than bytes.
    assert_eq!(display_len("Café", "Ångström", ""), 4 + 8 + 3);
}

// The line needs the marquee only when it does not fit the LCD's character
// budget: a fitting line is shown whole, so the timer would have nothing to
// scroll.
#[test]
fn needs_marquee_only_for_a_line_too_long_for_the_lcd() {
    // A label alone fills the budget exactly and still fits.
    assert!(!needs_marquee(&"x".repeat(MARQUEE_WIDTH), "", ""));
    assert!(needs_marquee(&"x".repeat(MARQUEE_WIDTH + 1), "", ""));
    // With an artist, the artist and the `" - "` separator count toward the
    // budget: "The Sample Band" is 15 characters, so a 4-character label fits
    // the 22-character budget and a 5-character one does not.
    let artist = "The Sample Band";
    assert!(!needs_marquee(&"x".repeat(4), artist, ""));
    assert!(needs_marquee(&"x".repeat(5), artist, ""));
    // The prefix counts too: a 4-character label that just fits with this
    // artist needs the marquee once a `"1. "` prefix joins it.
    assert!(!needs_marquee(&"x".repeat(4), artist, ""));
    assert!(needs_marquee(&"x".repeat(4), artist, "1. "));
}

// The row the Now Playing bar renders must use that window: an overlong line
// shows a different LCD text as the offset advances, while a fitting line
// still renders the separate borrowed artist and title at every offset.
#[test]
fn overlong_now_playing_line_renders_a_new_window_as_the_offset_advances() {
    let label = "A Title Much Longer Than The Display";
    let artist = "The Sample Band";
    let display = format!("{artist} - {label}");
    let first_window = marquee_text(&display, 0, MARQUEE_WIDTH);
    let second_window = marquee_text(&display, 1, MARQUEE_WIDTH);
    assert_ne!(first_window, second_window);

    let first = captured_now_playing_texts("00:53", label, artist, "", 0, TransportMode::Playing);
    let second = captured_now_playing_texts("00:53", label, artist, "", 1, TransportMode::Playing);
    assert!(first.iter().any(|text| text == &first_window), "{first:?}");
    assert!(
        second.iter().any(|text| text == &second_window),
        "{second:?}"
    );

    // A line that fits still renders the separate artist and title at every
    // offset: "Band" (4) plus the 3-character separator plus "Opening" (7)
    // is 14 characters, within the 22-character budget.
    let still_artist = "Band";
    let still_a = captured_now_playing_texts(
        "00:00",
        "Opening",
        still_artist,
        "",
        0,
        TransportMode::Playing,
    );
    let still_b = captured_now_playing_texts(
        "00:00",
        "Opening",
        still_artist,
        "",
        5,
        TransportMode::Playing,
    );
    assert_eq!(still_a, still_b);
    assert!(still_a.iter().any(|text| text == still_artist));
    assert!(still_a.iter().any(|text| text == "Opening"));
}

// The status row's prefix names the encoded bitrate and the sample rate in
// whole kilohertz: 44.1 kHz truncates to 44, 48 kHz reads 48, the bitrate
// value is kept, and an unknown bitrate is omitted.
#[test]
fn format_playback_prefix_formats_khz_and_bitrate() {
    assert_eq!(
        format_playback_prefix(PlaybackFormat {
            sample_rate_hz: 44_100,
            channels: 2,
            bitrate_kbps: Some(256),
        }),
        "256 KBPS  44 KHZ"
    );
    assert_eq!(
        format_playback_prefix(PlaybackFormat {
            sample_rate_hz: 44_100,
            channels: 2,
            bitrate_kbps: None,
        }),
        "44 KHZ"
    );
    assert_eq!(
        format_playback_prefix(PlaybackFormat {
            sample_rate_hz: 48_000,
            channels: 2,
            bitrate_kbps: None,
        }),
        "48 KHZ"
    );
}

// A track the library supplied without an artist must read as `Now Playing:
// <title>`: the row pushes the artist and its " - " separator only when the
// artist is non-empty, so an artist-less track shows no dangling separator.
// The app's empty state takes the same path — `now_playing_artist` answers ""
// for an unknown or an absent track (both pinned above), so the bar reads
/// Asserts that `expected` is one of the captured `texts`, naming `what` on failure.
fn assert_text_drawn(texts: &[String], expected: &str, what: &str) {
    assert!(
        texts.iter().any(|text| text == expected),
        "{what}: {texts:?}"
    );
}

/// Asserts that `unexpected` is none of the captured `texts`, naming `what` on failure.
fn assert_text_not_drawn(texts: &[String], unexpected: &str, what: &str) {
    assert!(
        !texts.iter().any(|text| text == unexpected),
        "{what}: {texts:?}"
    );
}

// "Now Playing: Nothing" before anything is played. Nothing pinned the row's
// half of that contract: the test above always passed an artist, and the
// empty-artist row was only constructed by the smoke test, never read back.
#[test]
fn now_playing_row_omits_the_artist_separator_when_the_track_has_no_artist() {
    let texts = captured_now_playing_texts("00:00", "Opening", "", "", 0, TransportMode::Playing);

    assert_text_drawn(&texts, "Opening", "the title must be drawn");
    assert_text_not_drawn(
        &texts,
        " - ",
        "a track without an artist must not draw the separator",
    );
    assert_text_drawn(&texts, "3:30", "the length readout must be drawn");
}

// The same artist-less track with a title too long for the LCD scrolls the
// title alone: the window is taken over the title, not over a " - <title>"
// string, so the scrolling line never starts with the separator the empty
// artist dropped. A regression that composed the window from the
// artist-prefixed string would show the separator and lose the title's first
// character instead.
#[test]
fn an_overlong_title_without_an_artist_scrolls_the_title_alone() {
    let label = "A Title Much Longer Than The Display";
    let window = marquee_text(label, 0, MARQUEE_WIDTH);

    let texts = captured_now_playing_texts("00:00", label, "", "", 0, TransportMode::Playing);

    assert_text_drawn(
        &texts,
        &window,
        "the window must be taken over the title alone",
    );
    assert_text_not_drawn(
        &texts,
        " - ",
        "a track without an artist must not draw the separator",
    );
}

// The playlist number comes between the caption and the artist, not before
// the caption or after the title: that is the mockup's `Now Playing: 1. The
// Sample Band - Opening` order. The fragments are read in draw order, so the
// indices pin the position as well as the presence of each string.
#[test]
fn now_playing_row_draws_the_number_between_the_caption_and_the_artist() {
    // A short line so the row takes its fitting branch and draws the number as
    // its own fragment; the overlong branch is pinned separately below.
    let texts =
        captured_now_playing_texts("00:00", "One", "Band", "1. ", 0, TransportMode::Playing);

    let caption = texts
        .iter()
        .position(|text| text == "Now Playing: ")
        .expect("the caption must be drawn");
    let number = texts
        .iter()
        .position(|text| text == "1. ")
        .expect("the number must be drawn");
    let artist = texts
        .iter()
        .position(|text| text == "Band")
        .expect("the artist must be drawn");
    let title = texts
        .iter()
        .position(|text| text == "One")
        .expect("the title must be drawn");
    assert!(
        caption < number && number < artist && artist < title,
        "expected caption, number, artist, title in order: {texts:?}"
    );
}

// An overlong line scrolls one window that includes the number prefix: the
// window is taken over the same `"<n>. <artist> - <title>"` string the bar
// draws whole when it fits, so the prefix cannot overflow the budget or stay
// fixed while the rest scrolls.
#[test]
fn an_overlong_now_playing_line_windows_the_number_with_the_title() {
    let label = "A Title Much Longer Than The Display";
    let artist = "The Sample Band";
    let number = "1. ";
    let display = format!("{number}{artist} - {label}");
    let window = marquee_text(&display, 0, MARQUEE_WIDTH);
    assert!(
        window.starts_with(number),
        "the window must include the prefix: {window:?}"
    );

    let texts =
        captured_now_playing_texts("00:00", label, artist, number, 0, TransportMode::Playing);

    assert_text_drawn(
        &texts,
        &window,
        "the window must be taken over the prefixed line",
    );
    assert_text_not_drawn(
        &texts,
        number,
        "the scrolling branch must not draw the prefix separately",
    );
}

// With no known number the row draws no prefix at all, whether the track has
// no artist, is unknown, or none is current. The row only pushes the fragment
// when the resolved number is non-empty, matching `now_playing_number`'s
// empty fallbacks.
#[test]
fn now_playing_row_draws_no_number_when_none_is_known() {
    let looks_like_number = |text: &str| {
        text.strip_suffix(". ")
            .is_some_and(|digits| !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()))
    };

    let artist_less =
        captured_now_playing_texts("00:00", "Opening", "", "", 0, TransportMode::Playing);
    assert!(
        !artist_less.iter().any(|text| looks_like_number(text)),
        "an artist-less track with no number must draw no prefix: {artist_less:?}"
    );

    let tracks = known_tracks(&[("song-1", "The Sample Band", "Opening")]);
    for current_track in [Some("no-such-song"), None] {
        let label = now_playing_label(&tracks, current_track);
        let artist = now_playing_artist(&tracks, current_track);
        let number = now_playing_number(&tracks, current_track);
        assert_eq!(number, "");
        let texts = captured_now_playing_texts(
            "00:00",
            &label,
            &artist,
            &number,
            0,
            TransportMode::Playing,
        );
        assert!(
            !texts.iter().any(|text| looks_like_number(text)),
            "an unknown or absent track must draw no prefix: {texts:?}"
        );
    }
}

#[test]
fn format_track_time_formats_minutes_and_zero_padded_seconds() {
    assert_eq!(format_track_time(210_000), "3:30");
    assert_eq!(format_track_time(45_000), "0:45");
}

// The doc comment promises a track past an hour keeps counting minutes
// (`60:00`, `61:05`) rather than rolling into an `h:mm:ss` hours field,
// and the tests above stop at 3:30. Pin the boundary: an exact hour is
// `60:00` (not `0:00`, and not `1:00:00`) and an hour plus a remainder
// keeps the same `m:ss` shape, so a regression that special-cased hours or
// reset the minute counter at 60 fails here.
#[test]
fn format_track_time_keeps_counting_minutes_past_an_hour() {
    assert_eq!(format_track_time(3_600_000), "60:00");
    assert_eq!(format_track_time(3_665_000), "61:05");
}

// `0` is the only "source supplied no duration" sentinel (see
// `Song::duration_ms`); every nonzero value is a real length and formats by
// truncating its sub-second remainder. Pin the boundary so a regression that
// treated a duration under one second as missing would show `--:--` for a
// real short track instead of `0:00`, while the first full second still
// advances the display.
#[test]
fn format_track_time_distinguishes_no_duration_from_a_sub_second_track() {
    assert_eq!(format_track_time(1), "0:00");
    assert_eq!(format_track_time(999), "0:00");
    assert_eq!(format_track_time(1_000), "0:01");
}

// The bar is rebuilt on every frame, and `now_playing_label` documents
// that the common cases borrow — the title straight out of the id→title
// index, or the static "Nothing" literal — so only the rare unknown-id
// fallback allocates. The value-equality tests above pass even if the
// function clones the title into a `Cow::Owned`, so pin the variants
// (and, for the known title, that the borrow points into the caller's
// map) here; otherwise a reintroduced clone would silently allocate per
// frame.
#[test]
fn now_playing_label_borrows_the_title_and_nothing_literal() {
    let tracks = known_tracks(&[("song-1", "The Sample Band", "Opening")]);

    let label = now_playing_label(&tracks, Some("song-1"));
    assert!(matches!(&label, Cow::Borrowed(_)));
    assert_eq!(label.as_ptr(), tracks["song-1"].title.as_ptr());

    let label = now_playing_label(&tracks, None);
    assert!(matches!(&label, Cow::Borrowed(_)));

    // The one allocating case stays owned: the unknown id is copied into
    // the fallback label.
    let label = now_playing_label(&tracks, Some("no-such-song"));
    assert!(matches!(&label, Cow::Owned(_)));
}

// A mixer readout replaces the caption and the title, even a marquee-length
// one, while the glyph, elapsed readout, and total stay.
#[test]
fn now_playing_row_with_a_readout_replaces_the_title() {
    let long = "A Title Too Long For The LCD Bar Indeed";
    let mut element: iced::Element<'_, Message, iced::Theme, ()> = now_playing_row(
        Cow::Borrowed("00:53"),
        Cow::Borrowed(long),
        Cow::Borrowed("Band"),
        Cow::Borrowed("1. "),
        Cow::Borrowed("3:30"),
        3,
        TransportMode::Playing,
        true,
        Some("Volume: 72%".to_string()),
    )
    .into();
    let capture = layout_capture::capture_text(&mut element, &());
    let texts: Vec<&str> = capture.texts().collect();
    assert_eq!(
        texts,
        vec![
            transport_glyph(TransportMode::Playing),
            "00:53",
            "Volume: 72%",
            "3:30"
        ]
    );
}

#[test]
fn dragged_marquee_offset_follows_the_pointer_and_wraps() {
    use crate::ui::views::dragged_marquee_offset;
    assert_eq!(dragged_marquee_offset(0, -25.0, 30), 2);
    assert_eq!(dragged_marquee_offset(0, 10.0, 30), 29);
    assert_eq!(dragged_marquee_offset(5, 9.0, 30), 5);
    assert_eq!(dragged_marquee_offset(5, -9.0, 30), 5);
    assert_eq!(dragged_marquee_offset(35, 0.0, 30), 5);
}

#[test]
fn marquee_cycle_len_is_the_line_plus_its_gap() {
    use crate::ui::views::marquee_cycle_len;
    // "3. " + "Title" + " - " + "Band" = 3 + 5 + 3 + 4 = 15, plus the 7-character separator.
    assert_eq!(marquee_cycle_len("Title", "Band", "3. "), 22);
    assert_eq!(marquee_cycle_len("Title", "", ""), 12);
}

#[test]
fn the_overlong_title_publishes_a_drag_press_and_a_fitting_one_does_not() {
    let mut renderer = layout_capture::headless_renderer();
    for (label, expected) in [
        ("A Title Much Longer Than The Display", true),
        ("Short", false),
    ] {
        let mut element: iced::Element<'_, Message, iced::Theme, iced::Renderer> = now_playing_row(
            Cow::Borrowed("00:53"),
            Cow::Borrowed(label),
            Cow::Borrowed(""),
            Cow::Borrowed(""),
            Cow::Borrowed("3:30"),
            0,
            TransportMode::Playing,
            true,
            None,
        )
        .into();
        let capture = layout_capture::capture_text(&mut element, &renderer);
        let title = capture.find(if expected {
            "A Title Much Longer Th"
        } else {
            "Short"
        });
        let messages = layout_capture::publish_at(
            element,
            &mut renderer,
            &[iced::Event::Mouse(iced::mouse::Event::ButtonPressed(
                iced::mouse::Button::Left,
            ))],
            iced::Point::new(title.center_x(), title.center_y()),
        );
        let presses = messages
            .iter()
            .filter(|message| matches!(message, Message::TitleDragPressed))
            .count();
        assert_eq!(presses, usize::from(expected), "{label}: {messages:?}");
        if !expected {
            assert!(messages.is_empty(), "{label}: {messages:?}");
        }
    }
}
