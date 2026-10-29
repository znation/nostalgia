use super::*;

// The Home, End, Page Up, and Page Down keys all target the stable
// `BROWSE_LIST_ID` (pinned by the message tests above), but shade mode builds
// no browse list. This lays out the real root view in both modes and records
// every scrollable's id: the normal tree carries the browse target and the
// shaded tree carries none, so the same key press has nothing to move.
#[test]
fn shade_mode_builds_no_browse_scroll_target() {
    use iced::advanced::widget::Id;

    let (mut player, _state) = test_player();
    let browse_target = Some(Id::new(BROWSE_LIST_ID));
    assert!(
        root_scrollable_ids(&player).contains(&browse_target),
        "the open library must build the browse scroll target"
    );

    player.shaded = true;
    let shaded = root_scrollable_ids(&player);
    assert!(
        !shaded.contains(&browse_target),
        "shade mode must build no browse scroll target: {shaded:?}"
    );
}

#[test]
fn about_panel_names_the_active_library_source() {
    let (mut player, _state) = test_player();
    player.about_visible = true;

    let unsigned = captured_root_view_text_bounds(&player);
    let texts: Vec<&str> = unsigned.texts().collect();
    assert!(
        texts.contains(&"Library: Sample library"),
        "the unsigned About panel must name the sample library: {texts:?}"
    );
    assert!(
        !texts.contains(&"Library: Apple Music"),
        "the unsigned About panel must not claim Apple Music: {texts:?}"
    );

    player
        .apple_music_service
        .authenticate_with("dev", &|_| Ok(sign_in_session()))
        .expect("the stub sign-in must store its session");
    let signed = captured_root_view_text_bounds(&player);
    let texts: Vec<&str> = signed.texts().collect();
    assert!(
        texts.contains(&"Library: Apple Music"),
        "the signed-in About panel must name Apple Music: {texts:?}"
    );
    assert!(
        !texts.contains(&"Library: Sample library"),
        "the signed-in About panel must not claim the sample library: {texts:?}"
    );
}

#[test]
fn new_player_starts_at_artists_with_nothing_selected() {
    let (player, _state) = test_player();

    assert_view(&player, CurrentView::Artists);
    assert_browse_lists_empty(&player);
    // The artists fetch `boot` schedules is in flight, so the panel shows
    // "Loading…" rather than claiming an empty library (see
    // `views::browse::rows::text::browse_placeholder`); the lists below Artists are not
    // loading, including the two flat lists.
    assert!(player.artists.loading);
    assert!(!player.albums.loading);
    assert!(!player.songs.loading);
    assert!(!player.all_songs.loading);
    assert!(!player.all_albums.loading);
    assert!(!player.playlists.loading);
}

// `view` is the per-frame assembly: it locks the shared state, resolves
// the now-playing label, matches the current browse view onto its list
// buffer, and stacks the title bar, the Now Playing bar, the seek strip, the
// transport row, the equalizer panel, and the browse controls (Back, Retry,
// Open, Top, Sort, Reload, the flat-list buttons, and the search box with its
// Clear button) above the browse list. No other
// test reaches this assembly — the shade test stops at `view`'s title-bar
// early return, the `views` tests stop at the individual builders, and
// the `update` tests stop before the view layer — so a regression
// that made this assembly panic (a bad slider range, a `view_*` builder
// failing over the input the app produces) would take the window down on
// every refresh with no test catching it. iced `Element`s expose no tree
// introspection, so the observable contract — as with the `views`
// builder tests — is that `view` builds its widget tree without panicking
// over the space the app actually produces: every browse view (including
// the pre-load empty buffers), both play states, both repeat states, both
// shuffle states, the
// volume endpoints the update arm can store, and each now-playing
// resolution. `view` uses `blocking_lock`, which panics inside an async
// runtime, so this stays a plain test.
#[test]
fn view_constructs_over_the_apps_full_input_space() {
    let (mut player, state) = test_player();

    // The pre-load shape: nothing fetched, no current track. This is what
    // the window shows before the first `LoadArtists` fetch lands, in
    // every browse view.
    construct_view_in_every_browse_view(&mut player);

    // The loaded shape: every browse buffer populated, rendered for every
    // now-playing resolution a session can reach, including with the About
    // panel open over the otherwise unchanged base.
    seed_browse_lists(&mut player);

    let now_playing_options = [None, Some("song-1"), Some("no-such-song")];
    let play_states = [false, true];
    let repeats = [false, true];
    let shuffles = [false, true];
    let volumes = [0.0, 0.5, 1.0];

    for current_view in BROWSE_VIEWS {
        player.current_view = current_view;
        for current_track in now_playing_options {
            for is_playing in play_states {
                for repeat in repeats {
                    for shuffle in shuffles {
                        for volume in volumes {
                            {
                                let mut state = state.blocking_lock();
                                state.current_track = current_track.map(str::to_string);
                                state.is_playing = is_playing;
                                state.repeat = repeat;
                                state.shuffle = shuffle;
                                state.set_volume(volume);
                            }
                            let _screen = view(&player);
                        }
                    }
                }
            }
        }
    }

    // The fetch-failure shape: a list that failed to load is empty and
    // carries an error report, and `view` must render the report rather than
    // an empty-list label. Clearing the buffers matters: `scrollable_list`
    // only shows the placeholder when there are no rows, so with the loaded
    // rows still present the error branch would never render.
    player.artists.items.clear();
    player.albums.items.clear();
    player.songs.items.clear();
    player.all_songs.items.clear();
    player.all_albums.items.clear();
    player.playlists.items.clear();
    player.artists.error = Some("music-library fetch failed: boom".to_string());
    player.albums.error = Some("music-library fetch failed: boom".to_string());
    player.songs.error = Some("music-library fetch failed: boom".to_string());
    player.all_songs.error = Some("music-library fetch failed: boom".to_string());
    player.all_albums.error = Some("music-library fetch failed: boom".to_string());
    player.playlists.error = Some("music-library fetch failed: boom".to_string());
    construct_view_in_every_browse_view(&mut player);

    // The About panel over the loaded normal-mode tree must build without
    // panicking too.
    player.about_visible = true;
    construct_view_in_every_browse_view(&mut player);
}

#[test]
fn view_constructs_when_the_window_is_shaded() {
    let (mut player, _state) = test_player();
    player.shaded = true;

    // The rolled-up frame builds only the title bar, so it must not panic
    // over either the pre-load empty buffers or the loaded browse lists.
    construct_view_in_every_browse_view(&mut player);

    seed_browse_lists(&mut player);
    construct_view_in_every_browse_view(&mut player);

    // An About flag left set while shaded must not draw the panel: the shade
    // branch is the title bar plus the strip whatever the flag says.
    player.about_visible = true;
    construct_view_in_every_browse_view(&mut player);
    let shaded = captured_root_view_text_bounds(&player);
    let texts: Vec<&str> = shaded.texts().collect();
    assert!(
        !texts.contains(&"ABOUT NOSTALGIA"),
        "shade mode must never draw the About panel: {texts:?}"
    );

    // The same holds for a File info flag left set while shaded.
    player.about_visible = false;
    player.file_info_visible = true;
    player
        .known_tracks
        .insert("song-1".to_string(), known_track("Opening", "", ""));
    _state.blocking_lock().current_track = Some("song-1".to_string());
    let shaded = captured_root_view_text_bounds(&player);
    let texts: Vec<&str> = shaded.texts().collect();
    assert!(
        !texts.contains(&"FILE INFO"),
        "shade mode must never draw the File info panel: {texts:?}"
    );

    // And for a Jump to time flag left set while shaded.
    player.file_info_visible = false;
    player.jump_time_visible = true;
    player.playback_duration = Some(std::time::Duration::from_secs(30));
    let shaded = captured_root_view_text_bounds(&player);
    let texts: Vec<&str> = shaded.texts().collect();
    assert!(
        !texts.contains(&"JUMP TO TIME"),
        "shade mode must never draw the Jump to time panel: {texts:?}"
    );
}

#[test]
fn file_info_panel_draws_the_loaded_previews_format() {
    let (mut player, state) = test_player();
    seed_current_title(&mut player, &state, "Opening");
    player.file_info_visible = true;
    player.playback_format = Some(PlaybackFormat {
        sample_rate_hz: 44_100,
        channels: 2,
        bitrate_kbps: Some(256),
    });

    let loaded = captured_root_view_text_bounds(&player);
    let texts: Vec<&str> = loaded.texts().collect();
    assert!(
        texts.contains(&"Format: 256 KBPS  44 KHZ STEREO"),
        "the File info panel must draw the loaded preview's format: {texts:?}"
    );

    // Stop clears the format (see `handlers::playback::reset_position_readout`);
    // the panel then keeps its other lines but drops the Format line.
    player.playback_format = None;
    let stopped = captured_root_view_text_bounds(&player);
    let texts: Vec<&str> = stopped.texts().collect();
    assert!(
        texts.contains(&"Title: Opening"),
        "the panel must keep its title without a loaded preview: {texts:?}"
    );
    assert!(
        texts.iter().all(|text| !text.starts_with("Format:")),
        "the panel must drop the Format line once no preview is loaded: {texts:?}"
    );
}

#[test]
fn file_info_panel_draws_each_metadata_line_only_when_the_track_has_it() {
    let (mut player, state) = test_player();
    seed_current_title(&mut player, &state, "Opening");
    player.file_info_visible = true;

    let bare = captured_root_view_text_bounds(&player);
    let texts: Vec<&str> = bare.texts().collect();
    assert!(texts.contains(&"Title: Opening"), "{texts:?}");
    assert!(texts.contains(&"Length: 3:30"), "{texts:?}");
    for label in [
        "Artist:",
        "Album:",
        "Year:",
        "Genre:",
        "Track #:",
        "Disc #:",
        "Composer:",
        "Rating:",
    ] {
        assert!(
            texts.iter().all(|text| !text.starts_with(label)),
            "a track with no {label} data must not draw that line: {texts:?}"
        );
    }

    let track = player.known_tracks.get_mut("song-1").unwrap();
    track.artist = "Band".to_string();
    track.album = "Record".to_string();
    track.year = Some(1999);
    track.genre = "Rock".to_string();
    track.track_number = Some(4);
    track.disc_number = Some(2);
    track.composer = "Writer".to_string();
    track.content_rating = Some(crate::library::ContentRating::Explicit);
    let full = captured_root_view_text_bounds(&player);
    let texts: Vec<&str> = full.texts().collect();
    for line in [
        "Artist: Band",
        "Album: Record",
        "Year: 1999",
        "Genre: Rock",
        "Track #: 4",
        "Disc #: 2",
        "Composer: Writer",
        "Rating: Explicit",
    ] {
        assert!(texts.contains(&line), "missing {line:?}: {texts:?}");
    }

    let track = player.known_tracks.get_mut("song-1").unwrap();
    track.content_rating = Some(crate::library::ContentRating::Clean);
    let clean = captured_root_view_text_bounds(&player);
    assert!(
        clean.texts().any(|text| text == "Rating: Clean"),
        "a Clean track must draw Rating: Clean"
    );
}

#[test]
fn title_bar_constructs_for_both_about_states() {
    // iced `Element`s expose no tree introspection, so the observable
    // contract is that the title bar builds for both About states — the
    // sunken M slot and the raised one — without panicking.
    let _ = views::view_title_bar(false);
    let _ = views::view_title_bar(true);
}

// `TITLE_BAR_HEIGHT` is the title bar band alone. The rolled-up window is
// that band plus the compact transport strip: `views::SHADED_HEIGHT` owns the
// sum, and both the shaded root view and `resize_to_shade` read it. The shade
// tests above never read the value — they pass a literal 24.0 only as a stale
// measurement argument — so a zero or oversized band could leave the
// rolled-up window blank or push the strip out of view while every test
// stayed green. Pin the user-visible band height.
#[test]
fn title_bar_height_pins_the_shade_strip() {
    assert_eq!(views::TITLE_BAR_HEIGHT, 24.0);
}

// `resize_to_shade` sends `shade_target_size` to the window; pin the target
// itself, because the `Task` it returns is opaque and the shade tests above
// only see that some work was scheduled. A title-bar-only target would still
// schedule work while cutting the compact strip off below the window.
#[test]
fn shade_target_size_is_the_title_bar_plus_the_strip() {
    let target = handlers::window::shade_target_size(800.0);
    assert_eq!(target, iced::Size::new(800.0, views::SHADED_HEIGHT));
    assert_ne!(target.height, views::TITLE_BAR_HEIGHT);
}

// In shade mode the root view is the title bar over the compact strip, so the
// whole laid-out tree must resolve to `SHADED_HEIGHT` — the same height
// `resize_to_shade` gives the window. A title-bar-only tree would leave the
// controls cut off below the resized window and fail here.
#[test]
fn shaded_root_view_is_the_title_bar_plus_the_strip() {
    let (mut player, _state) = test_player();
    player.shaded = true;

    let capture = captured_root_view_text_bounds(&player);
    assert_eq!(capture.size().height, views::SHADED_HEIGHT);
}

// The rolled-up strip's seek slider previews through the readout the root
// view already derives from the player: a held drag shows its target there,
// over the preview's own length, so scrubbing works with the LCD hidden.
#[test]
fn shade_root_readout_previews_the_seek_drag() {
    let (mut player, _current_track) = play_song_1_then_browse_to_album_2();
    player.shaded = true;
    player.playback_position = Some(Duration::from_secs(45));
    player.playback_duration = Some(Duration::from_secs(180));

    // The shaded root must actually lay out the strip's seek control, not just
    // read the player's preview position: without this the readout assertions
    // below would also pass in a build with no shade seek slider at all.
    let seek = captured_root_view_container(&player, views::MINI_SEEK_ID)
        .expect("the shaded strip's seek slider must be laid out");
    let volume = captured_root_view_volume_slider(&player)
        .expect("the shaded strip's volume slider must be laid out");
    let before = captured_root_view_text_bounds(&player);
    let readout = before.find("00:45/3:30");
    assert!(
        readout.x + readout.width <= seek.x + 0.5,
        "the seek slider must follow the shaded readout: {readout:?} then {seek:?}"
    );
    assert!(
        seek.x + seek.width <= volume.x + 0.5,
        "the seek slider must precede the volume slider: {seek:?} then {volume:?}"
    );

    let _ = update(&mut player, Message::SeekPreviewChanged(12.0));
    let after = captured_root_view_text_bounds(&player);
    assert!(
        after.texts().any(|text| text == "00:12/3:30"),
        "the shaded readout must preview the held seek drag: {:?}",
        after.fragments()
    );
    assert!(
        !after.texts().any(|text| text == "00:45/3:30"),
        "the backend position must give way to the drag preview: {:?}",
        after.fragments()
    );
    assert_eq!(after.size().height, views::SHADED_HEIGHT);

    // Normal mode's library footer shares the compact row but never the seek
    // slider, so the same position and length must not lay one out there.
    player.shaded = false;
    assert!(
        captured_root_view_container(&player, views::MINI_SEEK_ID).is_none(),
        "the library footer must not gain a seek slider"
    );
}

// The strip is shade-mode-only. Both modes now draw a Previous glyph — the
// full transport row has one too — so pin where they land: in normal mode the
// first belongs to the full transport row and a second to the library footer,
// and in shade mode exactly one sits inside the strip between the title bar
// and `SHADED_HEIGHT`.
#[test]
fn shade_strip_draws_only_while_shaded() {
    let (mut player, _state) = test_player();

    let normal = captured_root_view_text_bounds(&player);
    let normal_previous: Vec<iced::Rectangle> = normal
        .fragments()
        .iter()
        .filter(|(text, _)| text == "◀◀")
        .map(|(_, bounds)| *bounds)
        .collect();
    assert_eq!(
        normal_previous.len(),
        2,
        "normal mode must draw the full transport row and the library footer: {:?}",
        normal.fragments()
    );
    assert!(
        normal_previous[0].y >= views::SHADED_HEIGHT,
        "normal mode's first Previous glyph belongs to the full transport row, not the strip: {:?}",
        normal.fragments()
    );
    assert!(
        normal_previous[1].y > normal_previous[0].y,
        "the library footer's Previous glyph must sit below the full transport row: {:?}",
        normal.fragments()
    );

    player.shaded = true;
    let shaded = captured_root_view_text_bounds(&player);
    let shaded_previous: Vec<iced::Rectangle> = shaded
        .fragments()
        .iter()
        .filter(|(text, _)| text == "◀◀")
        .map(|(_, bounds)| *bounds)
        .collect();
    assert_eq!(
        shaded_previous.len(),
        1,
        "shade mode must draw exactly the shade strip: {:?}",
        shaded.fragments()
    );
    assert!(
        shaded_previous[0].y >= views::TITLE_BAR_HEIGHT
            && shaded_previous[0].y < views::SHADED_HEIGHT,
        "shade mode's Previous glyph must sit inside the strip: {:?}",
        shaded.fragments()
    );
}

// In normal mode the library footer sits beneath the browse list, with the
// same five glyphs, time readout, and right-edge volume slider as the shade
// strip. A builder-only test cannot catch the root dropping the footer, so
// lay out the whole window and measure it.
#[test]
fn normal_root_view_draws_the_library_strip_beneath_the_browse_list() {
    let (player, _state) = test_player();
    let capture = captured_root_view_text_bounds(&player);

    let previous: Vec<iced::Rectangle> = capture
        .fragments()
        .iter()
        .filter(|(text, _)| text == "◀◀")
        .map(|(_, bounds)| *bounds)
        .collect();
    assert_eq!(
        previous.len(),
        2,
        "normal mode must draw the full transport row and the library footer: {:?}",
        capture.fragments()
    );
    let footer = previous[1];
    let top = capture.find("Top");
    assert!(
        footer.y > top.y,
        "the library footer must sit below the browse list's Top button: footer {footer:?}, Top {top:?}"
    );
    assert!(
        footer.y + footer.height <= capture.size().height,
        "the library footer must stay inside the window: footer {footer:?} in {:?}",
        capture.size()
    );

    let elapsed = player.now_playing_elapsed(None);
    assert!(
        capture
            .fragments()
            .iter()
            .any(|(text, bounds)| text == elapsed.as_ref() && (bounds.y - footer.y).abs() < 5.0),
        "missing the footer's {elapsed:?} time readout: {:?}",
        capture.fragments()
    );
    assert!(
        captured_root_view_volume_slider(&player).is_some(),
        "the library footer's volume slider must be laid out: {:?}",
        capture.fragments()
    );
}

// Both strips pin the two `view` call sites, not just the builders: after a
// song with a cached total is playing, the normal footer and the shade strip
// must draw the elapsed time over that total, and never the double placeholder.
#[test]
fn root_view_footer_shows_elapsed_over_total() {
    let (mut player, _state) = play_song_1_then_browse_to_album_2();

    for shaded in [false, true] {
        player.shaded = shaded;
        let capture = captured_root_view_text_bounds(&player);
        let expected = "00:00/3:30";
        assert!(
            capture.fragments().iter().any(|(text, _)| text == expected),
            "the shaded={shaded} strip must draw {expected}: {:?}",
            capture.fragments()
        );
        assert!(
            !capture
                .fragments()
                .iter()
                .any(|(text, _)| text == "--:--/--:--"),
            "the shaded={shaded} strip must not double the placeholder: {:?}",
            capture.fragments()
        );
    }
}

// The README and the Browse row order invariant both say Sort renders beside
// Top. This lays out the whole root `view` — the same column whose separate
// `.push()` calls once stacked the buttons — and reads the text bounds it
// would draw: Top and Sort must share a row, with Sort at or after Top's
// right edge. A unit test of the nav row alone would not catch the root
// pushing the two buttons itself.
#[test]
fn browse_nav_buttons_place_sort_beside_top() {
    let (player, _state) = test_player();
    let capture = captured_root_view_text_bounds(&player);
    let top = capture.find("Top");
    let sort = capture.find("Sort");

    assert_eq!(
        top.y,
        sort.y,
        "Sort must share Top's row: {:?}",
        capture.fragments()
    );
    assert!(
        sort.x >= top.x + top.width,
        "Sort must render to the right of Top: {:?}",
        capture.fragments()
    );
}

// Reload is a browse-list control too: it must render on every level so a
// stale library can be refreshed without restarting or losing the current
// task's playback. Lay out the root view for each level and find its label.
#[test]
fn reload_button_is_shown_on_every_browse_level() {
    let (mut player, _state) = test_player();

    for current_view in BROWSE_VIEWS {
        let view_name = format!("{current_view:?}");
        player.current_view = current_view;
        let capture = captured_root_view_text_bounds(&player);
        let reload = capture.find("Reload");
        assert!(
            reload.width > 0.0,
            "the Reload button must lay out on {view_name}: {reload:?}"
        );
    }
}

// Reload joins the same nav row as Top and Sort, after Sort, so the three
// buttons line up rather than stacking or pushing the list down another row.
#[test]
fn browse_nav_buttons_place_reload_beside_sort() {
    let (player, _state) = test_player();
    let capture = captured_root_view_text_bounds(&player);
    let top = capture.find("Top");
    let sort = capture.find("Sort");
    let reload = capture.find("Reload");

    assert_eq!(
        reload.y,
        top.y,
        "Reload must share Top's and Sort's row: {:?}",
        capture.fragments()
    );
    assert_eq!(
        sort.y,
        reload.y,
        "Reload must share Sort's row: {:?}",
        capture.fragments()
    );
    assert!(
        reload.x >= sort.x + sort.width,
        "Reload must render to the right of Sort: {:?}",
        capture.fragments()
    );
}

// Open is a browse-list control: it sits above the list it opens from, so the
// root must push it above the first row on every level.
#[test]
fn browse_open_button_sits_above_the_browse_list() {
    let (mut player, _state) = test_player();
    player.artists.items = vec![sample_artist()];
    player.artists.loading = false;

    let capture = captured_root_view_text_bounds(&player);
    let open = capture.find("Open");
    let row = capture.find("The Sample Band");
    assert!(
        open.y < row.y,
        "Open must sit above the browse list's first row: {open:?}, row {row:?}"
    );
}

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

// The transport row draws both Play and Pause in every playback state, so
// the row is playback-independent. Lay out the whole root `view` while
// stopped and while playing and assert the Play and Pause glyphs keep the
// same bounds, alongside the Previous/Next markers that bracket the row.
#[test]
fn transport_row_keeps_play_and_pause_glyphs_fixed_across_playback_states() {
    use iced::Rectangle;

    let (player, state) = test_player();
    let row_glyphs = |capture: &layout_capture::TextCapture| {
        let previous = capture.find("◀◀");
        let next = capture.find("▶▶");
        capture
            .fragments()
            .iter()
            .filter(|(text, bounds)| {
                bounds.y == previous.y
                    && bounds.x >= previous.x
                    && bounds.x <= next.x + next.width
                    && ["◀◀", "▶", "II", "■", "▶▶"].contains(&text.as_str())
            })
            .map(|(text, bounds)| (text.clone(), *bounds))
            .collect::<Vec<(String, Rectangle)>>()
    };

    state.blocking_lock().is_playing = false;
    let stopped = captured_root_view_text_bounds(&player);
    let stopped_glyphs = row_glyphs(&stopped);

    state.blocking_lock().is_playing = true;
    let playing = captured_root_view_text_bounds(&player);
    let playing_glyphs = row_glyphs(&playing);

    assert!(
        stopped_glyphs.iter().any(|(text, _)| text == "▶")
            && stopped_glyphs.iter().any(|(text, _)| text == "II"),
        "the transport row must draw both Play and Pause: {:?}",
        stopped.fragments()
    );
    assert_eq!(
        stopped_glyphs,
        playing_glyphs,
        "the transport row must not move or change when playback starts: {:?}",
        playing.fragments()
    );
}

// The main transport row is built in `src/ui/views/transport.rs`, but only the
// root `view` shows whether it reaches the window. Lay out the whole root and
// find the Previous and Next glyphs: they must share one row with Previous
// left of Next, matching the mockup's compact button order.
#[test]
fn root_view_draws_the_transport_glyph_row() {
    let (player, _state) = test_player();
    let capture = captured_root_view_text_bounds(&player);
    let previous = capture.find("◀◀");
    let next = capture.find("▶▶");

    assert_eq!(
        previous.y,
        next.y,
        "Previous and Next must share the transport row: {:?}",
        capture.fragments()
    );
    assert!(
        next.x >= previous.x + previous.width,
        "Next must render to the right of Previous: {:?}",
        capture.fragments()
    );
}

// The root `view` resolves the LCD's transport mode from the shared state, so
// the glyph the user sees follows Play, Pause, and Stop. The unit tests in
// `src/ui/views/tests/` pass a mode straight to the row and cannot catch the
// root handing it a stale or hard-coded mode. Lay out the whole root `view`
// after each state change and read the glyphs it would draw.
#[test]
fn root_view_draws_the_glyph_for_the_shared_transport_mode() {
    let (player, state) = test_player();
    // The LCD's glyph is drawn before the transport row's, so the first
    // fragment carrying one of these three glyphs is the LCD's.
    let shown_glyphs = |player: &WinampPlayer| {
        let capture = captured_root_view_text_bounds(player);
        capture
            .fragments()
            .iter()
            .find(|(text, _)| ["▶", "II", "■"].contains(&text.as_str()))
            .map(|(text, _)| vec![text.clone()])
            .unwrap_or_default()
    };

    state.blocking_lock().stop();
    assert_eq!(
        shown_glyphs(&player),
        vec!["■"],
        "a stopped transport must draw the stopped glyph"
    );

    state.blocking_lock().play();
    assert_eq!(
        shown_glyphs(&player),
        vec!["▶"],
        "a playing transport must draw the playing glyph"
    );

    state.blocking_lock().pause();
    assert_eq!(
        shown_glyphs(&player),
        vec!["II"],
        "a paused transport must draw the paused glyph"
    );

    state.blocking_lock().stop();
    assert_eq!(
        shown_glyphs(&player),
        vec!["■"],
        "Stop after Pause must draw the stopped glyph again"
    );
}

// Normal mode introduces the equalizer and browse sections with the mockups'
// title bands. A per-band widget test can pass while the root never pushes the
// bands or pushes them in the wrong place, so lay out the whole root with an
// album's songs open and read the vertical positions. The Songs view draws
// Back above its list, so the library band must come before that button too,
// not after it as a push placed with the Top/Sort group would.
#[test]
fn root_view_draws_the_equalizer_and_library_title_bands_in_order() {
    let (mut player, _state) = player_stepping_from(None);
    player.current_view = CurrentView::Songs;

    let capture = captured_root_view_text_bounds(&player);
    let equalizer = capture.find("NOSTALGIA EQUALIZER");
    let library = capture.find("NOSTALGIA LIBRARY");
    let back = capture.find("Back");
    let first_row = capture.find("1. ");

    assert!(
        equalizer.y < library.y,
        "the equalizer band must sit above the library band: {equalizer:?} vs {library:?}"
    );
    assert!(
        library.y < back.y,
        "the library band must sit above Back: {library:?} vs {back:?}"
    );
    assert!(
        back.y < first_row.y,
        "Back must sit above the first browse row: {back:?} vs {first_row:?}"
    );
}

// The section bands are part of the normal column, not the rolled-up frame:
// shade mode still draws the app bar but neither section label.
#[test]
fn shaded_root_view_draws_no_section_title_bands() {
    let (mut player, _state) = test_player();
    player.shaded = true;

    let capture = captured_root_view_text_bounds(&player);
    let texts: Vec<&str> = capture.texts().collect();
    assert!(
        texts.contains(&"NOSTALGIA"),
        "the shaded app bar must still draw the app name: {texts:?}"
    );
    assert!(
        !texts.contains(&"NOSTALGIA EQUALIZER"),
        "shade mode must not draw the equalizer band: {texts:?}"
    );
    assert!(
        !texts.contains(&"NOSTALGIA LIBRARY"),
        "shade mode must not draw the library band: {texts:?}"
    );
}

// The root `view` decides which chrome the current level offers: Back below
// the top of the hierarchy, Retry only for a failed top-level artists fetch,
// and the All Songs/All Albums/Playlists buttons only on Artists. The
// `can_go_back` and `can_retry_artists` unit tests check the predicates, and
// the per-level view tests build single panels, so a root mistake — the wrong
// predicate, a missing `matches!`, or a stale error checked on every level —
// would pass them. Lay out the whole root for each level and read what it
// draws.
#[test]
fn root_view_offers_back_retry_and_flat_list_buttons_only_where_they_belong() {
    let (mut player, _state) = test_player();
    let shows = |player: &WinampPlayer, label: &str| {
        captured_root_view_text_bounds(player)
            .texts()
            .any(|text| text == label)
    };
    let flat_list_buttons = [
        "All Songs",
        "All Albums",
        "Playlists",
        "Recently Added",
        "Recently Played",
    ];

    // Artists with a healthy fetch: the three flat-list entry points, but no
    // Back (nothing above it) and no Retry (nothing failed).
    for label in flat_list_buttons {
        assert!(
            shows(&player, label),
            "Artists must draw the {label:?} button"
        );
    }
    assert!(!shows(&player, "Back"), "Artists must not draw Back");
    assert!(!shows(&player, "Retry"), "Artists must not draw Retry");

    // A failed artists fetch adds Retry without adding Back, and keeps the
    // flat-list entry points.
    player.artists.error = Some("music-library fetch failed: boom".to_string());
    assert!(
        shows(&player, "Retry"),
        "a failed artists fetch must draw Retry"
    );
    assert!(!shows(&player, "Back"), "Retry must not add Back");
    for label in flat_list_buttons {
        assert!(
            shows(&player, label),
            "a failed artists fetch must keep the {label:?} button"
        );
    }

    // Every level below Artists offers Back and drops the flat-list entry
    // points. The stale artists error stays set to prove Retry is gated on the
    // level as well as the error, not on the error alone.
    for view in [
        CurrentView::Albums,
        CurrentView::Songs,
        CurrentView::AllSongs,
        CurrentView::AllAlbums,
        CurrentView::RecentlyAdded,
        CurrentView::RecentlyPlayed,
        CurrentView::Playlists,
    ] {
        let level = format!("{view:?}");
        player.current_view = view;
        assert!(shows(&player, "Back"), "{level} must draw Back");
        assert!(!shows(&player, "Retry"), "{level} must not draw Retry");
        for label in flat_list_buttons {
            assert!(
                !shows(&player, label),
                "{level} must not draw the {label:?} button"
            );
        }
    }
}

// The root `view` chooses the browse list to draw from `current_view`, passing
// each level its own buffer, cached row text, and current-track id. The
// per-level view tests feed each builder directly, and every root-view test
// above lays out the default Artists level, so an arm wired to the wrong
// buffer — or dropping the song caches or the playing marker — would still
// pass them. Give every buffer distinct row text, labels, and total time, then
// lay out the whole root once per level and read what it actually drew.
#[test]
fn root_view_draws_the_current_browse_levels_own_rows() {
    let (mut player, state) = test_player();

    let mut artist = sample_artist();
    artist.name = "Artist Level Row".to_string();
    let mut album = sample_album();
    album.title = "Album Level Row".to_string();
    let mut all_album = sample_album();
    all_album.title = "All Albums Level Row".to_string();
    let mut song = sample_song();
    song.title = "Songs Item Title".to_string();
    song.duration_ms = 210_000;
    let mut all_song = sample_song();
    all_song.title = "All Songs Item Title".to_string();
    all_song.duration_ms = 95_000;
    let mut playlist = sample_playlist();
    playlist.name = "Playlist Level Row".to_string();

    player.artists.items = vec![artist];
    player.albums.items = vec![album];
    player.songs.items = vec![song.clone()];
    player.songs.titles = vec!["Songs Cached Row".to_string()];
    player.songs.labels = vec!["9:01".to_string()];
    player.all_songs.items = vec![all_song];
    player.all_songs.titles = vec!["All Songs Cached Row".to_string()];
    player.all_songs.labels = vec!["9:02".to_string()];
    player.all_albums.items = vec![all_album];
    player.playlists.items = vec![playlist];
    // Both song lists hold the playing track, so each arm must pass the
    // current id through to mark its own row.
    state.blocking_lock().current_track = Some("song-1".to_string());

    // The expected primary row text, the footer and cached label unique to a
    // song list, and whether this level marks the playing row.
    let cases = [
        (
            CurrentView::Artists,
            "Artist Level Row",
            "1 artist",
            "",
            false,
        ),
        (
            CurrentView::Albums,
            "The Sample Band - Album Level Row",
            "1 album - 0:00+",
            "",
            false,
        ),
        (
            CurrentView::Songs,
            "Songs Cached Row",
            "1 song - 3:30",
            "9:01",
            true,
        ),
        (
            CurrentView::AllSongs,
            "All Songs Cached Row",
            "1 song - 1:35",
            "9:02",
            true,
        ),
        (
            CurrentView::AllAlbums,
            "The Sample Band - All Albums Level Row",
            "1 album - 0:00+",
            "",
            false,
        ),
        (
            CurrentView::Playlists,
            "Playlist Level Row",
            "1 playlist",
            "",
            false,
        ),
    ];
    let row_texts = [
        "Artist Level Row",
        "The Sample Band - Album Level Row",
        "Songs Cached Row",
        "All Songs Cached Row",
        "The Sample Band - All Albums Level Row",
        "Playlist Level Row",
    ];

    for (view, row, footer, label, marked) in cases {
        let level = format!("{view:?}");
        player.current_view = view;
        let capture = captured_root_view_text_bounds(&player);
        let texts: Vec<&str> = capture.texts().collect();

        assert!(
            texts.contains(&row),
            "{level} must draw its own row text {row:?}: {:?}",
            capture.fragments()
        );
        assert!(
            texts.contains(&footer),
            "{level} must draw its own footer {footer:?}: {:?}",
            capture.fragments()
        );
        if !label.is_empty() {
            assert!(
                texts.contains(&label),
                "{level} must pass its cached row label {label:?}: {:?}",
                capture.fragments()
            );
        }
        for other in row_texts {
            if other != row {
                assert!(
                    !texts.contains(&other),
                    "{level} also drew another level's row {other:?}: {:?}",
                    capture.fragments()
                );
            }
        }
        assert_eq!(
            texts.iter().any(|text| text.starts_with("▶ ")),
            marked,
            "{level} marked the playing row: {:?}",
            capture.fragments()
        );
    }
}

// The Artists footer adds the total of the rows' album counts after the
// artist count.
#[test]
fn root_view_artists_footer_totals_the_album_counts() {
    let (mut player, _state) = test_player();
    let mut artist = sample_artist();
    artist.album_count = 2;
    player.artists.items = vec![artist];
    player.artists.loading = false;
    player.current_view = CurrentView::Artists;
    let capture = captured_root_view_text_bounds(&player);
    assert!(
        capture.texts().any(|text| text == "1 artist - 2 albums"),
        "{:?}",
        capture.fragments()
    );
}

// A browse list with no rows shows only its empty wording: `list_with_count`
// returns the list unchanged at a zero count, and `song_list` skips its
// count-and-time footer, so the loading, error, or empty placeholder stays
// alone. A footer such as "0 artists" or "0 songs - 0:00" would describe rows
// the panel does not have, so pin its absence at every level along with the
// wording that must remain.
#[test]
fn an_empty_browse_level_draws_no_count_footer() {
    let (mut player, _state) = test_player();
    // A fresh player is empty but has the boot-time artists fetch in flight;
    // mark every list loaded so each level shows its empty wording.
    player.artists.loading = false;
    player.albums.loading = false;
    player.songs.loading = false;
    player.all_songs.loading = false;
    player.all_albums.loading = false;
    player.playlists.loading = false;

    let cases = [
        (CurrentView::Artists, "No artists", "0 artists"),
        (CurrentView::Albums, "No albums", "0 albums - 0:00"),
        (CurrentView::Songs, "No songs", "0 songs - 0:00"),
        (CurrentView::AllSongs, "No songs", "0 songs - 0:00"),
        (CurrentView::AllAlbums, "No albums", "0 albums - 0:00"),
        (CurrentView::Playlists, "No playlists", "0 playlists"),
    ];

    for (view, empty_label, footer) in cases {
        let level = format!("{view:?}");
        player.current_view = view;
        let capture = captured_root_view_text_bounds(&player);
        let texts: Vec<&str> = capture.texts().collect();
        assert!(
            texts.contains(&empty_label),
            "{level} must draw its empty wording {empty_label:?}: {:?}",
            capture.fragments()
        );
        assert!(
            !texts.contains(&footer),
            "{level} must not draw a zero-count footer {footer:?}: {:?}",
            capture.fragments()
        );
    }
}

// Every browse level's placeholder is chosen by `browse_placeholder` from
// that level's own `loading`, `error`, and (for Songs) `search_active` flags.
// The pure wording tests and each builder test cover that choice, but only a
// laid-out root proves `view` passes the right level's flags into the
// matching builder. Give one level at a time a loading flag, then a failed
// report, and read the placeholder the root actually draws; finally show an
// empty search, whose no-match wording replaces the empty-album label.
#[test]
fn root_view_draws_each_browse_levels_own_placeholder() {
    // One setter per level, so each case names its own buffer and sets
    // `current_view` itself (the enum is not `Copy`). One report per level
    // too, so a wrong arm reading its neighbour's error is caught rather than
    // matched by a shared string.
    type SetLevel = fn(&mut WinampPlayer, bool, Option<&str>);
    let cases: [(SetLevel, &str); 6] = [
        (
            |player, loading, error| {
                player.current_view = CurrentView::Artists;
                player.artists.loading = loading;
                player.artists.error = error.map(str::to_string);
            },
            "artists failed",
        ),
        (
            |player, loading, error| {
                player.current_view = CurrentView::Albums;
                player.albums.loading = loading;
                player.albums.error = error.map(str::to_string);
            },
            "albums failed",
        ),
        (
            |player, loading, error| {
                player.current_view = CurrentView::Songs;
                player.songs.loading = loading;
                player.songs.error = error.map(str::to_string);
            },
            "songs failed",
        ),
        (
            |player, loading, error| {
                player.current_view = CurrentView::AllSongs;
                player.all_songs.loading = loading;
                player.all_songs.error = error.map(str::to_string);
            },
            "all songs failed",
        ),
        (
            |player, loading, error| {
                player.current_view = CurrentView::AllAlbums;
                player.all_albums.loading = loading;
                player.all_albums.error = error.map(str::to_string);
            },
            "all albums failed",
        ),
        (
            |player, loading, error| {
                player.current_view = CurrentView::Playlists;
                player.playlists.loading = loading;
                player.playlists.error = error.map(str::to_string);
            },
            "playlists failed",
        ),
    ];

    for (set_state, report) in cases {
        let (mut player, _state) = test_player();
        // The boot-time artists fetch starts loading; clear it so only the
        // level under test decides what the root draws.
        player.artists.loading = false;

        set_state(&mut player, true, None);
        let capture = captured_root_view_text_bounds(&player);
        let texts: Vec<&str> = capture.texts().collect();
        assert!(
            texts.contains(&"Loading…"),
            "the {report} case must draw the loading placeholder: {texts:?}"
        );

        set_state(&mut player, false, Some(report));
        let capture = captured_root_view_text_bounds(&player);
        let texts: Vec<&str> = capture.texts().collect();
        assert!(
            texts.contains(&report),
            "the {report} case must draw its own report: {texts:?}"
        );
        assert!(
            !texts.contains(&"Loading…"),
            "the {report} case must drop the loading placeholder on failure: {texts:?}"
        );
    }

    let (mut player, _state) = test_player();
    player.artists.loading = false;
    player.current_view = CurrentView::Songs;
    player.search_active = true;
    let capture = captured_root_view_text_bounds(&player);
    assert!(
        capture
            .texts()
            .any(|text| text == "No songs match your search"),
        "an empty search must draw its no-match wording: {:?}",
        capture.texts().collect::<Vec<_>>()
    );
}

// The options menu draws over the normal-mode player with its row labels, and
// shade mode never draws it even if the flag is left set.
#[test]
fn options_panel_draws_over_the_player_and_drops_out_in_shade_mode() {
    let (mut player, _state) = test_player();
    player.options_visible = true;

    let normal = captured_root_view_text_bounds(&player);
    let texts: Vec<&str> = normal.texts().collect();
    for expected in ["OPTIONS", "Always on Top: Off", "Time: Elapsed", "Close"] {
        assert!(
            texts.contains(&expected),
            "the options menu must draw {expected:?}: {texts:?}"
        );
    }

    player.shaded = true;
    let shaded = captured_root_view_text_bounds(&player);
    let texts: Vec<&str> = shaded.texts().collect();
    assert!(
        !texts.contains(&"OPTIONS"),
        "shade mode must never draw the options menu: {texts:?}"
    );
}
