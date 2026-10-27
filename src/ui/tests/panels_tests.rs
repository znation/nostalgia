use super::window_tests::make_current_track_known;
use super::*;

// Clicking the large readout switches it to the track's remaining time and
// back. The mode is presentation-only: the right-edge total never changes,
// Pause leaves the remaining value frozen, and a new selection counts down
// from that track's own length. The mode itself survives both transports and
// selections until the next click.
#[test]
fn toggle_time_display_switches_to_remaining_and_persists() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 0 });
    player.playback_position = Some(Duration::from_secs(53));

    let _ = update(&mut player, Message::ToggleTimeDisplay);
    assert_eq!(player.time_display, views::TimeDisplay::Remaining);
    assert_eq!(player.now_playing_elapsed(Some("song-1")), "-02:37");
    assert_eq!(player.now_playing_time(Some("song-1")), "3:30");

    let _ = update(&mut player, Message::Pause);
    assert_eq!(player.now_playing_elapsed(Some("song-1")), "-02:37");

    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 1 });
    assert_eq!(player.time_display, views::TimeDisplay::Remaining);
    assert_eq!(player.now_playing_elapsed(Some("song-2")), "-02:05");
    assert_eq!(player.now_playing_time(Some("song-2")), "2:05");

    let _ = update(&mut player, Message::ToggleTimeDisplay);
    assert_eq!(player.time_display, views::TimeDisplay::Elapsed);
    assert_eq!(player.now_playing_elapsed(Some("song-2")), "00:00");
    assert_eq!(player.now_playing_time(Some("song-2")), "2:05");
}

#[test]
fn toggle_time_display_with_no_track_keeps_the_placeholder() {
    let (mut player, _state) = test_player();

    let _ = update(&mut player, Message::ToggleTimeDisplay);

    assert_eq!(player.time_display, views::TimeDisplay::Remaining);
    assert_eq!(player.now_playing_elapsed(None), "--:--");
}

// The indicator strip's V control hides and shows the LCD bars. It is
// presentation-only: the stored spectrum stays, so showing the bars again
// draws the current ones, the spectrum poll keeps running while they are
// hidden, and no transport or mixer state moves.
#[test]
fn toggle_visualizer_hides_and_shows_the_bars_without_clearing_the_spectrum() {
    let (mut player, state) = test_player();
    let spectrum = recording_spectrum();
    player.spectrum = Some(spectrum);
    player.spectrum_peaks = Some(spectrum);
    state.blocking_lock().play();
    let before = {
        let state = state.blocking_lock();
        (
            state.transport_mode(),
            state.is_playing,
            state.volume(),
            state.balance(),
            state.repeat,
            state.shuffle,
        )
    };
    assert!(player.visualizer_visible);
    assert!(player.needs_spectrum_poll());

    assert_message_schedules_no_work(&mut player, Message::ToggleVisualizer);
    assert!(!player.visualizer_visible);
    assert_eq!(player.spectrum, Some(spectrum));
    assert_eq!(player.spectrum_peaks, Some(spectrum));
    assert!(
        player.needs_spectrum_poll(),
        "hiding the bars must leave the spectrum poll running"
    );

    assert_message_schedules_no_work(&mut player, Message::ToggleVisualizer);
    assert!(player.visualizer_visible);
    assert_eq!(player.spectrum, Some(spectrum));
    assert_eq!(player.spectrum_peaks, Some(spectrum));
    assert!(player.needs_spectrum_poll());

    let after = {
        let state = state.blocking_lock();
        (
            state.transport_mode(),
            state.is_playing,
            state.volume(),
            state.balance(),
            state.repeat,
            state.shuffle,
        )
    };
    assert_eq!(
        after, before,
        "the visualizer toggle must leave transport and mixer state alone"
    );
}

// The root view must apply the stored flag, not just expose `visible_spectrum`:
// lay the real `view` out with a loaded spectrum and read the pixels in the
// LCD's visualizer row. Showing it paints a bright green bar; hiding it leaves
// that row empty. Passing the raw spectrum through instead of the gated one
// would keep the bars green here.
#[test]
fn hidden_visualizer_draws_no_bars_in_the_root_view() {
    let (mut player, _state) = test_player();
    player.spectrum = Some(recording_spectrum());
    let mut renderer = layout_capture::headless_renderer();

    player.visualizer_visible = true;
    let (shown_text, shown_pixels) = {
        let mut shown = view(&player);
        layout_capture::capture_text_and_pixels(&mut shown, &mut renderer, &iced::Theme::Dark)
    };

    player.visualizer_visible = false;
    let hidden_pixels = {
        let mut hidden = view(&player);
        let (_hidden_text, hidden_pixels) =
            layout_capture::capture_text_and_pixels(&mut hidden, &mut renderer, &iced::Theme::Dark);
        hidden_pixels
    };

    // `MONO` starts just below the 24px visualizer row, and the strip's `O`
    // locates its left edge (the strip itself is 12px wide). Stop 4px above the
    // status text so only the bar row's pixels are read.
    let status = shown_text.find("MONO");
    let strip_left = shown_text.find("O").x;
    let bars = iced::Rectangle {
        x: strip_left + 12.0,
        y: status.y - 24.0,
        width: 200.0,
        height: 20.0,
    };
    let shown_green = shown_pixels.max_green_in(bars);
    let hidden_green = hidden_pixels.max_green_in(bars);
    assert!(
        shown_green > 200,
        "the visible visualizer must draw its green bars: {bars:?} has green {shown_green}"
    );
    assert!(
        hidden_green < 50,
        "hiding the visualizer must leave its row without bars: {bars:?} has green {hidden_green}"
    );
}

// The player row's EQ button shows and hides the equalizer section. It is a
// layout toggle only: the shared equalizer state and the values the audio
// layer would read must survive it, and hiding the section must leave the
// library section in place but move it up into the freed height.
#[test]
fn toggle_equalizer_panel_hides_and_restores_the_section() {
    let (mut player, state) = test_player();
    let mut artist = sample_artist();
    artist.name = "Panel Toggle Artist".to_string();
    player.artists.items = vec![artist];
    // Turn the audio equalizer on so the panel's own button reads `EQ: On`;
    // hiding the panel must not switch it off.
    state.blocking_lock().toggle_equalizer();
    let before = {
        let state = state.blocking_lock();
        (
            state.eq_enabled,
            state.eq_preamp(),
            state.eq_bands(),
            state.eq_preset(),
        )
    };
    assert!(player.eq_panel_visible);

    let open = captured_root_view_text_bounds(&player);
    let open_band = open.find("NOSTALGIA EQUALIZER");
    let open_library = open.find("NOSTALGIA LIBRARY");
    open.find("EQ: On");
    open.find("Panel Toggle Artist");

    assert_message_schedules_no_work(&mut player, Message::ToggleEqualizerPanel);
    assert!(!player.eq_panel_visible);

    let hidden = captured_root_view_text_bounds(&player);
    let hidden_texts: Vec<&str> = hidden.texts().collect();
    assert!(
        !hidden_texts.contains(&"NOSTALGIA EQUALIZER"),
        "hiding the panel must drop its title band: {hidden_texts:?}"
    );
    assert!(
        !hidden_texts.contains(&"EQ: On"),
        "hiding the panel must drop the equalizer's own on/off button: {hidden_texts:?}"
    );
    assert!(
        hidden_texts.contains(&"EQ"),
        "the transport row's EQ button must stay drawn: {hidden_texts:?}"
    );
    let hidden_library = hidden.find("NOSTALGIA LIBRARY");
    let hidden_artist = hidden.find("Panel Toggle Artist");
    assert!(
        hidden_library.y < open_library.y,
        "the library band must move up into the freed height: {hidden_library:?} vs {open_library:?}"
    );
    assert!(
        hidden_artist.y > hidden_library.y,
        "the browse list must keep drawing its rows below the library band"
    );

    assert_message_schedules_no_work(&mut player, Message::ToggleEqualizerPanel);
    assert!(player.eq_panel_visible);

    let restored = captured_root_view_text_bounds(&player);
    assert_eq!(restored.find("NOSTALGIA EQUALIZER"), open_band);
    assert_eq!(restored.find("EQ: On").y, open.find("EQ: On").y);
    assert_eq!(restored.find("NOSTALGIA LIBRARY"), open_library);

    let after = {
        let state = state.blocking_lock();
        (
            state.eq_enabled,
            state.eq_preamp(),
            state.eq_bands(),
            state.eq_preset(),
        )
    };
    assert_eq!(
        after, before,
        "the panel toggle must leave the equalizer's state untouched"
    );
}

// The player row's PL button shows and hides the whole library section. Like
// the EQ toggle it is layout-only: the player panels stay drawn, the shared
// playback and equalizer state and the browse level survive it, and toggling
// back restores every removed element at its original bounds.
#[test]
fn toggle_library_panel_hides_and_restores_the_library_section() {
    let (mut player, state) = test_player();
    let mut artist = sample_artist();
    artist.name = "Library Toggle Artist".to_string();
    player.artists.items = vec![artist];
    let before = playback_snapshot(&state);
    assert_eq!(player.current_view, CurrentView::Artists);
    assert!(player.library_panel_visible);
    let browse_target = Some(iced::advanced::widget::Id::new(BROWSE_LIST_ID));
    assert!(
        root_scrollable_ids(&player).contains(&browse_target),
        "the open library must build the browse scroll target"
    );

    let open = captured_root_view_text_bounds(&player);
    let open_band = open.find("NOSTALGIA LIBRARY");
    let open_artist = open.find("Library Toggle Artist");
    let open_footer = captured_root_view_volume_slider(&player)
        .expect("the library footer's volume slider must be drawn while the library shows");
    let eq = open.find("EQ");
    let pl = open.find("PL");
    assert_eq!(pl.y, eq.y, "PL must share the transport row's EQ row");
    assert!(
        pl.x >= eq.x + eq.width,
        "PL must start at or after EQ's right edge: {pl:?} vs {eq:?}"
    );

    assert_message_schedules_no_work(&mut player, Message::ToggleLibraryPanel);
    assert!(!player.library_panel_visible);
    let hidden_scrollables = root_scrollable_ids(&player);
    assert!(
        !hidden_scrollables.contains(&browse_target),
        "hiding the library must build no browse scroll target: {hidden_scrollables:?}"
    );

    let hidden = captured_root_view_text_bounds(&player);
    let hidden_texts: Vec<&str> = hidden.texts().collect();
    for gone in [
        "NOSTALGIA LIBRARY",
        "Library Toggle Artist",
        "Open",
        "Top",
        "Clear",
    ] {
        assert!(
            !hidden_texts.contains(&gone),
            "hiding the library must drop {gone}: {hidden_texts:?}"
        );
    }
    for kept in ["PL", "EQ", "NOSTALGIA EQUALIZER", "◀◀", "■", "▶▶"] {
        assert!(
            hidden_texts.contains(&kept),
            "the player panels must keep drawing {kept}: {hidden_texts:?}"
        );
    }
    assert_eq!(
        captured_root_view_volume_slider(&player),
        None,
        "hiding the library must drop its footer"
    );

    assert_message_schedules_no_work(&mut player, Message::ToggleLibraryPanel);
    assert!(player.library_panel_visible);

    let restored = captured_root_view_text_bounds(&player);
    assert_eq!(restored.find("NOSTALGIA LIBRARY"), open_band);
    assert_eq!(restored.find("Library Toggle Artist"), open_artist);
    assert_eq!(captured_root_view_volume_slider(&player), Some(open_footer));

    let after = playback_snapshot(&state);
    assert_eq!(
        after, before,
        "the library panel toggle must leave playback, volume, and equalizer state untouched"
    );
    assert_eq!(
        player.current_view,
        CurrentView::Artists,
        "the library panel toggle must not change the browse level"
    );
}

// Escape closes whichever transient panel is showing, like its `Close` button
// or the backdrop, without touching playback, transport, mixer, browse, or
// selection state. With no panel open it is a no-op.
#[test]
fn escape_closes_each_transient_panel_without_touching_anything_else() {
    let (mut player, state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    player.current_view = CurrentView::Songs;
    make_current_track_known(&mut player, &state);
    player.playback_duration = Some(Duration::from_secs(210));
    let _ = update(&mut player, Message::SelectRow { epoch: 1, index: 0 });

    assert!(!player.transient_panel_open());
    assert_message_schedules_no_work(&mut player, Message::EscapePressed);
    assert!(!player.transient_panel_open());

    let before = {
        let state = state.blocking_lock();
        (
            state.transport_mode(),
            state.current_track.clone(),
            state.volume(),
            state.balance(),
            state.repeat,
            state.shuffle,
            player.songs.selected(),
        )
    };

    for toggle in [
        Message::ToggleAbout,
        Message::ToggleOptions,
        Message::ToggleFileInfo,
        Message::ToggleJumpTime,
        Message::ToggleJumpFile,
    ] {
        let _ = update(&mut player, toggle);
        assert!(player.transient_panel_open());
        assert_message_schedules_no_work(&mut player, Message::EscapePressed);
        assert!(!player.transient_panel_open());
    }

    let after = {
        let state = state.blocking_lock();
        (
            state.transport_mode(),
            state.current_track.clone(),
            state.volume(),
            state.balance(),
            state.repeat,
            state.shuffle,
            player.songs.selected(),
        )
    };
    assert_eq!(
        after, before,
        "Escape must close panels without changing playback, mixer, or browse state"
    );
    assert_eq!(
        player.current_view,
        CurrentView::Songs,
        "Escape must not change the browse level"
    );
}

// The Stop after current row arms the one-shot and closes the menu; choosing
// it again (here by keyboard) disarms it.
#[test]
fn options_stop_after_current_toggles_the_flag_and_closes_the_panel() {
    let (mut player, _state) = test_player();
    player.options_visible = true;
    let _ = update(
        &mut player,
        Message::OptionsChoice(OptionsChoice::StopAfterCurrent),
    );
    assert!(player.stop_after_current);
    assert!(!player.options_visible);

    player.options_visible = true;
    player.options_selected = OptionsRow::ALL
        .iter()
        .position(|row| matches!(row, OptionsRow::StopAfterCurrent))
        .unwrap();
    let _ = update(&mut player, Message::OptionsActivate);
    assert!(!player.stop_after_current);
    assert!(!player.options_visible);
}

// The Repeat and Shuffle rows flip only their shared AppState flag and close
// the menu; activating each by keyboard flips it back.
#[test]
fn options_repeat_and_shuffle_toggle_shared_state_and_close_the_panel() {
    for choice in [OptionsChoice::Repeat, OptionsChoice::Shuffle] {
        let is_repeat = matches!(choice, OptionsChoice::Repeat);
        let (mut player, state) = test_player();
        let flags = || {
            let state = state.blocking_lock();
            (state.repeat, state.shuffle)
        };
        let before = flags();
        player.options_visible = true;
        let _ = update(&mut player, Message::OptionsChoice(choice));
        let expected = if is_repeat {
            (!before.0, before.1)
        } else {
            (before.0, !before.1)
        };
        assert_eq!(flags(), expected, "{choice:?} must flip only its own flag");
        assert!(!player.options_visible);

        player.options_visible = true;
        player.options_selected = OptionsRow::ALL
            .iter()
            .position(|row| {
                matches!(
                    (row, is_repeat),
                    (OptionsRow::Repeat, true) | (OptionsRow::Shuffle, false)
                )
            })
            .unwrap();
        let _ = update(&mut player, Message::OptionsActivate);
        assert_eq!(flags(), before, "{choice:?} by keyboard must flip it back");
        assert!(!player.options_visible);
    }
}

// Each options-menu row flips exactly the flag its existing toggle owns, then
// closes the menu. Everything else, including playback, stays put.
#[test]
fn options_choice_applies_each_toggle_and_closes_the_panel() {
    for (index, choice) in [
        OptionsChoice::AlwaysOnTop,
        OptionsChoice::TimeDisplay,
        OptionsChoice::Visualizer,
        OptionsChoice::EqualizerPanel,
        OptionsChoice::LibraryPanel,
    ]
    .into_iter()
    .enumerate()
    {
        for keyboard in [false, true] {
            let (mut player, state) = test_player();
            player.options_visible = true;
            let before = {
                let state = state.blocking_lock();
                (
                    player.always_on_top,
                    player.time_display,
                    player.visualizer_visible,
                    player.eq_panel_visible,
                    player.library_panel_visible,
                    state.transport_mode(),
                    state.current_track.clone(),
                )
            };

            player.options_selected = index;
            let _ = update(
                &mut player,
                if keyboard {
                    Message::OptionsActivate
                } else {
                    Message::OptionsChoice(choice)
                },
            );

            let (
                always_on_top,
                time_display,
                visualizer_visible,
                eq_panel_visible,
                library_panel_visible,
            ) = (
                player.always_on_top,
                player.time_display,
                player.visualizer_visible,
                player.eq_panel_visible,
                player.library_panel_visible,
            );
            assert!(!player.options_visible, "{choice:?} must close the menu");
            let expected = match choice {
                OptionsChoice::AlwaysOnTop => (!before.0, before.1, before.2, before.3, before.4),
                OptionsChoice::TimeDisplay => {
                    (before.0, before.1.toggled(), before.2, before.3, before.4)
                }
                OptionsChoice::Visualizer => (before.0, before.1, !before.2, before.3, before.4),
                OptionsChoice::EqualizerPanel => {
                    (before.0, before.1, before.2, !before.3, before.4)
                }
                OptionsChoice::LibraryPanel => (before.0, before.1, before.2, before.3, !before.4),
                OptionsChoice::StopAfterCurrent => {
                    unreachable!(
                        "covered by options_stop_after_current_toggles_the_flag_and_closes_the_panel"
                    )
                }
                OptionsChoice::Repeat | OptionsChoice::Shuffle => {
                    unreachable!(
                        "covered by options_repeat_and_shuffle_toggle_shared_state_and_close_the_panel"
                    )
                }
            };
            assert_eq!(
                (
                    always_on_top,
                    time_display,
                    visualizer_visible,
                    eq_panel_visible,
                    library_panel_visible,
                ),
                expected,
                "{choice:?} must flip only its own flag and leave the others alone"
            );
            let after = {
                let state = state.blocking_lock();
                (state.transport_mode(), state.current_track.clone())
            };
            assert_eq!(
                after,
                (before.5, before.6),
                "{choice:?} must leave playback alone"
            );
        }
    }
}

#[test]
fn options_keyboard_selection_clamps_and_activates_each_row() {
    let (mut player, _state) = test_player();
    player.options_selected = 3;
    for message in [
        Message::OptionsSelectionUp,
        Message::OptionsSelectionDown,
        Message::OptionsActivate,
    ] {
        let _ = update(&mut player, message);
        assert_eq!(player.options_selected, 3);
        assert!(!player.options_visible);
        assert!(!player.about_visible);
    }
    let _ = update(&mut player, Message::ToggleOptions);
    assert_eq!(player.options_selected, 0);
    let _ = update(&mut player, Message::OptionsSelectionUp);
    assert_eq!(player.options_selected, 0);
    for index in 1..10 {
        let _ = update(&mut player, Message::OptionsSelectionDown);
        assert_eq!(player.options_selected, index);
    }
    let _ = update(&mut player, Message::OptionsSelectionDown);
    assert_eq!(player.options_selected, 9);
    let _ = update(&mut player, Message::OptionsActivate);
    assert!(!player.options_visible);
    assert!(!player.about_visible);
    let _ = update(&mut player, Message::ToggleOptions);
    assert_eq!(player.options_selected, 0);
    player.options_selected = OptionsRow::ALL
        .iter()
        .position(|row| matches!(row, OptionsRow::About))
        .unwrap();
    let _ = update(&mut player, Message::OptionsActivate);
    assert!(player.about_visible);
    assert!(!player.options_visible);
}

// The options menu is transient like the other panels: opening it closes each
// of them, opening any of them closes it, and with nothing shaded a toggle
// schedules no work.
#[test]
fn toggle_options_opens_and_closes_and_excludes_each_other_panel() {
    let (mut player, state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    player.current_view = CurrentView::Songs;
    make_current_track_known(&mut player, &state);
    player.playback_duration = Some(Duration::from_secs(210));

    assert_message_schedules_no_work(&mut player, Message::ToggleOptions);
    assert!(player.options_visible);
    assert_message_schedules_no_work(&mut player, Message::ToggleOptions);
    assert!(!player.options_visible);

    for (toggle, is_open) in [
        (
            Message::ToggleAbout,
            (|player: &WinampPlayer| player.about_visible) as fn(&WinampPlayer) -> bool,
        ),
        (Message::ToggleFileInfo, |player| player.file_info_visible),
        (Message::ToggleJumpTime, |player| player.jump_time_visible),
        (Message::ToggleJumpFile, |player| player.jump_file_visible),
    ] {
        let _ = update(&mut player, Message::EscapePressed);
        let _ = update(&mut player, Message::ToggleOptions);
        assert!(player.options_visible);
        let _ = update(&mut player, toggle.clone());
        assert!(
            !player.options_visible,
            "opening {toggle:?} must close the options menu"
        );
        assert!(is_open(&player), "opening {toggle:?} must open its panel");

        let _ = update(&mut player, Message::EscapePressed);
        let _ = update(&mut player, toggle);
        assert!(is_open(&player));
        assert_message_schedules_no_work(&mut player, Message::ToggleOptions);
        assert!(
            player.options_visible && !is_open(&player),
            "opening the options menu must close the other panel"
        );
    }
}
