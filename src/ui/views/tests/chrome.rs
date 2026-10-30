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

    let mut containers = CapturedContainers::default();
    layout_capture::operate(&mut element, &renderer, &mut containers);

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
    super::super::view_file_info_overlay(base, track.clone(), format)
}

fn sample_known_track(artist: &str) -> KnownTrack {
    KnownTrack {
        title: "Opening".to_string(),
        artist: artist.to_string(),
        album: "First Record".to_string(),
        year: None,
        genre: String::new(),
        track_number: None,
        disc_number: None,
        composer: String::new(),
        content_rating: None,
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
fn file_info_overlay_draws_the_tracks_disc_number() {
    let mut track = sample_known_track("The Sample Band");
    track.track_number = Some(3);
    track.disc_number = Some(2);
    track.composer = "Jane Doe".to_string();
    let texts = file_info_texts(&track, None);
    let number_at = texts.iter().position(|text| text == "Track #: 3").unwrap();
    let disc_at = texts.iter().position(|text| text == "Disc #: 2").unwrap();
    let composer_at = texts
        .iter()
        .position(|text| text == "Composer: Jane Doe")
        .unwrap();
    assert!(
        number_at < disc_at && disc_at < composer_at,
        "the disc line must sit between Track # and Composer: {texts:?}"
    );
}

#[test]
fn file_info_overlay_omits_the_disc_number_line_when_the_source_supplied_none() {
    let track = sample_known_track("The Sample Band");
    let texts = file_info_texts(&track, None);
    assert_no_file_info_line(&texts, "Disc #:");
}

#[test]
fn file_info_overlay_draws_the_tracks_composer() {
    let mut track = sample_known_track("The Sample Band");
    track.track_number = Some(3);
    track.composer = "Jane Doe".to_string();
    let texts = file_info_texts(&track, None);
    let number_at = texts.iter().position(|text| text == "Track #: 3").unwrap();
    let composer_at = texts
        .iter()
        .position(|text| text == "Composer: Jane Doe")
        .unwrap();
    let length_at = texts
        .iter()
        .position(|text| text == "Length: 3:30")
        .unwrap();
    assert!(
        number_at < composer_at && composer_at < length_at,
        "the composer line must sit between Track # and Length: {texts:?}"
    );
}

#[test]
fn file_info_overlay_omits_the_composer_line_when_the_source_supplied_none() {
    let track = sample_known_track("The Sample Band");
    let texts = file_info_texts(&track, None);
    assert_no_file_info_line(&texts, "Composer:");
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

fn save_preset_element() -> Element<'static, Message> {
    let base: Element<'_, Message> = iced::widget::Space::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
    super::super::view_save_preset_overlay(base, "Very Fuzzed", Some(super::super::EMPTY_NAME_NOTE))
}

fn save_list_element() -> Element<'static, Message> {
    let base: Element<'_, Message> = iced::widget::Space::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
    super::super::view_save_list_overlay(
        base,
        "Morning Drive",
        Some(super::super::save_list::SAVE_LIST_EMPTY_NAME_NOTE),
    )
}

// The Save list overlay draws its heading, box, note, and buttons through the
// same modal script the Save preset overlay uses.
#[test]
fn save_list_overlay_draws_its_heading_note_and_buttons() {
    let mut element = save_list_element();
    let capture = layout_capture::capture_text(&mut element, &layout_capture::headless_renderer());
    let texts: Vec<&str> = capture.texts().collect();
    for expected in ["SAVE LIST", "Type a name", "Save", "Delete", "Close"] {
        assert!(
            texts.contains(&expected),
            "the Save list overlay must draw {expected:?}: {texts:?}"
        );
    }
}

// A Save list panel without a stored note draws none, so a just-opened panel
// shows only the heading, the box, and the buttons.
#[test]
fn save_list_overlay_without_a_note_draws_no_note() {
    let base: Element<'_, Message> = iced::widget::Space::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
    let mut element = super::super::view_save_list_overlay(base, "", None);
    let capture = layout_capture::capture_text(&mut element, &layout_capture::headless_renderer());
    let texts: Vec<&str> = capture.texts().collect();
    assert!(
        !texts.contains(&"Type a name"),
        "a fresh panel must not pre-draw the empty-name note: {texts:?}"
    );
}

// `Close` and the backdrop each publish one ToggleSaveList, `Save`
// publishes one SaveListSubmitted, and `Delete` publishes one
// DeleteListSubmitted: the same modal script the other naming panels use.
#[test]
fn save_list_overlay_buttons_and_backdrop_publish_their_messages() {
    let mut renderer = layout_capture::headless_renderer();
    for target in ["Save", "Delete"] {
        let mut element = save_list_element();
        let capture = layout_capture::capture_text(&mut element, &renderer);
        let bounds = capture.find(target);
        let point = iced::Point::new(bounds.center_x(), bounds.center_y());
        let messages = layout_capture::click_at(element, &mut renderer, point);
        if target == "Save" {
            assert!(
                matches!(messages.as_slice(), [Message::SaveListSubmitted]),
                "Save must publish one SaveListSubmitted: {messages:?}"
            );
        } else {
            assert!(
                matches!(messages.as_slice(), [Message::DeleteListSubmitted]),
                "Delete must publish one DeleteListSubmitted: {messages:?}"
            );
        }
    }
    // The `Delete` button sits between `Save` and `Close` in the row: its
    // center lands between their centers.
    let mut element = save_list_element();
    let capture = layout_capture::capture_text(&mut element, &renderer);
    let save_x = capture.find("Save").center_x();
    let delete_x = capture.find("Delete").center_x();
    let close_x = capture.find("Close").center_x();
    assert!(
        save_x < delete_x && delete_x < close_x,
        "the button row must read Save, Delete, Close: {save_x} {delete_x} {close_x}"
    );
    for target in ["Close", "BACKDROP"] {
        let mut element = save_list_element();
        let capture = layout_capture::capture_text(&mut element, &renderer);
        let point = if target == "BACKDROP" {
            iced::Point::new(2.0, capture.size().height - 2.0)
        } else {
            let bounds = capture.find(target);
            iced::Point::new(bounds.center_x(), bounds.center_y())
        };
        let messages = layout_capture::click_at(element, &mut renderer, point);
        assert!(
            matches!(messages.as_slice(), [Message::ToggleSaveList]),
            "{target} must publish one ToggleSaveList: {messages:?}"
        );
    }
}

#[test]
fn save_preset_overlay_draws_its_heading_note_and_buttons() {
    // The box's own text is iced's, not an ordinary text draw, so the
    // captured texts pin the heading, the stored inline note, and the
    // buttons; the typed name is covered by `save_preset_name_changed` and
    // the submit tests in `src/ui/tests/mixer_tests.rs`.
    let mut element = save_preset_element();
    let capture = layout_capture::capture_text(&mut element, &layout_capture::headless_renderer());
    let texts: Vec<&str> = capture.texts().collect();
    for expected in [
        "SAVE PRESET",
        super::super::EMPTY_NAME_NOTE,
        "Save",
        "Delete",
        "Close",
    ] {
        assert!(
            texts.contains(&expected),
            "the Save preset overlay must draw {expected:?}: {texts:?}"
        );
    }
}

// A Save preset panel without a stored note draws none, so a just-opened
// panel shows only the heading, the box, and the buttons.
#[test]
fn save_preset_overlay_without_a_note_draws_no_note() {
    let base: Element<'_, Message> = iced::widget::Space::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
    let mut element = super::super::view_save_preset_overlay(base, "", None);
    let capture = layout_capture::capture_text(&mut element, &layout_capture::headless_renderer());
    let texts: Vec<&str> = capture.texts().collect();
    assert!(
        !texts.contains(&super::super::EMPTY_NAME_NOTE),
        "a fresh panel must not pre-draw the empty-name note: {texts:?}"
    );
}

#[test]
fn save_preset_overlay_buttons_and_backdrop_publish_their_messages() {
    let mut renderer = layout_capture::headless_renderer();
    for target in ["Save", "Delete"] {
        let mut element = save_preset_element();
        let capture = layout_capture::capture_text(&mut element, &renderer);
        let bounds = capture.find(target);
        let point = iced::Point::new(bounds.center_x(), bounds.center_y());
        let messages = layout_capture::click_at(element, &mut renderer, point);
        if target == "Save" {
            assert!(
                matches!(messages.as_slice(), [Message::SavePresetSubmitted]),
                "Save must publish one SavePresetSubmitted: {messages:?}"
            );
        } else {
            assert!(
                matches!(messages.as_slice(), [Message::DeletePresetSubmitted]),
                "Delete must publish one DeletePresetSubmitted: {messages:?}"
            );
        }
    }
    // The `Delete` button sits between `Save` and `Close` in the row: its
    // center lands between their centers.
    let mut element = save_preset_element();
    let capture = layout_capture::capture_text(&mut element, &renderer);
    let save_x = capture.find("Save").center_x();
    let delete_x = capture.find("Delete").center_x();
    let close_x = capture.find("Close").center_x();
    assert!(
        save_x < delete_x && delete_x < close_x,
        "the button row must read Save, Delete, Close: {save_x} {delete_x} {close_x}"
    );
    for target in ["Close", "BACKDROP"] {
        let mut element = save_preset_element();
        let capture = layout_capture::capture_text(&mut element, &renderer);
        let point = if target == "BACKDROP" {
            iced::Point::new(2.0, capture.size().height - 2.0)
        } else {
            let bounds = capture.find(target);
            iced::Point::new(bounds.center_x(), bounds.center_y())
        };
        let messages = layout_capture::click_at(element, &mut renderer, point);
        assert!(
            matches!(messages.as_slice(), [Message::ToggleSavePreset]),
            "{target} must publish one ToggleSavePreset: {messages:?}"
        );
    }
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

/// The options overlay over a blank full-space base, so its layout is measured
/// against the same window limits as the root view. `all_on` selects the
/// on/remaining/shown labels; `false` selects off/elapsed/hidden.
fn options_overlay_element(all_on: bool) -> Element<'static, Message> {
    let base: Element<'static, Message> = iced::widget::Space::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
    let state = super::super::OptionsState {
        always_on_top: all_on,
        time_remaining: all_on,
        autoscroll: all_on,
        visualizer_visible: all_on,
        eq_panel_visible: all_on,
        library_panel_visible: all_on,
        stop_after_current: all_on,
        repeat: all_on,
        shuffle: all_on,
        manual_advance: all_on,
    };
    super::super::view_options_overlay(base, state, 0)
}

#[test]
fn options_overlay_draws_each_row_with_its_state() {
    let cases = [
        (
            false,
            [
                "Always on Top: Off",
                "Time: Elapsed",
                "Autoscroll song name: Off",
                "Visualizer: Hidden",
                "Equalizer: Hidden",
                "Library: Hidden",
                "Stop with fadeout",
                "Stop after current: Off",
                "Repeat: Off",
                "Shuffle: Off",
                "Manual playlist advance: Off",
            ],
        ),
        (
            true,
            [
                "Always on Top: On",
                "Time: Remaining",
                "Autoscroll song name: On",
                "Visualizer: Shown",
                "Equalizer: Shown",
                "Library: Shown",
                "Stop with fadeout",
                "Stop after current: On",
                "Repeat: On",
                "Shuffle: On",
                "Manual playlist advance: On",
            ],
        ),
    ];
    for (all_on, rows) in cases {
        let mut element = options_overlay_element(all_on);
        let capture =
            layout_capture::capture_text(&mut element, &layout_capture::headless_renderer());
        let texts: Vec<&str> = capture.texts().collect();
        for expected in ["OPTIONS"].into_iter().chain(rows).chain([
            "10 songs back",
            "10 songs forward",
            "Back 5 seconds",
            "Fwd 5 seconds",
            "Start of list",
            "End of list",
            "Jump to time",
            "Jump to file",
            "File info",
            "About Nostalgia",
            "Close",
        ]) {
            assert!(
                texts.contains(&expected),
                "the options overlay must draw {expected:?} (all_on: {all_on}): {texts:?}"
            );
        }
    }
}

// A press on each row publishes that choice's message; Close and the backdrop
// each dismiss the menu with one ToggleOptions; a press on the panel's own
// text chrome is swallowed with Ignored, so the menu stays open.
#[test]
fn options_overlay_publishes_each_choice_and_closes_from_close_and_backdrop() {
    let mut renderer = layout_capture::headless_renderer();
    for label in [
        "Always on Top: Off",
        "Time: Elapsed",
        "Autoscroll song name: Off",
        "Visualizer: Hidden",
        "Equalizer: Hidden",
        "Library: Hidden",
        "Stop with fadeout",
        "Stop after current: Off",
        "Repeat: Off",
        "Shuffle: Off",
        "Manual playlist advance: Off",
        "10 songs back",
        "10 songs forward",
        "Back 5 seconds",
        "Fwd 5 seconds",
        "Start of list",
        "End of list",
        "Jump to time",
        "Jump to file",
        "File info",
        "About Nostalgia",
        "Close",
        "OPTIONS",
        "BACKDROP",
    ] {
        let mut element = options_overlay_element(false);
        let capture = layout_capture::capture_text(&mut element, &renderer);
        let point = if label == "BACKDROP" {
            iced::Point::new(2.0, capture.size().height - 2.0)
        } else {
            let bounds = capture.find(label);
            iced::Point::new(bounds.center_x(), bounds.center_y())
        };
        let messages = layout_capture::click_at(element, &mut renderer, point);
        let toggles = messages
            .iter()
            .filter(|message| matches!(message, Message::ToggleOptions))
            .count();
        match label {
            "Always on Top: Off" => assert!(
                messages.iter().any(|message| matches!(
                    message,
                    Message::OptionsChoice(OptionsChoice::AlwaysOnTop)
                )),
                "the always-on-top row must publish its choice: {messages:?}"
            ),
            "Time: Elapsed" => assert!(
                messages.iter().any(|message| matches!(
                    message,
                    Message::OptionsChoice(OptionsChoice::TimeDisplay)
                )),
                "the time row must publish its choice: {messages:?}"
            ),
            "Autoscroll song name: Off" => assert!(
                messages.iter().any(|message| matches!(
                    message,
                    Message::OptionsChoice(OptionsChoice::Autoscroll)
                )),
                "the autoscroll row must publish its choice: {messages:?}"
            ),
            "Visualizer: Hidden" => assert!(
                messages.iter().any(|message| matches!(
                    message,
                    Message::OptionsChoice(OptionsChoice::Visualizer)
                )),
                "the visualizer row must publish its choice: {messages:?}"
            ),
            "Equalizer: Hidden" => assert!(
                messages.iter().any(|message| matches!(
                    message,
                    Message::OptionsChoice(OptionsChoice::EqualizerPanel)
                )),
                "the equalizer row must publish its choice: {messages:?}"
            ),
            "Library: Hidden" => assert!(
                messages.iter().any(|message| matches!(
                    message,
                    Message::OptionsChoice(OptionsChoice::LibraryPanel)
                )),
                "the library row must publish its choice: {messages:?}"
            ),
            "Stop with fadeout" => assert!(
                messages.iter().any(|message| matches!(
                    message,
                    Message::OptionsChoice(OptionsChoice::FadeOutAndStop)
                )),
                "the stop-with-fadeout row must publish its choice: {messages:?}"
            ),
            "Stop after current: Off" => assert!(
                messages.iter().any(|message| matches!(
                    message,
                    Message::OptionsChoice(OptionsChoice::StopAfterCurrent)
                )),
                "the stop-after-current row must publish its choice: {messages:?}"
            ),
            "Repeat: Off" => assert!(
                messages.iter().any(|message| matches!(
                    message,
                    Message::OptionsChoice(OptionsChoice::Repeat)
                )),
                "the repeat row must publish its choice: {messages:?}"
            ),
            "Shuffle: Off" => assert!(
                messages.iter().any(|message| matches!(
                    message,
                    Message::OptionsChoice(OptionsChoice::Shuffle)
                )),
                "the shuffle row must publish its choice: {messages:?}"
            ),
            "Manual playlist advance: Off" => assert!(
                messages.iter().any(|message| matches!(
                    message,
                    Message::OptionsChoice(OptionsChoice::ManualAdvance)
                )),
                "the manual-advance row must publish its choice: {messages:?}"
            ),
            "10 songs back" => assert!(
                messages.iter().any(|message| matches!(
                    message,
                    Message::OptionsChoice(OptionsChoice::JumpTracksBack)
                )),
                "the ten-songs-back row must publish its choice: {messages:?}"
            ),
            "10 songs forward" => assert!(
                messages.iter().any(|message| matches!(
                    message,
                    Message::OptionsChoice(OptionsChoice::JumpTracksForward)
                )),
                "the ten-songs-forward row must publish its choice: {messages:?}"
            ),
            "Back 5 seconds" => assert!(
                messages.iter().any(|message| matches!(
                    message,
                    Message::OptionsChoice(OptionsChoice::SeekBackward)
                )),
                "the back-5-seconds row must publish its choice: {messages:?}"
            ),
            "Fwd 5 seconds" => assert!(
                messages.iter().any(|message| matches!(
                    message,
                    Message::OptionsChoice(OptionsChoice::SeekForward)
                )),
                "the fwd-5-seconds row must publish its choice: {messages:?}"
            ),
            "Start of list" => assert!(
                messages.iter().any(|message| matches!(
                    message,
                    Message::OptionsChoice(OptionsChoice::StartOfList)
                )),
                "the start-of-list row must publish its choice: {messages:?}"
            ),
            "End of list" => assert!(
                messages.iter().any(|message| matches!(
                    message,
                    Message::OptionsChoice(OptionsChoice::EndOfList)
                )),
                "the end-of-list row must publish its choice: {messages:?}"
            ),
            "Jump to time" => assert!(
                messages.iter().any(|message| matches!(
                    message,
                    Message::OptionsChoice(OptionsChoice::JumpTime)
                )),
                "the jump-to-time row must publish its choice: {messages:?}"
            ),
            "Jump to file" => assert!(
                messages.iter().any(|message| matches!(
                    message,
                    Message::OptionsChoice(OptionsChoice::JumpFile)
                )),
                "the jump-to-file row must publish its choice: {messages:?}"
            ),
            "File info" => assert!(
                messages.iter().any(|message| matches!(
                    message,
                    Message::OptionsChoice(OptionsChoice::FileInfo)
                )),
                "the file-info row must publish its choice: {messages:?}"
            ),
            "About Nostalgia" => assert!(
                messages
                    .iter()
                    .any(|message| matches!(message, Message::ToggleAbout)),
                "the About row must publish ToggleAbout: {messages:?}"
            ),
            "OPTIONS" => {
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
            }
            _ => assert_eq!(
                toggles, 1,
                "{label} must publish exactly one ToggleOptions: {messages:?}"
            ),
        }
    }
}

#[test]
fn transport_options_button_publishes_toggle_options() {
    let mut renderer = layout_capture::headless_renderer();
    let mut element = view_transport_controls(0.5, 0.0, false, false, true, true);
    let capture = layout_capture::capture_text(&mut element, &renderer);
    let library = capture.find("PL");
    // Part 2/4 of the pixel-font split swapped the lightning glyph for `≡`,
    // which the shape face covers crisply.
    let options = capture.find("≡");
    assert!(
        options.x > library.x,
        "the options button must sit after PL: PL {:?}, options {:?}",
        library,
        options
    );
    let messages = layout_capture::click_at(
        element,
        &mut renderer,
        iced::Point::new(options.center_x(), options.center_y()),
    );
    assert_eq!(
        messages
            .iter()
            .filter(|message| matches!(message, Message::ToggleOptions))
            .count(),
        1,
        "a press on the options button must publish exactly one ToggleOptions: {messages:?}"
    );
}

#[test]
fn file_info_overlay_draws_the_tracks_content_rating() {
    let mut track = sample_known_track("The Sample Band");
    track.composer = "Jane Doe".to_string();
    track.content_rating = Some(crate::library::ContentRating::Explicit);
    let texts = file_info_texts(&track, None);
    let composer_at = texts
        .iter()
        .position(|text| text == "Composer: Jane Doe")
        .unwrap();
    let rating_at = texts
        .iter()
        .position(|text| text == "Rating: Explicit")
        .unwrap();
    let length_at = texts
        .iter()
        .position(|text| text == "Length: 3:30")
        .unwrap();
    assert!(
        composer_at < rating_at && rating_at < length_at,
        "the rating line must sit between Composer and Length: {texts:?}"
    );
}

#[test]
fn file_info_overlay_omits_the_content_rating_line_when_the_source_supplied_none() {
    let track = sample_known_track("The Sample Band");
    let texts = file_info_texts(&track, None);
    assert_no_file_info_line(&texts, "Rating:");
}

// Part 2/4 of the pixel-font split put the app bar's label in the vendored
// Silkscreen face at the on-grid 16px size. A fixed-width button reports its
// content box rather than the text's advance, but the band's label is the one
// chrome text iced hands its own shrink width, so the pin carries the family's
// measured run: "NOSTALGIA" at 16 measures 100.0, the exact number part 1/4's
// seam test pinned for the vendored face. A lost registration or a swapped
// family (the system sans measured `H` at 11.856) moves the number, so the
// regression fails here instead of silently reaping the chrome look.
#[test]
fn title_band_label_draws_the_silkscreen_face() {
    let renderer = layout_capture::headless_renderer();
    let mut band: iced::Element<'_, Message, iced::Theme, iced::Renderer> =
        view_title_band("NOSTALGIA");
    let capture = layout_capture::capture_text(&mut band, &renderer);
    let label = capture.find("NOSTALGIA");
    assert_eq!(
        label.width, 100.0,
        "the band label must draw in the vendored Silkscreen face: {label:?}"
    );
}
