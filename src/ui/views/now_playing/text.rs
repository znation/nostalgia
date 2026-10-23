//! Pure text resolution and formatting for the Now Playing bar: the label,
//! artist, elapsed and total readouts, the marquee window, and the status
//! row's bitrate/sample-rate prefix and channel label pair. Everything here is
//! data in, string out, so it is testable without an iced renderer; the
//! widgets that draw these strings live in [`super`].

use std::borrow::Cow;
use std::collections::HashMap;
use std::time::Duration;

use crate::audio::PlaybackFormat;

/// The LCD's character budget for the combined artist/title line: sized to
/// keep an overlong line inside the roughly 550px-wide window beside the
/// large elapsed readout, the `"Now Playing: "` caption, and the total at
/// the right edge. A line that fits renders still and borrowed; a longer one
/// scrolls through [`marquee_text`].
pub(in crate::ui::views) const MARQUEE_WIDTH: usize = 22;

/// The spaces between the end of a scrolling line and its next repeat.
const MARQUEE_GAP: usize = 3;

/// The number of characters the Now Playing line occupies: the label's
/// characters, plus the artist's and the three characters of the `" - "`
/// separator that joins them.
///
/// The bar shows the line whole while it fits [`MARQUEE_WIDTH`] and windows it
/// when it does not, so this is the one measure both the view and the marquee
/// timer read — the timer runs only while the line is too long, and the view
/// windows exactly the lines the timer scrolls.
pub(in crate::ui::views) fn display_len(label: &str, artist: &str) -> usize {
    label.chars().count()
        + if artist.is_empty() {
            0
        } else {
            artist.chars().count() + 3
        }
}

/// Whether the Now Playing line is too long for the LCD's character budget
/// and so has to scroll: the combined artist and title exceed
/// [`MARQUEE_WIDTH`] characters.
///
/// The marquee timer runs only while this holds. A line that fits is shown
/// whole, so ticking for it would advance an offset the view ignores — and
/// every tick makes iced rebuild the whole screen, browse list included.
pub fn needs_marquee(label: &str, artist: &str) -> bool {
    display_len(label, artist) > MARQUEE_WIDTH
}

/// Returns the `width`-character window of `text` starting at
/// `offset % cycle`, where `cycle` is the line plus [`MARQUEE_GAP`] trailing
/// spaces. A line that fits `width` is returned unchanged, so the caller's
/// common case stays still; an overlong line wraps through the gap back to
/// its start, with a space for each position past the text's end. Counts
/// characters, not bytes, so a non-ASCII title windows on character
/// boundaries.
pub(in crate::ui::views) fn marquee_text(text: &str, offset: usize, width: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= width {
        return text.to_string();
    }
    let cycle = chars.len() + MARQUEE_GAP;
    let start = offset % cycle;
    (0..width)
        .map(|position| {
            let index = (start + position) % cycle;
            chars.get(index).copied().unwrap_or(' ')
        })
        .collect()
}

/// Which time the Now Playing LCD's large left-hand readout shows. The mode
/// is presentation-only: it changes nothing about playback or the right-edge
/// total, and the player keeps it across Pause, Stop, Next/Previous, and a new
/// selection until the readout is clicked again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimeDisplay {
    /// The elapsed playback position, the default (`00:53`).
    #[default]
    Elapsed,
    /// The time left in the track (`-02:37`).
    Remaining,
}

impl TimeDisplay {
    /// The other mode: [`Self::Elapsed`] becomes [`Self::Remaining`] and back.
    pub fn toggled(self) -> Self {
        match self {
            Self::Elapsed => Self::Remaining,
            Self::Remaining => Self::Elapsed,
        }
    }
}

/// A track the Now Playing bar knows about: the title and artist it renders
/// and the formatted length it shows beside them. Recorded once when a track
/// is played, so the bar keeps naming and timing the playing track after the
/// user browses to a different album.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownTrack {
    /// The track's title, without the artist (the bar renders
    /// [`Self::artist`] separately).
    pub title: String,
    /// The performing artist's display name; `""` means the source supplied
    /// no artist, and the Now Playing bar falls back to the title alone.
    pub artist: String,
    /// The track's formatted length ([`crate::ui::views::format_track_time`]), cached when the
    /// track is recorded so a per-frame bar refresh borrows it instead of
    /// formatting the same duration again.
    pub time: String,
    /// The track's raw length in milliseconds, the value the remaining
    /// readout subtracts the playback position from; `0` means the source
    /// supplied no duration.
    pub duration_ms: u64,
}

/// The Now Playing bar label: the title of `current_track` when `tracks`
/// knows it; the raw id when it doesn't; "Nothing" when no track is current
/// (Stop keeps the interrupted track's title). `tracks` is
/// the player's known id→track index, so this per-frame resolution is a single
/// map get rather than a scan of every song the player has ever loaded. The
/// result borrows the title from `tracks` (or the `"Nothing"` literal) in the
/// common cases, so a per-frame refresh allocates only for the rare unknown-id
/// fallback instead of cloning the title on every view. Pure data → `Cow` so
/// the title resolution is testable without an iced renderer.
pub fn now_playing_label<'a>(
    tracks: &'a HashMap<String, KnownTrack>,
    current_track: Option<&str>,
) -> Cow<'a, str> {
    match current_track {
        Some(id) => known_track_for(tracks, Some(id))
            .map(|track| Cow::Borrowed(track.title.as_str()))
            .unwrap_or_else(|| Cow::Owned(id.to_string())),
        None => Cow::Borrowed("Nothing"),
    }
}

/// Looks up the [`KnownTrack`] for `current_track` in `tracks`, returning
/// `None` when no track is current or the id is unknown. Shared by
/// [`now_playing_label`], [`now_playing_artist`], [`now_playing_elapsed`], and
/// [`now_playing_time`] so the four stay in sync on the lookup.
fn known_track_for<'a>(
    tracks: &'a HashMap<String, KnownTrack>,
    current_track: Option<&str>,
) -> Option<&'a KnownTrack> {
    current_track.and_then(|id| tracks.get(id))
}

/// The Now Playing bar's artist for `current_track`: the artist of the track
/// `tracks` knows, or the `""` literal when the track is unknown or none is
/// current. Mirrors [`now_playing_label`], so resolving the artist adds no
/// per-frame `String` allocation — the known case borrows from `tracks`.
/// Pure data → `Cow` so the artist resolution is testable without an iced
/// renderer.
pub fn now_playing_artist<'a>(
    tracks: &'a HashMap<String, KnownTrack>,
    current_track: Option<&str>,
) -> Cow<'a, str> {
    known_track_for(tracks, current_track)
        .map(|track| Cow::Borrowed(track.artist.as_str()))
        .unwrap_or(Cow::Borrowed(""))
}

/// The Now Playing bar's right-edge length readout for `current_track`: the
/// plain total cached in [`KnownTrack::time`], or `"--:--"` when the track is
/// unknown or none is current. A reported playback position never changes
/// this readout; it drives [`now_playing_elapsed`] instead. The known case
/// borrows from `tracks`, so a per-frame bar refresh formats and allocates
/// nothing. Mirrors [`now_playing_label`], [`now_playing_artist`], and
/// [`now_playing_elapsed`]. Pure data → `Cow` so the resolution is testable
/// without an iced renderer.
pub fn now_playing_time<'a>(
    tracks: &'a HashMap<String, KnownTrack>,
    current_track: Option<&str>,
) -> Cow<'a, str> {
    known_track_for(tracks, current_track)
        .map(|track| Cow::Borrowed(track.time.as_str()))
        .unwrap_or(Cow::Borrowed("--:--"))
}

/// The Now Playing bar's large left-hand time readout for `current_track`,
/// as chosen by `display`: elapsed `MM:SS` (the default) or the remaining
/// time with a leading `-`. In [`TimeDisplay::Elapsed`] the backend-reported
/// position formats as zero-padded `MM:SS` when there is one (`"00:53"`),
/// `"00:00"` when the track is known but no position is reported (no preview
/// loaded, a failed start, or a backend that cannot answer), or `"--:--"`
/// when the track is unknown or none is current. In [`TimeDisplay::Remaining`]
/// a known track with a length reads `-MM:SS` counting down from that length,
/// treating a missing position as zero so a stopped track computes its full
/// length remaining (which the stopped view then paints transparent); a
/// missing position past the length saturates at `"-00:00"`, and a
/// known track whose source supplied no length (`duration_ms == 0`) or an
/// unknown/absent track reads the `"--:--"` placeholder. Mirrors
/// [`now_playing_label`], [`now_playing_artist`], and [`now_playing_time`].
/// The placeholder cases borrow static strings, while a reported position
/// allocates its formatted `String`. Pure data → `Cow` so the resolution is
/// testable without an iced renderer.
pub fn now_playing_elapsed<'a>(
    tracks: &'a HashMap<String, KnownTrack>,
    current_track: Option<&str>,
    position: Option<Duration>,
    display: TimeDisplay,
) -> Cow<'a, str> {
    let Some(track) = known_track_for(tracks, current_track) else {
        return Cow::Borrowed("--:--");
    };
    match display {
        TimeDisplay::Elapsed => match position {
            Some(position) => Cow::Owned(format_lcd_elapsed(duration_millis(position))),
            None => Cow::Borrowed("00:00"),
        },
        TimeDisplay::Remaining => {
            if track.duration_ms == 0 {
                return Cow::Borrowed("--:--");
            }
            let position_ms = position.map(duration_millis).unwrap_or(0);
            Cow::Owned(format_lcd_remaining(
                track.duration_ms.saturating_sub(position_ms),
            ))
        }
    }
}

/// `duration` in milliseconds, saturating at [`u64::MAX`] rather than wrapping
/// for a position that large (which no real preview reaches).
fn duration_millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

/// Formats an elapsed playback position in milliseconds as the LCD's
/// zero-padded `MM:SS` form, so `0` reads `"00:00"`, `53_000` reads `"00:53"`,
/// and `192_000` reads `"03:12"`. Minutes are unbounded, so an hour or more
/// reads `"60:00"`/`"61:05"` rather than rolling over.
pub fn format_lcd_elapsed(position_ms: u64) -> String {
    let total_seconds = position_ms / 1000;
    format!("{:02}:{:02}", total_seconds / 60, total_seconds % 60)
}

/// Formats a remaining time in milliseconds as the LCD's countdown form with
/// a leading `-`, so `0` reads `"-00:00"`, `157_000` reads `"-02:37"`, and
/// `3_600_000` reads `"-60:00"`. Shares [`format_lcd_elapsed`]'s unbounded
/// minutes and zero padding after the sign.
pub fn format_lcd_remaining(remaining_ms: u64) -> String {
    format!("-{}", format_lcd_elapsed(remaining_ms))
}

/// Formats a loaded preview's bitrate and sample rate as the LCD status row's
/// prefix, `"{kbps} KBPS  {khz} KHZ"` (for example `256 KBPS  44 KHZ`). The
/// rate is truncated to whole kilohertz (`44_100` reads `44`, `48_000` reads
/// `48`), and the bitrate is omitted when the backend could not compute it,
/// leaving `44 KHZ`. The row draws the channel labels beside this prefix. Pure
/// so the formatting is testable without an iced renderer.
pub fn format_playback_prefix(format: PlaybackFormat) -> String {
    let rate = format!("{} KHZ", format.sample_rate_hz / 1000);
    match format.bitrate_kbps {
        Some(kbps) => format!("{kbps} KBPS  {rate}"),
        None => rate,
    }
}

/// Names the loaded preview's channel mode: one channel is `MONO` and any
/// other count is `STEREO`. Pure so the row's label mapping is testable
/// without an iced renderer.
pub fn channel_mode_label(channels: u16) -> &'static str {
    if channels == 1 { "MONO" } else { "STEREO" }
}
