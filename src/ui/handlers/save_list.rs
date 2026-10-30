//! The `Save list` button's naming panel and the saved song lists' open path:
//! saving the showing song list's ids under a name, and resolving a saved
//! list's ids back to songs when its Playlists row opens.
//!
//! Saving mirrors the equalizer's Save preset panel (see `handlers::mixer`),
//! because the two are the same naming-panel shape over different payloads:
//! the toggle closes every other transient panel, an empty or colon-bearing
//! name is refused inline, and a same-name save replaces in place.

use iced::Task;

use crate::settings::SavedPlaylist;
use crate::ui::views::{SAVE_LIST_COLON_NAME_NOTE, SAVE_LIST_EMPTY_NAME_NOTE, SAVE_LIST_INPUT_ID};
use crate::ui::{CurrentView, Message, SongsOrigin, WinampPlayer};

use super::browse::selection::show_group_songs;
use super::browse::switch_view;
use super::window::toggle_window_shade;

/// The browse button row's `Save list` button: flip the naming panel that
/// saves the showing song list. On a view other than the shared songs list,
/// or with it empty, loading, or failed, the press is a no-op like Sort's
/// no-ops — there is no list to save. Opening closes every other transient
/// panel, starts with an empty box, and focuses and selects the name box;
/// while shaded it first restores the window. Closing only clears the flag.
/// The flag is layout-only and not persisted.
pub(in crate::ui) fn toggle_save_list(player: &mut WinampPlayer) -> Task<Message> {
    if player.save_list_open {
        player.save_list_open = false;
        return Task::none();
    }
    if !can_save_showing_list(player) {
        return Task::none();
    }
    player.close_transient_panels();
    player.save_list_open = true;
    player.save_list_name.clear();
    player.save_list_note = None;
    let focus = Task::batch([
        iced::widget::operation::focus(SAVE_LIST_INPUT_ID),
        iced::widget::operation::select_all(SAVE_LIST_INPUT_ID),
    ]);
    if player.shaded {
        Task::batch([toggle_window_shade(player), focus])
    } else {
        focus
    }
}

/// Whether the shared songs list is showing something saveable: a loaded,
/// failed-free, non-empty `Songs` view. The flat lists and every parent level
/// are out — the button saves what the rows show, and only the shared songs
/// buffer holds song rows this run assembled.
fn can_save_showing_list(player: &WinampPlayer) -> bool {
    player.current_view == CurrentView::Songs
        && !player.songs.loading
        && player.songs.error.is_none()
        && !player.songs.items.is_empty()
}

/// A keystroke in the Save list box: store the typed name and clear the
/// panel's inline note, so the next submit re-asks from a clean slate.
pub(in crate::ui) fn save_list_name_changed(
    player: &mut WinampPlayer,
    text: String,
) -> Task<Message> {
    player.save_list_name = text;
    player.save_list_note = None;
    Task::none()
}

/// Enter or the panel's `Save` button: with the panel open, save the showing
/// song list — its song ids in showing order — under the trimmed typed name
/// and close the panel; an empty name keeps the panel open with the
/// `Type a name` note, and a name containing `:` shows `Use it without a
/// colon` (the settings line's separator). On success an existing save of the
/// same name (case-insensitive) is replaced in place — its ids update and
/// its stored name as first saved stays — otherwise the list pushes onto the
/// end of the saved lists. The showing list is untouched, exactly as
/// Winamp's save clears nothing in the playlist editor. The persist runs
/// through the shared [`WinampPlayer::persist_settings_if_changed`] sweep
/// after the message, so the saved file carries the new `user_playlist=`
/// line with no dedicated write here.
pub(in crate::ui) fn save_list_submitted(player: &mut WinampPlayer) -> Task<Message> {
    if !player.save_list_open {
        return Task::none();
    }
    let name = player.save_list_name.trim();
    if name.is_empty() {
        player.save_list_note = Some(SAVE_LIST_EMPTY_NAME_NOTE);
        return Task::none();
    }
    if name.contains(':') {
        player.save_list_note = Some(SAVE_LIST_COLON_NAME_NOTE);
        return Task::none();
    }
    let song_ids = player
        .songs
        .items
        .iter()
        .map(|song| song.id.clone())
        .collect();
    if let Some(existing) = player
        .saved_playlists
        .iter_mut()
        .find(|saved| saved.name.eq_ignore_ascii_case(name))
    {
        existing.song_ids = song_ids;
    } else {
        player.saved_playlists.push(SavedPlaylist {
            name: name.to_string(),
            song_ids,
        });
    }
    player.save_list_open = false;
    player.save_list_note = None;
    Task::none()
}

/// A Playlists row whose id is the sentinel pressed once: open the saved
/// list the row names. The row carries the saved list's name, so a
/// Sort/reverse of the Playlists view cannot mis-route the press; the saved
/// list with that name — or none, when a stale press outlived its row —
/// decides. The songs level is cleared and marked in flight while the ids
/// resolve, so a press during the resolution cannot act on stale rows, and
/// the task reports one message the ordinary browse-reply path delivers,
/// with the ids resolved in saved order and the ones the library no longer
/// answers dropped.
pub(in crate::ui) fn user_playlist_selected(
    player: &mut WinampPlayer,
    row_name: &str,
) -> Task<Message> {
    let Some(song_ids) = player
        .saved_playlists
        .iter()
        .find(|saved| saved.name == row_name)
        .map(|saved| saved.song_ids.clone())
    else {
        return Task::none();
    };
    let service = player.apple_music_service.clone();
    // The twin of every browse navigation's clear-then-issue step: drop the
    // stale rows first, then issue the request's generation, so a genre or
    // album fetch in flight is superseded and stale rows cannot be pressed.
    player.songs.clear();
    let generation = player.songs.begin_fetch();
    player.songs_origin = SongsOrigin::UserPlaylist;
    switch_view(player, CurrentView::Songs);
    Task::future(async move {
        let mut songs = Vec::with_capacity(song_ids.len());
        for id in song_ids {
            if let Some(song) = service.resolve_song(&id).await {
                songs.push(song);
            }
        }
        if generation.is_current() {
            Message::BrowseReply {
                issued: generation.issued(),
                reply: Box::new(Message::UserPlaylistLoaded(songs)),
            }
        } else {
            // A newer songs request for the same level supersedes this
            // resolution, exactly as a browse fetch does.
            Message::Ignored
        }
    })
}

/// A resolved saved song list is showing: store the songs in the shared
/// songs buffer under `SongsOrigin::UserPlaylist`, exactly as a genre's or a
/// year's group stores. The rows play, sort, reverse, shuffle, and report
/// through the ordinary song-list paths from here.
pub(in crate::ui) fn user_playlist_loaded(
    player: &mut WinampPlayer,
    songs: Vec<crate::library::Song>,
) -> Task<Message> {
    show_group_songs(player, songs, SongsOrigin::UserPlaylist)
}
