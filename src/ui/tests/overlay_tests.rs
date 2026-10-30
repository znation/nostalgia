//! `apply_overlays`' selection through the root view: the options menu
//! drawing in, About taking precedence over it, and shading hiding both.
//!
//! The overlays themselves are pinned at the direct-call level in
//! `views::tests::chrome`; `file_info` ahead of `jump_time` is pinned by
//! `jump_time_tests`. What those misses leave is `apply_overlays` itself —
//! no other test reaches it — so these tests drive it through the root
//! view's capture, like [`super::jump_time_tests`] does.

use super::*;

fn player_showing_options() -> WinampPlayer {
    let (mut player, _state) = test_player();
    player.options_visible = true;
    player
}

#[test]
fn the_root_view_draws_the_options_menu_with_its_default_labels() {
    let player = player_showing_options();

    let capture = captured_root_view_text_bounds(&player);
    let texts: Vec<&str> = capture.texts().collect();
    for expected in ["OPTIONS", "Always on Top: Off", "Visualizer: Shown"] {
        assert!(texts.contains(&expected), "{expected:?} missing: {texts:?}");
    }
}

// The handler that opens About while the options menu is open clears its
// flag (window_tests pins that), so these flags are set directly to isolate
// `apply_overlays`' own order: whichever pair of flags ends up set together,
// both About and Options visible must still draw only the About panel.
#[test]
fn the_about_overlay_wins_over_the_options_menu() {
    let mut player = player_showing_options();
    player.about_visible = true;

    let capture = captured_root_view_text_bounds(&player);
    let texts: Vec<&str> = capture.texts().collect();
    assert!(texts.contains(&"ABOUT NOSTALGIA"), "{texts:?}");
    assert!(!texts.contains(&"OPTIONS"), "{texts:?}");
}

pub(super) fn player_showing_save_panels() -> WinampPlayer {
    let (mut player, _state) = test_player();
    player.save_list_open = true;
    player.save_preset_open = true;
    player
}

// The handler that opens one of the save panels closes the other (and every
// other panel), so the flags are set directly to isolate `apply_overlays`'
// own order: whichever pair ends up set together, the Save list panel must
// still win, as it sits ahead of the Save preset panel in `apply_overlays`.
#[test]
fn the_save_list_panel_wins_over_the_save_preset_panel() {
    let player = player_showing_save_panels();

    let capture = captured_root_view_text_bounds(&player);
    let texts: Vec<&str> = capture.texts().collect();
    assert!(texts.contains(&"SAVE LIST"), "{texts:?}");
    assert!(!texts.contains(&"SAVE PRESET"), "{texts:?}");
}

#[test]
fn the_root_view_draws_the_save_preset_panel_when_only_its_flag_is_set() {
    let (mut player, _state) = test_player();
    player.save_preset_open = true;

    let capture = captured_root_view_text_bounds(&player);
    let texts: Vec<&str> = capture.texts().collect();
    assert!(texts.contains(&"SAVE PRESET"), "{texts:?}");
    assert!(!texts.contains(&"SAVE LIST"), "{texts:?}");
}

#[test]
fn the_root_view_draws_no_save_panels_while_shaded() {
    let mut player = player_showing_save_panels();
    player.shaded = true;

    let capture = captured_root_view_text_bounds(&player);
    assert!(!capture.texts().any(|t| t == "SAVE LIST"));
    assert!(!capture.texts().any(|t| t == "SAVE PRESET"));
}

#[test]
fn the_root_view_draws_no_options_menu_while_shaded() {
    let mut player = player_showing_options();
    player.shaded = true;

    let capture = captured_root_view_text_bounds(&player);
    assert!(!capture.texts().any(|t| t == "OPTIONS"));
}
