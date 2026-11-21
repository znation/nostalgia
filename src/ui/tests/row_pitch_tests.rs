use super::*;

// The scroll-to-playing-row operation offsets by `BROWSE_LIST_PADDING +
// index * BROWSE_ROW_HEIGHT`, so that constant must equal the pitch the browse
// list actually lays its rows out at. A stale constant would scroll a later
// row out of position. Lay the root view out with the stepping album showing
// and its first title replaced by `title` — far too long for one line if it
// wrapped — so a test can measure the distance between the first two row
// prefixes. Returns the capture and the first row's full `display_title`, so a
// caller can find the title fragment the row draws.
fn capture_songs_list_with_first_title(title: &str) -> (layout_capture::TextCapture, String) {
    let (mut player, _state) = player_stepping_from(None);
    player.current_view = CurrentView::Songs;
    player.songs.items[0].title = title.to_string();
    let display_title = player.songs.items[0].display_title().into_owned();
    (captured_root_view_text_bounds(&player), display_title)
}

// Assert that the first two song rows sit one `BROWSE_ROW_HEIGHT` apart,
// naming `context` so a failure says which title shape broke the pitch.
fn assert_first_song_row_pitch(capture: &layout_capture::TextCapture, context: &str) {
    let first = capture.find("1. ");
    let second = capture.find("2. ");
    let pitch = second.y - first.y;
    assert!(
        (pitch - views::BROWSE_ROW_HEIGHT).abs() < 0.01,
        "row pitch {pitch} must equal BROWSE_ROW_HEIGHT {} {context}; rows at {first:?} and {second:?}",
        views::BROWSE_ROW_HEIGHT
    );
}

// The same long title must stay inside its own column instead of drawing
// under the right-aligned length.
#[test]
fn browse_row_height_matches_the_laid_out_row_pitch() {
    let long_title = "A very long title ".repeat(20);

    let (capture, display_title) = capture_songs_list_with_first_title(&long_title);
    let title = capture.find(&display_title);
    let label = capture.find("3:30");

    assert_first_song_row_pitch(&capture, "with a long title");
    assert!(
        title.x + title.width <= label.x,
        "the title must be clipped to its column, not drawn under the label: {title:?} ends after {label:?} begins"
    );
}

// `Wrapping::None` only suppresses soft wrapping: cosmic-text still splits a
// hard line break into another `BufferLine`, which would make the first row
// taller than `BROWSE_ROW_HEIGHT`. The list flattens line breaks to spaces, so
// a title carrying one keeps the same row pitch as any other.
#[test]
fn a_hard_line_break_in_a_title_does_not_change_the_row_pitch() {
    let (capture, _) = capture_songs_list_with_first_title("One\nTwo");

    assert_first_song_row_pitch(&capture, "despite the title's line break");
}

// The row flattener replaces both halves of a line break with spaces. A
// lone carriage return (old Mac line ending) is not cosmic-text's `\r\n`
// split, but the guard still replaces it so the title reads as two words on
// one line instead of keeping the control character; a `\r\n` pair becomes
// two spaces for the same reason.
#[test]
fn a_carriage_return_in_a_title_does_not_change_the_row_pitch() {
    let (capture, _) = capture_songs_list_with_first_title("One\rTwo");

    assert_first_song_row_pitch(&capture, "despite the title's carriage return");
    assert!(
        capture
            .fragments()
            .iter()
            .any(|(text, _)| text.contains("One Two")),
        "the carriage return must be flattened to a space: {:?}",
        capture.fragments()
    );
}

// Windows-style CRLF is the line ending cosmic-text splits on most directly,
// so the flattener must replace both bytes. A `\r\n` title keeps the row
// pitch and reads as two spaces between the words rather than keeping a
// control character or starting a second `BufferLine`.
#[test]
fn a_crlf_in_a_title_does_not_change_the_row_pitch() {
    let (capture, _) = capture_songs_list_with_first_title("One\r\nTwo");

    assert_first_song_row_pitch(&capture, "despite the title's CRLF");
    assert!(
        capture
            .fragments()
            .iter()
            .any(|(text, _)| text.contains("One  Two")),
        "both bytes of the CRLF must be flattened to spaces: {:?}",
        capture.fragments()
    );
}
