//! The Jump to File overlay: its filter, its rows, and the messages each
//! control publishes.

use super::super::{
    jump_file_matches, jump_file_row_label, jump_file_row_style, view_jump_file_overlay,
};
use super::*;

fn overlay<'a>(query: &'a str, titles: &'a [String], epoch: u64) -> Element<'a, Message> {
    let base: Element<'a, Message> = iced::widget::Space::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
    view_jump_file_overlay(base, query, titles, epoch)
}

fn press_at(element: Element<'_, Message>, point: iced::Point) -> Vec<Message> {
    let mut renderer = layout_capture::headless_renderer();
    let mut user_interface = iced_runtime::user_interface::UserInterface::build(
        element,
        layout_capture::limits().max(),
        iced_runtime::user_interface::Cache::new(),
        &mut renderer,
    );
    let mut messages = Vec::new();
    let mut clipboard = iced::advanced::clipboard::Null;
    let _ = user_interface.update(
        &[
            iced::Event::Mouse(iced::mouse::Event::ButtonPressed(iced::mouse::Button::Left)),
            iced::Event::Mouse(iced::mouse::Event::ButtonReleased(
                iced::mouse::Button::Left,
            )),
        ],
        iced::mouse::Cursor::Available(point),
        &mut renderer,
        &mut clipboard,
        &mut messages,
    );
    messages
}

fn press_label(query: &str, titles: &[String], epoch: u64, label: &str) -> Vec<Message> {
    let mut element = overlay(query, titles, epoch);
    let capture = layout_capture::capture_text(&mut element, &layout_capture::headless_renderer());
    let bounds = capture.find(label);
    press_at(
        element,
        iced::Point::new(bounds.center_x(), bounds.center_y()),
    )
}

#[test]
fn matches_blank_all_case_insensitive_trimmed_in_order() {
    let titles = vec![
        "Opening".to_string(),
        "Middle".to_string(),
        "Closing".to_string(),
    ];
    assert_eq!(jump_file_matches(&titles, ""), vec![0, 1, 2]);
    assert_eq!(jump_file_matches(&titles, "  "), vec![0, 1, 2]);
    assert_eq!(jump_file_matches(&titles, " MID "), vec![1]);
    assert_eq!(jump_file_matches(&titles, "ing"), vec![0, 2]);
    assert_eq!(jump_file_matches(&titles, "zzz"), Vec::<usize>::new());
}

#[test]
fn jump_file_row_label_numbers_the_showing_list_position() {
    assert_eq!(jump_file_row_label(0, "Opening"), "1. Opening");
    assert_eq!(jump_file_row_label(1, "Middle"), "2. Middle");
    assert_eq!(jump_file_row_label(9, "Closing"), "10. Closing");
    // A title that already carries a number gains exactly one new prefix.
    assert_eq!(jump_file_row_label(1, "2. Middle"), "2. 2. Middle");
}

#[test]
fn jump_file_row_style_highlights_only_the_default_row() {
    use iced::widget::button::Status;

    let default = jump_file_row_style(true, Status::Active);
    assert_eq!(
        default.background,
        Some(iced::Background::Color(theme::ROW_HIGHLIGHT))
    );
    assert_eq!(default.text_color, theme::PLAYING_ROW_TEXT);

    let plain = jump_file_row_style(false, Status::Active);
    assert_eq!(plain.background, None);
    assert_eq!(plain.text_color, theme::LCD_GREEN);

    let hovered = jump_file_row_style(false, Status::Hovered);
    assert_eq!(
        hovered.background,
        style::playlist_row_style(Status::Hovered).background
    );
}

#[test]
fn overlay_draws_the_heading_matching_rows_and_buttons() {
    let titles = vec![
        "Opening".to_string(),
        "Middle".to_string(),
        "Closing".to_string(),
    ];
    let mut element = overlay("mid", &titles, 7);
    let capture = layout_capture::capture_text(&mut element, &layout_capture::headless_renderer());
    let texts: Vec<&str> = capture.texts().collect();
    for expected in ["JUMP TO FILE", "2. Middle", "Jump", "Close"] {
        assert!(
            texts.contains(&expected),
            "the Jump to File overlay must draw {expected:?}: {texts:?}"
        );
    }
    for omitted in ["1. Opening", "3. Closing"] {
        assert!(
            !texts.contains(&omitted),
            "the filter must omit {omitted:?}: {texts:?}"
        );
    }
}

#[test]
fn row_and_buttons_publish_their_messages() {
    let titles = vec![
        "Opening".to_string(),
        "Middle".to_string(),
        "Closing".to_string(),
    ];
    assert!(matches!(
        press_label("mid", &titles, 7, "2. Middle").as_slice(),
        [Message::JumpFileRowSelected { epoch: 7, index: 1 }]
    ));
    assert!(matches!(
        press_label("mid", &titles, 7, "Jump").as_slice(),
        [Message::JumpFileSubmitted]
    ));
    assert!(matches!(
        press_label("mid", &titles, 7, "Close").as_slice(),
        [Message::ToggleJumpFile]
    ));

    let mut element = overlay("mid", &titles, 7);
    let capture = layout_capture::capture_text(&mut element, &layout_capture::headless_renderer());
    let backdrop = iced::Point::new(2.0, capture.size().height - 2.0);
    assert!(matches!(
        press_at(element, backdrop).as_slice(),
        [Message::ToggleJumpFile]
    ));
}

#[test]
fn empty_and_no_match_lists_have_their_own_wording() {
    let empty: Vec<String> = Vec::new();
    let mut element = overlay("", &empty, 0);
    let capture = layout_capture::capture_text(&mut element, &layout_capture::headless_renderer());
    let texts: Vec<&str> = capture.texts().collect();
    assert!(texts.contains(&"No songs"), "{texts:?}");
    assert!(!texts.contains(&"No matching songs"), "{texts:?}");

    let titles = vec!["Opening".to_string()];
    let mut element = overlay("zzz", &titles, 0);
    let capture = layout_capture::capture_text(&mut element, &layout_capture::headless_renderer());
    let texts: Vec<&str> = capture.texts().collect();
    assert!(texts.contains(&"No matching songs"), "{texts:?}");
    assert!(!texts.contains(&"Opening"), "{texts:?}");
}
