use super::*;

const ROW: f32 = views::BROWSE_ROW_HEIGHT;
const PAD: f32 = views::BROWSE_LIST_PADDING;

fn artist(name: &str) -> Artist {
    Artist {
        id: name.to_string(),
        name: name.to_string(),
        album_count: 0,
    }
}

fn select(player: &mut WinampPlayer, epoch: u64, index: usize) {
    let _ = update(player, Message::SelectRow { epoch, index });
}

fn twenty_artists_with_row_ten_selected() -> WinampPlayer {
    let (mut player, _state) = test_player();
    let names: Vec<Artist> = (0..20).map(|i| artist(&format!("a{i:02}"))).collect();
    let _ = update(&mut player, Message::ArtistsLoaded(names));
    let epoch = player.artists.epoch;
    select(&mut player, epoch, 10);
    player
}

#[test]
fn the_chords_step_the_selection_to_the_neighbour_row() {
    let (mut player, _state) = test_player();
    let _ = update(
        &mut player,
        Message::ArtistsLoaded(vec![artist("a"), artist("b"), artist("c")]),
    );
    let epoch = player.artists.epoch;
    select(&mut player, epoch, 0);
    let order: Vec<String> = player
        .artists
        .items
        .iter()
        .map(|a| a.name.clone())
        .collect();

    let _ = update(&mut player, Message::MoveBrowseSelectionDown);
    assert_eq!(player.artists.selected(), Some(1));
    let after: Vec<String> = player
        .artists
        .items
        .iter()
        .map(|a| a.name.clone())
        .collect();
    assert_eq!(after, order, "the selection step reorders nothing");

    let _ = update(&mut player, Message::MoveBrowseSelectionUp);
    assert_eq!(player.artists.selected(), Some(0));
}

// Unlike the Alt row move a selection step names the showing list itself, so
// it bumps no epoch: an index-carrying press rendered before the step still
// resolves against the list.
#[test]
fn a_selection_step_bumps_no_epoch_so_rendered_presses_stay_valid() {
    let (mut player, _state) = test_player();
    let _ = update(
        &mut player,
        Message::ArtistsLoaded(vec![artist("a"), artist("b"), artist("c")]),
    );
    let epoch = player.artists.epoch;
    select(&mut player, epoch, 0);

    let _ = update(&mut player, Message::MoveBrowseSelectionDown);

    assert_eq!(player.artists.epoch, epoch);
    select(&mut player, epoch, 2);
    assert_eq!(player.artists.selected(), Some(2));
}

#[tokio::test]
async fn a_step_past_the_viewport_bottom_scrolls_the_row_into_view() {
    let mut player = twenty_artists_with_row_ten_selected();
    let task = update(&mut player, Message::MoveBrowseSelectionDown);

    let effects =
        browse_scroll_effects_at(task, "a step past the viewport bottom", 5.0 * ROW, 0.0).await;

    assert_eq!(
        effects.scrolled,
        Some(
            iced::advanced::widget::operation::scrollable::AbsoluteOffset {
                x: None,
                y: Some(PAD + 12.0 * ROW - 5.0 * ROW),
            }
        )
    );
}

#[tokio::test]
async fn a_step_within_the_viewport_does_not_scroll() {
    let mut player = twenty_artists_with_row_ten_selected();
    let task = update(&mut player, Message::MoveBrowseSelectionDown);

    let effects =
        browse_scroll_effects_at(task, "a step within the viewport", 100.0 * ROW, 0.0).await;

    assert!(!effects.any());
}

#[tokio::test]
async fn a_step_above_the_viewport_top_scrolls_up_to_the_row() {
    let mut player = twenty_artists_with_row_ten_selected();
    let task = update(&mut player, Message::MoveBrowseSelectionUp);

    let effects = browse_scroll_effects_at(
        task,
        "a step above the viewport top",
        5.0 * ROW,
        PAD + 10.0 * ROW,
    )
    .await;

    assert_eq!(
        effects.scrolled,
        Some(
            iced::advanced::widget::operation::scrollable::AbsoluteOffset {
                x: None,
                y: Some(PAD + 9.0 * ROW),
            }
        )
    );
}

#[test]
fn a_step_on_a_list_with_no_selection_is_a_no_op() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::ArtistsLoaded(vec![artist("a")]));
    assert_no_task(update(&mut player, Message::MoveBrowseSelectionDown));
    assert_eq!(player.artists.selected(), None);
}

#[test]
fn a_step_at_the_edge_is_a_no_op_that_keeps_the_selection() {
    let (mut player, _state) = test_player();
    let _ = update(
        &mut player,
        Message::ArtistsLoaded(vec![artist("a"), artist("b"), artist("c")]),
    );
    let epoch = player.artists.epoch;

    select(&mut player, epoch, 0);
    assert_no_task(update(&mut player, Message::MoveBrowseSelectionUp));
    assert_eq!(player.artists.selected(), Some(0));

    select(&mut player, epoch, 2);
    assert_no_task(update(&mut player, Message::MoveBrowseSelectionDown));
    assert_eq!(player.artists.selected(), Some(2));
}

// A list that is loading or failed has no rows the bar can step over, even
// with a selection carried over from a previous list.
#[test]
fn a_step_on_a_loading_or_failed_list_is_a_no_op() {
    for failed in [false, true] {
        let (mut player, _state) = test_player();
        let _ = update(
            &mut player,
            Message::ArtistsLoaded(vec![artist("a"), artist("b"), artist("c")]),
        );
        let epoch = player.artists.epoch;
        select(&mut player, epoch, 1);
        if failed {
            player.artists.error = Some("browse failed".to_string());
        } else {
            player.artists.loading = true;
        }
        assert_no_task(update(&mut player, Message::MoveBrowseSelectionDown));
        assert_eq!(player.artists.selected(), Some(1));
        assert_no_task(update(&mut player, Message::MoveBrowseSelectionUp));
        assert_eq!(player.artists.selected(), Some(1));
    }
}

// The routing matches over all eleven views; pin a second arm to a flat list
// so a refactor that drops one view falls here too.
#[test]
fn the_chords_step_the_all_songs_list_too() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::AllSongsLoaded(stepping_songs()));
    player.current_view = CurrentView::AllSongs;
    let order: Vec<String> = player
        .all_songs
        .items
        .iter()
        .map(|s| s.id.clone())
        .collect();

    let epoch = player.all_songs.epoch;
    select(&mut player, epoch, order.len().saturating_sub(1));
    let _ = update(&mut player, Message::MoveBrowseSelectionUp);

    assert_eq!(player.all_songs.selected(), Some(order.len() - 2));
}
