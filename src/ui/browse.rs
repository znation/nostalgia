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
//! Artists and Playlists `*Loaded` arms call `store`, the Songs, All Songs,
//! Albums, and All Albums `*Loaded` arms call `store_with_row_text` so each
//! row's display title and formatted length or running time are composed once
//! per load, every `*LoadFailed` arm calls `fail`, and
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
    /// every artist and title. The song and album lists populate it;
    /// [`Self::store`] leaves it empty, and the row builder falls back to
    /// composing the title in that case. Every transition that changes `items`
    /// keeps the two in lockstep: `store` clears the titles,
    /// `store_with_row_text` rebuilds them beside the rows, and
    /// `clear`/`fail` drop them with the rows.
    pub(super) titles: Vec<String>,
    /// The secondary label each row renders, cached when the rows are stored
    /// (see [`Self::store_with_row_text`]) so a view refresh does not reformat
    /// every row. The song and album lists populate it; [`Self::store`] leaves
    /// it empty, and the row builder falls back to formatting in that case.
    /// Every transition that changes `items` keeps the two in lockstep: `store`
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
    /// The row a single press last selected, or `None` while nothing is
    /// selected. Only [`Self::select_row`] sets it; every transition that
    /// changes the rows ([`Self::replace_rows`], [`Self::clear`],
    /// [`Self::fail`]) or their order ([`Self::sort_rows_by`],
    /// [`Self::restore_library_order`]) drops it, because the index it stores
    /// names a row in the previous list or order. The view paints the
    /// selected row with the blue `ROW_HIGHLIGHT` bar and white
    /// text; the `▶` marker stays tied to the playing track.
    selected: Option<usize>,
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
    /// list's empty wording (see `views::browse::rows::browse_placeholder`).
    pub(super) loading: bool,
    /// The formatted report of this level's most recent failed fetch, or
    /// `None` when it has not failed. [`Self::fail`] sets it (and clears
    /// `loading`); [`Self::clear`] drops it before a retry, and a successful
    /// reply clears it. The view shows it in place of the empty-list wording
    /// (see `views::browse::rows::browse_placeholder`), so a backend failure is not
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
            selected: None,
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

    /// Selects the row a browse-row press names without opening it.
    ///
    /// Returns `Some(false)` when the press selected the row, `Some(true)`
    /// when that row was already selected (the second press of a double-click,
    /// which opens it), and `None` when [`Self::select`] rejects a stale epoch
    /// or out-of-range index. A rejected press leaves the selection alone, so
    /// a stale message can neither select nor open the wrong row.
    pub(super) fn select_row(&mut self, epoch: u64, index: usize) -> Option<bool> {
        self.select(epoch, index)?;
        let already_selected = self.selected == Some(index);
        self.selected = Some(index);
        Some(already_selected)
    }

    /// The selected row, or `None` when nothing is selected. The view paints
    /// this row with the blue selection bar and white text.
    pub(super) fn selected(&self) -> Option<usize> {
        self.selected
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
        self.selected = None;
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
    /// (see [`Self::reorder_rows`]) whenever they are populated, so a
    /// reordered song list still shows each track's own title and length. The
    /// epoch is bumped like
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
        // The selection names a row in the old order, which this reorder (and
        // the epoch bump) just invalidated.
        self.selected = None;
        self.reorder_rows(&order);
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
        // Like sorting, restoring changes what every index names, so the
        // selected index no longer identifies the row the user picked.
        self.selected = None;
        self.reorder_rows(&inverse);
        self.epoch = self.epoch.wrapping_add(1);
    }

    /// Reorders `items`, and the cached `titles` and `labels` when they are
    /// populated, by the permutation `order`: entry `i` of each buffer takes
    /// the value that was at `order[i]`. Reordering all three together keeps
    /// each row's cached title and label beside it. [`Self::sort_rows_by`] and
    /// [`Self::restore_library_order`] both go through this, so neither can
    /// forget one of the caches.
    fn reorder_rows(&mut self, order: &[usize])
    where
        T: Clone,
    {
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
    }

    /// Empties the rows and marks the list loading before a new fetch for this
    /// level is issued, so the view never shows — or lets the user press — the
    /// previous selection's rows while the new fetch is in flight. `loading`
    /// makes the view show "Loading…" instead of the list's empty wording (see
    /// `views::browse::rows::browse_placeholder`); the epoch is deliberately left alone, so
    /// the reply that replaces the rows still gets a bumped epoch that differs
    /// from the cleared list's and a press rendered from the old rows cannot
    /// match the new list. Any earlier failure is cleared too, so the retry
    /// shows "Loading…" rather than the stale error.
    pub(super) fn clear(&mut self) {
        self.items.clear();
        self.titles.clear();
        self.labels.clear();
        self.selected = None;
        self.library_positions = None;
        self.loading = true;
        self.error = None;
    }

    /// Records a failed fetch's report: clears the rows (a navigation already
    /// emptied them via [`Self::clear`], but a failure must never leave stale
    /// rows behind), marks the list not-loading, and stores the report so the
    /// view can show it (see `views::browse::rows::browse_placeholder`).
    pub(super) fn fail(&mut self, report: String) {
        self.items.clear();
        self.titles.clear();
        self.labels.clear();
        self.selected = None;
        self.library_positions = None;
        self.loading = false;
        self.error = Some(report);
    }
}

#[cfg(test)]
mod tests {
    use super::BrowseList;

    /// A loaded list of `items`, with the epoch [`BrowseList::store`] gives it.
    fn loaded(items: &[&str]) -> BrowseList<String> {
        let mut list = BrowseList::new(false);
        list.store(items.iter().map(|item| item.to_string()).collect());
        list
    }

    #[test]
    fn select_row_records_a_press_and_reports_the_second_one() {
        let mut list = loaded(&["one", "two"]);
        let epoch = list.epoch;

        assert_eq!(list.selected(), None);
        assert_eq!(list.select_row(epoch, 1), Some(false));
        assert_eq!(list.selected(), Some(1));
        // The second press on the same row is the double-click's open.
        assert_eq!(list.select_row(epoch, 1), Some(true));
        assert_eq!(list.selected(), Some(1));
        // A press on another row moves the selection without opening.
        assert_eq!(list.select_row(epoch, 0), Some(false));
        assert_eq!(list.selected(), Some(0));
    }

    #[test]
    fn select_row_rejects_a_stale_epoch_or_out_of_range_index() {
        let mut list = loaded(&["one", "two"]);
        let epoch = list.epoch;
        assert_eq!(list.select_row(epoch, 0), Some(false));

        // A stale epoch and an out-of-range index both leave the existing
        // selection untouched, so the press cannot move or open it.
        assert_eq!(list.select_row(epoch.wrapping_add(1), 0), None);
        assert_eq!(list.selected(), Some(0));
        assert_eq!(list.select_row(epoch, 9), None);
        assert_eq!(list.selected(), Some(0));
    }

    #[test]
    fn replacing_clearing_or_failing_rows_drops_the_selection() {
        let mut list = loaded(&["one", "two"]);
        let epoch = list.epoch;
        assert_eq!(list.select_row(epoch, 0), Some(false));
        list.store(vec!["three".to_string()]);
        assert_eq!(list.selected(), None);

        let epoch = list.epoch;
        assert_eq!(list.select_row(epoch, 0), Some(false));
        list.clear();
        assert_eq!(list.selected(), None);

        list.store(vec!["four".to_string()]);
        let epoch = list.epoch;
        assert_eq!(list.select_row(epoch, 0), Some(false));
        list.fail("boom".to_string());
        assert_eq!(list.selected(), None);
    }

    #[test]
    fn sorting_or_restoring_rows_drops_the_selection() {
        let mut list = loaded(&["banana", "apple"]);
        let epoch = list.epoch;
        assert_eq!(list.select_row(epoch, 0), Some(false));

        list.sort_rows_by(|item| item.clone());
        assert_eq!(list.selected(), None);

        let epoch = list.epoch;
        assert_eq!(list.select_row(epoch, 0), Some(false));
        list.restore_library_order();
        assert_eq!(list.selected(), None);
    }
}
