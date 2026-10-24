use super::*;

// The channel label mapping follows the reported channel count: one channel
// is mono, and any other count is stereo.
#[test]
fn channel_mode_label_names_the_reported_channels() {
    assert_eq!(channel_mode_label(1), "MONO");
    assert_eq!(channel_mode_label(2), "STEREO");
    assert_eq!(channel_mode_label(0), "STEREO");
}

// The status row lights only the reported channel mode: stereo lights
// `STEREO` and dims `MONO`, mono does the reverse, and no reported format
// leaves both labels dim. The exact palette values are pinned here too, so a
// recolored dim green is caught rather than shipped.
#[test]
fn channel_label_color_lights_only_the_reported_mode() {
    assert_eq!(channel_label_color(Some(2), "STEREO"), theme::LCD_GREEN);
    assert_eq!(channel_label_color(Some(2), "MONO"), theme::LCD_GREEN_DIM);
    assert_eq!(channel_label_color(Some(1), "MONO"), theme::LCD_GREEN);
    assert_eq!(channel_label_color(Some(1), "STEREO"), theme::LCD_GREEN_DIM);
    assert_eq!(channel_label_color(None, "MONO"), theme::LCD_GREEN_DIM);
    assert_eq!(channel_label_color(None, "STEREO"), theme::LCD_GREEN_DIM);
    assert_eq!(theme::LCD_GREEN, iced::Color::from_rgb(0.0, 1.0, 0.0));
    assert_eq!(theme::LCD_GREEN_DIM, iced::Color::from_rgb(0.0, 0.35, 0.0));
}

/// Lays the LCD status row out with the headless tiny-skia renderer and returns
/// the text fragments it draws, the RGBA pixels it paints, and its height.
fn captured_status_row(
    format: Option<PlaybackFormat>,
) -> (layout_capture::TextCapture, layout_capture::PixelCapture) {
    let mut renderer = layout_capture::headless_renderer();
    let mut element: iced::Element<'_, Message, iced::Theme, iced::Renderer> =
        now_playing_status_row(format).into();
    layout_capture::capture_text_and_pixels(&mut element, &mut renderer, &iced::Theme::Dark)
}

// The status row lays out in the mockup's two ends: the bitrate/sample-rate
// prefix at the left edge and both channel labels at the right. It lights only
// the reported channel mode by drawing it brighter green than the other label,
// so a row that drew both labels lit would fail here even though its text
// still matched. With no reported format both labels stay dim, and the row
// keeps its fixed height either way so the bar does not jump when a preview
// loads.
#[test]
fn now_playing_status_row_lights_the_reported_channel_mode() {
    let (capture, pixels) = captured_status_row(Some(PlaybackFormat {
        sample_rate_hz: 44_100,
        channels: 2,
        bitrate_kbps: Some(256),
    }));
    assert_eq!(
        capture.texts().collect::<Vec<_>>(),
        vec!["256 KBPS  44 KHZ", "MONO", "STEREO"]
    );
    let prefix = capture.find("256 KBPS  44 KHZ");
    let mono = capture.find("MONO");
    let stereo = capture.find("STEREO");
    assert!(
        prefix.x < 10.0,
        "the prefix must sit at the row's left edge: {prefix:?}"
    );
    assert!(
        mono.x > prefix.x + prefix.width,
        "the labels must follow the prefix, not overlap it: prefix {prefix:?}, mono {mono:?}"
    );
    assert!(
        stereo.x + stereo.width > 990.0,
        "the label pair must sit at the row's right edge: {stereo:?}"
    );

    let stereo_green = pixels.max_green_in(stereo);
    let mono_green = pixels.max_green_in(mono);
    assert!(
        stereo_green > mono_green + 100,
        "stereo must be lit and mono dim: stereo green {stereo_green}, mono green {mono_green}"
    );

    let (mono_capture, mono_pixels) = captured_status_row(Some(PlaybackFormat {
        sample_rate_hz: 44_100,
        channels: 1,
        bitrate_kbps: None,
    }));
    let lit_mono = mono_pixels.max_green_in(mono_capture.find("MONO"));
    let dim_stereo = mono_pixels.max_green_in(mono_capture.find("STEREO"));
    assert!(
        lit_mono > dim_stereo + 100,
        "mono must be lit and stereo dim: mono green {lit_mono}, stereo green {dim_stereo}"
    );

    let (blank, blank_pixels) = captured_status_row(None);
    assert_eq!(blank.texts().collect::<Vec<_>>(), vec!["MONO", "STEREO"]);
    let blank_mono = blank_pixels.max_green_in(blank.find("MONO"));
    let blank_stereo = blank_pixels.max_green_in(blank.find("STEREO"));
    assert!(
        blank_mono < 128 && blank_stereo < 128,
        "no reported format must leave both labels dim: mono green {blank_mono}, \
         stereo green {blank_stereo}"
    );
    assert!(
        blank.find("STEREO").x + blank.find("STEREO").width > 990.0,
        "the dim label pair must still sit at the row's right edge"
    );
    assert_eq!(
        capture.size().height,
        blank.size().height,
        "the status row must keep its height whether or not a format is reported"
    );
    assert_eq!(capture.size().height, 16.0, "the status row's fixed height");
}

/// The visualizer's bar heights come straight from the reported bands: a
/// missing spectrum is all zero, each band scales the row height, and values
/// outside `0.0..=1.0` clamp to the strip's bounds.
#[test]
fn visualizer_bar_heights_scale_and_clamp_the_reported_bands() {
    let mut bands = [0.0; BAND_COUNT];
    bands[0] = 0.0;
    bands[1] = 0.5;
    bands[2] = 1.0;
    bands[3] = 2.0;
    let heights = visualizer_bar_heights(Some(Spectrum { bands }));

    assert_eq!(heights.len(), BAND_COUNT);
    assert_eq!(heights[0], 0.0);
    assert_eq!(heights[1], 12.0);
    assert_eq!(heights[2], 24.0);
    assert_eq!(heights[3], 24.0);
    assert_eq!(&heights[4..], &[0.0; BAND_COUNT - 4]);
    assert_eq!(visualizer_bar_heights(None), [0.0; BAND_COUNT]);
}

// The clamp's lower edge: a spectrum carrying a negative band must not draw a
// bar below the strip. The test above pins the upper edge, so a regression to
// `min(1.0)` would keep every existing assertion green while a negative band
// produced a negative height.
#[test]
fn visualizer_bar_heights_floors_a_negative_band_at_zero() {
    let mut bands = [0.0; BAND_COUNT];
    bands[0] = -1.0;
    bands[1] = -0.5;
    bands[2] = 0.25;
    let heights = visualizer_bar_heights(Some(Spectrum { bands }));

    assert_eq!(
        &heights[..3],
        &[0.0, 0.0, 6.0],
        "a negative band must clamp to the strip's bottom"
    );
}

// `f32::clamp` passes NaN through unchanged, and a NaN height would poison the
// visualizer row's layout. A stub backend can report one, so the documented
// `0.0..=1.0` clamp must map it to an empty bar like the shared NaN-safe clamp
// does for the sliders. Pin it beside a finite band so the rest of the row
// still scales normally.
#[test]
fn visualizer_bar_heights_maps_a_nan_band_to_empty() {
    let mut bands = [0.0; BAND_COUNT];
    bands[0] = f32::NAN;
    bands[1] = 0.5;
    let heights = visualizer_bar_heights(Some(Spectrum { bands }));

    assert_eq!(heights[0], 0.0, "a NaN band must draw an empty bar");
    assert_eq!(heights[1], 12.0);
    assert!(
        heights.iter().all(|height| height.is_finite()),
        "every bar height must be finite: {heights:?}"
    );
}

/// Lays out the LCD content for `spectrum` and returns the text fragments it
/// would draw, each fragment's bounds, the height the content occupies, and
/// the bounds of every container it draws.
fn captured_content_with_spectrum(
    spectrum: Option<Spectrum>,
) -> (Vec<String>, Vec<Rectangle>, f32, Vec<Rectangle>) {
    let mut element: iced::Element<'_, Message, iced::Theme, ()> = now_playing_content(
        Cow::Borrowed("00:53"),
        Cow::Borrowed("Opening"),
        Cow::Borrowed("Band"),
        Cow::Borrowed(""),
        Cow::Borrowed("3:30"),
        0,
        Some(PlaybackFormat {
            sample_rate_hz: 44_100,
            channels: 2,
            bitrate_kbps: Some(256),
        }),
        spectrum,
        TransportMode::Playing,
        true,
        None,
    )
    .into();
    let capture = layout_capture::capture_text(&mut element, &());
    let height = capture.size().height;
    let texts = capture.texts().map(str::to_string).collect();
    let bounds = capture
        .fragments()
        .iter()
        .map(|(_, bounds)| *bounds)
        .collect();
    let mut tree = Tree::new(element.as_widget());
    let limits = Limits::new(Size::ZERO, Size::new(1000.0, 1000.0));
    let node = element.as_widget_mut().layout(&mut tree, &(), &limits);
    let mut containers = CapturedContainers::default();
    element
        .as_widget_mut()
        .operate(&mut tree, Layout::new(&node), &(), &mut containers);
    (texts, bounds, height, containers.0)
}

// The visualizer row keeps its fixed height whether a preview reports a
// spectrum or not, so loading one leaves the title and status text in place.
// The status text sits at least one visualizer-row height below the title, so
// the row is really wired between them rather than merely adding height.
#[test]
fn now_playing_content_keeps_the_visualizer_height_with_and_without_a_spectrum() {
    let mut bands = [0.0; BAND_COUNT];
    bands[0] = 1.0;
    let (reporting, reporting_bounds, reporting_height, reporting_containers) =
        captured_content_with_spectrum(Some(Spectrum { bands }));
    let (blank, _blank_bounds, blank_height, _blank_containers) =
        captured_content_with_spectrum(None);

    assert!(
        reporting.iter().any(|text| text == "Opening"),
        "the title must still draw: {reporting:?}"
    );
    assert!(
        reporting.iter().any(|text| text == "256 KBPS  44 KHZ"),
        "the status row must still draw its prefix: {reporting:?}"
    );
    assert!(
        CHANNEL_LABELS
            .iter()
            .all(|label| reporting.iter().any(|text| text == label)),
        "the status row must draw both channel labels: {reporting:?}"
    );
    assert!(
        blank.iter().any(|text| text == "Opening"),
        "the title must draw without a spectrum: {blank:?}"
    );

    let fragment_y = |text: &str| -> f32 {
        reporting
            .iter()
            .position(|drawn| drawn == text)
            .map(|index| reporting_bounds[index].y)
            .unwrap_or_else(|| panic!("missing {text}: {reporting:?}"))
    };
    let title_y = fragment_y("Opening");
    let status_y = fragment_y("256 KBPS  44 KHZ");
    assert!(
        status_y - title_y >= 24.0,
        "the 24px visualizer row must sit between the title and status rows: \
         title y {title_y}, status y {status_y}"
    );
    for label in CHANNEL_LABELS {
        let label_y = fragment_y(label);
        assert!(
            label_y - title_y >= 24.0,
            "the {label} label must also sit below the visualizer row: \
             title y {title_y}, label y {label_y}"
        );
    }

    // The bars themselves, not just the extra height, must sit between the
    // title and status text. Moving the visualizer row above the title or
    // below the status row leaves the spacing above intact, so check the bar
    // containers' own bounds as well.
    let bars: Vec<Rectangle> = reporting_containers
        .iter()
        .copied()
        .filter(|bounds| bounds.width == 8.0)
        .collect();
    assert_eq!(
        bars.len(),
        BAND_COUNT,
        "the visualizer row must draw one bar per band: {bars:?}"
    );
    assert!(
        bars.iter().all(|bar| bar.y >= title_y),
        "the visualizer bars must sit below the title: title y {title_y}, bars {bars:?}"
    );
    assert!(
        bars.iter().all(|bar| bar.y + bar.height <= status_y),
        "the visualizer bars must sit above the status row: status y {status_y}, bars {bars:?}"
    );
    assert_eq!(
        reporting_height, blank_height,
        "the visualizer row must keep its height whether or not a spectrum is reported"
    );
}

// The visualizer row keeps the fixed height even when every bar is empty; the
// silent bands still occupy a zero-height fixed-width slot inside the row.
#[test]
fn now_playing_visualizer_row_keeps_its_height_without_a_spectrum() {
    for spectrum in [
        None,
        Some(Spectrum {
            bands: [0.0; BAND_COUNT],
        }),
    ] {
        let mut element: iced::Element<'_, Message, iced::Theme, ()> =
            now_playing_visualizer_row(spectrum).into();
        let mut tree = Tree::new(element.as_widget());
        let limits = Limits::new(Size::ZERO, Size::new(1000.0, 1000.0));
        let node = element.as_widget_mut().layout(&mut tree, &(), &limits);
        assert_eq!(node.bounds().height, 24.0);
    }
}

// The row draws exactly one fixed-width bar per band, scaled by the band
// level, so a refactor cannot collapse the visualizer to a single bar while
// the pure height helper stays tested. The bands include zeros because a real
// spectrum does: iced's `Row` drops a `Fixed(0.0)`-height child, so this is
// the state that would silently lose the silent bars.
#[test]
fn now_playing_visualizer_row_draws_one_fixed_width_bar_per_band() {
    let mut bands = [0.0; BAND_COUNT];
    for (index, band) in bands.iter_mut().enumerate() {
        *band = (index % 3) as f32 * 0.25;
    }
    let mut element: iced::Element<'_, Message, iced::Theme, ()> =
        now_playing_visualizer_row(Some(Spectrum { bands })).into();
    let mut tree = Tree::new(element.as_widget());
    let limits = Limits::new(Size::ZERO, Size::new(1000.0, 1000.0));
    let node = element.as_widget_mut().layout(&mut tree, &(), &limits);

    let mut captured = CapturedContainers::default();
    element
        .as_widget_mut()
        .operate(&mut tree, Layout::new(&node), &(), &mut captured);
    let bars: Vec<_> = captured
        .0
        .iter()
        .filter(|bounds| bounds.width == 8.0)
        .collect();
    assert_eq!(
        bars.len(),
        BAND_COUNT,
        "the row must draw one bar per band, including silent bands"
    );
    for (bar, band) in bars.iter().zip(bands) {
        assert_eq!(bar.width, 8.0, "each bar keeps its fixed width: {bar:?}");
        assert_eq!(
            bar.height,
            band * 24.0,
            "a {band} band draws a proportionally tall bar: {bar:?}"
        );
    }
}

// The seek bar's thumb never exceeds the preview's own length, and a missing
// position, a missing length, or a zero length hides the bar entirely: the
// bar only appears once both a position and a scale are reported.
#[test]
fn seek_bar_value_clamps_the_thumb_and_hides_without_a_length() {
    assert_eq!(
        seek_bar_value(Some(Duration::from_secs(45)), Some(Duration::from_secs(30))),
        Some((30.0, 30.0))
    );
    assert_eq!(seek_bar_value(None, Some(Duration::from_secs(30))), None);
    assert_eq!(seek_bar_value(Some(Duration::from_secs(5)), None), None);
    assert_eq!(
        seek_bar_value(Some(Duration::from_secs(5)), Some(Duration::ZERO)),
        None
    );
    assert_eq!(seek_bar_value(None, None), None);
}

// Both shapes of the strip must construct: the slider when a preview length
// is known and the empty fixed-height stand-in when it is not.
#[test]
fn view_seek_bar_constructs_with_and_without_a_length() {
    let _known = view_seek_bar(Some(Duration::from_secs(10)), Some(Duration::from_secs(30)));
    let _unknown = view_seek_bar(Some(Duration::from_secs(10)), None);
}
