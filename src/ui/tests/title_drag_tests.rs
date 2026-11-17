use super::*;

#[test]
fn the_marquee_pauses_while_the_title_is_dragged_and_resumes_from_there() {
    let (mut player, state) = test_player();
    seed_current_title(&mut player, &state, "A Title Too Long For The LCD Bar");
    assert!(player.needs_marquee());
    let _ = update(&mut player, Message::TitleDragPressed);
    assert!(!player.needs_marquee());
    let _ = update(&mut player, Message::TitleDragMoved(100.0));
    assert_eq!(player.marquee_offset, 0, "the first move only anchors");
    let _ = update(&mut player, Message::TitleDragMoved(70.0));
    assert_eq!(player.marquee_offset, 3);
    let _ = update(&mut player, Message::TitleDragReleased);
    assert!(player.needs_marquee());
    assert_eq!(player.marquee_offset, 3);
    let _ = update(&mut player, Message::TitleDragMoved(10.0));
    assert_eq!(
        player.marquee_offset, 3,
        "a move without a press changes nothing"
    );
}

#[test]
fn dragging_the_title_right_wraps_backward_and_a_second_drag_starts_from_there() {
    let (mut player, state) = test_player();
    seed_current_title(&mut player, &state, "A Title Too Long For The LCD Bar");
    let cycle = {
        let current = state.blocking_lock().current_track.clone();
        let current = current.as_deref();
        views::marquee_cycle_len(
            &player.now_playing_label(current),
            &player.now_playing_artist(current),
            &player.now_playing_number(current),
        )
    };
    assert!(cycle > 3);

    let _ = update(&mut player, Message::TitleDragPressed);
    let _ = update(&mut player, Message::TitleDragMoved(100.0));
    let _ = update(&mut player, Message::TitleDragMoved(130.0));
    assert_eq!(
        player.marquee_offset,
        cycle - 3,
        "dragging right from the start wraps to the end of the cycle"
    );
    let _ = update(&mut player, Message::TitleDragReleased);

    let _ = update(&mut player, Message::TitleDragPressed);
    let _ = update(&mut player, Message::TitleDragMoved(50.0));
    assert_eq!(
        player.marquee_offset,
        cycle - 3,
        "a new press anchors afresh instead of jumping"
    );
    let _ = update(&mut player, Message::TitleDragMoved(20.0));
    assert_eq!(
        player.marquee_offset, 0,
        "dragging left three characters returns to the start of the cycle"
    );
}

#[test]
fn a_new_selection_clears_a_title_drag() {
    let (mut player, state) = test_player();
    seed_current_title(&mut player, &state, "A Title Too Long For The LCD Bar");
    let _ = update(&mut player, Message::TitleDragPressed);
    assert!(player.title_drag.is_some());
    let _ = update(
        &mut player,
        Message::SongsLoaded(crate::test_support::stepping_songs()),
    );
    let epoch = player.songs.epoch;
    let _ = update(&mut player, Message::TrackSelected { epoch, index: 0 });
    assert!(player.title_drag.is_none());
}
