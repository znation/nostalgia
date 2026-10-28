use super::*;

fn titles(player: &WinampPlayer) -> Vec<String> {
    player
        .recently_added
        .items
        .iter()
        .map(|album| album.title.clone())
        .collect()
}

// The Recently Added button fetches the newest-first album list and opens its
// first row's songs with the Recently Added origin; Back steps to the list and
// then to Artists.
#[tokio::test]
async fn show_recently_added_lists_newest_first_and_back_steps_up() {
    let (mut player, _state) = test_player();

    let task = update(&mut player, Message::ShowRecentlyAdded);
    let message = task_output(task, "load recently added albums").await;
    let _ = update(&mut player, message);
    assert_view(&player, CurrentView::RecentlyAdded);

    let expected: Vec<String> = player
        .apple_music_service
        .get_all_albums()
        .await
        .expect("sample albums")
        .into_iter()
        .rev()
        .map(|album| album.title)
        .collect();
    assert_eq!(titles(&player), expected);

    let epoch = player.recently_added.epoch;
    let task = update(&mut player, Message::SelectRow { epoch, index: 0 });
    assert!(player.recently_added.selected() == Some(0));
    drop(task);
    let task = update(&mut player, Message::AlbumSelected { epoch, index: 0 });
    let message = task_output(task, "load songs from the first recently added album").await;
    let _ = update(&mut player, message);
    assert_view(&player, CurrentView::Songs);
    assert_eq!(player.songs_origin, SongsOrigin::RecentlyAdded);
    assert!(!player.songs.items.is_empty());

    let _ = update(&mut player, Message::Back);
    assert_view(&player, CurrentView::RecentlyAdded);
    let _ = update(&mut player, Message::Back);
    assert_view(&player, CurrentView::Artists);
}

#[test]
fn recently_added_load_failed_shows_its_report() {
    let (mut player, _state) = test_player();
    let _ = update(
        &mut player,
        Message::RecentlyAddedLoadFailed("boom".to_string()),
    );
    assert_eq!(player.recently_added.error.as_deref(), Some("boom"));
}

#[test]
fn recently_added_reverse_sort_and_back_restore_newest_first() {
    let (mut player, _state) = test_player();
    let albums = vec![
        Album {
            id: "zulu".to_string(),
            title: "Zulu".to_string(),
            ..sample_album()
        },
        Album {
            id: "alpha".to_string(),
            title: "alpha".to_string(),
            ..sample_album()
        },
    ];
    let _ = update(&mut player, Message::RecentlyAddedLoaded(albums));
    player.current_view = CurrentView::RecentlyAdded;

    let _ = update(&mut player, Message::ReverseBrowse);
    assert_eq!(titles(&player), ["alpha", "Zulu"]);
    let _ = update(&mut player, Message::ReverseBrowse);
    assert_eq!(titles(&player), ["Zulu", "alpha"]);
    let _ = update(&mut player, Message::SortBrowse);
    assert_eq!(titles(&player), ["alpha", "Zulu"]);

    let epoch = player.recently_added.epoch;
    let _ = update(&mut player, Message::AlbumSelected { epoch, index: 0 });
    let _ = update(&mut player, Message::Back);
    assert_view(&player, CurrentView::RecentlyAdded);
    assert_eq!(titles(&player), ["Zulu", "alpha"]);
}

fn three_albums() -> Vec<Album> {
    ["one", "two", "three"]
        .into_iter()
        .map(|id| Album {
            id: id.to_string(),
            title: id.to_string(),
            ..sample_album()
        })
        .collect()
}

// Open (Enter) on a selected Recently Added row opens that row's album
// against the list's epoch.
#[test]
fn open_selected_in_recently_added_opens_the_selected_album() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::RecentlyAddedLoaded(three_albums()));
    player.current_view = CurrentView::RecentlyAdded;
    let epoch = player.recently_added.epoch;
    assert_message_schedules_no_work(&mut player, Message::SelectRow { epoch, index: 2 });

    let task = update(&mut player, Message::OpenSelected);
    let opened = futures::executor::block_on(task_output(task, "open the selected row"));

    assert!(
        matches!(&opened, Message::AlbumSelected { epoch: e, index: 2 } if *e == epoch),
        "Open must dispatch the selected album, got {opened:?}"
    );
}

// Alt+Up moves the selected row within Recently Added only and keeps it
// selected.
#[test]
fn move_selected_row_reorders_only_recently_added() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::RecentlyAddedLoaded(three_albums()));
    player.current_view = CurrentView::RecentlyAdded;
    let epoch = player.recently_added.epoch;
    let _ = update(&mut player, Message::SelectRow { epoch, index: 2 });

    let _ = update(&mut player, Message::MoveSelectedRowUp);

    assert_eq!(titles(&player), ["one", "three", "two"]);
    assert_eq!(player.recently_added.selected(), Some(1));
    assert!(player.albums.items.is_empty());
}
