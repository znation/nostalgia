//! The message handlers behind the app's `update` loop, one module per
//! concern.
//!
//! `crate::ui`'s dispatch table routes each `Message` variant to the handler
//! that owns it: the transport buttons and keys, the play request and its
//! completion, the elapsed/remaining toggle, the marquee tick in `playback`;
//! the track stepping and the edge and ten-song jumps in
//! `playback::stepping`; the seek drag and release and the backend polls in
//! `playback::preview`; the track-id-to-song resolutions through the service
//! seam in `playback::resolve`; the volume, balance, seek nudges, and equalizer audio
//! controls in `mixer`; the browse row presses in `browse::selection`, the
//! Artist-view lists and the Back/Reload navigation in `browse::lists`, and
//! the row-order edits in `browse::ordering`; the library search with its
//! Clear button in `browse::search`; the list's scroll
//! operations in `browse::scroll`; the initial load and the fetch results in
//! `browse::fetch`; and
//! the custom title bar's window actions and panel visibility in `window`.
//! Each handler owns its message's whole behavior, so a change to one concern
//! never reaches into another's code.

pub(super) mod browse;
pub(super) mod jump;
pub(super) mod mixer;
pub(super) mod playback;
pub(super) mod save_list;
pub(super) mod window;
