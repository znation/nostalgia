//! The row-order operations of a [`BrowseList`]: Sort, reverse, shuffle, move
//! one place, and the restore to library order, plus the permutation helpers
//! they share.
//!
//! Each reorder is one-shot over the loaded rows and their cached titles and
//! labels: it bumps the list's epoch because a rendered row's index now names
//! a different entry, and it composes into the permutation
//! [`BrowseList::restore_library_order`] later inverts. The fetch, selection,
//! loading, and failure state machine lives in the parent module.

use super::BrowseList;

/// Applies the permutation `order` to `values` in place: entry `i` takes the
/// value that was at `order[i]`. Each cycle is walked once, swapping the
/// values into place, so no element is cloned; `visited` is scratch marking
/// the slots already placed and is reset before each buffer. The caller passes
/// the same `order` for the rows and their cached text, so all three stay
/// index-aligned.
///
/// `order` must be a permutation of `0..values.len()`; every caller builds one
/// (a stable sort's index order, a reversal, [`shuffled_order`], or the
/// composition/inverse of those in [`BrowseList::apply_reorder`] and
/// [`BrowseList::restore_library_order`]).
fn apply_permutation<T>(values: &mut [T], order: &[usize], visited: &mut [bool]) {
    debug_assert_eq!(values.len(), order.len());
    debug_assert_eq!(values.len(), visited.len());
    visited.fill(false);
    for start in 0..values.len() {
        if visited[start] {
            continue;
        }
        let mut current = start;
        loop {
            visited[current] = true;
            let source = order[current];
            if source == start {
                break;
            }
            values.swap(current, source);
            current = source;
        }
    }
}

/// A permutation of `0..len` from a Fisher–Yates shuffle driven by splitmix64
/// seeded with `seed`. Pure: the same `len` and `seed` always give the same
/// order, and `len` 0 and 1 give `[]` and `[0]`.
pub(in crate::ui) fn shuffled_order(len: usize, seed: u64) -> Vec<usize> {
    let mut state = seed;
    let mut draw = move || {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    };
    let mut order: Vec<usize> = (0..len).collect();
    for i in (1..len).rev() {
        let pick = draw().checked_rem(i as u64 + 1).unwrap_or(0) as usize;
        order.swap(i, pick);
    }
    order
}

impl<T> BrowseList<T> {
    /// Reorders the loaded rows in place by `key`, comparing each key
    /// lowercased so the order ignores case, and keeps library order for keys
    /// that compare equal (the sort is stable).
    ///
    /// The cached row titles and labels are reordered by the same permutation
    /// (see [`Self::reorder_rows`]) whenever they are populated, so a
    /// reordered song list still shows each track's own title and length. The
    /// epoch is bumped like
    /// [`Self::replace_rows`] because the indices a rendered row emits now name
    /// different entries; `loading`, `error`, and the request generation are
    /// left alone, and a list with fewer than two rows or one still loading or
    /// failed is a no-op.
    pub(in crate::ui) fn sort_rows_by(&mut self, key: impl Fn(&T) -> String) {
        self.reorder_with(|items| {
            let keys: Vec<String> = items.iter().map(|item| key(item).to_lowercase()).collect();
            let mut order: Vec<usize> = (0..items.len()).collect();
            order.sort_by(|&left, &right| keys[left].cmp(&keys[right]));
            order
        });
    }

    /// Reverses the loaded rows in place, one-shot like [`Self::sort_rows_by`]:
    /// the cached titles and labels follow the rows, the epoch is bumped, the
    /// selection is dropped, and the reversal composes into the permutation
    /// [`Self::restore_library_order`] undoes. A list with fewer than two
    /// rows or one still loading or failed is a no-op.
    pub(in crate::ui) fn reverse_rows(&mut self) {
        self.reorder_with(|items| (0..items.len()).rev().collect());
    }

    /// Shuffles the loaded rows in place by [`shuffled_order`]`(len, seed)`,
    /// one-shot like [`Self::reverse_rows`]: cached titles and labels follow
    /// the rows, the epoch is bumped (even when the permutation happens to be
    /// the identity), the selection is dropped, and the shuffle composes into
    /// the permutation [`Self::restore_library_order`] undoes. A list with
    /// fewer than two rows or one still loading or failed is a no-op.
    pub(in crate::ui) fn shuffle_rows(&mut self, seed: u64) {
        self.reorder_with(|items| shuffled_order(items.len(), seed));
    }

    /// Moves the selected row one place up (`down` false) or down, keeping it
    /// selected at its new index so repeated calls keep moving it. One-shot
    /// like [`Self::reverse_rows`]: cached titles and labels follow the rows,
    /// the epoch is bumped, and the swap composes with [`Self::sort_rows_by`],
    /// [`Self::reverse_rows`], and [`Self::shuffle_rows`] into the permutation
    /// [`Self::restore_library_order`] undoes. With nothing selected, the
    /// selected row already at the edge it would move past, or the list
    /// loading or failed, nothing changes.
    ///
    /// The move swaps the row and its cached title and label in place, and
    /// swaps the two entries in the saved library permutation, rather than
    /// rebuilding the whole list through a permutation like Sort and Shuffle
    /// do. Only the first move after a store allocates the identity
    /// permutation it then keeps in step; every later move of a held key is a
    /// constant number of swaps, so moving a row in a large library does not
    /// deep-copy every row's strings on each key repeat.
    pub(in crate::ui) fn move_selected(&mut self, down: bool) {
        let Some(from) = self.selected else {
            return;
        };
        let to = if down {
            from.saturating_add(1)
        } else {
            match from.checked_sub(1) {
                Some(to) => to,
                None => return,
            }
        };
        if self.loading || self.error.is_some() || to >= self.items.len() {
            return;
        }
        self.items.swap(from, to);
        if !self.titles.is_empty() {
            self.titles.swap(from, to);
        }
        if !self.labels.is_empty() {
            self.labels.swap(from, to);
        }
        match &mut self.library_positions {
            Some(positions) => positions.swap(from, to),
            None => {
                let mut positions: Vec<usize> = (0..self.items.len()).collect();
                positions.swap(from, to);
                self.library_positions = Some(positions);
            }
        }
        // The moved row is now at `to`, and the epoch bump invalidates every
        // press rendered from the old order.
        self.selected = Some(to);
        self.epoch = self.epoch.wrapping_add(1);
    }

    /// Applies the permutation `order` computes from the rows, unless the list
    /// has fewer than two rows or is still loading or failed, in which case
    /// nothing changes.
    fn reorder_with(&mut self, order: impl FnOnce(&[T]) -> Vec<usize>) {
        // A loading list's rows are being replaced by a fetch, so reordering
        // them would be undone (or raced) by the reply; the documented
        // guarantee is that Sort while loading changes nothing. The app's
        // transitions empty the rows in `clear` before setting `loading`, but
        // this check pins the guarantee independently of that ordering. The
        // same holds for a failed list, whose rows `fail` clears today.
        if self.items.len() < 2 || self.loading || self.error.is_some() {
            return;
        }
        let order = order(&self.items);
        self.apply_reorder(order);
    }

    /// Applies the permutation `order` (entry `i` names the current row that
    /// moves to position `i`) and records it for [`Self::restore_library_order`].
    fn apply_reorder(&mut self, order: Vec<usize>) {
        // Keep the total permutation back to library order: a later reorder
        // composes with it instead of overwriting it, so restoring after any
        // number of reorders still yields the order the rows were stored in.
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

    /// Undoes every [`Self::sort_rows_by`], [`Self::reverse_rows`], [`Self::shuffle_rows`],
    /// and [`Self::move_selected`] since the rows were stored, putting `items` (and
    /// the cached titles and labels) back in library order and
    /// bumping the epoch because the indices a rendered row emits now name
    /// other entries. A list that is not sorted — or whose rows have since
    /// been replaced — is a no-op, so callers can restore unconditionally.
    pub(in crate::ui) fn restore_library_order(&mut self) {
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
    /// each row's cached title and label beside it. [`Self::apply_reorder`] and
    /// [`Self::restore_library_order`] both go through this, so neither can
    /// forget one of the caches.
    ///
    /// The rows move through the permutation's cycles in place. The earlier
    /// rebuild cloned every row and both cached strings, so a sort or Back's
    /// restore of a whole-library list deep-copied every owned field (each
    /// song's id, title, artist, album, and preview URL) just to reorder it.
    fn reorder_rows(&mut self, order: &[usize]) {
        let mut visited = vec![false; order.len()];
        apply_permutation(&mut self.items, order, &mut visited);
        if !self.titles.is_empty() {
            apply_permutation(&mut self.titles, order, &mut visited);
        }
        if !self.labels.is_empty() {
            apply_permutation(&mut self.labels, order, &mut visited);
        }
    }
}
