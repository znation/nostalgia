//! The recording `Reporter` stub for the Apple Music service's failure paths.

use std::sync::Mutex;

use crate::apple_music::{ReportChannel, Reporter};

/// Records every report a service hands it, so a test asserts the exact line
/// and channel the service's default [`crate::apple_music::StdReporter`] would
/// only print to the operator.
#[derive(Debug, Default)]
pub(crate) struct RecordingReporter {
    lines: Mutex<Vec<(ReportChannel, String)>>,
}

impl RecordingReporter {
    /// An empty recorder.
    pub(crate) fn new() -> Self {
        Self {
            lines: Mutex::new(Vec::new()),
        }
    }

    /// The reports so far, oldest first, each with the channel it was handed
    /// to.
    pub(crate) fn recorded(&self) -> Vec<(ReportChannel, String)> {
        self.lines
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

impl Reporter for RecordingReporter {
    fn report(&self, channel: ReportChannel, line: String) {
        self.lines
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push((channel, line));
    }
}
