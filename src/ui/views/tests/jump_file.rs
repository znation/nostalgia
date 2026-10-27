//! The Jump to File overlay: its filter, its rows, and the messages each
//! control publishes.

use super::super::jump_file::{JUMP_FILE_LIST_HEIGHT, match_list_build_count};
use super::super::{
    JUMP_FILE_LIST_PADDING, JUMP_FILE_PAGE_ROWS, JUMP_FILE_ROW_HEIGHT, JUMP_FILE_ROW_SPACING,
    jump_file_highlighted, jump_file_matches, jump_file_row_label, jump_file_row_style,
    view_jump_file_overlay,
};
use super::*;

fn overlay<'a>(
    query: &'a str,
    titles: &'a [String],
    epoch: u64,
    selected: Option<usize>,
) -> Element<'a, Message> {
    let base: Element<'a, Message> = iced::widget::Space::new()
        .width(Length::Fill)
        .height(Length::Fill)
        .into();
    view_jump_file_overlay(base, query, titles, epoch, selected)
}

fn press_label(query: &str, titles: &[String], epoch: u64, label: &str) -> Vec<Message> {
    let mut element = overlay(query, titles, epoch, None);
    let mut renderer = layout_capture::headless_renderer();
    let capture = layout_capture::capture_text(&mut element, &renderer);
    let bounds = capture.find(label);
    layout_capture::click_at(
        element,
        &mut renderer,
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
fn jump_file_page_step_fits_the_visible_list() {
    let page_height = JUMP_FILE_PAGE_ROWS as f32 * JUMP_FILE_ROW_HEIGHT
        + (JUMP_FILE_PAGE_ROWS as f32 - 1.0) * JUMP_FILE_ROW_SPACING
        + 2.0 * JUMP_FILE_LIST_PADDING;
    assert!(
        page_height <= JUMP_FILE_LIST_HEIGHT,
        "a page of {JUMP_FILE_PAGE_ROWS} rows must fit the {JUMP_FILE_LIST_HEIGHT}px list, but it needs {page_height}px"
    );
    let with_one_more_row = page_height + JUMP_FILE_ROW_HEIGHT + JUMP_FILE_ROW_SPACING;
    assert!(
        with_one_more_row > JUMP_FILE_LIST_HEIGHT,
        "one more row must overflow the {JUMP_FILE_LIST_HEIGHT}px list, but {with_one_more_row}px would fit"
    );
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
fn jump_file_highlighted_prefers_a_selected_match_and_falls_back_to_first() {
    let matches = [2, 5, 9];
    assert_eq!(jump_file_highlighted(&matches, Some(5)), Some(5));
    assert_eq!(jump_file_highlighted(&matches, None), Some(2));
    // A selection that outlived a filter change falls back to the first match.
    assert_eq!(jump_file_highlighted(&matches, Some(7)), Some(2));
    assert_eq!(jump_file_highlighted(&[], Some(5)), None);
    assert_eq!(jump_file_highlighted(&[], None), None);
}

#[test]
fn jump_file_row_style_highlights_only_the_highlighted_row() {
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
    let mut element = overlay("mid", &titles, 7, None);
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
fn the_match_list_reuses_its_rows_until_the_filter_or_highlight_changes() {
    let titles = vec![
        "Opening".to_string(),
        "Middle".to_string(),
        "Closing".to_string(),
    ];
    let before = match_list_build_count();
    let first = overlay("", &titles, 7, None);
    let mut tree = Tree::new(first.as_widget());
    assert_eq!(
        match_list_build_count() - before,
        1,
        "the first view builds the match list once"
    );

    let same = overlay("", &titles, 7, None);
    tree.diff(same.as_widget());
    assert_eq!(
        match_list_build_count() - before,
        1,
        "an unchanged filter and highlight reuses the built rows"
    );

    let filtered = overlay("ing", &titles, 7, None);
    tree.diff(filtered.as_widget());
    assert_eq!(
        match_list_build_count() - before,
        2,
        "a changed query rebuilds the list"
    );

    let highlighted = overlay("ing", &titles, 7, Some(2));
    tree.diff(highlighted.as_widget());
    assert_eq!(
        match_list_build_count() - before,
        3,
        "a moved highlight rebuilds the list"
    );

    let reloaded = overlay("ing", &titles, 8, Some(2));
    tree.diff(reloaded.as_widget());
    assert_eq!(
        match_list_build_count() - before,
        4,
        "a new list epoch rebuilds the list"
    );
}

#[test]
fn overlay_highlights_the_selected_row() {
    let titles = vec![
        "Opening".to_string(),
        "Middle".to_string(),
        "Closing".to_string(),
    ];
    let mut element = overlay("", &titles, 7, Some(2));
    let mut renderer = layout_capture::headless_renderer();
    let (capture, pixels) =
        layout_capture::capture_text_and_pixels(&mut element, &mut renderer, &iced::Theme::Dark);
    // The highlighted row draws the selection's white text; a plain row keeps
    // the LCD green, whose red channel stays low.
    let selected_red = pixels.max_red_in(capture.find("3. Closing"));
    let plain_red = pixels.max_red_in(capture.find("1. Opening"));
    assert!(
        selected_red > plain_red,
        "the selected row must draw the white selection text (selected red {selected_red}, plain red {plain_red})"
    );
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

    let mut element = overlay("mid", &titles, 7, None);
    let mut renderer = layout_capture::headless_renderer();
    let capture = layout_capture::capture_text(&mut element, &renderer);
    let backdrop = iced::Point::new(2.0, capture.size().height - 2.0);
    assert!(matches!(
        layout_capture::click_at(element, &mut renderer, backdrop).as_slice(),
        [Message::ToggleJumpFile]
    ));
}

#[test]
fn empty_and_no_match_lists_have_their_own_wording() {
    let empty: Vec<String> = Vec::new();
    let mut element = overlay("", &empty, 0, None);
    let capture = layout_capture::capture_text(&mut element, &layout_capture::headless_renderer());
    let texts: Vec<&str> = capture.texts().collect();
    assert!(texts.contains(&"No songs"), "{texts:?}");
    assert!(!texts.contains(&"No matching songs"), "{texts:?}");

    let titles = vec!["Opening".to_string()];
    let mut element = overlay("zzz", &titles, 0, None);
    let capture = layout_capture::capture_text(&mut element, &layout_capture::headless_renderer());
    let texts: Vec<&str> = capture.texts().collect();
    assert!(texts.contains(&"No matching songs"), "{texts:?}");
    assert!(!texts.contains(&"Opening"), "{texts:?}");
}
