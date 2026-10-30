//! The browse-list state machine: one level of the browse hierarchy's rows
//! plus the state that guards them.
//!
//! The `WinampPlayer` holds one `BrowseList<T>` per level. The rows, the
//! cached row titles and labels, the epoch that makes an index-carrying selection message
//! safe, the loading flag the view reads for its "Loading…" placeholder, the
//! failure report it shows instead, and the request counter that drops a
//! superseded reply all live together here, so an `update` arm names the
//! transition (`store`, `store_with_row_text`, `store_with_labels`, `clear`,
//! `fail`, `begin_fetch`)
//! instead of spelling out the parallel-field bookkeeping at each site.
//!
//! The `update` arms go through
//! `store`/`store_with_row_text`/`store_with_labels`/`clear`/`fail`: the
//! Artists `*Loaded` arm calls `store_with_labels`, the Playlists arm calls
//! `store`, and the Songs, All Songs, Albums, and All Albums `*Loaded` arms
//! call `store_with_row_text` so each row's display title and formatted length
//! or running time are composed once per load, every `*LoadFailed` arm calls
//! `fail`, and
//! the navigation arms, `LoadArtists`, `ReloadLibrary`, the flat-list/Playlists
//! buttons, and `SearchSubmitted` start their fetch through the event loop's
//! `fetch_level`,
//! which calls `clear` before `begin_fetch`. `select` resolves all four
//! selection arms' presses (track, artist, album, and playlist).
//!
//! The row-order operations (Sort, reverse, shuffle, move one place, and the
//! restore to library order) and the permutation helpers they share live in
//! the `ordering` submodule, beside the state machine they reorder.

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use super::loading::RequestGeneration;

pub(super) mod ordering;

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
    /// (see [`Self::store_with_row_text`] and [`Self::store_with_labels`]) so a
    /// view refresh does not reformat every row. The song, album, and artist
    /// lists populate it; [`Self::store`] leaves it empty, and the row builder
    /// falls back to formatting in that case. Every transition that changes
    /// `items` keeps the two in lockstep: `store` clears the labels,
    /// `store_with_row_text` and `store_with_labels` rebuild them beside the
    /// rows, and `clear`/`fail` drop them with the rows.
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
    /// changes the rows ([`Self::replace_rows`], [`Self::clear`], [`Self::fail`])
    /// or their order ([`Self::sort_rows_by`], [`Self::reverse_rows`],
    /// [`Self::shuffle_rows`], [`Self::restore_library_order`]) drops it, because
    /// the index it stores names a row in the previous list or order. The view
    /// paints the selected row with the blue `ROW_HIGHLIGHT` bar and white text;
    /// the `▶` marker stays tied to the playing track.
    selected: Option<usize>,
    /// The permutation [`Self::sort_rows_by`], [`Self::reverse_rows`], and
    /// [`Self::shuffle_rows`] have applied to the rows, or `None` while they are in
    /// library order. Entry `i` is the library index
    /// of the row now at position `i`, so [`Self::restore_library_order`] can
    /// undo every reorder since the rows were stored. Any transition that
    /// replaces the rows ([`Self::store`], [`Self::store_with_row_text`],
    /// [`Self::store_with_labels`], [`Self::clear`], [`Self::fail`]) drops it:
    /// a freshly fetched list is already in library order.
    library_positions: Option<Vec<usize>>,
    /// True while a fetch for this level is waiting on a reply: it starts
    /// true for the artists fetch `boot` schedules, and [`Self::clear`] sets
    /// it whenever the rows are emptied for a new fetch. [`Self::store`]
    /// clears it. The view reads it to show "Loading…" instead of the
    /// list's empty wording (see `views::browse::rows::text::browse_placeholder`).
    pub(super) loading: bool,
    /// The formatted report of this level's most recent failed fetch, or
    /// `None` when it has not failed. [`Self::fail`] sets it (and clears
    /// `loading`); [`Self::clear`] drops it before a retry, and a successful
    /// reply clears it. The view shows it in place of the empty-list wording
    /// (see `views::browse::rows::text::browse_placeholder`), so a backend failure is not
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

    /// Moves the selection bar one row down (`down`) or up over the showing
    /// list, the keyboard selection step Winamp's playlist editor offers on
    /// its arrow keys.
    ///
    /// Unlike [`Self::select_row`] this is not an index-carrying press, so it
    /// checks no epoch: a step relative to the selection names the showing
    /// list itself, and the selection it leaves lives at the same rows the
    /// next render paints. It returns the newly selected row's index, or
    /// `None` when the selection did not change — a loading or failed list,
    /// nothing selected, or a step that would leave the first (up) or last
    /// (down) row keeps the selection where it was, mirroring
    /// `move_selected`'s no-op rules without reordering anything.
    pub(super) fn move_selection(&mut self, down: bool) -> Option<usize> {
        if self.loading || self.error.is_some() {
            return None;
        }
        let index = self.selected?;
        let target = if down {
            index + 1
        } else {
            index.saturating_sub(1)
        };
        if target == index || target >= self.items.len() {
            return None;
        }
        self.selected = Some(target);
        Some(target)
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

    /// Replaces the rows like [`Self::store`], and caches one secondary label
    /// per row by running `label` once per item, in row order, leaving the
    /// primary-title cache empty. The artists list uses this because its row
    /// title is borrowed straight from the artist name and only the
    /// album-count label is composed: caching the title too would clone every
    /// name for no view-time saving. [`Self::store_with_row_text`] covers the
    /// lists that need both caches.
    pub(super) fn store_with_labels(&mut self, items: Vec<T>, label: impl Fn(&T) -> String) {
        self.titles.clear();
        self.labels = items.iter().map(label).collect();
        self.replace_rows(items);
    }

    /// Replaces `items` and bumps the list state that a replacement changes,
    /// leaving the cached row text to the caller: [`Self::store`] clears both
    /// caches, [`Self::store_with_row_text`] rebuilds both, and
    /// [`Self::store_with_labels`] rebuilds only the labels.
    fn replace_rows(&mut self, items: Vec<T>) {
        self.items = items;
        self.selected = None;
        self.library_positions = None;
        self.epoch = self.epoch.wrapping_add(1);
        self.loading = false;
        self.error = None;
    }

    /// Empties the rows and marks the list loading before a new fetch for this
    /// level is issued, so the view never shows — or lets the user press — the
    /// previous selection's rows while the new fetch is in flight. `loading`
    /// makes the view show "Loading…" instead of the list's empty wording (see
    /// `views::browse::rows::text::browse_placeholder`); the epoch is deliberately left alone, so
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
    /// view can show it (see `views::browse::rows::text::browse_placeholder`).
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
    fn move_selected_swaps_with_the_neighbour_and_keeps_the_selection() {
        let mut list = loaded(&["a", "b", "c"]);
        list.selected = Some(1);
        list.move_selected(true);
        assert_eq!(list.items, ["a", "c", "b"]);
        assert_eq!(list.selected(), Some(2));
        list.move_selected(false);
        list.move_selected(false);
        assert_eq!(list.items, ["b", "a", "c"]);
        assert_eq!(list.selected(), Some(0));
        list.restore_library_order();
        assert_eq!(list.items, ["a", "b", "c"]);
    }

    // Ctrl+Up/Down's selection step cannot leave the list: at either edge the
    // step is a no-op and the selection stays, and a loading or failed list or
    // an empty selection is one too.
    #[test]
    fn move_selection_keeps_the_selection_at_the_edges_and_on_no_selection() {
        let mut list = loaded(&["a", "b", "c"]);
        list.selected = Some(0);
        assert_eq!(list.move_selection(false), None);
        assert_eq!(list.selected(), Some(0));
        list.selected = Some(2);
        assert_eq!(list.move_selection(true), None);
        assert_eq!(list.selected(), Some(2));

        list.selected = None;
        assert_eq!(list.move_selection(true), None);
        assert_eq!(list.move_selection(false), None);
        assert_eq!(list.selected(), None);

        list.selected = Some(1);
        list.loading = true;
        assert_eq!(list.move_selection(true), None);
        assert_eq!(list.selected(), Some(1));
        list.loading = false;
        list.error = Some("fetch failed".to_string());
        assert_eq!(list.move_selection(false), None);
        assert_eq!(list.selected(), Some(1));
    }

    #[test]
    fn move_selected_swaps_cached_text_and_still_restores_library_order() {
        let mut list = BrowseList::new(false);
        list.store_with_row_text(
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            |item| format!("title-{item}"),
            |item| format!("label-{item}"),
        );
        list.selected = Some(2);
        list.move_selected(false);
        list.move_selected(false);
        assert_eq!(list.items, ["c", "a", "b"]);
        assert_eq!(list.titles, ["title-c", "title-a", "title-b"]);
        assert_eq!(list.labels, ["label-c", "label-a", "label-b"]);

        list.restore_library_order();

        assert_eq!(list.items, ["a", "b", "c"]);
        assert_eq!(list.titles, ["title-a", "title-b", "title-c"]);
        assert_eq!(list.labels, ["label-a", "label-b", "label-c"]);
    }

    #[test]
    fn storing_row_text_bumps_the_epoch_that_revises_cached_rows() {
        // The browse row cache keys on the epoch instead of rescanning each
        // row's title and label every view, so a text change must revise the
        // epoch. `store_with_row_text` and `store_with_labels` are the two
        // paths that install cached text for a freshly loaded list.
        let mut songs = BrowseList::new(false);
        let before = songs.epoch;
        songs.store_with_row_text(
            vec!["a".to_string()],
            |item| format!("title-{item}"),
            |item| format!("label-{item}"),
        );
        let stored = songs.epoch;
        assert_ne!(stored, before, "storing a loaded list revises its epoch");

        songs.store_with_row_text(
            vec!["a".to_string()],
            |_| "another title".to_string(),
            |_| "another label".to_string(),
        );
        assert_ne!(
            songs.epoch, stored,
            "changed cached row text revises the epoch"
        );

        let mut artists = BrowseList::new(false);
        let before = artists.epoch;
        artists.store_with_labels(vec!["a".to_string()], |_| "1 album".to_string());
        assert_ne!(
            artists.epoch, before,
            "storing cached labels revises the epoch"
        );
    }

    #[test]
    fn reordering_moves_rows_without_cloning_them() {
        // Reorder paths used to rebuild every buffer by cloning each row, so
        // they required `T: Clone`. A row that deliberately does not implement
        // Clone proves they now move the rows through the permutation instead.
        #[derive(Debug, PartialEq)]
        struct Unclonable(String);

        let mut list = BrowseList::new(false);
        list.store_with_row_text(
            vec![
                Unclonable("b".to_string()),
                Unclonable("a".to_string()),
                Unclonable("c".to_string()),
            ],
            |row| format!("title-{}", row.0),
            |row| format!("label-{}", row.0),
        );

        list.sort_rows_by(|row| row.0.clone());
        assert_eq!(
            list.items,
            ["a", "b", "c"].map(|value| Unclonable(value.to_string()))
        );
        assert_eq!(list.titles, ["title-a", "title-b", "title-c"]);
        assert_eq!(list.labels, ["label-a", "label-b", "label-c"]);

        list.reverse_rows();
        assert_eq!(
            list.items,
            ["c", "b", "a"].map(|value| Unclonable(value.to_string()))
        );
        assert_eq!(list.titles, ["title-c", "title-b", "title-a"]);

        list.restore_library_order();
        assert_eq!(
            list.items,
            ["b", "a", "c"].map(|value| Unclonable(value.to_string()))
        );
        assert_eq!(list.titles, ["title-b", "title-a", "title-c"]);
        assert_eq!(list.labels, ["label-b", "label-a", "label-c"]);
    }

    #[test]
    fn sort_walks_disjoint_cycles_and_keeps_cached_text_aligned() {
        // The sort key sends each row to a position with this source order:
        // [0, 3, 1, 2, 5, 4, 6] — fixed points at 0 and 6, a 3-cycle over
        // 1..=3, and a 2-cycle over 4 and 5. A reorder that swapped each slot
        // with `order[slot]` once instead of walking the cycles would leave
        // this list wrong, even though smaller single-cycle reorders pass.
        let rank = |item: &str| match item {
            "a" => 0,
            "c" => 2,
            "d" => 3,
            "b" => 1,
            "f" => 5,
            "e" => 4,
            "g" => 6,
            other => panic!("unexpected item {other}"),
        };
        let mut list = BrowseList::new(false);
        list.store_with_row_text(
            vec!["a", "c", "d", "b", "f", "e", "g"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            |item| format!("title-{item}"),
            |item| format!("label-{item}"),
        );

        list.sort_rows_by(|item| rank(item).to_string());

        assert_eq!(list.items, ["a", "b", "c", "d", "e", "f", "g"]);
        assert_eq!(
            list.titles,
            [
                "title-a", "title-b", "title-c", "title-d", "title-e", "title-f", "title-g"
            ]
        );
        assert_eq!(
            list.labels,
            [
                "label-a", "label-b", "label-c", "label-d", "label-e", "label-f", "label-g"
            ]
        );

        list.restore_library_order();

        assert_eq!(list.items, ["a", "c", "d", "b", "f", "e", "g"]);
        assert_eq!(
            list.titles,
            [
                "title-a", "title-c", "title-d", "title-b", "title-f", "title-e", "title-g"
            ]
        );
        assert_eq!(
            list.labels,
            [
                "label-a", "label-c", "label-d", "label-b", "label-f", "label-e", "label-g"
            ]
        );
    }

    // The edge move swaps through the stretch between the row and the edge
    // with the one-place move's exact pattern, so every buffer and the library
    // permutation shift in step and the selection lands on the moved row.
    #[test]
    fn move_selected_to_edge_shifts_rows_cached_text_and_permutation_in_step() {
        let mut list = BrowseList::new(false);
        list.store_with_row_text(
            vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "d".to_string(),
            ],
            |item| format!("title-{item}"),
            |item| format!("label-{item}"),
        );
        list.selected = Some(1);
        let epoch = list.epoch;

        list.move_selected_to_edge(false);

        assert_eq!(list.items, ["b", "a", "c", "d"]);
        assert_eq!(list.titles, ["title-b", "title-a", "title-c", "title-d"]);
        assert_eq!(list.labels, ["label-b", "label-a", "label-c", "label-d"]);
        assert_eq!(list.selected(), Some(0));
        assert_eq!(list.library_positions, Some(vec![1, 0, 2, 3]));
        assert_eq!(list.epoch, epoch.wrapping_add(1));

        list.selected = Some(2);
        list.move_selected_to_edge(true);
        assert_eq!(list.items, ["b", "a", "d", "c"]);
        assert_eq!(list.titles, ["title-b", "title-a", "title-d", "title-c"]);
        assert_eq!(list.selected(), Some(3));
        assert_eq!(list.library_positions, Some(vec![1, 0, 3, 2]));

        list.restore_library_order();
        assert_eq!(list.items, ["a", "b", "c", "d"]);
    }

    #[test]
    fn move_selected_to_edge_is_a_noop_without_a_selection_at_an_edge_while_loading_or_failed() {
        let mut list = loaded(&["a", "b", "c"]);
        let epoch = list.epoch;
        list.move_selected_to_edge(true);
        assert_eq!((list.epoch, list.selected()), (epoch, None));
        list.selected = Some(0);
        list.move_selected_to_edge(false);
        list.selected = Some(2);
        list.move_selected_to_edge(true);
        list.selected = Some(5);
        list.move_selected_to_edge(false);
        assert_eq!(list.items, ["a", "b", "c"]);
        assert_eq!(list.library_positions, None);
        assert_eq!(list.epoch, epoch);
        list.selected = Some(1);
        list.loading = true;
        list.move_selected_to_edge(false);
        list.loading = false;
        list.error = Some("offline".to_string());
        list.move_selected_to_edge(true);
        assert_eq!(list.items, ["a", "b", "c"]);
        assert_eq!((list.epoch, list.selected()), (epoch, Some(1)));
    }

    // A press whose row already sits at the target edge allocates and swaps
    // nothing, so a held second press after the first is itself a no-op.
    #[test]
    fn move_selected_to_edge_repeats_noop_once_the_row_is_at_the_edge() {
        let mut list = loaded(&["a", "b", "c", "d"]);
        list.selected = Some(3);
        list.move_selected_to_edge(false);
        assert_eq!(list.items, ["d", "a", "b", "c"]);
        list.selected = Some(0);
        list.move_selected_to_edge(false);
        let epoch = list.epoch;
        let positions = list.library_positions.clone();
        list.move_selected_to_edge(false);
        assert_eq!(list.items, ["d", "a", "b", "c"]);
        assert_eq!(list.library_positions, positions);
        assert_eq!((list.epoch, list.selected()), (epoch, Some(0)));

        // The same held-repeat no-op guards the bottom edge: the first press
        // after the top move still moves, the repeat does not.
        list.selected = Some(0);
        list.move_selected_to_edge(true);
        assert_eq!(list.items, ["a", "b", "c", "d"]);
        let epoch = list.epoch;
        list.move_selected_to_edge(true);
        assert_eq!(list.items, ["a", "b", "c", "d"]);
        assert_eq!((list.epoch, list.selected()), (epoch, Some(3)));
    }

    // Sort records a permutation and the edge move swaps into it, so the
    // restore that Back performs returns to library order, and one more move
    // afterwards still composes with the saved permutation.
    #[test]
    fn sort_then_edge_moves_restore_library_order_with_one_more_move_still_composing() {
        let mut list = BrowseList::new(false);
        list.store_with_row_text(
            ["pear", "apple", "mango", "kiwi"]
                .iter()
                .map(|item| (*item).to_string())
                .collect(),
            |item| format!("title-{item}"),
            |item| format!("label-{item}"),
        );
        list.sort_rows_by(|item| item.clone());
        assert_eq!(list.items, ["apple", "kiwi", "mango", "pear"]);

        list.selected = Some(2);
        list.move_selected_to_edge(false);
        assert_eq!(list.items, ["mango", "apple", "kiwi", "pear"]);
        list.selected = Some(3);
        list.move_selected_to_edge(false);
        assert_eq!(list.items, ["pear", "mango", "apple", "kiwi"]);

        // A neighbour swap composes with the saved permutation.
        list.selected = Some(1);
        list.move_selected(true);

        list.restore_library_order();
        assert_eq!(list.items, ["pear", "apple", "mango", "kiwi"]);
        assert_eq!(list.selected(), None);
    }

    #[test]
    fn move_selected_is_a_noop_without_a_selection_at_an_edge_or_while_loading() {
        let mut list = loaded(&["a", "b", "c"]);
        let epoch = list.epoch;
        list.move_selected(true);
        assert_eq!((list.epoch, list.selected()), (epoch, None));
        list.selected = Some(0);
        list.move_selected(false);
        list.selected = Some(2);
        list.move_selected(true);
        assert_eq!(list.items, ["a", "b", "c"]);
        assert_eq!((list.epoch, list.selected()), (epoch, Some(2)));
        list.selected = Some(1);
        list.loading = true;
        list.move_selected(true);
        assert_eq!(list.items, ["a", "b", "c"]);
        assert_eq!((list.epoch, list.selected()), (epoch, Some(1)));
    }

    #[test]
    fn remove_selected_row_drops_the_selection_and_bumps_the_epoch() {
        let mut list = loaded(&["a", "b", "c"]);
        let epoch = list.epoch;
        assert_eq!(list.select_row(epoch, 1), Some(false));

        assert!(list.remove_selected_row());

        assert_eq!(list.items, ["a", "c"]);
        assert_eq!(list.selected(), None);
        assert_eq!(list.epoch, epoch.wrapping_add(1));
        // The rows were still in library order, so the removal left no
        // materialized permutation to adjust.
        assert_eq!(list.library_positions, None);

        // A one-row list empties: the last row is removably there.
        let mut list = loaded(&["only"]);
        let epoch = list.epoch;
        assert_eq!(list.select_row(epoch, 0), Some(false));
        assert!(list.remove_selected_row());
        assert!(list.items.is_empty());
        assert_eq!(list.epoch, epoch.wrapping_add(1));
    }

    #[test]
    fn remove_selected_row_drops_the_cached_text_with_the_row() {
        let mut list = BrowseList::new(false);
        list.store_with_row_text(
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            |item| format!("title-{item}"),
            |item| format!("label-{item}"),
        );
        assert_eq!(list.select_row(list.epoch, 1), Some(false));

        list.remove_selected_row();

        assert_eq!(list.titles, ["title-a", "title-c"]);
        assert_eq!(list.labels, ["label-a", "label-c"]);
    }

    #[test]
    fn removing_a_sorted_row_restores_library_order_minus_it() {
        // The removal must keep `library_positions` a permutation of the
        // remaining rows: deleting a middle entry and then restoring has to
        // yield the library order without the removed row, not a collapsed or
        // out-of-bounds permutation.
        let mut list = loaded(&["banana", "apple", "cherry"]);
        list.sort_rows_by(|item| item.clone());
        assert_eq!(list.items, ["apple", "banana", "cherry"]);
        assert_eq!(list.selected(), None);
        assert_eq!(list.select_row(list.epoch, 0), Some(false));

        list.remove_selected_row();

        assert_eq!(list.items, ["banana", "cherry"]);
        list.restore_library_order();
        assert_eq!(list.items, ["banana", "cherry"]);
    }

    #[test]
    fn removing_from_an_unreordered_list_leaves_no_permutation_to_adjust() {
        let mut list = loaded(&["a", "b", "c"]);
        assert!(list.library_positions.is_none());
        assert_eq!(list.select_row(list.epoch, 1), Some(false));

        list.remove_selected_row();

        assert_eq!(list.items, ["a", "c"]);
        // The rows are still in library order, so no materialized permutation
        // appears; restore's no-op on `None` already yields the shortened
        // order.
        assert!(list.library_positions.is_none());
        list.restore_library_order();
        assert_eq!(list.items, ["a", "c"]);
    }

    #[test]
    fn remove_all_rows_empties_the_buffers_drops_the_selection_and_bumps_the_epoch() {
        let mut list = BrowseList::new(false);
        list.store_with_row_text(
            vec!["a".to_string(), "b".to_string(), "c".to_string()],
            |item| format!("title-{item}"),
            |item| format!("label-{item}"),
        );
        // Sort first so there is a materialized permutation to drop and a
        // selection into the reordered rows.
        list.sort_rows_by(|item| item.clone());
        assert_eq!(list.select_row(list.epoch, 2), Some(false));
        assert!(list.library_positions.is_some());
        let epoch = list.epoch;

        assert!(list.remove_all_rows());

        assert_eq!(list.titles, Vec::<String>::new());
        assert_eq!(list.labels, Vec::<String>::new());
        assert_eq!(list.selected(), None);
        assert!(!list.loading);
        assert_eq!(list.error, None);
        // An emptied list is trivially back in library order: no permutation
        // survives to be restored over nothing.
        assert_eq!(list.library_positions, None);
        assert_eq!(list.epoch, epoch.wrapping_add(1));
    }

    #[test]
    fn remove_all_rows_twice_sees_the_second_clear_noop() {
        let mut list = loaded(&["a", "b"]);
        list.sort_rows_by(|item| item.clone());
        assert!(list.remove_all_rows());
        let epoch = list.epoch;

        assert!(!list.remove_all_rows());

        assert_eq!(list.epoch, epoch);
        assert_eq!(list.items, Vec::<String>::new());
    }

    #[test]
    fn remove_all_rows_is_a_noop_while_loading_or_failed() {
        let mut list = loaded(&["a", "b"]);
        list.selected = Some(1);
        let epoch = list.epoch;

        list.loading = true;
        assert!(!list.remove_all_rows());
        assert_eq!(list.items, ["a", "b"].map(String::from));
        assert_eq!(list.selected(), Some(1));
        assert_eq!(list.epoch, epoch);

        list.loading = false;
        list.error = Some("boom".to_string());
        assert!(!list.remove_all_rows());
        assert_eq!(list.items, ["a", "b"].map(String::from));
        assert_eq!(list.selected(), Some(1));
        assert_eq!(list.epoch, epoch);
    }

    #[test]
    fn remove_selected_row_is_a_noop_without_a_selection_while_loading_or_failed() {
        let mut list = loaded(&["a", "b"]);
        let epoch = list.epoch;
        assert!(!list.remove_selected_row());
        assert_eq!((list.items.len(), list.epoch), (2, epoch));

        list.selected = Some(1);
        list.loading = true;
        assert!(!list.remove_selected_row());
        assert_eq!(list.items, ["a", "b"]);
        assert_eq!(list.selected(), Some(1));
        assert_eq!(list.epoch, epoch);

        list.loading = false;
        list.error = Some("boom".to_string());
        assert!(!list.remove_selected_row());
        assert_eq!(list.items, ["a", "b"]);
        assert_eq!(list.selected(), Some(1));
        assert_eq!(list.epoch, epoch);
        assert_eq!(list.library_positions, None);
    }

    #[test]
    fn sort_leaves_a_failed_list_holding_rows_untouched() {
        let mut list = loaded(&["b", "a"]);
        list.error = Some("boom".to_string());
        let epoch = list.epoch;

        list.sort_rows_by(|item| item.clone());

        assert_eq!(list.items, ["b", "a"]);
        assert_eq!(list.epoch, epoch);
        assert!(list.library_positions.is_none());
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
