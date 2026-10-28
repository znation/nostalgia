use super::*;

#[test]
fn await_or_timeout_returns_the_output_when_the_future_completes() {
    let output = futures::executor::block_on(await_or_timeout(
        "quick",
        std::time::Duration::from_secs(1),
        async { 7 },
    ));
    assert_eq!(output, 7);
}

// A task that never completes is the failure `await_or_timeout` exists to
// bound: without the timeout this test would hang forever, taking the whole
// suite with it. The panic names the awaited task, so the hang is diagnosable.
#[test]
#[should_panic(expected = "hung task did not complete within")]
fn await_or_timeout_fails_loudly_when_the_future_never_completes() {
    futures::executor::block_on(await_or_timeout(
        "hung",
        std::time::Duration::from_millis(10),
        futures::future::pending::<()>(),
    ));
}

// A browse reply can complete out of order: the user picks artist-1, then
// artist-2, and artist-1's slower fetch lands last. Storing that reply would
// replace artist-2's albums with artist-1's, showing the wrong artist's list.
// The per-list request generation rejects the superseded reply. This test
// holds the first task and drives it after the second has landed, which is
// the completion order a slow backend produces.
#[tokio::test]
async fn a_slow_stale_browse_reply_does_not_overwrite_the_newer_list() {
    let (mut player, _state) = test_player();
    player.artists.items = sample_library().artists.clone();

    // Issue two album fetches: artist-1 (index 0), then artist-2 (index 1).
    // The artists list is not replaced, so both presses carry epoch 0.
    let first = update(&mut player, Message::ArtistSelected { epoch: 0, index: 0 });
    let second = update(&mut player, Message::ArtistSelected { epoch: 0, index: 1 });

    // The newer request's reply lands first; the older one, landing late,
    // must be dropped and leave artist-2's albums in place.
    assert_stale_album_reply_is_dropped(&mut player, second, first, &["album-3"]).await;
}

// The stale-reply test above pins a superseded *success*; this pins the
// failure case. `fetch_into` checks the request generation before it matches
// the result, so a slow backend error is dropped exactly like a slow success:
// without that placement, a stale failure would run `BrowseList::fail` and
// wipe the newer artist's albums and record its error against them, so the
// panel would show the wrong artist's failure. Drive the failing request's
// task after the newer one has landed.
#[tokio::test]
async fn a_slow_stale_browse_failure_does_not_overwrite_the_newer_list_or_error() {
    let (mut player, _state) = test_player();
    // Index 0 is an artist whose blank id the service seam rejects, so its
    // fetch fails; index 1 is a real artist whose fetch succeeds.
    player.artists.items = vec![
        Artist {
            id: String::new(),
            name: "Blank".to_string(),
            album_count: 0,
        },
        sample_artist(),
    ];

    // Issue two album fetches: the failing artist-0, then the valid artist-1.
    // The artists list is not replaced, so both presses carry epoch 0.
    let first = update(&mut player, Message::ArtistSelected { epoch: 0, index: 0 });
    let second = update(&mut player, Message::ArtistSelected { epoch: 0, index: 1 });

    // The newer, successful reply lands first; the older request's failure
    // must be dropped rather than wiping the newer list and recording its
    // error against it.
    assert_stale_album_reply_is_dropped(&mut player, second, first, &["album-1", "album-2"]).await;
}

// The stale-reply tests above hold a reply in its task until after a newer
// request has landed, so `fetch_into`'s generation check on iced's executor
// thread already sees the reply as stale. This covers the narrower window
// that check cannot: the mapper runs while the reply is still current, and a
// newer request is issued on the UI thread before `update` processes the
// stamped reply. Re-checking the generation at store time must drop it, or the
// cleared list is repopulated with the superseded reply and the panel shows
// the previous level's rows.
#[tokio::test]
async fn a_reply_superseded_between_the_mapper_and_update_is_not_stored() {
    let (mut player, _state) = test_player();

    // Issue the first artists fetch and drive it to completion while it is
    // still current, so the mapper stamps its reply as current.
    let generation = player.artists.begin_fetch();
    let task = fetch_into(
        &player.apple_music_service,
        "loading favorite artists".to_string(),
        generation,
        std::time::Duration::from_secs(1),
        |service| async move { service.get_favorite_artists().await },
        Message::ArtistsLoaded,
        Message::ArtistsLoadFailed,
    );
    let reply = task_output(task, "first artists fetch").await;

    // A newer request for the same list is issued before `update` processes
    // the reply.
    let _ = update(&mut player, Message::LoadArtists);

    // The reply the newer request superseded must not be stored: the list
    // stays cleared and loading for the newer fetch.
    let _ = update(&mut player, reply);
    assert!(player.artists.items.is_empty());
    assert!(player.artists.loading);
}

// The songs twin of `a_reply_superseded_between_the_mapper_and_update_is_not_stored`.
// The songs browse level had no superseded-reply test at all, and
// `BrowseReply`'s UI-thread re-check names the songs list in its own match
// arm: without that arm a reply from the album the user browsed away from
// would repopulate the cleared songs buffer with the previous album's songs.
// Issue the first songs fetch and drive it to completion while it is still
// current, issue a newer songs request before `update` processes the reply,
// and assert the stamped reply is dropped.
#[tokio::test]
async fn a_stale_songs_reply_superseded_before_update_is_not_stored() {
    let (mut player, _state) = test_player();
    // Clear the songs buffer and mark it loading, as `AlbumSelected` does
    // before issuing a fetch, so the dropped reply's effect on `loading` is
    // observable below.
    player.songs.clear();

    // Issue the first songs fetch and drive it to completion while it is
    // still current, so the mapper stamps its reply as current rather than
    // dropping it as `Ignored` before it reaches the `BrowseReply` arm.
    let generation = player.songs.begin_fetch();
    let task = fetch_into(
        &player.apple_music_service,
        "loading songs from album \"album-1\"".to_string(),
        generation,
        std::time::Duration::from_secs(1),
        |service| async move { service.get_songs_from_album("album-1").await },
        Message::SongsLoaded,
        Message::SongsLoadFailed,
    );
    let reply = task_output(task, "first songs fetch").await;
    assert!(
        matches!(&reply, Message::BrowseReply { reply, .. } if matches!(reply.as_ref(), Message::SongsLoaded(_))),
        "the driven fetch must produce a stamped songs reply, got {reply:?}"
    );

    // A newer songs request is issued before `update` processes the reply.
    let _ = player.songs.begin_fetch();

    // The reply the newer request superseded must not be stored: the songs
    // buffer stays empty and loading for the newer fetch.
    let _ = update(&mut player, reply);
    assert!(player.songs.items.is_empty());
    assert!(player.songs.loading);
}

// The albums twin of `a_stale_songs_reply_superseded_before_update_is_not_stored`.
// `BrowseReply`'s UI-thread re-check names the albums list in its own match
// arm: without that arm a reply from the artist the user browsed away from
// would repopulate the cleared albums buffer with the previous artist's
// albums. Issue the first albums fetch and drive it to completion while it is
// still current, issue a newer albums request before `update` processes the
// reply, and assert the stamped reply is dropped.
#[tokio::test]
async fn a_stale_albums_reply_superseded_before_update_is_not_stored() {
    let (mut player, _state) = test_player();
    // Clear the albums buffer and mark it loading, as `ArtistSelected` does
    // before issuing a fetch, so the dropped reply's effect on `loading` is
    // observable below.
    player.albums.clear();

    // Issue the first albums fetch and drive it to completion while it is
    // still current, so the mapper stamps its reply as current rather than
    // dropping it as `Ignored` before it reaches the `BrowseReply` arm.
    let generation = player.albums.begin_fetch();
    let task = fetch_into(
        &player.apple_music_service,
        "loading albums for artist \"artist-1\"".to_string(),
        generation,
        std::time::Duration::from_secs(1),
        |service| async move { service.get_albums_by_artist("artist-1").await },
        Message::AlbumsLoaded,
        Message::AlbumsLoadFailed,
    );
    let reply = task_output(task, "first albums fetch").await;
    assert!(
        matches!(&reply, Message::BrowseReply { reply, .. } if matches!(reply.as_ref(), Message::AlbumsLoaded(albums) if !albums.is_empty())),
        "the driven fetch must produce a stamped non-empty albums reply, got {reply:?}"
    );

    // A newer albums request is issued before `update` processes the reply.
    let _ = player.albums.begin_fetch();

    // The reply the newer request superseded must not be stored: the albums
    // buffer stays empty and loading for the newer fetch.
    let _ = update(&mut player, reply);
    assert!(player.albums.items.is_empty());
    assert!(player.albums.loading);
}

// `BrowseReply` names every browse list in its own match arm, but only the
// Artists, Songs, and Albums arms had their UI-thread re-check pinned. The
// three flat lists behind the Artists view's buttons are fetched the same
// way, and pressing Back then the button again issues a newer request before
// the first reply is stored. Without each list's arm the stale reply would
// fall through the `_ => true` catch-all and store the previous rows — or its
// error — over the list the newer request cleared. Pin both reply kinds for
// each flat list: a stale reply leaves the cleared list loading with no rows
// and no error, and that list's own current generation still stores or
// records its reply.
#[test]
fn stale_flat_list_replies_superseded_before_update_are_not_stored() {
    let (mut player, _state) = test_player();

    macro_rules! assert_list_rechecks_its_generation {
        ($list:ident, $loaded:expr, $failed:expr) => {{
            player.$list.clear();
            let stale = player.$list.begin_fetch();
            let current = player.$list.begin_fetch();

            let _ = update(
                &mut player,
                Message::BrowseReply {
                    issued: stale.issued(),
                    reply: Box::new($loaded),
                },
            );
            assert!(
                player.$list.items.is_empty(),
                concat!(stringify!($list), " stored a stale success reply")
            );
            assert!(player.$list.loading);
            assert!(player.$list.error.is_none());

            let _ = update(
                &mut player,
                Message::BrowseReply {
                    issued: stale.issued(),
                    reply: Box::new($failed("stale error".to_string())),
                },
            );
            assert!(
                player.$list.items.is_empty(),
                concat!(stringify!($list), " stored a stale failure reply")
            );
            assert!(player.$list.loading);
            assert!(player.$list.error.is_none());

            let _ = update(
                &mut player,
                Message::BrowseReply {
                    issued: current.issued(),
                    reply: Box::new($loaded),
                },
            );
            assert_eq!(
                player.$list.items.len(),
                1,
                concat!(stringify!($list), " dropped its current success reply")
            );
            assert!(!player.$list.loading);
            assert!(player.$list.error.is_none());

            let _ = update(
                &mut player,
                Message::BrowseReply {
                    issued: current.issued(),
                    reply: Box::new($failed("current error".to_string())),
                },
            );
            assert!(player.$list.items.is_empty());
            assert!(!player.$list.loading);
            assert_eq!(player.$list.error.as_deref(), Some("current error"));
        }};
    }

    assert_list_rechecks_its_generation!(
        all_songs,
        Message::AllSongsLoaded(vec![sample_song()]),
        Message::AllSongsLoadFailed
    );
    assert_list_rechecks_its_generation!(
        all_albums,
        Message::AllAlbumsLoaded(vec![sample_album()]),
        Message::AllAlbumsLoadFailed
    );
    assert_list_rechecks_its_generation!(
        recently_added,
        Message::RecentlyAddedLoaded(vec![sample_album()]),
        Message::RecentlyAddedLoadFailed
    );
    assert_list_rechecks_its_generation!(
        playlists,
        Message::PlaylistsLoaded(vec![sample_playlist()]),
        Message::PlaylistsLoadFailed
    );
}

// A play reply can complete out of order just like a browse reply: the user
// clicks song-1, then song-2, and song-1's slower play lands last. The
// playback counter's guard, evaluated inside `play_track` under the state
// lock, rejects the superseded play so it cannot overwrite the newer track —
// without it the Now Playing bar would name song-1 after the user's last
// click was song-2. This drives the two `TrackSelected` tasks in the reverse
// of their issue order, the completion order a slow backend produces.
#[tokio::test]
async fn a_slow_stale_play_does_not_overwrite_the_newer_track() {
    let (mut player, state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));

    // Issue two plays: song-1 (index 0), then song-2 (index 1). The songs
    // list is not replaced, so both presses carry epoch 1.
    let first = update(&mut player, Message::TrackSelected { epoch: 1, index: 0 });
    let second = update(&mut player, Message::TrackSelected { epoch: 1, index: 1 });

    // The newer play's reply lands first.
    drive_task(second, "newer play", |_| {}).await;
    assert_eq!(state.lock().await.current_track.as_deref(), Some("song-2"));

    // The older play's reply lands late and must be dropped, leaving the
    // newer track playing.
    drive_task(first, "stale play", |_| {}).await;
    assert_eq!(state.lock().await.current_track.as_deref(), Some("song-2"));
}

#[tokio::test]
async fn boot_schedules_loading_the_artist_list() {
    let state = Arc::new(Mutex::new(AppState::default()));
    let service =
        AppleMusicService::with_transport(state.clone(), Box::new(StubTransport::returning("")));

    let (player, task) = boot(state, service);
    assert_view(&player, CurrentView::Artists);

    // The batched task carries both the artists fetch and the window-id query,
    // and iced runs both after the window opens; the fetch's `LoadArtists`
    // output is the first the stream yields.
    await_boot_artist_loads(task, 1).await;
}

// The startup sign-in runs on a background thread, so the first artists fetch
// can land before the session exists and answer from the sample library. The
// boot task must watch for the session and re-issue `LoadArtists` when it
// arrives, or a signed-in user keeps seeing the sample artists for the rest of
// the process. A regression that dropped that watch would exhaust the boot
// stream after the initial fetch, so [`await_boot_artist_loads`] would time
// out waiting for a second `LoadArtists`.
#[tokio::test]
async fn boot_reloads_artists_after_the_startup_sign_in_stores_a_session() {
    let state = Arc::new(Mutex::new(AppState::default()));
    let service = AppleMusicService::new(state.clone());
    let sign_in_service = service.clone();

    let (player, task) = boot(state, service);
    assert_view(&player, CurrentView::Artists);

    // Store the session from a background thread, as `init_service` does, so
    // the boot task observes a session that arrives after it started waiting.
    let sign_in = std::thread::spawn(move || {
        sign_in_service
            .authenticate_with("dev-token", &|_| Ok(sign_in_session()))
            .expect("the stub sign-in must store its session");
    });

    // The batched task carries the initial fetch, the session wait, and the
    // window-id query; drive until the wait's second `LoadArtists` lands.
    await_boot_artist_loads(task, 2).await;
    sign_in.join().expect("the sign-in thread must not panic");
}

#[tokio::test]
async fn load_artists_fetches_favorite_artists_into_the_player() {
    let (mut player, _state) = test_player();

    let task = update(&mut player, Message::LoadArtists);
    drive_fetch_and_assert_loaded(
        &mut player,
        task,
        "load artists",
        |player| &mut player.artists.items,
        |artist| artist.id.as_str(),
        &["artist-1", "artist-2", "artist-3"],
    )
    .await;
}

// A failed artists fetch is a dead end: `boot` issues `LoadArtists` once and
// no navigation re-issues it, so the Artists view's Retry button sends
// `LoadArtists` again. The arm must clear the stored failure before the
// re-fetch, or the view would keep showing the old error (and never
// "Loading…") while the retry is in flight. Pin the clear alongside the
// empty buffer and the loading flag.
#[test]
fn load_artists_clears_a_prior_failure_before_refetching() {
    let (mut player, _state) = test_player();
    player.artists.items = vec![sample_artist()];
    player.artists.error =
        Some("music-library fetch failed (loading favorite artists): boom".to_string());
    player.artists.loading = false;

    let _task = update(&mut player, Message::LoadArtists);

    assert!(player.artists.items.is_empty());
    assert!(player.artists.loading);
    assert!(player.artists.error.is_none());
}

// `fetch_into` schedules the fetch as an iced `Task`; the arm itself only
// builds it, so the real behavior lives in the returned task. Drive that
// task to completion and assert the mapped `*Loaded` message, as the
// load-path arms would produce it.
#[tokio::test]
async fn fetch_into_schedules_fetch_and_maps_result_to_loaded_message() {
    let (player, _state) = test_player();

    let generation = test_generation();
    let task = fetch_into(
        &player.apple_music_service,
        "loading favorite artists".to_string(),
        generation,
        std::time::Duration::from_secs(1),
        |service| async move { service.get_favorite_artists().await },
        Message::ArtistsLoaded,
        Message::ArtistsLoadFailed,
    );
    drive_task(task, "fetch", |message| {
        assert!(matches!(
            browse_reply(message),
            Message::ArtistsLoaded(artists) if !artists.is_empty()
        ));
    })
    .await;
}

// The success path above proves `fetch_into` delivers a `*Loaded` message
// for a successful fetch; this pins the error path — a fetch that returns
// `Err` must yield the matching `*LoadFailed` message carrying the formatted
// report (which names the failed query and includes the backend's cause),
// not a `*Loaded` message with an empty list that the panel would misread as
// an empty library. `fetch_failure_report` is tested in `loading`'s test
// module, but no test drives a failing fetch *through* `fetch_into` — the real
// service always succeeds, so the error path is reachable only by injecting a
// failing fetch closure, which is exactly what this does. A regression that
// swallowed the error would leave the browse panel claiming the library is
// empty, and only this test would catch it.
#[tokio::test]
async fn fetch_into_maps_a_failed_fetch_to_a_load_failed_message() {
    let (player, _state) = test_player();

    let generation = test_generation();
    let task = fetch_into(
        &player.apple_music_service,
        "loading albums for artist \"artist-1\"".to_string(),
        generation,
        std::time::Duration::from_secs(1),
        |_service| async { Err::<Vec<Album>, String>("boom".to_string()) },
        Message::AlbumsLoaded,
        Message::AlbumsLoadFailed,
    );
    drive_task(task, "failed fetch", |message| {
        assert!(matches!(
            browse_reply(message),
            Message::AlbumsLoadFailed(report)
                if report
                    == "music-library fetch failed (loading albums for artist \"artist-1\"): boom"
        ));
    })
    .await;
}

// A fetch that never answers is the failure `with_timeout` exists to bound:
// without it the browse panel stays on "Loading…" forever, because `loading`
// only clears when the reply lands. The real service always answers, so the
// hang is reachable only by injecting a pending fetch closure; the short
// timeout keeps the test fast. A timeout is reported through the `*LoadFailed`
// path, exactly like a returned error, so the panel names the failed fetch
// (and, for the top-level artists fetch, offers the Retry button) instead of
// misreading the hang as an empty library.
#[tokio::test]
async fn fetch_into_maps_a_hanging_fetch_to_a_failed_message() {
    let (player, _state) = test_player();

    let generation = test_generation();
    let task = fetch_into(
        &player.apple_music_service,
        "loading favorite artists".to_string(),
        generation,
        std::time::Duration::from_millis(50),
        |_service| std::future::pending::<Result<Vec<Artist>, String>>(),
        Message::ArtistsLoaded,
        Message::ArtistsLoadFailed,
    );
    drive_task(task, "hanging fetch", |message| {
        assert!(matches!(
            browse_reply(message),
            Message::ArtistsLoadFailed(report)
                if report
                    == "music-library fetch failed (loading favorite artists); timed out after 50ms"
        ));
    })
    .await;
}

// The play path has the same unbounded-wait failure as the browse path: a
// backend that never answers leaves the play's task pending forever, so its
// `TrackPlayed` completion never fires and `update`'s title-index prune never
// runs. `play_into` bounds the play with its `timeout`; the real service
// always answers, so the hang is reachable only by injecting a pending play
// closure. The short timeout keeps the test fast, and the completion must
// still arrive.
#[tokio::test]
async fn play_into_maps_a_hanging_play_to_a_completed_message() {
    let (player, _state) = test_player();
    let generation = test_generation();
    let expected = generation.issued();

    let task = play_into(
        &player.apple_music_service,
        "song-1".to_string(),
        generation,
        std::time::Duration::from_millis(50),
        |_service, _id, _generation| std::future::pending::<Result<(), String>>(),
        |_err| {},
    );
    assert_track_played(task, expected, false).await;
}

// The play path's error is handed to `play_into`'s injected reporter; the real
// service rejects only a blank id, which the `update`-level test covers, so
// this injects a failing play closure and pins that the reporter sees the
// error while the completion still arrives. A swallowed error would leave a
// rejected play looking like it worked.
#[tokio::test]
async fn play_into_reports_a_failed_play_and_still_completes() {
    let (player, _state) = test_player();
    let generation = test_generation();
    let expected = generation.issued();
    let (reported, received) = std::sync::mpsc::channel();

    let task = play_into(
        &player.apple_music_service,
        "song-1".to_string(),
        generation,
        std::time::Duration::from_secs(1),
        |_service, _id, _generation| async { Err::<(), String>("boom".to_string()) },
        move |err| {
            let _ = reported.send(err.clone());
        },
    );
    assert_track_played(task, expected, false).await;

    assert_eq!(received.try_recv().ok().as_deref(), Some("boom"));
}
