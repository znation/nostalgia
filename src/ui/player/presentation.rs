//! The app model's presentation helpers: resolving what [`WinampPlayer`] holds
//! into the window title and the Now Playing bar's labels, time readouts, and
//! readout position. The views draw these; the model's own fields, construction,
//! settings persistence, and timer predicates stay in [`super`].

use std::borrow::Cow;
use std::time::Duration;

use super::WinampPlayer;
use crate::ui::views;

/// The title `init_ui` gives iced when no known track is current.
const APP_TITLE: &str = "nostalgia";

impl WinampPlayer {
    /// The current track resolved against [`Self::known_tracks`], or `None`
    /// with no current track or an unknown id. The state lock is released
    /// before this returns.
    pub(in crate::ui) fn current_known_track(&self) -> Option<&views::KnownTrack> {
        let id = self.state.blocking_lock().current_track.clone()?;
        self.known_tracks.get(&id)
    }

    /// The title iced gives the application window. It follows the Now Playing
    /// bar: the current track's number, artist, and title, resolved against
    /// [`Self::known_tracks`], then ` - nostalgia`. With no current track or an
    /// unknown id it is the plain [`APP_TITLE`]. This names the iced window
    /// (what the desktop environment shows), not the custom title bar, whose
    /// label is the fixed `NOSTALGIA`. Stop leaves `current_track` in place, so
    /// the interrupted track's name stays in the title as it does on the LCD.
    pub(in crate::ui) fn window_title(&self) -> String {
        let state = self.state.blocking_lock();
        let Some(track) = state
            .current_track
            .as_deref()
            .and_then(|id| self.known_tracks.get(id))
        else {
            return APP_TITLE.to_string();
        };
        let mut title = String::with_capacity(track.number.len() + track.artist.len() + 32);
        title.push_str(&track.number);
        if !track.artist.is_empty() {
            title.push_str(&track.artist);
            title.push_str(" - ");
        }
        title.push_str(&track.title);
        title.push_str(" - ");
        title.push_str(APP_TITLE);
        title
    }

    /// The Now Playing bar's label for `current_track`, resolved against the
    /// accumulated [`Self::known_tracks`] index rather than the currently
    /// loaded song list: `view` renders the bar from this, so the bar keeps
    /// naming a playing track even after a browse replaced the visible song
    /// buffer. The browse-away regression test asserts this same path.
    pub(in crate::ui) fn now_playing_label<'a>(
        &'a self,
        current_track: Option<&str>,
    ) -> Cow<'a, str> {
        views::now_playing_label(&self.known_tracks, current_track)
    }

    /// The Now Playing bar's artist for `current_track`, resolved against the
    /// accumulated [`Self::known_tracks`] index like
    /// [`Self::now_playing_label`]: the artist is borrowed from the index, or
    /// the `""` literal when the track is unknown or none is current, so
    /// `view` renders the artist before the title only when one is known.
    pub(in crate::ui) fn now_playing_artist<'a>(
        &'a self,
        current_track: Option<&str>,
    ) -> Cow<'a, str> {
        views::now_playing_artist(&self.known_tracks, current_track)
    }

    /// The Now Playing bar's playlist number prefix for `current_track`,
    /// resolved against the accumulated [`Self::known_tracks`] index like
    /// [`Self::now_playing_label`]: the cached `"<n>. "` is borrowed from the
    /// index, or the `""` literal when the track is unknown or none is
    /// current, so `view` draws a number only for a track selected from a row.
    pub(in crate::ui) fn now_playing_number<'a>(
        &'a self,
        current_track: Option<&str>,
    ) -> Cow<'a, str> {
        views::now_playing_number(&self.known_tracks, current_track)
    }

    /// The Now Playing bar's right-edge length readout for `current_track`: the
    /// plain total borrowed from the label cached in [`Self::known_tracks`], or
    /// the static `--:--` placeholder for a track the index does not know. A
    /// reported playback position never changes it; the big elapsed readout
    /// comes from [`Self::now_playing_elapsed`] instead. Like the label and
    /// artist, the known case borrows from the index, so `view` renders the bar
    /// without formatting or allocating a `String` per frame.
    pub(in crate::ui) fn now_playing_time<'a>(
        &'a self,
        current_track: Option<&str>,
    ) -> Cow<'a, str> {
        views::now_playing_time(&self.known_tracks, current_track)
    }

    /// The one position the seek thumb and the large elapsed readout both
    /// show: an in-progress drag wins, otherwise the position
    /// [`Self::playback_position`] reports. Resolving both through this method
    /// keeps them from drifting apart. A drag is only a preview: it never
    /// touches the backend, so clearing it falls back to the reported position.
    pub(in crate::ui) fn readout_position(&self) -> Option<Duration> {
        self.seek_preview.or(self.playback_position)
    }

    /// The Now Playing bar's large left-hand time readout for
    /// `current_track`, in the mode [`Self::time_display`] selects: the
    /// position [`Self::readout_position`] resolves as zero-padded `MM:SS` by
    /// default, or the countdown from the known track's length when the user
    /// has clicked the readout. A held seek drag therefore previews its target
    /// time in both modes. The placeholder cases borrow, so a per-frame
    /// refresh allocates only while an actual time is shown.
    pub(in crate::ui) fn now_playing_elapsed<'a>(
        &'a self,
        current_track: Option<&str>,
    ) -> Cow<'a, str> {
        views::now_playing_elapsed(
            &self.known_tracks,
            current_track,
            self.readout_position(),
            self.time_display,
        )
    }
}
