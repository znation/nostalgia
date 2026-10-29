//! The settings persistence seam: the [`SettingsWriter`] trait the UI holds
//! behind a box.

use super::Settings;

/// Writes a [`Settings`] snapshot somewhere. The UI's `WinampPlayer` holds one
/// behind a `Box<dyn SettingsWriter>`; production uses
/// [`FileStore`](super::file_store::FileStore) and the tests use an in-memory
/// recorder.
pub trait SettingsWriter: Send + Sync {
    /// Persist `settings`. A failure is logged by the implementation rather
    /// than reported to the caller, so the UI's update loop never aborts.
    fn save(&self, settings: &Settings);
}
