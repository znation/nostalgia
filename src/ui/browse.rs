//! The browse-list state machine: one level of the artist → album → song
//! hierarchy's rows plus the state that guards them.
//!
//! The `WinampPlayer` holds one `BrowseList<T>` per level. The rows, the
//! epoch that makes an index-carrying selection message safe, the loading
//! flag the view reads for its "Loading…" placeholder, the failure report it
//! shows instead, and the request counter that drops a superseded reply all
//! live together here, so an `update` arm names the transition (`store`,
//! `clear`, `fail`, `begin_fetch`) instead of spelling out the parallel-field
//! bookkeeping at each site.
//!
//! Eight `update` arms go through `store`/`clear`/`fail`: the three
//! `*Loaded` arms call `store`, the three `*LoadFailed` arms call `fail`, and
//! the two navigation arms call `clear` before `begin_fetch`. `LoadArtists`
//! calls `begin_fetch` for the first fetch, and `select` resolves the three
//! selection arms' presses.

use std::sync::{Arc, atomic::AtomicU64};

use super::loading::RequestGeneration;

/// One browse level's rows plus the state that guards them.
pub(super) struct BrowseList<T> {
    /// The rows the view renders. Empty while a fetch for this level is in
    /// flight or after a failure.
    pub(super) items: Vec<T>,
    /// Counts list replacements, bumped by [`Self::store`] whenever a fetched
    /// list replaces the rows. A selection message carries the epoch of the
    /// list that rendered its row, so [`Self::select`] can reject a press that
    /// outlived that list. A navigation clears the rows without touching the
    /// epoch (see [`Self::clear`]), so the replacing list's epoch is bumped
    /// past the cleared one's and differs from it.
    pub(super) epoch: u64,
    /// True while a fetch for this level is waiting on a reply: it starts
    /// true for the artists fetch `boot` schedules, and [`Self::clear`] sets
    /// it when a navigation empties the rows for a new fetch. [`Self::store`]
    /// clears it. The view reads it to show "Loading…" instead of the
    /// list's empty wording (see `views::browse_placeholder`).
    pub(super) loading: bool,
    /// The formatted report of this level's most recent failed fetch, or
    /// `None` when it has not failed. [`Self::fail`] sets it (and clears
    /// `loading`); a navigation clears it before the retry, and a successful
    /// reply clears it. The view shows it in place of the empty-list wording
    /// (see `views::browse_placeholder`), so a backend failure is not
    /// mistaken for an empty library.
    pub(super) error: Option<String>,
    /// This level's browse-request counter, shared (`Arc`) with its fetch
    /// tasks so a reply that completes after a newer request for the same
    /// level is recognized as stale (see [`RequestGeneration`]) and does not
    /// overwrite the newer list.
    generation: Arc<AtomicU64>,
}

impl<T> BrowseList<T> {
    /// A list with no rows, the given initial loading flag, and a fresh
    /// request counter.
    pub(super) fn new(loading: bool) -> Self {
        Self {
            items: Vec::new(),
            epoch: 0,
            loading,
            error: None,
            generation: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Issues a request for this level and returns its generation guard, which
    /// the caller hands to `loading::fetch_into`.
    pub(super) fn begin_fetch(&mut self) -> RequestGeneration {
        RequestGeneration::issue(&self.generation)
    }

    /// The row a selection message names, or `None` when the press is stale.
    /// A selection message carries a row's index in the list that rendered it
    /// plus that list's epoch; `epoch` must still match this list's epoch (the
    /// list was not replaced after the row was rendered) and `index` must
    /// still name a row. Either failure means the press outlived its list and
    /// must be a no-op, so the epoch check and the bounds check live here once
    /// instead of in each of the three selection arms.
    pub(super) fn select(&self, epoch: u64, index: usize) -> Option<&T> {
        if epoch != self.epoch {
            return None;
        }
        self.items.get(index)
    }

    /// Replaces the rows with a freshly fetched list, bumps the epoch, marks
    /// the list loaded, and clears any earlier fetch failure.
    ///
    /// The epoch bump is what makes the index-carrying selection messages
    /// safe: a row rendered from the old list carries the old epoch, so if the
    /// list is replaced before the press is processed, the epoch no longer
    /// matches and the stale press is ignored rather than resolving its index
    /// against the new list. A navigation clears the rows without resetting
    /// the epoch, so the replacing list's epoch is bumped past the cleared
    /// one's and differs from it.
    pub(super) fn store(&mut self, items: Vec<T>) {
        self.items = items;
        self.epoch = self.epoch.wrapping_add(1);
        self.loading = false;
        self.error = None;
    }

    /// Empties the rows and marks the list loading before a new fetch for this
    /// level is issued, so the view never shows — or lets the user press — the
    /// previous selection's rows while the new fetch is in flight. `loading`
    /// makes the view show "Loading…" instead of the list's empty wording (see
    /// `views::browse_placeholder`); the epoch is deliberately left alone, so
    /// the reply that replaces the rows still gets a bumped epoch that differs
    /// from the cleared list's and a press rendered from the old rows cannot
    /// match the new list. Any earlier failure is cleared too, so the retry
    /// shows "Loading…" rather than the stale error.
    pub(super) fn clear(&mut self) {
        self.items.clear();
        self.loading = true;
        self.error = None;
    }

    /// Records a failed fetch's report: clears the rows (a navigation already
    /// emptied them via [`Self::clear`], but a failure must never leave stale
    /// rows behind), marks the list not-loading, and stores the report so the
    /// view can show it (see `views::browse_placeholder`).
    pub(super) fn fail(&mut self, report: String) {
        self.items.clear();
        self.loading = false;
        self.error = Some(report);
    }
}
