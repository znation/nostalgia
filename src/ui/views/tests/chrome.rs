use super::*;

// The app bar's drag region spans only the space between the M button and the
// shade button. Putting the tapes inside it must center NOSTALGIA in that
// middle span, not in the whole window, while the edge controls keep their
// pinned places.
#[test]
fn title_bar_centers_its_name_between_its_edge_controls() {
    let renderer = layout_capture::headless_renderer();
    let mut element = view_title_bar(false);
    let capture = layout_capture::capture_text(&mut element, &renderer);

    let mut tree = Tree::new(element.as_widget());
    let node = element
        .as_widget_mut()
        .layout(&mut tree, &renderer, &layout_capture::limits());
    let mut containers = CapturedContainers::default();
    element
        .as_widget_mut()
        .operate(&mut tree, Layout::new(&node), &renderer, &mut containers);

    // The M button and the shade button each draw a container at their pinned
    // width. Read the M slot's right edge and the leftmost 22px button's left
    // edge from the laid-out tree rather than from their glyph bounds, whose
    // measured widths vary with the renderer.
    let menu_right = containers
        .0
        .iter()
        .filter(|bounds| (bounds.width - TITLE_BAR_LEFT_BUTTON_WIDTH).abs() < 0.5)
        .map(|bounds| bounds.x + bounds.width)
        .fold(f32::NEG_INFINITY, f32::max);
    let shade_left = containers
        .0
        .iter()
        .filter(|bounds| (bounds.width - TITLE_BAR_BUTTON_WIDTH).abs() < 0.5)
        .map(|bounds| bounds.x)
        .fold(f32::INFINITY, f32::min);
    let middle_center = (menu_right + shade_left) / 2.0;

    let name = capture.find("NOSTALGIA");
    let name_center = name.x + name.width / 2.0;
    assert!(
        (name_center - middle_center).abs() < 0.5,
        "NOSTALGIA must center between the M and shade controls: {name:?}, \
         span {menu_right:?}..{shade_left:?}"
    );
}

// The title bar's left slot is the classic main-menu M button, and a press
// publishes the About toggle. It must not still be an always-on-top control:
// the LCD strip's A keeps that job.
#[test]
fn title_bar_m_button_publishes_toggle_about() {
    let mut renderer = layout_capture::headless_renderer();
    let mut element = view_title_bar(false);
    let capture = layout_capture::capture_text(&mut element, &renderer);
    let menu = capture.find("M");
    let messages = layout_capture::click_at(
        element,
        &mut renderer,
        iced::Point::new(menu.center_x(), menu.center_y()),
    );
    assert_eq!(
        messages
            .iter()
            .filter(|message| matches!(message, Message::ToggleAbout))
            .count(),
        1,
        "a press on M must publish exactly one ToggleAbout: {messages:?}"
    );
    assert!(
        !messages
            .iter()
            .any(|message| matches!(message, Message::ToggleAlwaysOnTop)),
        "the title bar's left slot must no longer toggle always-on-top: {messages:?}"
    );
}

/// The About overlay over a blank full-space base, so its layout is measured
/// against the same window limits as the root view.
fn about_overlay_element(signed_in: bool) -> Element<'static, Message> {
    let base: Element<'static, Message> = iced::widget::Space::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
    super::super::view_about_overlay(base, signed_in)
}

#[test]
fn about_overlay_draws_the_name_version_tagline_and_library_source() {
    let sources = [
        (false, "Library: Sample library"),
        (true, "Library: Apple Music"),
    ];
    for (signed_in, library_source) in sources {
        let mut element = about_overlay_element(signed_in);
        let capture =
            layout_capture::capture_text(&mut element, &layout_capture::headless_renderer());
        let texts: Vec<&str> = capture.texts().collect();
        for expected in [
            "ABOUT NOSTALGIA",
            concat!("v", env!("CARGO_PKG_VERSION")),
            "The classic Winamp look, rebuilt in Rust — with Apple Music as the library.",
            library_source,
            "Close",
        ] {
            assert!(
                texts.contains(&expected),
                "the About overlay must draw {expected:?} (signed in: {signed_in}): {texts:?}"
            );
        }
    }
}

// Close and the backdrop each dismiss the panel with one ToggleAbout; a press
// on the panel's own text chrome is swallowed with Ignored, so the panel stays
// open.
#[test]
fn about_overlay_closes_from_close_and_backdrop_but_ignores_its_chrome() {
    let mut renderer = layout_capture::headless_renderer();
    for target in ["Close", "ABOUT NOSTALGIA", "BACKDROP"] {
        let mut element = about_overlay_element(false);
        let capture = layout_capture::capture_text(&mut element, &renderer);
        let point = if target == "BACKDROP" {
            iced::Point::new(2.0, capture.size().height - 2.0)
        } else {
            let bounds = capture.find(target);
            iced::Point::new(bounds.center_x(), bounds.center_y())
        };
        let messages = layout_capture::click_at(element, &mut renderer, point);
        let toggles = messages
            .iter()
            .filter(|message| matches!(message, Message::ToggleAbout))
            .count();
        if target == "ABOUT NOSTALGIA" {
            assert_eq!(
                toggles, 0,
                "the panel's chrome must not dismiss it: {messages:?}"
            );
            assert_eq!(
                messages.len(),
                1,
                "the panel must publish exactly one message: {messages:?}"
            );
            assert!(
                matches!(messages.first(), Some(Message::Ignored)),
                "the panel must swallow the press with Ignored: {messages:?}"
            );
        } else {
            assert_eq!(
                toggles, 1,
                "{target} must publish exactly one ToggleAbout: {messages:?}"
            );
        }
    }
}

// The panel is modal for the wheel too: a scroll over the backdrop or the
// panel is captured, so a wheel handler in the base beneath never fires.
#[test]
fn about_overlay_captures_the_wheel_from_the_base_beneath() {
    let mut renderer = layout_capture::headless_renderer();
    for over_panel in [false, true] {
        let base: Element<'static, Message> = iced::widget::MouseArea::new(
            iced::widget::Space::new()
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .on_scroll(|_| Message::ToggleEqualizerPanel)
        .into();
        let mut element = super::super::view_about_overlay(base, false);
        let capture = layout_capture::capture_text(&mut element, &renderer);
        let point = if over_panel {
            let bounds = capture.find("ABOUT NOSTALGIA");
            iced::Point::new(bounds.center_x(), bounds.center_y())
        } else {
            iced::Point::new(2.0, capture.size().height - 2.0)
        };
        let messages = layout_capture::publish_at(
            element,
            &mut renderer,
            &[iced::Event::Mouse(iced::mouse::Event::WheelScrolled {
                delta: iced::mouse::ScrollDelta::Lines { x: 0.0, y: 1.0 },
            })],
            point,
        );
        assert!(
            !messages
                .iter()
                .any(|message| matches!(message, Message::ToggleEqualizerPanel)),
            "a wheel over the overlay (panel: {over_panel}) must not reach the base: {messages:?}"
        );
    }
}

// A section band must be exactly the title bar's height and carry only its
// centered label between the two filler tapes. The root-view tests check where
// the bands appear; this pins the band widget's own geometry and that no
// button or stray text rides along with the decorative label.
#[test]
fn title_band_centers_its_label_at_the_title_bar_height() {
    let mut element = view_title_band("NOSTALGIA EQUALIZER");
    let capture = layout_capture::capture_text(&mut element, &layout_capture::headless_renderer());

    assert_eq!(capture.size().height, TITLE_BAR_HEIGHT);
    let texts: Vec<&str> = capture.texts().collect();
    assert_eq!(texts, vec!["NOSTALGIA EQUALIZER"]);

    let label = capture.find("NOSTALGIA EQUALIZER");
    let label_center = label.x + label.width / 2.0;
    let band_center = capture.size().width / 2.0;
    assert!(
        (label_center - band_center).abs() < 0.5,
        "the section label must be centered in its band: {label:?} in {:?}",
        capture.size()
    );
}

fn file_info_element(track: &KnownTrack, format: Option<PlaybackFormat>) -> Element<'_, Message> {
    let base: Element<'_, Message> = iced::widget::Space::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
    super::super::view_file_info_overlay(base, track, format)
}

fn sample_known_track(artist: &str) -> KnownTrack {
    KnownTrack {
        title: "Opening".to_string(),
        artist: artist.to_string(),
        album: "First Record".to_string(),
        year: None,
        genre: String::new(),
        track_number: None,
        number: "1. ".to_string(),
        time: "3:30".to_string(),
        duration_ms: 210_000,
    }
}

fn file_info_texts(track: &KnownTrack, format: Option<PlaybackFormat>) -> Vec<String> {
    let mut element = file_info_element(track, format);
    let capture = layout_capture::capture_text(&mut element, &layout_capture::headless_renderer());
    capture.texts().map(str::to_owned).collect()
}

fn assert_no_file_info_line(texts: &[String], prefix: &str) {
    assert!(
        texts.iter().all(|text| !text.starts_with(prefix)),
        "the panel must draw no {prefix:?} line: {texts:?}"
    );
}

#[test]
fn file_info_overlay_draws_the_title_artist_album_and_length() {
    let track = sample_known_track("The Sample Band");
    let texts = file_info_texts(&track, None);
    for expected in [
        "FILE INFO",
        "Title: Opening",
        "Artist: The Sample Band",
        "Album: First Record",
        "Length: 3:30",
        "Close",
    ] {
        assert!(
            texts.iter().any(|text| text == expected),
            "the File info overlay must draw {expected:?}: {texts:?}"
        );
    }

    let artistless = sample_known_track("");
    let texts = file_info_texts(&artistless, None);
    assert_no_file_info_line(&texts, "Artist:");
}

#[test]
fn file_info_overlay_omits_the_album_line_when_the_source_supplied_none() {
    let mut track = sample_known_track("The Sample Band");
    track.album = String::new();
    let texts = file_info_texts(&track, None);
    assert_no_file_info_line(&texts, "Album:");
}

#[test]
fn file_info_overlay_draws_the_tracks_year() {
    let mut track = sample_known_track("The Sample Band");
    track.year = Some(2001);
    let texts = file_info_texts(&track, None);
    let album_at = texts
        .iter()
        .position(|text| text == "Album: First Record")
        .unwrap();
    let year_at = texts.iter().position(|text| text == "Year: 2001").unwrap();
    let length_at = texts
        .iter()
        .position(|text| text == "Length: 3:30")
        .unwrap();
    assert!(
        album_at < year_at && year_at < length_at,
        "the year line must sit between Album and Length: {texts:?}"
    );
}

#[test]
fn file_info_overlay_omits_the_year_line_without_one() {
    let track = sample_known_track("The Sample Band");
    let texts = file_info_texts(&track, None);
    assert_no_file_info_line(&texts, "Year:");
}

#[test]
fn file_info_overlay_draws_the_tracks_genre() {
    let mut track = sample_known_track("The Sample Band");
    track.year = Some(2001);
    track.genre = "Rock".to_string();
    let texts = file_info_texts(&track, None);
    let year_at = texts.iter().position(|text| text == "Year: 2001").unwrap();
    let genre_at = texts.iter().position(|text| text == "Genre: Rock").unwrap();
    let length_at = texts
        .iter()
        .position(|text| text == "Length: 3:30")
        .unwrap();
    assert!(
        year_at < genre_at && genre_at < length_at,
        "the genre line must sit between Year and Length: {texts:?}"
    );
}

#[test]
fn file_info_overlay_omits_the_genre_line_when_the_source_supplied_none() {
    let track = sample_known_track("The Sample Band");
    let texts = file_info_texts(&track, None);
    assert_no_file_info_line(&texts, "Genre:");
}

#[test]
fn file_info_overlay_draws_the_tracks_track_number() {
    let mut track = sample_known_track("The Sample Band");
    track.year = Some(2001);
    track.genre = "Rock".to_string();
    track.track_number = Some(3);
    let texts = file_info_texts(&track, None);
    let genre_at = texts.iter().position(|text| text == "Genre: Rock").unwrap();
    let number_at = texts.iter().position(|text| text == "Track #: 3").unwrap();
    let length_at = texts
        .iter()
        .position(|text| text == "Length: 3:30")
        .unwrap();
    assert!(
        genre_at < number_at && number_at < length_at,
        "the track-number line must sit between Genre and Length: {texts:?}"
    );
}

#[test]
fn file_info_overlay_omits_the_track_number_line_when_the_source_supplied_none() {
    let track = sample_known_track("The Sample Band");
    let texts = file_info_texts(&track, None);
    assert_no_file_info_line(&texts, "Track #:");
}

#[test]
fn file_info_overlay_draws_the_loaded_previews_format() {
    let track = sample_known_track("The Sample Band");
    let stereo = PlaybackFormat {
        sample_rate_hz: 44_100,
        channels: 2,
        bitrate_kbps: Some(256),
    };
    let texts = file_info_texts(&track, Some(stereo));
    let format_line = "Format: 256 KBPS  44 KHZ STEREO";
    assert!(
        texts.iter().any(|text| text == format_line),
        "a loaded stereo preview must draw {format_line:?}: {texts:?}"
    );
    let format_at = texts.iter().position(|text| text == format_line).unwrap();
    let length_at = texts
        .iter()
        .position(|text| text == "Length: 3:30")
        .unwrap();
    let close_at = texts.iter().position(|text| text == "Close").unwrap();
    assert!(
        length_at < format_at && format_at < close_at,
        "the format line must sit after Length and before Close: {texts:?}"
    );

    let mono = PlaybackFormat {
        sample_rate_hz: 48_000,
        channels: 1,
        bitrate_kbps: None,
    };
    let texts = file_info_texts(&track, Some(mono));
    assert!(
        texts.iter().any(|text| text == "Format: 48 KHZ MONO"),
        "a mono preview without a computed bitrate must omit the bitrate: {texts:?}"
    );
}

#[test]
fn file_info_overlay_omits_the_format_line_without_a_loaded_preview() {
    let track = sample_known_track("The Sample Band");
    let texts = file_info_texts(&track, None);
    assert_no_file_info_line(&texts, "Format:");
}

#[test]
fn file_info_overlay_closes_from_close_and_backdrop_but_ignores_its_chrome() {
    let track = sample_known_track("The Sample Band");
    let mut renderer = layout_capture::headless_renderer();
    for target in ["Close", "Title: Opening", "BACKDROP"] {
        let mut element = file_info_element(&track, None);
        let capture = layout_capture::capture_text(&mut element, &renderer);
        let point = if target == "BACKDROP" {
            iced::Point::new(2.0, capture.size().height - 2.0)
        } else {
            let bounds = capture.find(target);
            iced::Point::new(bounds.center_x(), bounds.center_y())
        };
        let messages = layout_capture::click_at(element, &mut renderer, point);
        let toggles = messages
            .iter()
            .filter(|message| matches!(message, Message::ToggleFileInfo))
            .count();
        if target == "Title: Opening" {
            assert_eq!(toggles, 0, "the panel's chrome must not dismiss it");
            assert!(
                matches!(messages.as_slice(), [Message::Ignored]),
                "the panel must swallow the press with Ignored: {messages:?}"
            );
        } else {
            assert_eq!(
                toggles, 1,
                "{target} must publish exactly one ToggleFileInfo: {messages:?}"
            );
        }
    }
}

fn jump_time_element() -> Element<'static, Message> {
    let base: Element<'_, Message> = iced::widget::Space::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
    super::super::view_jump_time_overlay(base, "", std::time::Duration::from_secs(30))
}

#[test]
fn jump_time_overlay_draws_its_heading_length_and_buttons() {
    let mut element = jump_time_element();
    let capture = layout_capture::capture_text(&mut element, &layout_capture::headless_renderer());
    let texts: Vec<&str> = capture.texts().collect();
    for expected in ["JUMP TO TIME", "Track length: 0:30", "Jump", "Close"] {
        assert!(
            texts.contains(&expected),
            "the Jump to time overlay must draw {expected:?}: {texts:?}"
        );
    }
}

#[test]
fn jump_time_overlay_buttons_and_backdrop_publish_their_messages() {
    let mut renderer = layout_capture::headless_renderer();
    for target in ["Close", "BACKDROP", "Jump"] {
        let mut element = jump_time_element();
        let capture = layout_capture::capture_text(&mut element, &renderer);
        let point = if target == "BACKDROP" {
            iced::Point::new(2.0, capture.size().height - 2.0)
        } else {
            let bounds = capture.find(target);
            iced::Point::new(bounds.center_x(), bounds.center_y())
        };
        let messages = layout_capture::click_at(element, &mut renderer, point);
        if target == "Jump" {
            assert!(
                matches!(messages.as_slice(), [Message::JumpTimeSubmitted]),
                "Jump must publish one JumpTimeSubmitted: {messages:?}"
            );
        } else {
            assert!(
                matches!(messages.as_slice(), [Message::ToggleJumpTime]),
                "{target} must publish one ToggleJumpTime: {messages:?}"
            );
        }
    }
}
