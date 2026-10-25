use super::*;

// Sends one query to a worker with no player installed and asserts it
// answers `None` without creating a player. `query` names the query in the
// panic message; `command` wraps the reply sender in the query's variant.
fn assert_unloaded_query_reports_none<T: std::fmt::Debug + PartialEq>(
    query: &str,
    command: fn(mpsc::Sender<Option<T>>) -> Command,
) {
    let (sender, receiver) = mpsc::channel();
    let create_player = || -> RecordingPlayer {
        panic!("a {query} must not create a player");
    };
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };
    let (reply, replies) = mpsc::channel();

    sender.send(command(reply)).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    assert_eq!(replies.recv().unwrap(), None);
}

// `play_track` commits the new track and returns `Ok` as soon as the
// command is queued, so the worker's later download failure is invisible
// to the caller. If that failure left the previous player running, the old
// preview would stay audible under the new track's name. The first Play
// installs a player and the second's download fails; the loop must stop
// the first player and install none. The second selection is queued from
// inside `create_player`, once the first player is installed, so the two
// do not coalesce into one.
#[test]
fn a_failed_preview_stops_the_previous_player() {
    let (sender, receiver) = mpsc::channel();
    let (players, create_player) = recording_players(&sender, Command::Play("second".to_string()));
    let fetches = Arc::new(AtomicUsize::new(0));

    let fetch = move |_url: &str| {
        if fetches.fetch_add(1, Ordering::SeqCst) == 0 {
            Ok(())
        } else {
            Err(AppleMusicError::new("the preview download failed"))
        }
    };

    sender.send(Command::Play("first".to_string())).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    let players = players.lock().unwrap();
    assert_eq!(
        players.len(),
        1,
        "a failed download must not install a player"
    );
    assert_eq!(players[0].appended.load(Ordering::SeqCst), 1);
    assert!(
        players[0].stopped.load(Ordering::SeqCst),
        "the failed preview must stop the previous player"
    );
}

// A successful Play installs a fresh player for the new track (rather than
// queueing it behind the old one) and starts it at the remembered gain, so
// a volume set while a previous track played carries to the next.
// The second selection is queued from inside `create_player`, once the
// first player has been installed, so the two downloads do not coalesce
// into one: this pins the replacement itself, not the supersession path.
#[test]
fn a_new_preview_replaces_the_previous_player_at_the_remembered_volume() {
    let (sender, receiver) = mpsc::channel();
    let (players, create_player) = recording_players(&sender, Command::Play("second".to_string()));
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };

    sender.send(Command::SetVolume(0.4)).unwrap();
    sender.send(Command::Play("first".to_string())).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    let players = players.lock().unwrap();
    assert_eq!(players.len(), 2, "each successful Play installs a player");
    assert_eq!(*players[0].volume.lock().unwrap(), 0.4);
    assert_eq!(*players[1].volume.lock().unwrap(), 0.4);
}

// Pause, Resume, SetVolume, SetBalance and Stop drive the player a
// previous Play installed; their `if let Some(player)` arms are the only
// way those commands reach the output. The commands are sent from inside
// the first `create_player` call, once that player is recorded, so they
// arrive in order after the Play installs it. The final Pause proves Stop
// dropped the player instead of leaving it for later commands: it must not
// reach the stopped player.
#[test]
fn transport_volume_and_balance_commands_drive_the_installed_player() {
    let (sender, receiver) = mpsc::channel();
    let players: Arc<Mutex<Vec<RecordingPlayer>>> = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&players);
    let command_sender = Arc::new(Mutex::new(Some(sender.clone())));
    let create_player = move || {
        let player = RecordingPlayer::default();
        recorded.lock().unwrap().push(player.clone());
        if let Some(sender) = command_sender.lock().unwrap().take() {
            sender.send(Command::Pause).unwrap();
            sender.send(Command::Resume).unwrap();
            sender.send(Command::SetVolume(0.6)).unwrap();
            sender.send(Command::SetBalance(0.3)).unwrap();
            sender.send(Command::SeekBy(-5)).unwrap();
            sender.send(Command::Stop).unwrap();
            sender.send(Command::Pause).unwrap();
        }
        player
    };
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };

    sender.send(Command::Play("first".to_string())).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    let players = players.lock().unwrap();
    assert_eq!(players.len(), 1, "the Play must install one player");
    assert_eq!(players[0].paused.load(Ordering::SeqCst), 1);
    assert_eq!(players[0].resumed.load(Ordering::SeqCst), 1);
    assert_eq!(*players[0].volume.lock().unwrap(), 0.6);
    assert_eq!(*players[0].balance.lock().unwrap(), 0.3);
    assert_eq!(*players[0].seeks.lock().unwrap(), vec![-5]);
    assert!(
        players[0].stopped.load(Ordering::SeqCst),
        "Stop must stop the installed player"
    );
}

// A seek reaches only the player a previous Play installed: with none
// installed yet it must not create one, and after Stop drops the player
// it must not reach the stopped one. The first SeekBy arrives before the
// Play; Stop and the second SeekBy are sent once the player is installed,
// so the recording player's empty seek list proves neither went through.
#[test]
fn a_seek_while_stopped_reaches_no_player() {
    let (sender, receiver) = mpsc::channel();
    let players: Arc<Mutex<Vec<RecordingPlayer>>> = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&players);
    let command_sender = Arc::new(Mutex::new(Some(sender.clone())));
    let create_player = move || {
        let player = RecordingPlayer::default();
        recorded.lock().unwrap().push(player.clone());
        if let Some(sender) = command_sender.lock().unwrap().take() {
            sender.send(Command::Stop).unwrap();
            sender.send(Command::SeekBy(5)).unwrap();
        }
        player
    };
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };

    sender.send(Command::SeekBy(-5)).unwrap();
    sender.send(Command::Play("first".to_string())).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    let players = players.lock().unwrap();
    assert_eq!(players.len(), 1, "only the Play may create a player");
    assert!(
        players[0].seeks.lock().unwrap().is_empty(),
        "a seek with no installed player must not reach one"
    );
    assert!(
        players[0].stopped.load(Ordering::SeqCst),
        "Stop must drop the installed player"
    );
}

// A position query with a player installed must answer from that player.
// The Position command follows the Play in the channel, so the worker
// handles it after the Play's download installs the player.
#[test]
fn position_query_returns_the_loaded_players_position() {
    let (sender, receiver) = mpsc::channel();
    let (players, create_player) = recording_players_without_queue();
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };
    let (reply, replies) = mpsc::channel();

    sender.send(Command::Play("first".to_string())).unwrap();
    sender.send(Command::Position(reply)).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    assert_eq!(
        players.lock().unwrap().len(),
        1,
        "the Play installs a player"
    );
    assert_eq!(
        replies.recv().unwrap(),
        Some(Duration::from_secs(42)),
        "the query must answer with the installed player's position"
    );
}

// A seek must move the player, so the next position query reports the new
// position rather than the one before it. SeekBy follows the Play in the
// channel, and the Position query follows the seek.
#[test]
fn a_seek_is_reflected_by_the_next_position_query() {
    let (sender, receiver) = mpsc::channel();
    let (players, create_player) = recording_players_without_queue();
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };
    let (reply, replies) = mpsc::channel();

    sender.send(Command::Play("first".to_string())).unwrap();
    sender.send(Command::SeekBy(5)).unwrap();
    sender.send(Command::Position(reply)).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    assert_eq!(*players.lock().unwrap()[0].seeks.lock().unwrap(), vec![5]);
    assert_eq!(
        replies.recv().unwrap(),
        Some(Duration::from_secs(47)),
        "the query must report the position the seek moved the player to"
    );
}

// An absolute seek must move the loaded player to that exact position, so the
// next position query reports it. SeekTo follows the Play in the channel, and
// the Position query follows the seek.
#[test]
fn seek_to_command_reaches_the_loaded_player() {
    let (sender, receiver) = mpsc::channel();
    let (players, create_player) = recording_players_without_queue();
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };
    let (reply, replies) = mpsc::channel();

    sender.send(Command::Play("first".to_string())).unwrap();
    sender
        .send(Command::SeekTo(Duration::from_secs(10)))
        .unwrap();
    sender.send(Command::Position(reply)).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    assert_eq!(
        *players.lock().unwrap()[0].absolute_seeks.lock().unwrap(),
        vec![Duration::from_secs(10)]
    );
    assert_eq!(
        replies.recv().unwrap(),
        Some(Duration::from_secs(10)),
        "the query must report the position the absolute seek moved to"
    );
}

// A duration query with a player installed answers from that player's own
// preview length.
#[test]
fn duration_query_returns_the_loaded_players_duration() {
    let (sender, receiver) = mpsc::channel();
    let (_, create_player) = recording_players_without_queue();
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };
    let (reply, replies) = mpsc::channel();

    sender.send(Command::Play("first".to_string())).unwrap();
    sender.send(Command::Duration(reply)).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    assert_eq!(
        replies.recv().unwrap(),
        Some(Duration::from_secs(30)),
        "the query must answer with the installed player's duration"
    );
}

// With no player installed, a duration query reports `None` instead of
// creating a player or inventing a length.
#[test]
fn duration_query_reports_none_when_nothing_is_loaded() {
    assert_unloaded_query_reports_none("duration query", Command::Duration);
}

// Pause must not unload the player: the worker keeps it installed and
// paused, so the next poll reads its own frozen clock instead of reporting
// that no preview is loaded.
#[test]
fn a_pause_keeps_the_loaded_player_reporting_its_position() {
    let (sender, receiver) = mpsc::channel();
    let (players, create_player) = recording_players_without_queue();
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };
    let (reply, replies) = mpsc::channel();

    sender.send(Command::Play("first".to_string())).unwrap();
    sender.send(Command::Pause).unwrap();
    sender.send(Command::Position(reply)).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    let players = players.lock().unwrap();
    assert_eq!(players[0].paused.load(Ordering::SeqCst), 1);
    assert!(
        !players[0].stopped.load(Ordering::SeqCst),
        "Pause must not stop the player"
    );
    assert_eq!(
        replies.recv().unwrap(),
        Some(Duration::from_secs(42)),
        "a paused player must still report its position"
    );
}

// With no player installed, a position query reports `None` instead of
// creating a player or inventing a zero position.
#[test]
fn position_query_reports_none_when_nothing_is_loaded() {
    assert_unloaded_query_reports_none("position query", Command::Position);
}

// A format query with a player installed must answer from that player, the
// same way a position query does.
#[test]
fn playback_format_query_returns_the_loaded_players_format() {
    let (sender, receiver) = mpsc::channel();
    let (_, create_player) = recording_players_without_queue();
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };
    let (reply, replies) = mpsc::channel();

    sender.send(Command::Play("first".to_string())).unwrap();
    sender.send(Command::PlaybackFormat(reply)).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    assert_eq!(
        replies.recv().unwrap(),
        Some(PlaybackFormat {
            sample_rate_hz: 44_100,
            channels: 2,
            bitrate_kbps: Some(256),
        }),
        "the query must answer with the installed player's format"
    );
}

// With no player installed, a format query reports `None` instead of
// inventing a sample rate for a preview that is not loaded.
#[test]
fn playback_format_query_reports_none_when_nothing_is_loaded() {
    assert_unloaded_query_reports_none("format query", Command::PlaybackFormat);
}

// The Rodio player's query guard reports its meter only once a preview is
// loaded, so a player that has not appended one cannot answer with an old
// window left behind by a previous preview. This drives the real
// `RodioPlayer::spectrum` against its format cell, not a stand-in for the
// guard.
#[test]
fn loaded_player_reports_its_meter_but_an_unloaded_one_reports_nothing() {
    let meter = Arc::new(SpectrumMeter::default());
    meter.set_sample_rate(44_100);
    for sample in sine_samples(1_000.0, 44_100, SPECTRUM_WINDOW) {
        meter.push(sample, 1);
    }

    let player = RodioPlayer::without_device(Arc::clone(&meter)).0;
    assert_eq!(
        player.spectrum(),
        None,
        "a player that has not appended a preview must not report the window"
    );

    player.mark_loaded_for_test();
    let loaded = player
        .spectrum()
        .expect("a loaded player must report its meter's spectrum");
    assert!(
        loaded.bands[4] > 0.9,
        "the loaded player must report the recorded sine"
    );
}

// The player's append path must feed its meter from the source it actually
// appends: a hand-filled meter would pass the guard test above even if
// production stopped wrapping the preview in `MeterSource`.
#[test]
fn append_records_the_appended_source_through_the_players_meter() {
    let meter = Arc::new(SpectrumMeter::default());
    let (player, mut queue) = RodioPlayer::without_device(Arc::clone(&meter));
    let sine = sine_samples(1_000.0, 44_100, SPECTRUM_WINDOW);
    let buffer = rodio::buffer::SamplesBuffer::new(
        ChannelCount::new(1).unwrap(),
        SampleRate::new(44_100).unwrap(),
        sine.clone(),
    );

    player.append_source(buffer, 0);
    assert_eq!(
        player.spectrum(),
        Some(Spectrum {
            bands: [0.0; BAND_COUNT]
        }),
        "the meter stays empty until the appended source is pulled"
    );

    // Pull the appended sound through the player's queue, as the output
    // device would, so the meter records exactly the samples production plays.
    queue.by_ref().take(sine.len()).for_each(drop);

    let spectrum = player
        .spectrum()
        .expect("an appended preview must report a spectrum");
    assert!(
        spectrum.bands[4] > 0.9,
        "the appended sine must reach the player's own meter, got {spectrum:?}"
    );
}

// The production player's finished state must come from rodio's own queue:
// the format cell alone is set by an append, and `empty` alone also holds
// before a preview is ever queued. This drives the real
// `RodioPlayer::is_finished` over a real `rodio::Player`.
#[test]
fn rodio_player_reports_finished_only_after_its_appended_preview_drains() {
    let (player, mut queue) = RodioPlayer::without_device(Arc::new(SpectrumMeter::default()));
    assert!(
        !player.is_finished(),
        "a player that has appended nothing must not report a finished preview"
    );

    let samples = sine_samples(1_000.0, 44_100, 16);
    let total = samples.len();
    let buffer = rodio::buffer::SamplesBuffer::new(
        ChannelCount::new(1).unwrap(),
        SampleRate::new(44_100).unwrap(),
        samples,
    );
    player.append_source(buffer, 0);
    assert!(
        !player.is_finished(),
        "a preview that is still queued must not report finished"
    );

    // Pull every sample but leave the source unpolled for its end, so the
    // queue still holds the preview even though its audio is exhausted.
    queue.by_ref().take(total).for_each(drop);
    assert!(
        !player.is_finished(),
        "a preview whose end has not been observed must not report finished"
    );

    // The output device polls once more, which is when rodio observes the
    // source ending and decrements its queued-sound count.
    let _ = queue.next();
    assert!(
        player.is_finished(),
        "a drained appended preview must report finished"
    );
}

// A paused preview has not reached its end, so the production player must not
// report it as finished even though no more samples are being pulled.
#[test]
fn a_paused_preview_reports_unfinished() {
    let (player, _queue) = RodioPlayer::without_device(Arc::new(SpectrumMeter::default()));
    let buffer = rodio::buffer::SamplesBuffer::new(
        ChannelCount::new(1).unwrap(),
        SampleRate::new(44_100).unwrap(),
        sine_samples(1_000.0, 44_100, 16),
    );

    player.append_source(buffer, 0);
    player.pause();

    assert!(!player.is_finished());
}

// A spectrum query with a player installed must answer from that player, the
// same way a format query does.
#[test]
fn spectrum_query_returns_the_loaded_players_spectrum() {
    let (sender, receiver) = mpsc::channel();
    let (_, create_player) = recording_players_without_queue();
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };
    let (reply, replies) = mpsc::channel();

    sender.send(Command::Play("first".to_string())).unwrap();
    sender.send(Command::Spectrum(reply)).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    assert_eq!(
        replies.recv().unwrap(),
        Some(test_spectrum()),
        "the query must answer with the installed player's spectrum"
    );
}

// A finished query answers from the installed player, so the UI observes a
// preview's natural end without reading the worker's state itself.
#[test]
fn finished_query_returns_the_loaded_players_finished_state() {
    for finished in [true, false] {
        let (sender, receiver) = mpsc::channel();
        let player = RecordingPlayer::default();
        player.finished.store(finished, Ordering::SeqCst);
        let installed = player.clone();
        let create_player = move || installed.clone();
        let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };
        let (reply, replies) = mpsc::channel();

        sender.send(Command::Play("first".to_string())).unwrap();
        sender.send(Command::Finished(reply)).unwrap();
        drop(sender);
        serve_commands(receiver, create_player, fetch);

        assert_eq!(
            replies.recv().unwrap(),
            Some(finished),
            "the query must answer with the installed player's finished state"
        );
    }
}

// With no player installed, a finished query reports `None` instead of
// claiming a preview that is not loaded has ended.
#[test]
fn finished_query_reports_none_when_nothing_is_loaded() {
    assert_unloaded_query_reports_none("finished query", Command::Finished);
}

// With no player installed, a spectrum query reports `None` instead of
// inventing bars for a preview that is not loaded.
#[test]
fn spectrum_query_reports_none_when_nothing_is_loaded() {
    assert_unloaded_query_reports_none("spectrum query", Command::Spectrum);
}

// Stop drops the loaded player, so a later spectrum query cannot answer with
// the stopped preview's bars.
#[test]
fn stop_removes_the_player_so_a_spectrum_query_reports_none() {
    let (sender, receiver) = mpsc::channel();
    let (_, create_player) = recording_players_without_queue();
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };
    let (reply, replies) = mpsc::channel();

    sender.send(Command::Play("first".to_string())).unwrap();
    sender.send(Command::Stop).unwrap();
    sender.send(Command::Spectrum(reply)).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    assert_eq!(replies.recv().unwrap(), None);
}

// Stop drops the loaded player, so a later finished query cannot report the
// stopped preview as having reached its end. Stop and the query are sent from
// inside the first `create_player` call, once that player is recorded, so they
// arrive after the Play installs it; were the player still installed, the
// finished query would answer with its state instead of `None`.
#[test]
fn stop_removes_the_player_so_a_finished_query_reports_none() {
    let (sender, receiver) = mpsc::channel();
    let players: Arc<Mutex<Vec<RecordingPlayer>>> = Arc::new(Mutex::new(Vec::new()));
    let recorded = Arc::clone(&players);
    let (reply, replies) = mpsc::channel();
    let queued = Arc::new(Mutex::new(Some((
        sender.clone(),
        vec![Command::Stop, Command::Finished(reply)],
    ))));
    let create_player = move || {
        let player = RecordingPlayer::default();
        recorded.lock().unwrap().push(player.clone());
        if let Some((sender, commands)) = queued.lock().unwrap().take() {
            for command in commands {
                sender.send(command).unwrap();
            }
        }
        player
    };
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };

    sender.send(Command::Play("first".to_string())).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    let players = players.lock().unwrap();
    assert_eq!(players.len(), 1, "the Play must install one player");
    assert!(
        players[0].stopped.load(Ordering::SeqCst),
        "Stop must stop the installed player"
    );
    assert_eq!(replies.recv().unwrap(), None);
}

// A selection whose preview fails to install leaves no player behind, so
// the following position query reports `None` rather than a stale position
// for audio that is not playing. The second selection and the query are
// queued from inside the first player's creation, and both senders are
// dropped before the loop drains them, so it ends after answering.
#[test]
fn a_failed_preview_leaves_no_position_to_report() {
    let (sender, receiver) = mpsc::channel();
    let (reply, replies) = mpsc::channel();
    let queued = Arc::new(Mutex::new(Some((sender.clone(), reply))));
    let create_player = move || {
        let player = RecordingPlayer::default();
        if let Some((sender, reply)) = queued.lock().unwrap().take() {
            sender.send(Command::Play("second".to_string())).unwrap();
            sender.send(Command::Position(reply)).unwrap();
        }
        player
    };
    let fetches = Arc::new(AtomicUsize::new(0));
    let fetch = move |_url: &str| {
        if fetches.fetch_add(1, Ordering::SeqCst) == 0 {
            Ok(())
        } else {
            Err(AppleMusicError::new("the preview download failed"))
        }
    };

    sender.send(Command::Play("first".to_string())).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    assert_eq!(
        replies.recv().unwrap(),
        None,
        "a failed preview must leave no position to report"
    );
}

// A balance set while stopped must carry to the player a later Play
// creates, just as the remembered volume does.
#[test]
fn a_new_preview_starts_at_the_remembered_balance() {
    let (sender, receiver) = mpsc::channel();
    let (players, create_player) = recording_players(&sender, Command::Play("second".to_string()));
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };

    sender.send(Command::SetBalance(-0.4)).unwrap();
    sender.send(Command::Play("first".to_string())).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    let players = players.lock().unwrap();
    assert_eq!(players.len(), 2, "each successful Play installs a player");
    assert_eq!(*players[0].balance.lock().unwrap(), -0.4);
    assert_eq!(*players[1].balance.lock().unwrap(), -0.4);
}

// An equalizer curve set while stopped must carry to the player a later
// Play creates, just as the remembered volume and balance do.
#[test]
fn a_new_preview_starts_at_the_remembered_equalizer() {
    let (sender, receiver) = mpsc::channel();
    let (players, create_player) = recording_players(&sender, Command::Play("second".to_string()));
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };
    let equalizer = EqSnapshot {
        enabled: true,
        preamp_db: 3.0,
        bands_db: [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0],
    };

    sender.send(Command::SetEqualizer(equalizer)).unwrap();
    sender.send(Command::Play("first".to_string())).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    let players = players.lock().unwrap();
    assert_eq!(players.len(), 2, "each successful Play installs a player");
    assert_eq!(*players[0].equalizer.lock().unwrap(), equalizer);
    assert_eq!(*players[1].equalizer.lock().unwrap(), equalizer);
}

// An equalizer curve set during playback must reach the player the current
// Play installed, not just the players later Plays create. The command is
// queued from the player factory, so it is handled after the Play installs
// its player.
#[test]
fn an_equalizer_set_while_playing_updates_the_installed_player() {
    let (sender, receiver) = mpsc::channel();
    let equalizer = EqSnapshot {
        enabled: true,
        preamp_db: -2.0,
        bands_db: [0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, -3.0],
    };
    let (players, create_player) = recording_players(&sender, Command::SetEqualizer(equalizer));
    let fetch = |_url: &str| -> Result<(), AppleMusicError> { Ok(()) };

    sender.send(Command::Play("first".to_string())).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    let players = players.lock().unwrap();
    assert_eq!(players.len(), 1, "the Play must install one player");
    assert_eq!(
        *players[0].equalizer.lock().unwrap(),
        equalizer,
        "SetEqualizer must update the player already installed"
    );
}

// A Play's download runs on the worker, so commands issued meanwhile wait
// in the channel. If a newer Play arrives during the download, the UI has
// already committed the newer selection, and installing the older preview
// would play the wrong audio under the committed track's name. The worker
// must drop the superseded preview and handle the queued selection.
#[test]
fn a_preview_superseded_during_its_download_is_not_installed() {
    // "first" is still downloading when the user selects "second", so the
    // second Play is sent from inside the first fetch.
    let (players, fetched) = run_with_superseding_command(Command::Play("second".to_string()));

    assert_eq!(
        fetched,
        vec!["first".to_string(), "second".to_string()],
        "the superseded download still ran; only its preview is dropped"
    );
    assert_eq!(
        players.len(),
        1,
        "only the newer selection may install a player"
    );
    assert_eq!(
        players[0].appended.load(Ordering::SeqCst),
        1,
        "only the newer preview may be appended"
    );
}

// Not every command that arrives during a download supersedes it: Pause,
// Resume and SetVolume all act on the player the preview installs. The
// loop drains them into `queued` and handles them only after installing
// the preview, since before that there is no player to receive them. A
// drain that dropped them (or applied them ahead of the install) would
// silently lose the user's volume or transport action, so this pins that
// they reach the new player in the order they arrived.
#[test]
fn commands_queued_during_a_download_apply_to_the_new_player() {
    let (sender, receiver) = mpsc::channel();
    let fetched: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let (players, create_player) = recording_players_without_queue();
    // The commands are sent from inside the fetch, while the preview is
    // still downloading, so they are waiting in the channel when the loop
    // drains it. The sender clone is taken (and so dropped) with the
    // sends, letting the channel close and the worker loop end.
    let fetch_sender = Arc::new(Mutex::new(Some(sender.clone())));
    let seen = Arc::clone(&fetched);
    let fetch = move |url: &str| -> Result<(), AppleMusicError> {
        seen.lock().unwrap().push(url.to_string());
        if let Some(sender) = fetch_sender.lock().unwrap().take() {
            sender.send(Command::SetVolume(0.3)).unwrap();
            sender.send(Command::Pause).unwrap();
            sender.send(Command::Resume).unwrap();
        }
        Ok(())
    };

    sender.send(Command::Play("first".to_string())).unwrap();
    drop(sender);
    serve_commands(receiver, create_player, fetch);

    assert_eq!(
        *fetched.lock().unwrap(),
        vec!["first".to_string()],
        "the queued commands must not supersede the download"
    );
    let players = players.lock().unwrap();
    assert_eq!(players.len(), 1, "the download must install one player");
    assert_eq!(players[0].appended.load(Ordering::SeqCst), 1);
    assert_eq!(
        *players[0].volume.lock().unwrap(),
        0.3,
        "the SetVolume queued during the download must reach the new player"
    );
    assert_eq!(
        players[0].paused.load(Ordering::SeqCst),
        1,
        "the Pause queued during the download must reach the new player"
    );
    assert_eq!(
        players[0].resumed.load(Ordering::SeqCst),
        1,
        "the Resume queued during the download must reach the new player"
    );
    assert_eq!(
        *players[0].calls.lock().unwrap(),
        vec!["volume", "append", "volume", "pause", "resume"],
        "the queued commands must reach the new player after the install, in arrival order"
    );
}

// The supersession check above names `Command::Stop` as well as
// `Command::Play`: a Stop that arrives while a Play is downloading means
// the user has already silenced playback, so installing the
// just-downloaded preview would start audio they asked to stop. The queued
// Stop must drop the in-flight preview instead of installing it (it is then
// a no-op, since no player exists). A regression that checked only
// `Command::Play` would install a player and append its preview here, so
// this is the case that pins the Stop half of the check.
#[test]
fn a_preview_superseded_by_stop_during_its_download_is_not_installed() {
    // "first" is still downloading when the user presses Stop, so the Stop
    // is sent from inside the fetch.
    let (players, fetched) = run_with_superseding_command(Command::Stop);

    assert_eq!(
        fetched,
        vec!["first".to_string()],
        "the superseded download still ran; only its preview is dropped"
    );
    assert!(
        players.is_empty(),
        "a preview superseded by Stop must not install a player"
    );
}
