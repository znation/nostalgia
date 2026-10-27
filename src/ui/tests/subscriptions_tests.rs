use super::*;

#[test]
fn alt_o_routes_captured_text_with_options_open_or_closed() {
    use iced::advanced::subscription;
    use iced::futures::{StreamExt, executor::block_on, stream};
    use iced::{Event, event::Status, keyboard};

    for (jump_file_open, options_open) in [(false, false), (false, true), (true, false)] {
        let listener = subscriptions(
            false,
            false,
            false,
            false,
            jump_file_open,
            options_open,
            true,
        )
        .remove(0);
        let recipe = subscription::into_recipes(listener).remove(0);
        let (mut player, _) = test_player();
        player.search_query = "test".into();
        player.options_visible = options_open;
        player.jump_file_visible = jump_file_open;
        let mut renderer = layout_capture::headless_renderer();
        let mut interface = iced_runtime::user_interface::UserInterface::build(
            view::view(&player),
            layout_capture::limits().max(),
            iced_runtime::user_interface::Cache::new(),
            &mut renderer,
        );
        interface.operate(
            &renderer,
            &mut iced::advanced::widget::operation::focusable::focus::<()>(BROWSE_SEARCH_ID.into()),
        );
        // Exercise printable Alt+O through the real focused widget tree.
        let events = [Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Character("o".into()),
            modified_key: keyboard::Key::Character("o".into()),
            physical_key: keyboard::key::Physical::Code(keyboard::key::Code::KeyO),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::ALT,
            text: Some("o".into()),
            repeat: false,
        })];
        let mut messages = Vec::new();
        let (_, statuses) = interface.update(
            &events,
            iced::mouse::Cursor::Unavailable,
            &mut renderer,
            &mut iced::advanced::clipboard::Null,
            &mut messages,
        );
        assert!(
            !messages
                .iter()
                .any(|message| matches!(message, Message::SearchChanged(_)))
        );
        drop(interface);
        let mut interactions: Vec<_> = events
            .into_iter()
            .zip(statuses)
            .map(|(event, status)| subscription::Event::Interaction {
                window: iced::window::Id::unique(),
                event,
                status,
            })
            .collect();
        interactions.extend(["p"].map(|text| subscription::Event::Interaction {
            window: iced::window::Id::unique(),
            event: Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Character(text.into()),
                modified_key: keyboard::Key::Character(text.into()),
                physical_key: keyboard::key::Physical::Code(if text == "o" {
                    keyboard::key::Code::KeyO
                } else {
                    keyboard::key::Code::KeyP
                }),
                location: keyboard::Location::Standard,
                modifiers: keyboard::Modifiers::ALT,
                text: Some(text.into()),
                repeat: false,
            }),
            status: Status::Captured,
        }));
        let routed = block_on(
            recipe
                .stream(Box::pin(stream::iter(interactions)))
                .collect::<Vec<_>>(),
        );
        assert!(matches!(routed.as_slice(), [Message::ToggleOptions]));
        for message in messages.into_iter().chain(routed) {
            let _ = update(&mut player, message);
        }
        assert_eq!(player.search_query, "test");
        assert_eq!(player.options_visible, !options_open);
    }
}

#[test]
fn options_navigation_routes_captured_search_keys_without_submitting() {
    use iced::advanced::{subscription, widget::operation::focusable};
    use iced::futures::{StreamExt, executor::block_on, stream};
    use iced::{Event, event::Status, keyboard, mouse};

    let (mut player, _) = test_player();
    player.search_query = "test".into();
    let _ = update(&mut player, Message::ToggleOptions);
    let mut renderer = layout_capture::headless_renderer();
    let mut interface = iced_runtime::user_interface::UserInterface::build(
        view::view(&player),
        layout_capture::limits().max(),
        iced_runtime::user_interface::Cache::new(),
        &mut renderer,
    );
    // Ctrl+F can focus the search box even while the menu is open.
    interface.operate(
        &renderer,
        &mut focusable::focus::<()>(BROWSE_SEARCH_ID.into()),
    );
    let events: Vec<_> = [
        keyboard::key::Named::ArrowDown,
        keyboard::key::Named::ArrowUp,
        keyboard::key::Named::Enter,
    ]
    .into_iter()
    .map(|named| {
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(named),
            modified_key: keyboard::Key::Named(named),
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers: keyboard::Modifiers::NONE,
            text: None,
            repeat: false,
        })
    })
    .collect();
    let mut messages = Vec::new();
    let (_, statuses) = interface.update(
        &events,
        mouse::Cursor::Unavailable,
        &mut renderer,
        &mut iced::advanced::clipboard::Null,
        &mut messages,
    );
    assert_eq!(statuses[2], Status::Captured);
    assert!(
        !messages
            .iter()
            .any(|m| matches!(m, Message::SearchSubmitted))
    );
    drop(interface);

    // Exercise the installed subscription, including its event-status filter.
    let listener = subscriptions(false, false, false, false, false, true, true).remove(0);
    let recipe = subscription::into_recipes(listener).remove(0);
    let input = stream::iter(events.into_iter().zip(statuses).map(|(event, status)| {
        subscription::Event::Interaction {
            window: iced::window::Id::unique(),
            event,
            status,
        }
    }));
    let routed = block_on(recipe.stream(Box::pin(input)).collect::<Vec<_>>());
    assert!(matches!(
        routed.as_slice(),
        [
            Message::OptionsSelectionDown,
            Message::OptionsSelectionUp,
            Message::OptionsActivate
        ]
    ));
    for message in messages.into_iter().chain(routed) {
        let _ = update(&mut player, message);
    }
    assert!(player.always_on_top);
    assert!(!player.options_visible);
    assert!(!player.search_active);
}

// The marquee timer advances the title window by one character without
// scheduling any work. The timer itself runs only while the line is too long
// to fit the LCD (see `the_marquee_timer_runs_only_while_the_line_needs_to_scroll`),
// so every tick scrolls the bar.
#[test]
fn marquee_tick_advances_the_offset() {
    let (mut player, _state) = test_player();

    assert_message_schedules_no_work(&mut player, Message::MarqueeTick);
    assert_eq!(player.marquee_offset, 1);
    assert_message_schedules_no_work(&mut player, Message::MarqueeTick);
    assert_eq!(player.marquee_offset, 2);
}

// Every message makes iced rebuild the whole visible screen, the library panel
// included while it shows, so the marquee timer must not run for a line that
// fits the LCD whole: it would rebuild the UI four times a second to redraw a
// bar that cannot change. The keyboard listener always runs, so the
// subscription set is the two listeners until the line needs to scroll.
#[test]
fn the_marquee_timer_runs_only_while_the_line_needs_to_scroll() {
    assert_eq!(
        subscriptions(false, false, false, false, false, false, false).len(),
        2
    );
    assert_eq!(
        subscriptions(true, false, false, false, false, false, false).len(),
        3
    );
    assert_eq!(
        subscriptions(false, false, false, true, false, false, false).len(),
        3
    );
}

// The key listener that clears a mixer readout is always subscribed: a slider's
// arrow-key change sets the readout during the very event the listener must
// then see. The wheel listener joins only while a readout shows, so plain list
// scrolling rebuilds no screen.
// The Escape listener joins only while a transient panel is open, so Escape
// outside a panel rebuilds no screen.
#[test]
fn the_escape_listener_runs_only_while_a_transient_panel_is_open() {
    assert_eq!(
        subscriptions(false, false, false, false, false, false, false).len(),
        2
    );
    assert_eq!(
        subscriptions(false, false, false, false, false, false, true).len(),
        3
    );
}

// A plain or Shift-held Escape press is the panel-dismiss key, and the status
// is ignored so a focused text box that captures the press cannot swallow it.
// Modifier chords, the release, and other keys send nothing and keep their
// existing bindings.
#[test]
fn escape_key_message_maps_escape_but_not_its_chords_or_other_keys() {
    use iced::{Event, event::Status, keyboard, window};
    fn press(key: keyboard::Key, modifiers: keyboard::Modifiers) -> Event {
        Event::Keyboard(keyboard::Event::KeyPressed {
            key: key.clone(),
            modified_key: key,
            physical_key: keyboard::key::Physical::Unidentified(
                keyboard::key::NativeCode::Unidentified,
            ),
            location: keyboard::Location::Standard,
            modifiers,
            text: None,
            repeat: false,
        })
    }
    let escape = keyboard::Key::Named(keyboard::key::Named::Escape);

    assert!(matches!(
        escape_key_message(
            press(escape.clone(), keyboard::Modifiers::NONE),
            Status::Captured,
            window::Id::unique()
        ),
        Some(Message::EscapePressed)
    ));
    assert!(matches!(
        escape_key_message(
            press(escape.clone(), keyboard::Modifiers::SHIFT),
            Status::Captured,
            window::Id::unique()
        ),
        Some(Message::EscapePressed)
    ));
    for modifiers in [
        keyboard::Modifiers::CTRL,
        keyboard::Modifiers::ALT,
        keyboard::Modifiers::LOGO,
    ] {
        assert!(
            escape_key_message(
                press(escape.clone(), modifiers),
                Status::Ignored,
                window::Id::unique()
            )
            .is_none()
        );
    }

    let release = Event::Keyboard(keyboard::Event::KeyReleased {
        key: escape.clone(),
        modified_key: escape,
        physical_key: keyboard::key::Physical::Unidentified(
            keyboard::key::NativeCode::Unidentified,
        ),
        location: keyboard::Location::Standard,
        modifiers: keyboard::Modifiers::NONE,
    });
    assert!(escape_key_message(release, Status::Ignored, window::Id::unique()).is_none());
    assert!(
        escape_key_message(
            press(
                keyboard::Key::Named(keyboard::key::Named::Enter),
                keyboard::Modifiers::NONE
            ),
            Status::Ignored,
            window::Id::unique()
        )
        .is_none()
    );
}

#[test]
fn the_mixer_nudge_listener_is_always_subscribed() {
    assert_eq!(
        subscriptions(false, false, false, false, false, false, false).len(),
        2
    );
    assert_eq!(
        subscriptions(true, true, true, false, false, false, false).len(),
        5
    );
    assert_eq!(
        subscriptions(false, false, false, true, false, false, false).len(),
        3
    );
}

// The position poll has the same cost as the marquee: every tick and reply
// rebuilds the whole visible screen, the library panel included while it
// shows. A stopped player's view clears its current track's time readout
// (except the `--:--` placeholder), so the poll must leave the subscription
// set then, and join it again only while playback is running or the backend
// still holds a preview whose position a seek can move.
#[test]
fn the_position_timer_runs_only_while_there_is_a_position_to_refresh() {
    assert_eq!(
        subscriptions(false, true, false, false, false, false, false).len(),
        3
    );
    assert_eq!(
        subscriptions(true, true, false, false, false, false, false).len(),
        4
    );

    let (mut player, state) = test_player();
    assert!(!player.needs_position_poll());

    state.blocking_lock().play();
    assert!(player.needs_position_poll());

    player.playback_position = Some(Duration::from_secs(45));
    state.blocking_lock().pause();
    assert!(player.needs_position_poll());

    player.playback_position = None;
    state.blocking_lock().stop();
    assert!(!player.needs_position_poll());
}

// The spectrum poll costs the same as the others: it is subscribed only while
// playback runs, since a paused or stopped player keeps the last bars the view
// already stored.
#[test]
fn the_spectrum_timer_runs_only_while_playback_runs() {
    assert_eq!(
        subscriptions(false, false, false, false, false, false, false).len(),
        2
    );
    assert_eq!(
        subscriptions(false, false, true, false, false, false, false).len(),
        3
    );
    assert_eq!(
        subscriptions(true, true, true, false, false, false, false).len(),
        5
    );

    let (player, state) = test_player();
    assert!(!player.needs_spectrum_poll());

    state.blocking_lock().play();
    assert!(player.needs_spectrum_poll());

    state.blocking_lock().pause();
    assert!(!player.needs_spectrum_poll());

    state.blocking_lock().play();
    assert!(player.needs_spectrum_poll());

    state.blocking_lock().stop();
    assert!(!player.needs_spectrum_poll());
}

// The timer is subscribed from the player's own view of the bar: the same
// number, label, and artist the view renders, measured the same way. A line
// that fits leaves the timer unsubscribed; a line past the LCD's character
// budget needs it, and the playlist number prefix counts toward that budget.
#[test]
fn the_player_knows_when_the_now_playing_line_needs_the_marquee() {
    let (mut player, state) = test_player();

    // Nothing is current: the bar reads "Nothing", which fits.
    assert!(!player.needs_marquee());

    // A known track with a short title fits — until its row number joins the
    // line. "One" (3) plus "The Sample Band" (15) and " - " (3) is 21
    // characters; the 3-character `"1. "` prefix pushes it past the
    // 22-character budget.
    player.known_tracks.insert(
        "song-1".to_string(),
        views::KnownTrack {
            title: "One".to_string(),
            artist: "The Sample Band".to_string(),
            album: String::new(),
            year: None,
            genre: String::new(),
            track_number: None,
            number: String::new(),
            time: "3:30".to_string(),
            duration_ms: 210_000,
        },
    );
    state.blocking_lock().current_track = Some("song-1".to_string());
    assert!(!player.needs_marquee());
    player
        .known_tracks
        .get_mut("song-1")
        .expect("seeded")
        .number = "1. ".to_string();
    assert!(player.needs_marquee());

    // A title that pushes the line past the LCD's budget needs the marquee.
    player.known_tracks.insert(
        "song-2".to_string(),
        views::KnownTrack {
            title: "A Title Too Long For The LCD".to_string(),
            artist: "The Sample Band".to_string(),
            album: String::new(),
            year: None,
            genre: String::new(),
            track_number: None,
            number: "2. ".to_string(),
            time: "3:30".to_string(),
            duration_ms: 210_000,
        },
    );
    state.blocking_lock().current_track = Some("song-2".to_string());
    assert!(player.needs_marquee());
}

// A slow UI must not kill the timer: the futures channel holds
// `buffer + senders` messages — two for its one sender — so after the receiver
// lets two ticks queue, the next send finds it full. The sender must drop that
// tick and keep running rather than stop forever. Sleeping through several
// tick periods without polling first forces those full-channel sends; the
// third tick read below can then only arrive if the thread survived them.
#[test]
fn marquee_ticks_keep_ticking_when_the_receiver_falls_behind() {
    use futures::StreamExt;

    let mut ticks = marquee_tick_stream();
    // Queue both messages the channel can hold, then leave the sender several
    // ticks to attempt more sends while the channel is full.
    std::thread::sleep(MARQUEE_TICK * 6);

    let first = futures::executor::block_on(await_or_timeout(
        "first marquee tick",
        TASK_TIMEOUT,
        ticks.next(),
    ));
    assert!(matches!(first, Some(Message::MarqueeTick)));
    let second = futures::executor::block_on(await_or_timeout(
        "second marquee tick",
        TASK_TIMEOUT,
        ticks.next(),
    ));
    assert!(matches!(second, Some(Message::MarqueeTick)));
    let third = futures::executor::block_on(await_or_timeout(
        "marquee tick after a full channel",
        TASK_TIMEOUT,
        ticks.next(),
    ));
    assert!(
        matches!(third, Some(Message::MarqueeTick)),
        "the timer stopped after a full channel"
    );
}

// A refused timer thread must not take the player down: `std::thread::spawn`
// panics when the OS refuses a thread, and iced builds the subscription's
// stream on its main thread while it updates the subscriptions, so that panic
// would unwind the event loop and kill the app over a cosmetic timer. The
// fallible spawn reports the refusal and ends the tick stream instead, so the
// app keeps running with a Now Playing title that no longer scrolls.
#[test]
fn a_refused_marquee_thread_ends_the_tick_stream_instead_of_panicking() {
    assert_refused_thread_ends_the_tick_stream(
        "marquee",
        marquee_tick_stream_with(|_work| {
            Err(std::io::Error::other("the OS refused another thread"))
        }),
    );
}

// The refusal is reported, not swallowed: the report names the cause and
// says what the user loses, so a Now Playing title that never scrolls is
// diagnosable instead of mysterious.
#[test]
fn a_refused_marquee_thread_reports_the_cause_and_the_loss() {
    let report = marquee_thread_failure_report(&std::io::Error::other("no threads left"));

    assert_report_names_the_cause_and_the_loss(&report, "no threads left", "will not scroll");
}

// The timer thread carries a name, like the audio and REST workers, so a
// panic in it is attributable. The name is spelled out here rather than read
// from the production constant, so renaming the constant fails this test.
#[test]
fn the_marquee_thread_is_named_nostalgia_marquee() {
    assert_eq!(
        reported_thread_name(spawn_marquee_thread).as_deref(),
        Some("nostalgia-marquee")
    );
}

// A newly selected song must start its title at character zero rather than
// inheriting the previous song's scroll position.
#[test]
fn track_selected_resets_the_marquee_offset() {
    let (mut player, _state) = test_player();
    let _ = update(&mut player, Message::SongsLoaded(stepping_songs()));
    player.marquee_offset = 7;

    let _ = update(&mut player, Message::TrackSelected { epoch: 1, index: 0 });

    assert_eq!(player.marquee_offset, 0);
}

#[test]
fn spectrum_tick_queries_spectrum_and_carries_the_epoch() {
    let recording = Arc::new(RecordingAudio::default());
    let (mut player, _state) =
        player_with_audio(Arc::clone(&recording) as Arc<dyn crate::audio::AudioOutput>);
    let epoch = player.position_epoch;
    let replies = task_outputs(update(&mut player, Message::SpectrumTick), "spectrum tick");

    assert!(
        replies.iter().any(|message| matches!(
            message,
            Message::Spectrum { epoch: reply_epoch, spectrum: Some(spectrum) }
                if *reply_epoch == epoch && *spectrum == recording_spectrum()
        )),
        "the tick must query the spectrum at its epoch: {replies:?}"
    );
}

#[test]
fn position_tick_stream_emits_position_ticks() {
    use futures::StreamExt;

    let mut ticks = position_tick_stream();
    for what in ["first position tick", "second position tick"] {
        let tick = futures::executor::block_on(await_or_timeout(what, TASK_TIMEOUT, ticks.next()));
        assert!(
            matches!(tick, Some(Message::PositionTick)),
            "{what} did not yield PositionTick: {tick:?}"
        );
    }
}

// A refused poll thread must not take the player down: `std::thread::spawn`
// panics when the OS refuses a thread, and iced builds the subscription's
// stream on its main thread while it updates the subscriptions, so that panic
// would unwind the event loop and kill the app on startup. The fallible spawn
// reports the refusal and ends the tick stream instead, so the app keeps
// running with an elapsed readout that no longer advances.
#[test]
fn a_refused_position_thread_ends_the_tick_stream_instead_of_panicking() {
    assert_refused_thread_ends_the_tick_stream(
        "position",
        position_tick_stream_with(|_work| {
            Err(std::io::Error::other("the OS refused another thread"))
        }),
    );
}

// The refusal is reported, not swallowed: the report names the cause and
// says what the user loses, so a readout that never advances is diagnosable
// instead of mysterious.
#[test]
fn a_refused_position_thread_reports_the_cause_and_the_loss() {
    let report = position_thread_failure_report(&std::io::Error::other("no threads left"));

    assert_report_names_the_cause_and_the_loss(&report, "no threads left", "will not update");
}

// The poll thread carries a name, like the audio and REST workers, so a panic
// in it is attributable. The name is spelled out here rather than read from
// the production constant, so renaming the constant fails this test.
#[test]
fn the_position_thread_is_named_nostalgia_position_poll() {
    assert_eq!(
        reported_thread_name(spawn_position_thread).as_deref(),
        Some("nostalgia-position-poll")
    );
}

// The spectrum poll's stream emits the tick its `update` arm turns into a
// backend query, just as the position stream does: without this, the
// visualizer would never refresh even though the subscription is wired.
#[test]
fn spectrum_tick_stream_emits_spectrum_ticks() {
    use futures::StreamExt;

    let mut ticks = spectrum_tick_stream();
    for what in ["first spectrum tick", "second spectrum tick"] {
        let tick = futures::executor::block_on(await_or_timeout(what, TASK_TIMEOUT, ticks.next()));
        assert!(
            matches!(tick, Some(Message::SpectrumTick)),
            "{what} did not yield SpectrumTick: {tick:?}"
        );
    }
}

// A refused spectrum poll thread must not take the player down, for the same
// reason as the other timers: `std::thread::spawn` panics when the OS refuses
// a thread, and iced builds the stream on its main thread. The fallible spawn
// reports the refusal and ends the tick stream, so the app keeps running with
// a visualizer that stops updating.
#[test]
fn a_refused_spectrum_thread_ends_the_tick_stream_instead_of_panicking() {
    assert_refused_thread_ends_the_tick_stream(
        "spectrum",
        spectrum_tick_stream_with(|_work| {
            Err(std::io::Error::other("the OS refused another thread"))
        }),
    );
}

// The spectrum refusal is reported, not swallowed: the report names the cause
// and says what the user loses, so a visualizer that never moves is
// diagnosable instead of mysterious.
#[test]
fn a_refused_spectrum_thread_reports_the_cause_and_the_loss() {
    let report = spectrum_thread_failure_report(&std::io::Error::other("no threads left"));

    assert_report_names_the_cause_and_the_loss(&report, "no threads left", "will not update");
}

// The spectrum poll thread carries a name, like the audio, REST, and other
// timer workers, so a panic in it is attributable. The name is spelled out
// here rather than read from the production constant, so renaming it fails.
#[test]
fn the_spectrum_thread_is_named_nostalgia_spectrum_poll() {
    assert_eq!(
        reported_thread_name(spawn_spectrum_thread).as_deref(),
        Some("nostalgia-spectrum-poll")
    );
}

/// Asserts that a tick stream whose thread spawn was refused ends quietly
/// instead of panicking the UI thread. The three timer streams (`marquee`,
/// `position`, and `spectrum`) all share this contract; `what` names the
/// stream in the failure messages.
fn assert_refused_thread_ends_the_tick_stream(
    what: &str,
    ticks: impl futures::Stream<Item = Message> + Unpin,
) {
    use futures::StreamExt;

    let mut ticks = ticks;
    let stream = format!("the {what} stream after a refused thread");
    let next = futures::executor::block_on(await_or_timeout(&stream, TASK_TIMEOUT, ticks.next()));
    assert!(
        next.is_none(),
        "a refused {what} thread must end the tick stream, not tick or panic"
    );
}

/// Asserts that a timer thread's failure report names the cause it was given
/// and says what the user loses; the three timer reports pair a cause with
/// what stops updating (the title scroll, the position readout, or the
/// spectrum).
fn assert_report_names_the_cause_and_the_loss(report: &str, cause: &str, loss: &str) {
    assert!(
        report.contains(cause),
        "the report must name the cause, got: {report}"
    );
    assert!(
        report.contains(loss),
        "the report must say what the user loses, got: {report}"
    );
}

/// Runs `spawn`'s thread just long enough to capture the name it registers,
/// so a test can pin the name spelled out rather than read from the production
/// constant. `spawn` is one of the timer spawners (`spawn_marquee_thread`,
/// `spawn_position_thread`, or `spawn_spectrum_thread`).
fn reported_thread_name(
    spawn: fn(Box<dyn FnOnce() + Send>) -> std::io::Result<()>,
) -> Option<String> {
    let (sender, receiver) = std::sync::mpsc::channel();

    spawn(Box::new(move || {
        let _ = sender.send(std::thread::current().name().map(str::to_string));
    }))
    .expect("the timer thread starts");

    receiver
        .recv_timeout(TASK_TIMEOUT)
        .expect("the timer thread reports its name")
}
