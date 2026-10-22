//! The message handlers behind the app's `update` loop, one module per
//! concern.
//!
//! `crate::ui`'s dispatch table routes each `Message` variant to the handler
//! that owns it: the transport buttons and keys, the track stepping, the play
//! request and its completion in `playback`; the volume, balance, seek, and
//! equalizer in `mixer`; the browse hierarchy, the library search, and the
//! fetch results in `browse`; and the custom title bar's window actions in
//! `window`. Each handler owns its message's whole behaviour, so a change to
//! one concern never reaches into another's code.

pub(super) mod browse;
pub(super) mod mixer;
pub(super) mod playback;
pub(super) mod window;
