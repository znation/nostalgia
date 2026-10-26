//! A settings writer that persists snapshots on one background thread.
//!
//! The production file store stages a temp file, fsyncs it, renames it over
//! the target, and fsyncs the containing directory (see [`super::file_store`]).
//! The update loop calls the writer after every message whose snapshot
//! changed, and a slider drag changes the volume, the balance, or a band gain
//! on every movement, so writing on that thread would block it on two fsyncs
//! per movement. [`BackgroundWriter`] hands the snapshot to a worker instead
//! and lets a burst of saves collapse to its newest value.

use std::io;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;

use super::Settings;
use super::file_store::SettingsWriter;

/// The name the worker thread carries, so a panic report attributes it.
const WRITER_THREAD_NAME: &str = "nostalgia-settings-writer";

/// Wraps a [`SettingsWriter`] so its blocking write runs on one background
/// thread.
///
/// `save` stores the snapshot in a one-slot mailbox and wakes the worker, so
/// the update thread does not wait on the file's fsyncs; the fallback below is
/// the one exception. The mailbox holds only
/// the newest snapshot: a burst of saves (a slider drag, or key repeat) skips
/// the snapshots in between and writes the latest one. Snapshots are whole
/// values, so a skipped one costs nothing that the latest does not already
/// carry.
///
/// The writer is attached once by `ui::init_ui` and dropped with the
/// application state. Dropping it closes the mailbox and joins the worker, so
/// a pending snapshot reaches the underlying store before the drop returns.
/// If the OS refuses the worker thread, `new` logs it and the writer saves on
/// the calling thread instead, which is the behavior before this wrapper
/// existed rather than a silent loss.
pub struct BackgroundWriter<W: SettingsWriter + Clone> {
    mailbox: Arc<Mailbox>,
    worker: Option<JoinHandle<()>>,
    /// The store to save on directly, set only when the worker could not
    /// start. The running worker owns its own clone.
    store: Option<W>,
}

impl<W: SettingsWriter + Clone + 'static> BackgroundWriter<W> {
    /// Starts the writer for `store`, or falls back to saving synchronously
    /// when the thread cannot start.
    pub fn new(store: W) -> Self {
        Self::with_spawn(store, |work| {
            crate::threads::spawn_named_joined(WRITER_THREAD_NAME, work)
        })
    }

    /// [`Self::new`] with an injectable spawner, so a test can drive the
    /// thread-creation failure without making the OS refuse a thread. The
    /// work is boxed because the seam carries a `FnOnce` value rather than a
    /// generic type the caller and the test stub would have to share.
    fn with_spawn(
        store: W,
        spawn: impl FnOnce(Box<dyn FnOnce() + Send>) -> io::Result<JoinHandle<()>>,
    ) -> Self {
        let mailbox = Arc::new(Mailbox::default());
        let worker_mailbox = Arc::clone(&mailbox);
        let worker_store = store.clone();
        let work: Box<dyn FnOnce() + Send> =
            Box::new(move || persist_forever(&worker_mailbox, &worker_store));
        match spawn(work) {
            Ok(worker) => Self {
                mailbox,
                worker: Some(worker),
                store: None,
            },
            Err(error) => {
                eprintln!(
                    "starting the settings writer thread failed: {error}; \
                     saving on the calling thread"
                );
                Self {
                    mailbox,
                    worker: None,
                    store: Some(store),
                }
            }
        }
    }
}

impl<W: SettingsWriter + Clone> SettingsWriter for BackgroundWriter<W> {
    fn save(&self, settings: &Settings) {
        if self.worker.is_none() {
            if let Some(store) = &self.store {
                store.save(settings);
            }
            return;
        }
        {
            let mut slot = self.mailbox.lock();
            slot.latest = Some(settings.clone());
        }
        self.mailbox.ready.notify_one();
    }
}

impl<W: SettingsWriter + Clone> Drop for BackgroundWriter<W> {
    fn drop(&mut self) {
        let Some(worker) = self.worker.take() else {
            return;
        };
        // Close the mailbox, then let the worker write whatever snapshot is
        // pending and exit; joining is what makes that write finish before the
        // application does.
        self.mailbox.lock().closed = true;
        self.mailbox.ready.notify_all();
        let _ = worker.join();
    }
}

/// The worker's one-slot mailbox: the newest snapshot and whether the writer
/// has been dropped.
#[derive(Default)]
struct Mailbox {
    slot: Mutex<Slot>,
    ready: Condvar,
}

impl Mailbox {
    fn lock(&self) -> MutexGuard<'_, Slot> {
        self.slot.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[derive(Default)]
struct Slot {
    latest: Option<Settings>,
    closed: bool,
}

/// Writes each snapshot the mailbox holds, skipping any a newer save
/// overwrote while the previous write was running, until the mailbox closes.
fn persist_forever<W: SettingsWriter>(mailbox: &Mailbox, store: &W) {
    loop {
        let settings = {
            let mut slot = mailbox.lock();
            loop {
                // A pending snapshot is written even when the writer was
                // dropped: the drop only says no more will arrive.
                if let Some(settings) = slot.latest.take() {
                    break settings;
                }
                if slot.closed {
                    return;
                }
                slot = mailbox
                    .ready
                    .wait(slot)
                    .unwrap_or_else(PoisonError::into_inner);
            }
        };
        store.save(&settings);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Condvar, Mutex};
    use std::time::Duration;

    use super::{BackgroundWriter, SettingsWriter};
    use crate::settings::Settings;

    /// A gate that blocks the worker inside its first `save`, so a test can
    /// enqueue a burst while no write is in progress.
    #[derive(Clone, Default)]
    struct Gate {
        state: Arc<(Mutex<GateState>, Condvar)>,
    }

    #[derive(Default)]
    struct GateState {
        entered: bool,
        released: bool,
    }

    impl Gate {
        fn block_first_save(&self) {
            let (lock, ready) = &*self.state;
            let mut state = lock.lock().expect("the gate mutex");
            state.entered = true;
            ready.notify_all();
            while !state.released {
                state = ready.wait(state).expect("the gate mutex");
            }
        }

        fn wait_until_entered(&self) {
            let (lock, ready) = &*self.state;
            let mut state = lock.lock().expect("the gate mutex");
            while !state.entered {
                let (next, timeout) = ready
                    .wait_timeout(state, Duration::from_secs(5))
                    .expect("the gate mutex");
                state = next;
                assert!(
                    !timeout.timed_out(),
                    "the worker did not reach its first save"
                );
            }
        }

        fn release(&self) {
            let (lock, ready) = &*self.state;
            lock.lock().expect("the gate mutex").released = true;
            ready.notify_all();
        }
    }

    /// A [`SettingsWriter`] that records what it was asked to write, optionally
    /// blocking on `gate` so a test controls when its saves run.
    #[derive(Clone, Default)]
    struct RecordingStore {
        snapshots: Arc<Mutex<Vec<Settings>>>,
        gate: Option<Gate>,
    }

    impl RecordingStore {
        fn snapshots(&self) -> Vec<Settings> {
            self.snapshots.lock().expect("the recording mutex").clone()
        }
    }

    impl SettingsWriter for RecordingStore {
        fn save(&self, settings: &Settings) {
            if let Some(gate) = &self.gate {
                gate.block_first_save();
            }
            self.snapshots
                .lock()
                .expect("the recording mutex")
                .push(settings.clone());
        }
    }

    fn with_volume(volume: f32) -> Settings {
        Settings {
            volume,
            ..Settings::default()
        }
    }

    #[test]
    fn a_burst_of_saves_collapses_to_the_latest_snapshot() {
        let gate = Gate::default();
        let store = RecordingStore {
            gate: Some(gate.clone()),
            ..RecordingStore::default()
        };
        let writer = BackgroundWriter::new(store.clone());

        let first = with_volume(0.1);
        writer.save(&first);
        // The worker is now blocked inside the first save, so the next two
        // saves can only wait in the mailbox.
        gate.wait_until_entered();
        writer.save(&with_volume(0.2));
        let last = with_volume(0.3);
        writer.save(&last);
        gate.release();
        drop(writer);

        assert_eq!(store.snapshots(), vec![first, last]);
    }

    #[test]
    fn dropping_the_writer_flushes_the_pending_snapshot() {
        let store = RecordingStore::default();
        let writer = BackgroundWriter::new(store.clone());
        let settings = with_volume(0.9);
        writer.save(&settings);
        drop(writer);

        assert_eq!(store.snapshots(), vec![settings]);
    }

    #[test]
    fn a_refused_writer_thread_falls_back_to_saving_on_the_caller() {
        let store = RecordingStore::default();
        let writer = BackgroundWriter::with_spawn(store.clone(), |_work| {
            Err(std::io::Error::other("the OS refused another thread"))
        });
        let settings = with_volume(0.75);
        writer.save(&settings);
        drop(writer);

        assert_eq!(store.snapshots(), vec![settings]);
    }
}
