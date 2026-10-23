//! The browse-list state machine: one level of the browse hierarchy's rows
//! plus the state that guards them.
//!
//! The `WinampPlayer` holds one `BrowseList<T>` per level. The rows, the
//! cached row titles and labels, the epoch that makes an index-carrying selection message
//! safe, the loading flag the view reads for its "Loading…" placeholder, the
//! failure report it shows instead, and the request counter that drops a
//! superseded reply all live together here, so an `update` arm names the
//! transition (`store`, `store_with_row_text`, `clear`, `fail`, `begin_fetch`)
//! instead of spelling out the parallel-field bookkeeping at each site.
//!
//! The `update` arms go through `store`/`store_with_row_text`/`clear`/`fail`: the
//! Artists, Albums, All Albums, and Playlists `*Loaded` arms call `store`, the
//! Songs and All Songs `*Loaded` arms call `store_with_row_text` so each track's
//! display title and formatted length are composed once per load, every
//! `*LoadFailed` arm calls `fail`, and
//! the navigation arms, `LoadArtists`, the flat-list/Playlists buttons, and
//! `SearchSubmitted` start their fetch through the event loop's `fetch_level`,
//! which calls `clear` before `begin_fetch`. `select` resolves all four
//! selection arms' presses (track, artist, album, and playlist).

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use super::loading::RequestGeneration;

/// One browse level's rows plus the state that guards them.
pub(super) struct BrowseList<T> {
    /// The rows the view renders. Empty while a fetch for this level is in
    /// flight or after a failure.
    pub(super) items: Vec<T>,
    /// The primary title each row renders, cached when the rows are stored
    /// (see [`Self::store_with_row_text`]) so a view refresh does not re-join
    /// every artist and title. Only the song lists populate it; [`Self::store`]
    /// leaves it empty, and the row builder falls back to composing the title
    /// in that case. Every transition that changes `items` keeps the two in
    /// lockstep: `store` clears the titles, `store_with_row_text` rebuilds
    /// them beside the rows, and `clear`/`fail` drop them with the rows.
    pub(super) titles: Vec<String>,
    /// The secondary label each row renders, cached when the rows are stored
    /// (see [`Self::store_with_row_text`]) so a view refresh does not reformat
    /// every row. Only the song lists populate it; [`Self::store`] leaves it
    /// empty, and the row builder falls back to formatting in that case. Every
    /// transition that changes `items` keeps the two in lockstep: `store`
    /// clears the labels, `store_with_row_text` rebuilds them beside the rows,
    /// and `clear`/`fail` drop them with the rows.
    pub(super) labels: Vec<String>,
    /// Counts list replacements, bumped by [`Self::store`] whenever a fetched
    /// list replaces the rows. A selection message carries the epoch of the
    /// list that rendered its row, so [`Self::select`] can reject a press that
    /// outlived that list. A navigation clears the rows without touching the
    /// epoch (see [`Self::clear`]), so the replacing list's epoch is bumped
    /// past the cleared one's and differs from it.
    pub(super) epoch: u64,
    /// The permutation [`Self::sort_rows_by`] has applied to the rows, or
    /// `None` while they are in library order. Entry `i` is the library index
    /// of the row now at position `i`, so [`Self::restore_library_order`] can
    /// undo every sort since the rows were stored. Any transition that
    /// replaces the rows ([`Self::store`], [`Self::store_with_row_text`],
    /// [`Self::clear`], [`Self::fail`]) drops it: a freshly fetched list is
    /// already in library order.
    library_positions: Option<Vec<usize>>,
    /// True while a fetch for this level is waiting on a reply: it starts
    /// true for the artists fetch `boot` schedules, and [`Self::clear`] sets
    /// it whenever the rows are emptied for a new fetch. [`Self::store`]
    /// clears it. The view reads it to show "Loading…" instead of the
    /// list's empty wording (see `views::browse::browse_placeholder`).
    pub(super) loading: bool,
    /// The formatted report of this level's most recent failed fetch, or
    /// `None` when it has not failed. [`Self::fail`] sets it (and clears
    /// `loading`); [`Self::clear`] drops it before a retry, and a successful
    /// reply clears it. The view shows it in place of the empty-list wording
    /// (see `views::browse::browse_placeholder`), so a backend failure is not
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
            titles: Vec::new(),
            labels: Vec::new(),
            epoch: 0,
            library_positions: None,
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

    /// Whether `generation` is still the latest request issued for this level.
    ///
    /// [`RequestGeneration::is_current`] makes the same comparison on iced's
    /// executor thread, when the fetch's result is mapped to a message, but
    /// `update` stores the reply later on the UI thread; a navigation in that
    /// window issues a newer request. Re-checking here, at the store, lets the
    /// `*Loaded`/`*LoadFailed` arm drop a reply the newer request superseded
    /// instead of overwriting the newer list (or its error).
    pub(super) fn is_current(&self, generation: u64) -> bool {
        self.generation.load(Ordering::SeqCst) == generation
    }

    /// The row a selection message names, or `None` when the press is stale.
    /// A selection message carries a row's index in the list that rendered it
    /// plus that list's epoch; `epoch` must still match this list's epoch (the
    /// list was not replaced after the row was rendered) and `index` must
    /// still name a row. Either failure means the press outlived its list and
    /// must be a no-op, so the epoch check and the bounds check live here once
    /// instead of in each of the four selection arms.
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
        self.titles.clear();
        self.labels.clear();
        self.replace_rows(items);
    }

    /// Replaces the rows like [`Self::store`], and caches one primary title
    /// and one secondary label per row by running `title` and `label` once per
    /// item, in row order. The song lists use this so each track's display
    /// title and formatted length are composed when the list is loaded — once
    /// per fetch — instead of once per row on every view refresh, which for
    /// the whole-library list is an allocation per track per refresh. Both
    /// closures see the same items the rows will render, so the caches stay
    /// index-aligned with `items`.
    pub(super) fn store_with_row_text(
        &mut self,
        items: Vec<T>,
        title: impl Fn(&T) -> String,
        label: impl Fn(&T) -> String,
    ) {
        self.titles = items.iter().map(title).collect();
        self.labels = items.iter().map(label).collect();
        self.replace_rows(items);
    }

    /// Replaces `items` and bumps the list state that a replacement changes,
    /// leaving the cached row text to the caller: [`Self::store`] clears both
    /// caches and [`Self::store_with_row_text`] rebuilds them.
    fn replace_rows(&mut self, items: Vec<T>) {
        self.items = items;
        self.library_positions = None;
        self.epoch = self.epoch.wrapping_add(1);
        self.loading = false;
        self.error = None;
    }

    /// Reorders the loaded rows in place by `key`, comparing each key
    /// lowercased so the order ignores case, and keeps library order for keys
    /// that compare equal (the sort is stable).
    ///
    /// The cached row titles and labels are rebuilt by the same permutation
    /// whenever they are populated, so a reordered song list still shows each
    /// track's own title and length. The epoch is bumped like
    /// [`Self::replace_rows`] because the indices a rendered row emits now name
    /// different entries; `loading`, `error`, and the request generation are
    /// left alone, and a list with fewer than two rows or one still loading is
    /// a no-op.
    pub(super) fn sort_rows_by(&mut self, key: impl Fn(&T) -> String)
    where
        T: Clone,
    {
        // A loading list's rows are being replaced by a fetch, so reordering
        // them would be undone (or raced) by the reply; the documented
        // guarantee is that Sort while loading changes nothing. The app's
        // transitions empty the rows in `clear` before setting `loading`, but
        // this check pins the guarantee independently of that ordering.
        if self.items.len() < 2 || self.loading {
            return;
        }
        let keys: Vec<String> = self
            .items
            .iter()
            .map(|item| key(item).to_lowercase())
            .collect();
        let mut order: Vec<usize> = (0..self.items.len()).collect();
        order.sort_by(|&left, &right| keys[left].cmp(&keys[right]));
        // Keep the total permutation back to library order: a later Sort
        // composes with it instead of overwriting it, so restoring after any
        // number of sorts still yields the order the rows were stored in.
        self.library_positions = Some(match self.library_positions.take() {
            Some(positions) => order.iter().map(|&index| positions[index]).collect(),
            None => order.clone(),
        });
        self.items = order
            .iter()
            .map(|&index| self.items[index].clone())
            .collect();
        if !self.titles.is_empty() {
            self.titles = order
                .iter()
                .map(|&index| self.titles[index].clone())
                .collect();
        }
        if !self.labels.is_empty() {
            self.labels = order
                .iter()
                .map(|&index| self.labels[index].clone())
                .collect();
        }
        self.epoch = self.epoch.wrapping_add(1);
    }

    /// Undoes every [`Self::sort_rows_by`] since the rows were stored, putting
    /// `items` (and the cached titles and labels) back in library order and
    /// bumping the epoch because the indices a rendered row emits now name
    /// other entries. A list that is not sorted — or whose rows have since
    /// been replaced — is a no-op, so callers can restore unconditionally.
    pub(super) fn restore_library_order(&mut self)
    where
        T: Clone,
    {
        let Some(positions) = self.library_positions.take() else {
            return;
        };
        if positions.len() != self.items.len() {
            return;
        }
        // `positions[i]` is the library index of the row at `i`; invert it to
        // rebuild library order from the reordered buffers.
        let mut inverse = vec![0; positions.len()];
        for (current, &library) in positions.iter().enumerate() {
            inverse[library] = current;
        }
        self.items = inverse
            .iter()
            .map(|&index| self.items[index].clone())
            .collect();
        if !self.titles.is_empty() {
            self.titles = inverse
                .iter()
                .map(|&index| self.titles[index].clone())
                .collect();
        }
        if !self.labels.is_empty() {
            self.labels = inverse
                .iter()
                .map(|&index| self.labels[index].clone())
                .collect();
        }
        self.epoch = self.epoch.wrapping_add(1);
    }

    /// Empties the rows and marks the list loading before a new fetch for this
    /// level is issued, so the view never shows — or lets the user press — the
    /// previous selection's rows while the new fetch is in flight. `loading`
    /// makes the view show "Loading…" instead of the list's empty wording (see
    /// `views::browse::browse_placeholder`); the epoch is deliberately left alone, so
    /// the reply that replaces the rows still gets a bumped epoch that differs
    /// from the cleared list's and a press rendered from the old rows cannot
    /// match the new list. Any earlier failure is cleared too, so the retry
    /// shows "Loading…" rather than the stale error.
    pub(super) fn clear(&mut self) {
        self.items.clear();
        self.titles.clear();
        self.labels.clear();
        self.library_positions = None;
        self.loading = true;
        self.error = None;
    }

    /// Records a failed fetch's report: clears the rows (a navigation already
    /// emptied them via [`Self::clear`], but a failure must never leave stale
    /// rows behind), marks the list not-loading, and stores the report so the
    /// view can show it (see `views::browse::browse_placeholder`).
    pub(super) fn fail(&mut self, report: String) {
        self.items.clear();
        self.titles.clear();
        self.labels.clear();
        self.library_positions = None;
        self.loading = false;
        self.error = Some(report);
    }
}
