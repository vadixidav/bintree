mod heuristic;

pub use heuristic::*;

const HIGH: u32 = 0x8000_0000;

use std::slice;

/// Contains a list of 2 children node IDs.
///
/// Each child ID's highest bit indicates if it is an internal node or a
/// leaf node.
///
/// If a child is `0` then it is empty because the root node can never be pointed to.
#[derive(Copy, Clone, Debug, Default)]
struct Internal([u32; 2]);

#[derive(Clone, Debug)]
pub struct BinTrie {
    /// The root node is always at index `0`.
    internals: Vec<Internal>,
    /// The maximum depth to stop at.
    depth: u32,
}

impl BinTrie {
    /// Makes a new trie with a maximum `depth` of `8192`.
    ///
    /// ```
    /// # use bintrie::BinTrie;
    /// let trie = BinTrie::new();
    /// ```
    pub fn new() -> Self {
        Default::default()
    }

    /// Makes a new trie with a given maximum `depth`.
    ///
    /// ```
    /// # use bintrie::BinTrie;
    /// let trie = BinTrie::new_depth(128);
    /// ```
    pub fn new_depth(depth: u32) -> Self {
        assert!(depth > 0);
        Self {
            internals: vec![Internal::default()],
            depth,
        }
    }

    /// Inserts a number that does not have the most significant bit set.
    ///
    /// `K(n)` - A function that provides the `n`th bit for the key.
    /// `F(item, n)` - A function that must be able to look up the nth bit  
    ///    from a previously inserted item.
    ///
    /// Returns `Some` of a replaced leaf if a leaf was replaced, otherwise None.
    ///
    /// ```
    /// # use bintrie::BinTrie;
    /// let mut trie = BinTrie::new();
    /// // Note that the item, the key, and the lookup key all obey the
    /// // unsafe requirements.
    /// trie.insert(5, |_| false, |_, _| false);
    /// assert_eq!(trie.items().collect::<Vec<u32>>(), vec![5]);
    /// ```
    #[inline(always)]
    pub fn insert<K, F>(&mut self, item: u32, mut key: K, mut lookup: F) -> Option<u32>
    where
        K: FnMut(u32) -> bool,
        F: FnMut(u32, u32) -> bool,
    {
        // Always check that the high bit is not set in the item.
        assert!(item & HIGH == 0);
        // This unsafe block is only used to allow indexing [u32; 2] by a `1` or `0`.
        unsafe {
            let mut index = 0;
            for i in 0..self.depth - 1 {
                let position = if key(i) { 1 } else { 0 };
                match *self
                    .internals
                    .get_unchecked(index)
                    .0
                    .get_unchecked(position)
                {
                    // Empty node encountered.
                    0 => {
                        // Insert the item in the empty spot, making sure to set
                        // its most significant bit to indicate it is a leaf.
                        *self
                            .internals
                            .get_unchecked_mut(index)
                            .0
                            .get_unchecked_mut(position) = item | HIGH;
                        // That's it.
                        return None;
                    }
                    // Leaf node encountered.
                    m if m & HIGH != 0 => {
                        // Make an empty node.
                        let mut new_internal = Internal::default();
                        // Add the existing `m` to its proper location.
                        *new_internal
                            .0
                            .get_unchecked_mut(if lookup(m & !HIGH, i + 1) { 1 } else { 0 }) = m;
                        // Get the index of the next internal node.
                        let new_index = self.internals.len() as u32;
                        // Panic if we go too high to fit in our indices.
                        assert!(new_index & HIGH == 0);
                        // Insert the new internal node onto the internals vector.
                        self.internals.push(new_internal);
                        // Insert the new index to the parent node.
                        *self
                            .internals
                            .get_unchecked_mut(index)
                            .0
                            .get_unchecked_mut(position) = new_index;
                        // Fallthrough to the next iteration where it will either
                        // be expanded or hit the empty leaf node position.
                        index = new_index as usize;
                    }
                    // Internal node encountered.
                    m => {
                        // Move to the internal node.
                        index = m as usize;
                    }
                }
            }

            // For the last bit we only handle the case that we can insert it.
            // If something occupies the space we replace it and return it.
            let position = if key(self.depth - 1) { 1 } else { 0 };
            let spot = self
                .internals
                .get_unchecked_mut(index)
                .0
                .get_unchecked_mut(position);
            let old = *spot;
            *spot = item | HIGH;
            // Check if it was not an empty node.
            if old != 0 {
                // Return the item that was replaced.
                Some(old & !HIGH)
            } else {
                None
            }
        }
    }

    /// Perform a lookup for a particular item.
    ///
    /// `K(n)` - A function that provides the `n`th bit for the key.
    ///
    /// ```
    /// # use bintrie::BinTrie;
    /// let mut trie = BinTrie::new();
    /// let key = |_| false;
    /// let lookup = |_, _| false;
    /// trie.insert(5, key, lookup);
    /// assert_eq!(trie.get(key), Some(5));
    /// assert_eq!(trie.get(|_| true), None);
    /// ```
    #[inline(always)]
    pub fn get<K>(&self, mut key: K) -> Option<u32>
    where
        K: FnMut(u32) -> bool,
    {
        // This unsafe block is only used to allow indexing [u32; 2] by a `1` or `0`.
        unsafe {
            let mut index = 0;
            for i in 0..self.depth {
                match *self
                    .internals
                    .get_unchecked(index)
                    .0
                    .get_unchecked(if key(i) { 1 } else { 0 })
                {
                    // Empty node encountered.
                    0 => {
                        return None;
                    }
                    // Leaf node encountered.
                    m if m & HIGH != 0 => return Some(m & !HIGH),
                    // Internal node encountered.
                    m => {
                        // Move to the internal node.
                        index = m as usize;
                    }
                }
            }
            None
        }
    }

    /// Get an iterator over the items added to the trie.
    ///
    /// ```
    /// # use bintrie::BinTrie;
    /// let mut trie = BinTrie::new();
    /// trie.insert(3, |_| false, |_, _| false);
    /// assert_eq!(trie.items().collect::<Vec<u32>>(), vec![3]);
    /// ```
    pub fn items<'a>(&'a self) -> impl Iterator<Item = u32> + 'a {
        Iter::new(self)
    }

    /// Iterates over the trie while using the `heuristic` to guide iteration.
    ///
    /// This can be used to limit the search space or to guide the search space
    /// for a fast constant distance or other spatial heuristic search. This is
    /// not capable of directly outputting kNN, and would need to be combined
    /// with either a heuristic search that gets everything below a discrete
    /// distance and then sorts the output or a search that gets items
    /// with a discrete distance and iterates over each distance desired.
    ///
    /// `heuristic` must implement `IntoHeuristic`, which the normal
    /// `Heuristic` trait satisfies.
    ///
    /// ```
    /// # use bintrie::{BinTrie, FilterHeuristic};
    /// let mut trie = BinTrie::new();
    /// let lookup = |n, l| match n {
    ///     3 => false,
    ///     5 => if l == 1 { true } else { false },
    ///     7 => if l == 1 { false } else { true },
    ///     _ => true,
    /// };
    /// trie.insert(3, |n| lookup(3, n), lookup);
    /// trie.insert(5, |n| lookup(5, n), lookup);
    /// trie.insert(7, |n| lookup(7, n), lookup);
    /// assert_eq!(trie.explore(FilterHeuristic(|n| n)).collect::<Vec<u32>>(), vec![7]);
    /// let mut level = 0;
    /// // Try and find the 5.
    /// assert_eq!(trie.explore(FilterHeuristic(move |n: bool| {
    ///     level += 1;
    ///     match level {
    ///         // Go left.
    ///         1 => !n,
    ///         // Then go right.
    ///         2 => n,
    ///         _ => false,
    ///     }
    /// })).collect::<Vec<u32>>(), vec![5]);
    /// ```
    pub fn explore<'a, H>(&'a self, heuristic: H) -> impl Iterator<Item = u32> + 'a
    where
        H: IntoHeuristic,
        H::Heuristic: 'a,
    {
        ExploreIter::new(self, heuristic.into_heuristic())
    }
}

impl Default for BinTrie {
    fn default() -> Self {
        Self {
            internals: vec![Internal::default()],
            depth: 8192,
        }
    }
}

struct Iter<'a> {
    trie: &'a BinTrie,
    indices: Vec<slice::Iter<'a, u32>>,
}

impl<'a> Iter<'a> {
    fn new(trie: &'a BinTrie) -> Self {
        Self {
            trie,
            indices: vec![trie.internals[0].0.iter()],
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = u32;
    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Get the current slice. If there is none, then we return `None`.
            let mut current = self.indices.pop()?;
            // Get the next item in the slice or continue the loop if its empty.
            let n = if let Some(n) = current.next() {
                // Push the slice back.
                self.indices.push(current);
                n
            } else {
                continue;
            };
            // Check what kind of node it is.
            match n {
                // Empty node
                0 => {}
                // Leaf node
                n if n & HIGH != 0 => {
                    return Some(n & !HIGH);
                }
                // Internal node
                &n => self.indices.push(self.trie.internals[n as usize].0.iter()),
            }
        }
    }
}

struct ExploreIter<'a, H>
where
    H: Heuristic,
{
    trie: &'a BinTrie,
    indices: Vec<(&'a [u32; 2], H, H::Iter)>,
}

impl<'a, H> ExploreIter<'a, H>
where
    H: Heuristic,
{
    fn new(trie: &'a BinTrie, heuristic: H) -> Self {
        let iter = heuristic.iter();
        Self {
            trie,
            indices: vec![(&trie.internals[0].0, heuristic, iter)],
        }
    }
}

impl<'a, H> Iterator for ExploreIter<'a, H>
where
    H: Heuristic,
{
    type Item = u32;
    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Get the current array, heuristic, and iter.
            // If there is none, then we return `None`.
            let (array, heuristic, mut iter) = self.indices.pop()?;
            // Clone the heuristic before we put it back so we can
            // use it when descending further.
            let mut next_heuristic = heuristic.clone();
            // Get the next item in the array or continue the loop if its empty.
            let (choice, n) = if let Some(choice) = iter.next() {
                let n = unsafe { array.get_unchecked(if choice { 1 } else { 0 }) };
                // Push the state back.
                self.indices.push((array, heuristic, iter));
                (choice, n)
            } else {
                continue;
            };
            // Check what kind of node it is.
            match n {
                // Empty node
                0 => {}
                // Leaf node
                n if n & HIGH != 0 => {
                    return Some(n & !HIGH);
                }
                // Internal node
                &n => {
                    next_heuristic.enter(choice);
                    let iter = next_heuristic.iter();
                    self.indices
                        .push((&self.trie.internals[n as usize].0, next_heuristic, iter))
                }
            }
        }
    }
}
