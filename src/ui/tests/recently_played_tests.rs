use super::*;

fn titles(player: &WinampPlayer) -> Vec<String> {
    player
        .recently_played
        .items
        .iter()
        .map(|song| song.title.clone())
        .collect()
}

// The Recently Played button fetches the sample library's songs in reverse
// order; pressing the first row plays the library's last song, and Back
// returns to Artists.
#[tokio::test]
async fn show_recently_played_lists_reversed_sample_songs_and_plays_the_first() {
    let (mut player, _state) = test_player();

    let task = update(&mut player, Message::ShowRecentlyPlayed);
    let message = task_output(task, "load recently played songs").await;
    let _ = update(&mut player, message);
    assert_view(&player, CurrentView::RecentlyPlayed);

    let expected: Vec<String> = player
        .apple_music_service
        .get_all_songs()
        .await
        .expect("sample songs")
        .into_iter()
        .rev()
        .map(|song| song.title)
        .collect();
    assert_eq!(titles(&player), expected);

    let epoch = player.recently_played.epoch;
    let first_id = player.recently_played.items[0].id.clone();
    let task = update(&mut player, Message::TrackSelected { epoch, index: 0 });
    drive_task(task, "recently played play", |_| {}).await;
    assert_eq!(
        player.state.lock().await.current_track.as_deref(),
        Some(first_id.as_str())
    );

    let _ = update(&mut player, Message::Back);
    assert_view(&player, CurrentView::Artists);
}

#[test]
fn recently_played_load_failed_shows_its_report() {
    let (mut player, _state) = test_player();
    let _ = update(
        &mut player,
        Message::RecentlyPlayedLoadFailed("boom".to_string()),
    );
    assert_eq!(player.recently_played.error.as_deref(), Some("boom"));
}

#[test]
fn a_stale_recently_played_reply_is_ignored() {
    let (mut player, _state) = test_player();
    let stale = player.recently_played.begin_fetch().issued();
    let _newer = player.recently_played.begin_fetch();
    let reply = Box::new(Message::RecentlyPlayedLoaded(stepping_songs()));
    let _ = update(
        &mut player,
        Message::BrowseReply {
            issued: stale,
            reply,
        },
    );
    assert!(player.recently_played.items.is_empty());
}

#[test]
fn recently_played_reverse_and_sort_reorder_only_that_list() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::RecentlyPlayedLoaded(stepping_songs()));
    player.current_view = CurrentView::RecentlyPlayed;
    let before = titles(&player);

    let _ = update(&mut player, Message::ReverseBrowse);
    let mut reversed = before.clone();
    reversed.reverse();
    assert_eq!(titles(&player), reversed);

    let _ = update(&mut player, Message::SortBrowse);
    let mut sorted = before;
    sorted.sort_by_key(|title| title.to_lowercase());
    assert_eq!(titles(&player), sorted);
    assert!(player.songs.items.is_empty());
}

#[test]
fn next_track_in_recently_played_steps_that_list() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(second_album_songs()));
    let _ = update(&mut player, Message::RecentlyPlayedLoaded(stepping_songs()));
    player.current_view = CurrentView::RecentlyPlayed;
    player.state.blocking_lock().current_track = Some("song-1".to_string());

    let task = update(&mut player, Message::NextTrack);
    assert_track_selected(task, player.recently_played.epoch, 1);
}

#[test]
fn preview_end_in_recently_played_steps_that_list() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, _state) = player_playing_stepping_song(&recording, "song-1");
    player.songs.items = second_album_songs();
    let _ = update(&mut player, Message::RecentlyPlayedLoaded(stepping_songs()));
    player.current_view = CurrentView::RecentlyPlayed;
    let epoch = player.recently_played.epoch;

    let selected = drive_playback_finished(&mut player);
    assert!(
        matches!(selected, Message::TrackSelected { epoch: e, index: 1 } if e == epoch),
        "preview end must step the recently played list, got {selected:?}"
    );
}

#[test]
fn jump_to_file_submits_against_the_recently_played_list() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::RecentlyPlayedLoaded(stepping_songs()));
    player.current_view = CurrentView::RecentlyPlayed;
    let epoch = player.recently_played.epoch;
    let _ = update(&mut player, Message::ToggleJumpFile);

    let _ = update(&mut player, Message::JumpFileChanged("three".to_string()));
    let task = update(&mut player, Message::JumpFileSubmitted);

    assert_track_selected(task, epoch, 2);
}

// Open (Enter) on a selected Recently Played row dispatches that row's
// track selection against the list's epoch, like the other flat song list.
#[test]
fn open_selected_in_recently_played_plays_the_selected_row() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::RecentlyPlayedLoaded(stepping_songs()));
    player.current_view = CurrentView::RecentlyPlayed;
    let epoch = player.recently_played.epoch;
    assert_message_schedules_no_work(&mut player, Message::SelectRow { epoch, index: 1 });

    let task = update(&mut player, Message::OpenSelected);

    assert_track_selected(task, epoch, 1);
}

// Alt+Down moves the selected row within Recently Played only and keeps it
// selected.
#[test]
fn move_selected_row_reorders_only_recently_played() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::RecentlyPlayedLoaded(stepping_songs()));
    player.current_view = CurrentView::RecentlyPlayed;
    let before = titles(&player);
    let epoch = player.recently_played.epoch;
    let _ = update(&mut player, Message::SelectRow { epoch, index: 0 });

    let _ = update(&mut player, Message::MoveSelectedRowDown);

    let mut expected = before;
    expected.swap(0, 1);
    assert_eq!(titles(&player), expected);
    assert_eq!(player.recently_played.selected(), Some(1));
    assert!(player.songs.items.is_empty());
}

// Signed out (the test player's service stores no session), a committed play
// feeds the signed-out list, and reopening Recently Played shows the songs
// the run committed, newest first, instead of the sample stand-in. The stored
// list schedules no fetch: its rows are already there.
#[test]
fn signed_out_recently_played_lists_this_runs_played_songs_newest_first() {
    let (mut player, state) = test_player();
    player.current_view = CurrentView::RecentlyPlayed;
    let _ = update(&mut player, Message::RecentlyPlayedLoaded(stepping_songs()));
    record_a_play(&mut player, &state, "song-1");
    record_a_play(&mut player, &state, "song-3");

    assert_message_schedules_no_work(&mut player, Message::ShowRecentlyPlayed);

    assert_eq!(
        titles(&player),
        vec!["Three".to_string(), "One".to_string()]
    );
    assert_eq!(player.current_view, CurrentView::RecentlyPlayed);
    assert!(!player.recently_played.loading);
    assert_eq!(player.recently_played.error, None);
}

// Playing a song again keeps one row for it, moved to the newest position,
// as the play history's dedup does.
#[test]
fn replaying_a_song_moves_its_row_to_the_newest_played_position() {
    let (mut player, state) = test_player();
    player.current_view = CurrentView::RecentlyPlayed;
    let _ = update(&mut player, Message::RecentlyPlayedLoaded(stepping_songs()));
    record_a_play(&mut player, &state, "song-1");
    record_a_play(&mut player, &state, "song-2");
    record_a_play(&mut player, &state, "song-1");

    assert_message_schedules_no_work(&mut player, Message::ShowRecentlyPlayed);

    // One row per id, with the replay newest and the middle play oldest.
    assert_eq!(
        player
            .recently_played
            .items
            .iter()
            .map(|song| &song.id)
            .collect::<Vec<_>>(),
        vec![&"song-1".to_string(), &"song-2".to_string()]
    );
}

// A committed play whose track is absent from the showing list (say the
// resolved-but-listed-nowhere restored track) feeds the play history only and
// adds no played-list row.
#[test]
fn a_played_track_absent_from_the_showing_list_adds_no_played_row() {
    let (mut player, state) = test_player();
    player.current_view = CurrentView::RecentlyPlayed;
    let _ = update(&mut player, Message::RecentlyPlayedLoaded(stepping_songs()));
    record_a_play(&mut player, &state, "song-9");

    // Nothing has played, so the storing branch is skipped and the fetch
    // runs, as it always has.
    assert!(player.played_songs.is_empty());
    assert_message_schedules_work(&mut player, Message::ShowRecentlyPlayed);
    assert!(player.recently_played.loading);
}

// With a stored session, reopening Recently Played ignores the played rows
// and fetches the service's list as before: with a session stored, the
// service's REST client answers, so the stub reply's rows replace the
// played rows.
#[test]
fn signed_in_recently_played_still_fetches_the_service_list() {
    // The recently-played reply the stub transport serves: one catalog song
    // and one library song, the mapping the rest client's own tests pin. The
    // reply's rows are recognizably not the played row ("One").
    let reply = r#"{"data":[
        {"id":"1234","type":"songs","attributes":{"name":"Opening","artistName":"The Sample Band","albumName":"First Record","durationInMillis":1000}},
        {"id":"i.5","type":"library-songs","attributes":{"name":"Middle"}}
    ]}"#;
    let state = Arc::new(Mutex::new(AppState::default()));
    let service =
        AppleMusicService::with_transport(state.clone(), Box::new(StubTransport::returning(reply)));
    service
        .authenticate_with("dev-token", &|_| Ok(sign_in_session()))
        .expect("the stub sign-in must store its session");
    let mut player = WinampPlayer::new(state.clone(), service);
    player.current_view = CurrentView::RecentlyPlayed;
    let _ = update(&mut player, Message::RecentlyPlayedLoaded(stepping_songs()));
    record_a_play(&mut player, &state, "song-1");
    assert!(!player.played_songs.is_empty());

    // A committed `TrackPlayed` locks the shared state, so the fetch reply
    // runs on the executor instead of an async test's runtime.
    let task = update(&mut player, Message::ShowRecentlyPlayed);
    let message = futures::executor::block_on(task_output(task, "load recently played songs"));
    let _ = update(&mut player, message);

    assert_eq!(
        titles(&player),
        vec!["Opening".to_string(), "Middle".to_string()]
    );
}

// The boot's seeded restore: `ResolveRecentlyPlayed` with nothing seeded
// (no saved ids parsed) schedules no resolve, and with a stored session it
// schedules none either — the signed-in view fetches the service's own list
// and the seeded ids are dead.
#[test]
fn resolve_recently_played_is_a_no_op_with_a_session_or_nothing_restored() {
    let (mut player, _state) = test_player();

    assert_no_task(update(&mut player, Message::ResolveRecentlyPlayed));

    // Seeded but signed in: still nothing to resolve.
    player.recently_played_seeded = vec!["song-1".to_string()];
    player
        .apple_music_service
        .authenticate_with("dev-token", &|_| Ok(sign_in_session()))
        .expect("the stub sign-in must store its session");
    assert_no_task(update(&mut player, Message::ResolveRecentlyPlayed));
}

// The seeded ids resolve one by one, in the order the settings file saved
// them (newest first): the single task's reply carries each id's own song at
// its own position, so the restore walk can line the rows up with the saved
// order verbatim.
#[tokio::test]
async fn resolve_recently_played_resolves_each_seeded_id_in_order() {
    let (mut player, _state) = test_player();
    player.recently_played_seeded = vec!["song-3".to_string(), "song-1".to_string()];

    let message = task_output(
        update(&mut player, Message::ResolveRecentlyPlayed),
        "resolve recently played",
    )
    .await;
    let songs = match message {
        Message::RecentlyPlayedResolved { songs } => songs,
        other => panic!("unexpected resolve reply: {other:?}"),
    };

    assert_eq!(
        songs
            .iter()
            .map(|song| song.as_ref().map(|song| song.id.as_str()))
            .collect::<Vec<_>>(),
        vec![Some("song-3"), Some("song-1")]
    );
}

// The restore's answer funnels through `update`: the answered rows record
// into the signed-out list in save order (newest first), which the view's
// reopen shows and the next save writes back.
#[tokio::test]
async fn recently_played_resolved_stores_the_answered_rows_in_play_order() {
    let (mut player, _state) = test_player();
    player.recently_played_seeded = vec!["song-2".to_string(), "song-1".to_string()];

    let message = task_output(
        update(&mut player, Message::ResolveRecentlyPlayed),
        "resolve recently played",
    )
    .await;
    let songs = match message {
        Message::RecentlyPlayedResolved { songs } => songs,
        other => panic!("unexpected resolve reply: {other:?}"),
    };
    let _ = update(&mut player, Message::RecentlyPlayedResolved { songs });

    assert_eq!(
        player.played_songs.newest_first_ids(),
        vec!["song-2".to_string(), "song-1".to_string()]
    );
}

// An id the launch cannot answer contributes no row: the list is shorter
// than the seed, so the next save drops the stale id, exactly like a saved
// list's stale id disappears.
#[test]
fn recently_played_resolved_drops_unanswerable_ids() {
    let (mut player, _state) = test_player();
    let songs = stepping_songs();

    let _ = update(
        &mut player,
        Message::RecentlyPlayedResolved {
            songs: vec![Some(songs[1].clone()), None, None],
        },
    );

    assert_eq!(
        player.played_songs.newest_first_ids(),
        vec!["song-2".to_string()]
    );
}

// A sign-in racing the restore drops the restored rows: the reply landing
// after a session was stored stores nothing, because the signed-in view
// fetches the service's own list and the restored rows would be dead anyway.
#[test]
fn recently_played_resolved_skips_once_a_session_is_stored() {
    let (mut player, _state) = test_player();
    let songs = stepping_songs();
    player
        .apple_music_service
        .authenticate_with("dev-token", &|_| Ok(sign_in_session()))
        .expect("the stub sign-in must store its session");

    let _ = update(
        &mut player,
        Message::RecentlyPlayedResolved {
            songs: vec![Some(songs[0].clone())],
        },
    );

    assert!(player.played_songs.is_empty());
}

// `boot` hands iced its batch with the signed-out list's restore behind the
// last-played track's resolve: the same launch shape both depend on (the
// saved snapshot adopted in the init closure before the tasks run), so a
// regression that dropped `ResolveRecentlyPlayed` — or reordered it ahead
// of its sibling — fails this by name.
#[tokio::test]
async fn boot_resolves_the_saved_recently_played_after_the_last_played_track() {
    use futures::StreamExt;

    let state = Arc::new(Mutex::new(AppState::default()));
    let service =
        AppleMusicService::with_transport(state.clone(), Box::new(StubTransport::returning("")));

    let (_player, task) = boot(state, service);
    let mut stream = iced_runtime::task::into_stream(task).expect("boot must schedule work");
    let mut current_track = false;
    let mut recently_played = false;
    while !(current_track && recently_played) {
        let action = await_or_timeout("boot", TASK_TIMEOUT, stream.next())
            .await
            .expect("boot must yield both resolve messages");
        match &action {
            iced_runtime::Action::Output(Message::ResolveCurrentTrack) => current_track = true,
            iced_runtime::Action::Output(Message::ResolveRecentlyPlayed) => {
                assert!(
                    current_track,
                    "the played list's resolve must follow the last-played track's"
                );
                recently_played = true;
            }
            _ => {}
        }
    }
}

// The stored played rows last until the session arrives: a sign-in while the
// played list is showing refetches it, and the played rows drop.
#[test]
fn session_arrival_refetches_the_showing_played_list_and_drops_the_played_rows() {
    let (mut player, state) = test_player();
    player.current_view = CurrentView::RecentlyPlayed;
    let _ = update(&mut player, Message::RecentlyPlayedLoaded(stepping_songs()));
    record_a_play(&mut player, &state, "song-1");
    let _ = update(&mut player, Message::ShowRecentlyPlayed);
    assert_eq!(titles(&player), vec!["One".to_string()]);

    // The session arrives while the played list is showing.
    player
        .apple_music_service
        .authenticate_with("dev-token", &|_| Ok(sign_in_session()))
        .expect("the stub sign-in must store its session");
    let _ = update(&mut player, Message::SessionReady);

    // The refetch is in flight; the played rows are gone from the list.
    assert!(player.recently_played.loading);
    assert!(titles(&player).is_empty());
}
