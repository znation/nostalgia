//! The root screen assembly: the title bar, Now Playing bar, seek strip,
//! transport row, the player's EQ-button-controlled equalizer section, and the
//! browse level stacked into the tree `init_ui` hands to iced. Widget
//! construction itself lives in `views`.

use iced::{
    Element,
    widget::{Column, MouseArea},
};

use super::message::Message;
use super::player::{CurrentView, WinampPlayer};
use super::views;

/// Assembles the app screen: the title bar, Now Playing bar, seek strip,
/// transport row, and — while [`WinampPlayer::eq_panel_visible`] holds — the
/// equalizer band and panel, all built in `views` from the resolved title and
/// the shared playback, volume, Repeat, Shuffle, and EQ state — above the
/// current browse list, with the compact library transport footer beneath it.
/// Hiding the equalizer section moves the library band and browse list up into
/// the freed height; nothing resizes the window. When the window is shaded,
/// the tree is the title bar over the compact transport strip and the rest of
/// the screen is dropped. The strip's time readout publishes the same
/// [`Message::ToggleTimeDisplay`] as the hidden LCD's, so the elapsed/remaining
/// choice stays switchable in shade mode.
pub(in crate::ui) fn view(player: &WinampPlayer) -> Element<'_, Message> {
    // Shade mode builds only the title bar and its compact transport strip —
    // the Now Playing bar, seek strip, transport row, equalizer, and browse
    // list are all dropped, and the window itself is resized down to that
    // pair. The short state lock resolves only what the strip draws, so the
    // rolled-up frame stays cheap.
    if player.shaded {
        let (transport_mode, volume, elapsed, total) = {
            let state = player.state.blocking_lock();
            (
                state.transport_mode(),
                state.volume(),
                player.now_playing_elapsed(state.current_track.as_deref()),
                player.now_playing_time(state.current_track.as_deref()),
            )
        };
        return Column::new()
            .push(
                MouseArea::new(
                    Column::new()
                        .push(views::view_title_bar(player.always_on_top))
                        .push(views::view_shade_strip(
                            transport_mode,
                            volume,
                            elapsed,
                            total,
                        )),
                )
                .on_scroll(views::volume_wheel_message),
            )
            .into();
    }

    // The now-playing title, artist, playlist number, elapsed time, and total
    // are resolved while the state lock is held, from a borrowed `current_track`
    // against the accumulated `known_tracks` index (see
    // [`WinampPlayer::now_playing_label`], [`WinampPlayer::now_playing_artist`],
    // [`WinampPlayer::now_playing_number`],
    // [`WinampPlayer::now_playing_elapsed`], and
    // [`WinampPlayer::now_playing_time`]). The label, artist, number, and
    // elapsed placeholder borrow from `known_tracks` (or the `"Nothing"`/`""`/
    // `""`/`"00:00"`/`"--:--"` literals); the total borrows the cached length,
    // while a reported elapsed position allocates its formatted `MM:SS` string.
    let (
        now_playing,
        now_playing_artist,
        now_playing_number,
        now_playing_elapsed,
        now_playing_time,
        playback_format,
        spectrum,
        transport_mode,
        is_playing,
        volume,
        balance,
        repeat,
        shuffle,
        eq_enabled,
        eq_preamp,
        eq_bands,
        eq_preset,
    ) = {
        let state = player.state.blocking_lock();
        (
            player.now_playing_label(state.current_track.as_deref()),
            player.now_playing_artist(state.current_track.as_deref()),
            player.now_playing_number(state.current_track.as_deref()),
            player.now_playing_elapsed(state.current_track.as_deref()),
            player.now_playing_time(state.current_track.as_deref()),
            player.playback_format,
            player.spectrum,
            state.transport_mode(),
            state.is_playing,
            state.volume(),
            state.balance(),
            state.repeat,
            state.shuffle,
            state.eq_enabled,
            state.eq_preamp(),
            state.eq_bands(),
            state.eq_preset(),
        )
    };

    let main_content = match &player.current_view {
        CurrentView::Artists => views::view_artists(
            &player.artists.items,
            player.artists.epoch,
            player.artists.selected(),
            player.artists.loading,
            player.artists.error.as_deref(),
        ),
        CurrentView::Albums => views::view_albums(
            &player.albums.items,
            player.albums.epoch,
            player.albums.selected(),
            player.albums.loading,
            player.albums.error.as_deref(),
        ),
        // A second short lock (like the label block above) hands the current
        // track's id to the Songs view so it can mark the playing row. The
        // borrowed id is compared inside `song_row` and the guard drops at
        // the end of this arm, so no per-frame `Option<String>` clone is
        // needed.
        CurrentView::Songs => {
            let state = player.state.blocking_lock();
            views::view_songs(
                &player.songs.items,
                &player.songs.titles,
                &player.songs.labels,
                player.songs.epoch,
                player.songs.selected(),
                player.songs.loading,
                player.songs.error.as_deref(),
                state.current_track.as_deref(),
                player.search_active,
            )
        }
        // The flat whole-library view marks its playing row from the same
        // second lock the Songs arm takes.
        CurrentView::AllSongs => {
            let state = player.state.blocking_lock();
            views::view_all_songs(
                &player.all_songs.items,
                &player.all_songs.titles,
                &player.all_songs.labels,
                player.all_songs.epoch,
                player.all_songs.selected(),
                player.all_songs.loading,
                player.all_songs.error.as_deref(),
                state.current_track.as_deref(),
            )
        }
        // The flat album list renders like the per-artist Albums view: album
        // rows with no current-track marker.
        CurrentView::AllAlbums => views::view_all_albums(
            &player.all_albums.items,
            player.all_albums.epoch,
            player.all_albums.selected(),
            player.all_albums.loading,
            player.all_albums.error.as_deref(),
        ),
        CurrentView::Playlists => views::view_playlists(
            &player.playlists.items,
            player.playlists.epoch,
            player.playlists.selected(),
            player.playlists.loading,
            player.playlists.error.as_deref(),
        ),
    };

    // The player panels are one `MouseArea` so a wheel roll over any of them
    // nudges the volume; its content still sees each event first, so the
    // title-bar drag, LCD click, sliders, and buttons keep their behavior.
    // The library band and everything below stay outside the area, leaving
    // the browse list's own `Scrollable` to take the wheel without touching
    // the volume.
    //
    // The footer gets its own clones of the elapsed and total strings before
    // the LCD takes them, so the two can never disagree about elapsed versus
    // remaining mode or about the track's total.
    let library_elapsed = now_playing_elapsed.clone();
    let library_total = now_playing_time.clone();
    let mut player_panels = Column::new()
        .push(views::view_title_bar(player.always_on_top))
        .push(views::view_now_playing(
            now_playing_elapsed,
            now_playing,
            now_playing_artist,
            now_playing_number,
            now_playing_time,
            player.marquee_offset,
            playback_format,
            views::visible_spectrum(spectrum, player.visualizer_visible),
            transport_mode,
            player.blink_visible,
        ))
        .push(views::view_seek_bar(
            player.readout_position(),
            player.playback_duration,
        ))
        .push(views::view_transport_controls(
            is_playing,
            volume,
            balance,
            repeat,
            shuffle,
            player.eq_panel_visible,
        ));
    // The EQ button on the transport row is layout-only: it shows or hides this
    // band and panel, while the shared EQ state keeps driving the audio. The
    // freed height flows to the browse list below because nothing here pins the
    // column's height or resizes the window.
    if player.eq_panel_visible {
        player_panels = player_panels
            .push(views::view_title_band("NOSTALGIA EQUALIZER"))
            .push(views::view_equalizer(
                eq_enabled, eq_preamp, &eq_bands, eq_preset,
            ));
    }

    let mut column = Column::new()
        .push(MouseArea::new(player_panels).on_scroll(views::volume_wheel_message))
        .push(views::view_title_band("NOSTALGIA LIBRARY"));
    // The Back button sits above the list it navigates and exists only where
    // the hierarchy has a level above to return to (see `views::can_go_back`).
    if views::can_go_back(&player.current_view) {
        column = column.push(views::view_back_button());
    }
    // A failed top-level artists fetch is a dead end: no navigation re-issues
    // it, so the Artists view offers a Retry that re-runs `LoadArtists`.
    // Every other browse level recovers by navigating back into it, so those
    // failures need no button.
    if views::can_retry_artists(&player.current_view, player.artists.error.as_deref()) {
        column = column.push(views::view_retry_button());
    }
    // The Top and Sort buttons share one row, Sort beside Top, so the
    // current list can always be returned to its first row and reordered
    // in place. Open sits above them, on every browse level, and opens the
    // row a first click selected; with nothing selected it does nothing.
    column = column.push(views::view_open_button());
    column = column.push(views::view_browse_nav_buttons());
    // The All Songs, All Albums, and Playlists buttons open the flat lists;
    // they live on the Artists view, beside Back/Retry, so each list is one
    // press from the top of the hierarchy.
    if matches!(player.current_view, CurrentView::Artists) {
        column = column
            .push(views::view_all_songs_button())
            .push(views::view_all_albums_button())
            .push(views::view_playlists_button());
    }
    // The search box sits on every browse screen, so a query can be typed
    // from any level of the hierarchy.
    column = column.push(views::view_search_box(&player.search_query));
    column
        .push(main_content)
        .push(views::view_library_strip(
            transport_mode,
            volume,
            library_elapsed,
            library_total,
        ))
        .into()
}
