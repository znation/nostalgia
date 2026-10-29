use crate::ui::player::CurrentView;
use crate::ui::{message::Message, player::WinampPlayer, views};
use iced::Element;
use iced::widget::Column;

pub(super) fn view_library<'a>(
    player: &'a WinampPlayer,
    footer: Element<'a, Message>,
) -> Element<'a, Message> {
    let main_content = match &player.current_view {
        CurrentView::Artists => views::view_artists(
            &player.artists.items,
            &player.artists.labels,
            player.artists.epoch,
            player.artists.selected(),
            player.artists.loading,
            player.artists.error.as_deref(),
        ),
        CurrentView::Albums => views::view_albums(
            &player.albums.items,
            &player.albums.titles,
            &player.albums.labels,
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
        // rows with no current-track marker, drawing the cached title and
        // running time the load stored.
        CurrentView::AllAlbums => views::view_all_albums(
            &player.all_albums.items,
            &player.all_albums.titles,
            &player.all_albums.labels,
            player.all_albums.epoch,
            player.all_albums.selected(),
            player.all_albums.loading,
            player.all_albums.error.as_deref(),
        ),
        CurrentView::RecentlyAdded => views::view_recently_added(
            &player.recently_added.items,
            &player.recently_added.titles,
            &player.recently_added.labels,
            player.recently_added.epoch,
            player.recently_added.selected(),
            player.recently_added.loading,
            player.recently_added.error.as_deref(),
        ),
        // Like All Songs, the flat recently played list marks its playing row.
        CurrentView::RecentlyPlayed => {
            let state = player.state.blocking_lock();
            views::view_recently_played(
                &player.recently_played.items,
                &player.recently_played.titles,
                &player.recently_played.labels,
                player.recently_played.epoch,
                player.recently_played.selected(),
                player.recently_played.loading,
                player.recently_played.error.as_deref(),
                state.current_track.as_deref(),
            )
        }
        CurrentView::Genres => views::view_genres(
            &player.genres.items,
            &player.genres.labels,
            player.genres.epoch,
            player.genres.selected(),
            player.genres.loading,
            player.genres.error.as_deref(),
        ),
        CurrentView::Years => views::view_years(
            &player.years.items,
            &player.years.labels,
            player.years.epoch,
            player.years.selected(),
            player.years.loading,
            player.years.error.as_deref(),
        ),
        CurrentView::Playlists => views::view_playlists(
            &player.playlists.items,
            player.playlists.epoch,
            player.playlists.selected(),
            player.playlists.loading,
            player.playlists.error.as_deref(),
        ),
    };

    let mut library = Column::new().push(views::view_title_band("NOSTALGIA LIBRARY"));
    // The Back button sits above the list it navigates and exists only where
    // the hierarchy has a level above to return to (see `views::can_go_back`).
    if views::can_go_back(&player.current_view) {
        library = library.push(views::view_back_button());
    }
    // A failed top-level artists fetch is a dead end: no navigation re-issues
    // it, so the Artists view offers a Retry that re-runs `LoadArtists`.
    // Every other browse level recovers by navigating back into it, so those
    // failures need no button.
    if views::can_retry_artists(&player.current_view, player.artists.error.as_deref()) {
        library = library.push(views::view_retry_button());
    }
    // The Top, Sort, and Reload buttons share one row, Reload beside Sort, so
    // the current list can always be returned to its first row, reordered in
    // place, or replaced with a freshly fetched artists list. Open sits above
    // them, on every browse level, and opens the row a first click selected;
    // with nothing selected it does nothing.
    library = library.push(views::view_open_button());
    library = library.push(views::view_browse_nav_buttons());
    // The All Songs, All Albums, Playlists, Recently Added, and Recently Played buttons open the
    // flat lists; they live on the Artists view, beside Back/Retry, so each list is one press from
    // the top of the hierarchy.
    if matches!(player.current_view, CurrentView::Artists) {
        library = library
            .push(views::view_all_songs_button())
            .push(views::view_all_albums_button())
            .push(views::view_playlists_button())
            .push(views::view_genres_button())
            .push(views::view_years_button())
            .push(views::view_recently_added_button())
            .push(views::view_recently_played_button());
    }
    // The search box and its Clear button sit on every browse screen, so a
    // query can be typed from (and cleared at) any level of the hierarchy.
    library = library.push(views::view_search_row(&player.search_query));
    library.push(main_content).push(footer).into()
}
