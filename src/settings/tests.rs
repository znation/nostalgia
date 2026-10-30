use super::*;

fn state_with_every_field_set() -> AppState {
    let mut state = AppState::default();
    state.set_volume(0.8);
    state.current_track = Some("song-1".to_string());
    state.set_balance(-0.35);
    state.repeat = true;
    state.shuffle = true;
    state.manual_advance = true;
    state.eq_enabled = true;
    state.eq_auto = true;
    state.set_eq_preamp(3.5);
    state.set_eq_band(0, -4.0);
    state.set_eq_band(9, 7.5);
    state
}

#[test]
fn to_text_round_trips_every_field() {
    let mut state = state_with_every_field_set();
    let rock = *equalizer::PRESETS
        .iter()
        .find(|preset| preset.name == "Rock")
        .unwrap();
    state.apply_eq_preset(rock);

    let mut settings = Settings::from_state(&state);
    settings.set_time_display_remaining(true);
    settings.set_visualizer_visible(false);
    settings.set_autoscroll(false);
    settings.set_eq_panel_visible(false);
    settings.set_library_panel_visible(false);
    settings.set_always_on_top(true);
    settings.set_shaded(true);
    settings.set_window_position(Some((120.0, -40.5)));
    settings.set_music_kit_user_token(Some("a.b.c".to_string()));
    settings.set_recently_played(vec!["song-3".to_string(), "song-1".to_string()]);
    assert_eq!(Settings::from_text(&settings.to_text()), settings);
    assert!(Settings::from_text(&settings.to_text()).time_display_remaining());
    assert!(!Settings::from_text(&settings.to_text()).visualizer_visible());
    assert!(!Settings::from_text(&settings.to_text()).autoscroll());
    assert!(!Settings::from_text(&settings.to_text()).eq_panel_visible());
    assert!(!Settings::from_text(&settings.to_text()).library_panel_visible());
    assert!(Settings::from_text(&settings.to_text()).always_on_top());
    assert!(Settings::from_text(&settings.to_text()).shaded());

    let custom = Settings {
        eq_preset: None,
        ..settings
    };
    assert_eq!(Settings::from_text(&custom.to_text()), custom);
}

#[test]
fn time_display_round_trips_both_words_and_ignores_unknown_values() {
    let elapsed = Settings::default();
    assert!(!elapsed.time_display_remaining());
    assert!(!Settings::from_text(&elapsed.to_text()).time_display_remaining());

    let mut remaining = elapsed.clone();
    remaining.set_time_display_remaining(true);
    assert!(remaining.to_text().contains("time_display=remaining"));
    assert!(
        Settings::from_text(&remaining.to_text()).time_display_remaining(),
        "remaining must survive the round trip"
    );

    assert!(
        !Settings::from_text("time_display=sideways\n").time_display_remaining(),
        "an unknown word must fall back to elapsed"
    );
}

#[test]
fn visualizer_visibility_round_trips_and_ignores_malformed_values() {
    let shown = Settings::default();
    assert!(shown.visualizer_visible());
    assert!(Settings::from_text(&shown.to_text()).visualizer_visible());

    let mut hidden = shown.clone();
    hidden.set_visualizer_visible(false);
    assert!(hidden.to_text().contains("visualizer_visible=false"));
    assert!(
        !Settings::from_text(&hidden.to_text()).visualizer_visible(),
        "a hidden visualizer must survive the round trip"
    );

    assert!(
        Settings::from_text("visualizer_visible=true\n").visualizer_visible(),
        "an explicit true must stay shown"
    );
    assert!(
        Settings::from_text("visualizer_visible=maybe\n").visualizer_visible(),
        "an unknown value must fall back to shown"
    );
}

#[test]
fn autoscroll_round_trips_and_ignores_malformed_values() {
    let scrolling = Settings::default();
    assert!(scrolling.autoscroll());
    assert!(Settings::from_text(&scrolling.to_text()).autoscroll());

    let mut stopped = scrolling.clone();
    stopped.set_autoscroll(false);
    assert!(stopped.to_text().contains("autoscroll=false"));
    assert!(
        !Settings::from_text(&stopped.to_text()).autoscroll(),
        "a stopped autoscroll must survive the round trip"
    );

    assert!(
        Settings::from_text("autoscroll=true\n").autoscroll(),
        "an explicit true must stay scrolling"
    );
    assert!(
        Settings::from_text("autoscroll=sideways\n").autoscroll(),
        "an unknown value must fall back to scrolling"
    );
}

#[test]
fn panel_visibility_round_trips_and_ignores_malformed_values() {
    let shown = Settings::default();
    assert!(shown.eq_panel_visible());
    assert!(shown.library_panel_visible());

    let mut eq_hidden = shown.clone();
    eq_hidden.set_eq_panel_visible(false);
    let back = Settings::from_text(&eq_hidden.to_text());
    assert!(!back.eq_panel_visible());
    assert!(back.library_panel_visible());

    let mut library_hidden = shown.clone();
    library_hidden.set_library_panel_visible(false);
    let back = Settings::from_text(&library_hidden.to_text());
    assert!(back.eq_panel_visible());
    assert!(!back.library_panel_visible());

    let unknown = Settings::from_text("eq_panel_visible=maybe\nlibrary_panel_visible=maybe\n");
    assert!(
        unknown.eq_panel_visible(),
        "an unknown value must fall back to shown"
    );
    assert!(unknown.library_panel_visible());
}

#[test]
fn always_on_top_round_trips_and_ignores_malformed_values() {
    let unpinned = Settings::default();
    assert!(!unpinned.always_on_top());
    assert!(!Settings::from_text(&unpinned.to_text()).always_on_top());

    let mut pinned = unpinned.clone();
    pinned.set_always_on_top(true);
    assert!(pinned.to_text().contains("always_on_top=true"));
    assert!(
        Settings::from_text(&pinned.to_text()).always_on_top(),
        "a pinned window must survive the round trip"
    );

    assert!(
        !Settings::from_text("always_on_top=false\n").always_on_top(),
        "an explicit false must stay unpinned"
    );
    assert!(
        !Settings::from_text("always_on_top=yes\n").always_on_top(),
        "an unknown word must fall back to unpinned"
    );
}

#[test]
fn from_text_ignores_a_leading_utf8_bom() {
    let settings = Settings::from_text("\u{feff}volume=0.25\nbalance=0.25\n");

    assert_eq!(
        settings.volume, 0.25,
        "a leading BOM is an encoding marker, not part of the first key"
    );
    assert_eq!(settings.balance, 0.25);
}

#[test]
fn shade_round_trips_and_ignores_malformed_values() {
    let unshaded = Settings::default();
    assert!(!unshaded.shaded());
    assert!(!Settings::from_text(&unshaded.to_text()).shaded());
    assert!(
        !Settings::from_text("volume=0.5\n").shaded(),
        "a missing shade line must fall back to unshaded"
    );

    let mut shaded = unshaded.clone();
    shaded.set_shaded(true);
    assert!(shaded.to_text().contains("shade=true"));
    assert!(
        Settings::from_text(&shaded.to_text()).shaded(),
        "a shaded window must survive the round trip"
    );

    assert!(
        !Settings::from_text("shade=false\n").shaded(),
        "an explicit false must stay unshaded"
    );
    assert!(
        !Settings::from_text("shade=\n").shaded(),
        "an empty value must fall back to unshaded"
    );
    assert!(
        !Settings::from_text("shade=yes\n").shaded(),
        "an unknown word must fall back to unshaded"
    );
}

#[test]
fn manual_advance_round_trips_and_ignores_malformed_values() {
    let off = Settings::default();
    assert!(!off.manual_advance);
    assert!(
        !Settings::from_text("volume=0.5\n").manual_advance,
        "a missing manual_advance line must fall back to off"
    );
    assert!(!Settings::from_text(&off.to_text()).manual_advance);

    let mut on = off.clone();
    on.manual_advance = true;
    assert!(on.to_text().contains("manual_advance=true"));
    assert!(
        Settings::from_text(&on.to_text()).manual_advance,
        "an enabled manual advance must survive the round trip"
    );

    assert!(
        !Settings::from_text("manual_advance=false\n").manual_advance,
        "an explicit false must stay off"
    );
    assert!(
        !Settings::from_text("manual_advance=\n").manual_advance,
        "an empty value must fall back to off"
    );
    assert!(
        !Settings::from_text("manual_advance=yes\n").manual_advance,
        "an unknown word must fall back to off"
    );
}

#[test]
fn eq_auto_round_trips_and_ignores_malformed_values() {
    let off = Settings::default();
    assert!(!off.eq_auto);
    assert!(!Settings::from_text(&off.to_text()).eq_auto);
    assert!(
        !Settings::from_text("volume=0.5\n").eq_auto,
        "a missing eq_auto line must fall back to off"
    );

    let mut on = off.clone();
    on.eq_auto = true;
    assert!(on.to_text().contains("eq_auto=true"));
    assert!(
        Settings::from_text(&on.to_text()).eq_auto,
        "an enabled AUTO must survive the round trip"
    );

    assert!(
        !Settings::from_text("eq_auto=false\n").eq_auto,
        "an explicit false must stay off"
    );
    assert!(
        !Settings::from_text("eq_auto=\n").eq_auto,
        "an empty value must fall back to off"
    );
    assert!(
        !Settings::from_text("eq_auto=yes\n").eq_auto,
        "an unknown word must fall back to off"
    );
}

#[test]
fn from_text_ignores_nan_numbers() {
    // `f32::from_str` accepts `NaN`, but it is not a value the player can
    // use: an ignored line must fall back to its default, not to whichever
    // neutral value a later clamp happens to choose (volume's is silence).
    let text = "volume=NaN\n\
                balance=NaN\n\
                eq_preamp=NaN\n\
                eq_band_0=NaN\n";
    assert_eq!(Settings::from_text(text), Settings::default());

    let recovered = Settings::from_text("volume=NaN\nvolume=0.25\n");
    assert_eq!(recovered.volume, 0.25, "a later valid line still applies");
}

#[test]
fn from_text_ignores_malformed_and_unknown_lines() {
    let text = "volume=not-a-number\n\
                balance=abc\n\
                repeat=yes\n\
                shuffle=1\n\
                manual_advance=maybe\n\
                eq_enabled=maybe\n\
                eq_auto=maybe\n\
                eq_auto=maybe\n\
                eq_preamp=high\n\
                eq_band_10=3\n\
                eq_band_x=2\n\
                eq_band_0=NaN\n\
                eq_preset=Unknown\n\
                time_display=sideways\n\
                visualizer_visible=maybe\n\
                autoscroll=maybe\n\
                eq_panel_visible=maybe\n\
                library_panel_visible=maybe\n\
                always_on_top=yes\n\
                shade=yes\n\
                mystery=1\n";
    assert_eq!(Settings::from_text(text), Settings::default());
}

#[test]
fn a_later_eq_preset_line_clears_or_keeps_an_earlier_one() {
    let rock = *equalizer::PRESETS
        .iter()
        .find(|preset| preset.name == "Rock")
        .unwrap();

    let cleared = Settings::from_text("eq_preset=Rock\neq_preset=\n");
    assert_eq!(cleared.eq_preset, None);

    let kept = Settings::from_text("eq_preset=Rock\neq_preset=Unknown\n");
    assert_eq!(kept.eq_preset, Some(rock));
}

// The saved custom curves are a repeated `eq_custom_preset=` line in the
// same family as `eq_preamp`/`eq_band_*`: every valid line loads in file
// order, `from_curve` clamps out-of-range gains, and a line without a
// `:`, a blank name, the wrong field count, or a malformed-or-NaN gain
// is skipped without failing the load. The whole list must round-trip
// through `to_text` → `from_text` untouched.
// The saved song lists are a repeated `user_playlist=` line: every
// valid line loads in file order, and a line without a `:`, a blank
// name, or an empty id list is skipped without failing the load. The
// whole list must round-trip through `to_text` → `from_text` untouched.
#[test]
fn user_playlist_lines_round_trip_and_skip_the_malformed_ones() {
    let text = "user_playlist=Morning Drive:song-1, song-2 ,song-3\n\
                user_playlist:no\n\
                user_playlist=:song-1\n\
                user_playlist=Empty list:\n\
                ignore-me=x\n";
    let settings = Settings::from_text(text);
    assert_eq!(settings.saved_playlists().len(), 1);
    let saved = &settings.saved_playlists()[0];
    assert_eq!(saved.name, "Morning Drive");
    assert_eq!(saved.song_ids, vec!["song-1", "song-2", "song-3"]);

    assert_eq!(Settings::from_text(&settings.to_text()), settings);
    assert!(
        settings
            .to_text()
            .contains("user_playlist=Morning Drive:song-1,song-2,song-3")
    );
    // A default snapshot writes no saved-list lines at all.
    assert!(!Settings::default().to_text().contains("user_playlist"));
}

// The signed-out Recently Played list is one `recently_played=` line of
// ids only, no `:`-split name: trimmed tokens, empty ones dropped, order
// kept verbatim (newest first), stored only when at least one id
// survives. It is written only when non-empty, sits after the
// `user_playlist=` lines in the file, and must round-trip untouched —
// byte-for-byte id lists, order included.
#[test]
fn recently_played_line_round_trips_and_skips_the_malformed_values() {
    let text = "user_playlist=Morning Drive:song-1\n\
                recently_played=song-2, song-1 ,\n\
                recently_played:no-equals-line\n\
                recently_played=\n";
    let settings = Settings::from_text(text);
    // The empty tokens disappear; the order the file carried stays; only
    // the line with surviving ids is stored, and the blank line left the
    // stored value alone.
    assert_eq!(
        settings.recently_played(),
        &["song-2".to_string(), "song-1".to_string()]
    );

    assert_eq!(Settings::from_text(&settings.to_text()), settings);
    let written = settings.to_text();
    assert!(written.contains("recently_played=song-2,song-1\n"));
    let playlist_line = written
        .find("user_playlist=")
        .expect("the same snapshot carries the saved playlists");
    let played_line = written.find("recently_played=").unwrap();
    assert!(playlist_line < played_line);
    // A default snapshot writes no played-ids line at all.
    assert!(!Settings::default().to_text().contains("recently_played"));
}

#[test]
fn eq_custom_preset_lines_round_trip_and_skip_the_malformed_ones() {
    let text = "eq_custom_preset=Very Fuzzed:2.5,1,2,3,4,5,6,7,8,9,10\n\
                eq_custom_preset:missing=separator\n\
                eq_custom_preset=:1,2,3,4,5,6,7,8,9,10,11\n\
                eq_custom_preset=Short:1,2,3\n\
                eq_custom_preset=NaN gain:0,1,2,3,4,5,6,7,8,9,NaN\n\
                eq_custom_preset=Clamped:99,-99,0,0,0,0,0,0,0,0,0\n";
    let settings = Settings::from_text(text);
    assert_eq!(settings.eq_custom_presets().len(), 2);
    let fuzzed = &settings.eq_custom_presets()[0];
    assert_eq!(fuzzed.name, "Very Fuzzed");
    assert_eq!(fuzzed.preamp, 2.5);
    assert_eq!(
        fuzzed.bands,
        [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0]
    );
    let clamped = &settings.eq_custom_presets()[1];
    assert_eq!(clamped.name, "Clamped");
    assert_eq!(clamped.preamp, equalizer::GAIN_MAX_DB);
    assert_eq!(clamped.bands[0], equalizer::GAIN_MIN_DB);

    assert_eq!(Settings::from_text(&settings.to_text()), settings);
    assert!(
        settings
            .to_text()
            .contains("eq_custom_preset=Very Fuzzed:2.5")
    );
    // A default snapshot writes no custom lines at all.
    assert!(!Settings::default().to_text().contains("eq_custom_preset"));
}

// A saved custom's name is free text, but the file is line-based: a
// pasted line break in the name would split its `eq_custom_preset=`
// row in two and let the name's tail be read as settings lines of its
// own (a name like `Fuzz\neq_auto=on` would even flip a visible
// setting). The writer therefore flattens line breaks to spaces, so
// every saved curve keeps one whole line and round-trips under a
// line-break-free name.
#[test]
fn a_line_broken_preset_name_writes_one_whole_settings_line() {
    let mut bands = [0.0; equalizer::BAND_COUNT];
    bands[0] = 2.0;
    let mut settings = Settings::default();
    settings.set_eq_custom_presets(vec![
        equalizer::CustomPreset::from_curve(String::from("Fuzz\nBass"), 2.5, bands),
        equalizer::CustomPreset::from_curve(String::from("Fuzz\rBass"), -1.0, bands),
    ]);
    let mut scrubbed = Settings::default();
    scrubbed.set_eq_custom_presets(vec![
        equalizer::CustomPreset::from_curve(String::from("Fuzz Bass"), 2.5, bands),
        equalizer::CustomPreset::from_curve(String::from("Fuzz Bass"), -1.0, bands),
    ]);

    assert_eq!(settings.to_text(), scrubbed.to_text());
    assert_eq!(Settings::from_text(&settings.to_text()), scrubbed);
}

// The same line-based file shape holds for a saved list's name: a
// pasted line break in a `user_playlist=` name would split its row in
// two and let the name's tail be read as settings lines of its own (a
// name like `My List\neq_auto=on` would even flip a visible setting),
// so the writer flattens line breaks to spaces there too, and the
// naming panel refuses such a name outright.
#[test]
fn a_line_broken_playlist_name_writes_one_whole_settings_line() {
    let mut settings = Settings::default();
    settings.set_saved_playlists(vec![
        SavedPlaylist {
            name: String::from("My List\neq_auto=on"),
            song_ids: vec![String::from("song-1")],
        },
        SavedPlaylist {
            name: String::from("My\rList"),
            song_ids: vec![String::from("song-2")],
        },
    ]);
    let mut scrubbed = Settings::default();
    scrubbed.set_saved_playlists(vec![
        SavedPlaylist {
            name: String::from("My List eq_auto=on"),
            song_ids: vec![String::from("song-1")],
        },
        SavedPlaylist {
            name: String::from("My List"),
            song_ids: vec![String::from("song-2")],
        },
    ]);

    // Two whole `user_playlist=` rows and no injected `eq_auto=on` line.
    let text = settings.to_text();
    let rows: Vec<&str> = text
        .lines()
        .filter(|line| line.starts_with("user_playlist="))
        .collect();
    assert_eq!(rows.len(), 2);
    assert!(!text.lines().any(|line| line.starts_with("eq_auto=on")));
    assert!(!Settings::from_text(&text).eq_auto);
    assert_eq!(text, scrubbed.to_text());
    assert_eq!(Settings::from_text(&text), scrubbed);
}

// The two id rows the writer carries — `recently_played=` and
// `current_track=` — hold the played song's id verbatim, and the file is
// line-based: an id carrying a line break (a seam answers with one, or a
// future producer of ids lets one through) would split its row in two and
// let the id's tail be read as settings lines of its own (`song-7\neq_auto=on`
// would even flip a visible setting). The writer therefore flattens
// line breaks to spaces there too, like the `eq_custom_preset=` and
// `user_playlist=` names above — the play seams have no naming panel to
// refuse at.
#[test]
fn a_line_broken_played_id_writes_one_whole_settings_line() {
    let settings = Settings {
        recently_played: vec![
            String::from("song-7\neq_auto=on"),
            String::from("song-8\rshuffle=on"),
        ],
        current_track: Some(String::from("song-9\nvolume=1")),
        ..Settings::default()
    };
    let scrubbed = Settings {
        recently_played: vec![
            String::from("song-7 eq_auto=on"),
            String::from("song-8 shuffle=on"),
        ],
        current_track: Some(String::from("song-9 volume=1")),
        ..Settings::default()
    };

    // Two whole `recently_played=` tokens and one whole `current_track=`
    // row; no injected settings line of any tail's own.
    let text = settings.to_text();
    assert!(!text.lines().any(|line| line.starts_with("eq_auto=on")));
    assert!(!text.lines().any(|line| line.starts_with("shuffle=on")));
    assert!(!text.lines().any(|line| line.starts_with("volume=1")));
    assert_eq!(text, scrubbed.to_text());
    assert_eq!(Settings::from_text(&text), scrubbed);
}

// The same line-based file shape holds for a saved list's ids at the
// writer: the save flow copies them verbatim from the showing list
// (`save_list.rs`), and an id is seam text like a played id, so one
// carrying a line break would split the list's `user_playlist=` row in
// two and let the id's tail be read as settings lines of its own (an id
// like `song-7\neq_auto=on` would even flip a visible setting), so the
// writer flattens line breaks to spaces there too, like the
// `recently_played=` and `current_track=` id rows do.
#[test]
fn a_line_broken_playlist_song_id_writes_one_whole_settings_line() {
    let mut settings = Settings::default();
    settings.set_saved_playlists(vec![
        SavedPlaylist {
            name: String::from("My List"),
            song_ids: vec![String::from("song-7\neq_auto=on")],
        },
        SavedPlaylist {
            name: String::from("Other"),
            song_ids: vec![String::from("song-8\rshuffle=on")],
        },
    ]);
    let mut scrubbed = Settings::default();
    scrubbed.set_saved_playlists(vec![
        SavedPlaylist {
            name: String::from("My List"),
            song_ids: vec![String::from("song-7 eq_auto=on")],
        },
        SavedPlaylist {
            name: String::from("Other"),
            song_ids: vec![String::from("song-8 shuffle=on")],
        },
    ]);

    // Two whole `user_playlist=` rows and no injected settings line of
    // any tail's own.
    let text = settings.to_text();
    assert_eq!(
        text.lines()
            .filter(|line| line.starts_with("user_playlist="))
            .count(),
        2
    );
    assert!(!text.lines().any(|line| line.starts_with("eq_auto=on")));
    assert!(!text.lines().any(|line| line.starts_with("shuffle=on")));
    assert_eq!(text, scrubbed.to_text());
    assert_eq!(Settings::from_text(&text), scrubbed);
}

#[test]
fn from_text_reads_the_last_band_and_trims_around_the_equals_sign() {
    let last = equalizer::BAND_COUNT - 1;
    let settings = Settings::from_text(&format!("  eq_band_{last} = 4.5 \n"));
    assert_eq!(settings.eq_bands[last], 4.5);
    assert_eq!(settings.eq_bands[0], 0.0);
}

#[test]
fn apply_to_clamps_out_of_range_values_through_the_state_setters() {
    let settings = Settings {
        volume: 2.0,
        balance: -2.0,
        eq_preamp: 99.0,
        eq_bands: [99.0; equalizer::BAND_COUNT],
        ..Settings::default()
    };
    let mut state = AppState::default();
    settings.apply_to(&mut state);

    assert_eq!(state.volume(), 1.0);
    assert_eq!(state.balance(), -1.0);
    assert_eq!(state.eq_preamp(), equalizer::GAIN_MAX_DB);
    assert!(
        state
            .eq_bands()
            .iter()
            .all(|gain| *gain == equalizer::GAIN_MAX_DB)
    );
}

#[test]
fn infinite_numbers_load_and_clamp_to_the_range_ends() {
    // Unlike `NaN`, an infinity is kept by the parser and clamped by the
    // state setters, so it lands on the nearest range end.
    let settings =
        Settings::from_text("volume=inf\nbalance=-inf\neq_preamp=infinity\neq_band_0=-inf\n");
    let mut state = AppState::default();
    settings.apply_to(&mut state);

    assert_eq!(state.volume(), 1.0);
    assert_eq!(state.balance(), -1.0);
    assert_eq!(state.eq_preamp(), equalizer::GAIN_MAX_DB);
    assert_eq!(state.eq_bands()[0], equalizer::GAIN_MIN_DB);
}

#[test]
fn apply_to_restores_a_saved_preset() {
    let rock = *equalizer::PRESETS
        .iter()
        .find(|preset| preset.name == "Rock")
        .unwrap();
    let settings = Settings {
        repeat: true,
        eq_enabled: true,
        eq_preset: Some(rock),
        ..Settings::default()
    };
    let mut state = AppState::default();
    settings.apply_to(&mut state);

    assert!(state.repeat);
    assert!(state.eq_enabled);
    assert_eq!(state.eq_preset(), Some(rock));
    assert_eq!(state.eq_preamp(), rock.preamp);
    assert_eq!(state.eq_bands(), rock.bands);
}

#[test]
fn library_view_round_trips_and_ignores_unknown_words() {
    assert_eq!(Settings::default().library_view(), LibraryView::Artists);
    let mut settings = Settings::default();
    settings.set_library_view(LibraryView::Playlists);
    assert!(settings.to_text().contains("library_view=playlists"));
    assert_eq!(Settings::from_text(&settings.to_text()), settings);
    assert_eq!(
        Settings::from_text("library_view=bogus\n").library_view(),
        LibraryView::Artists
    );
    assert_eq!(
        Settings::from_text("volume=0.5\n").library_view(),
        LibraryView::Artists
    );
}

#[test]
fn window_position_round_trips_and_ignores_malformed_values() {
    let mut settings = Settings::default();
    assert!(!settings.to_text().contains("window_position"));

    settings.set_window_position(Some((120.0, -40.5)));
    assert!(settings.to_text().contains("window_position=120,-40.5"));
    assert_eq!(
        Settings::from_text(&settings.to_text()).window_position(),
        Some((120.0, -40.5))
    );

    for bad in ["abc", "10", "NaN,5", "inf,5", "5,-inf", "1,2,3"] {
        assert_eq!(
            Settings::from_text(&format!("window_position={bad}\n")).window_position(),
            None,
            "{bad:?} must leave the position unset"
        );
    }
}

#[test]
fn current_track_round_trips_and_defaults_to_none() {
    let none = Settings::default();
    assert_eq!(none.current_track, None);
    assert!(
        none.to_text().lines().any(|line| line == "current_track="),
        "a missing track writes an empty current_track line"
    );
    assert_eq!(
        Settings::from_text(&none.to_text()).current_track,
        None,
        "an empty value must fall back to no track"
    );
    assert_eq!(
        Settings::from_text("volume=0.5\n").current_track,
        None,
        "a missing current_track line must fall back to no track"
    );

    let mut saved = none.clone();
    saved.current_track = Some("song-42".to_string());
    assert!(saved.to_text().contains("current_track=song-42"));
    assert_eq!(
        Settings::from_text(&saved.to_text()).current_track,
        Some("song-42".to_string()),
        "a saved id must survive the round trip"
    );
    assert_eq!(
        Settings::from_text("current_track=a=b\n").current_track,
        Some("a=b".to_string()),
        "an id containing '=' must keep everything past the first '='"
    );
}

#[test]
fn music_kit_user_token_round_trips_and_defaults_to_none() {
    let none = Settings::default();
    assert_eq!(none.music_kit_user_token(), None);
    assert!(
        !none.to_text().contains("music_kit_user_token"),
        "an absent token writes no token line"
    );
    assert_eq!(
        Settings::from_text(&none.to_text()).music_kit_user_token(),
        None,
        "a file with no token line must fall back to none"
    );
    assert_eq!(
        Settings::from_text("volume=0.5\n").music_kit_user_token(),
        None,
        "a missing token line must fall back to none"
    );
    // An empty value means the line was saved blank (a hand-edited
    // deletion, say): that is no token.
    assert_eq!(
        Settings::from_text("music_kit_user_token=\n").music_kit_user_token(),
        None
    );

    let mut saved = none.clone();
    saved.set_music_kit_user_token(Some("a.b.c".to_string()));
    assert!(saved.to_text().contains("music_kit_user_token=a.b.c"));
    assert_eq!(
        Settings::from_text(&saved.to_text()).music_kit_user_token(),
        Some("a.b.c")
    );
    // Formatting is not the codec's job (`seed_session` validates at
    // boot), so unJWT-shaped text is stored verbatim.
    assert_eq!(
        Settings::from_text("music_kit_user_token=leftover\n").music_kit_user_token(),
        Some("leftover")
    );
}

#[test]
fn apply_to_restores_the_saved_current_track_stopped() {
    let saved = Settings {
        current_track: Some("song-1".to_string()),
        ..Settings::default()
    };
    let mut state = AppState::default();
    saved.apply_to(&mut state);
    assert_eq!(state.current_track.as_deref(), Some("song-1"));
    assert!(!state.is_playing, "the restored track sits stopped");
    assert!(!state.paused, "the restored track is not paused");

    let mut second = AppState::default();
    second.current_track = Some("song-1".to_string());
    Settings::default().apply_to(&mut second);
    assert_eq!(
        second.current_track, None,
        "a boot with no saved track loads none"
    );
}
