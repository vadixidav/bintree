const HIGH: u32 = 0x8000_0000;

use std::slice;

/// Contains a list of 16 children node IDs.
///
/// `16 * 32` (`512`) bits (`64` bytes) is the size of cache lines in Intel
/// processors. This struct attempts to take advantage of that.
///
/// Each child ID's highest bit indicates if it is an internal node or a
/// leaf node.
///
/// If a child is `0` then it is empty because the root node can never be pointed to.
#[derive(Copy, Clone, Debug, Default)]
struct Internal([u32; 16]);

#[derive(Clone, Debug)]
pub struct BinTrie {
    /// The root node is always at index `0` to simplify things.
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
    /// `K(n)` - A function that provides the `n`th group of `4` bits for the
    ///    key.
    /// `F(item, n)` - A function that must be able to look up the nth group
    ///    of `4` bits from a previously inserted `u32`.
    ///
    /// ```
    /// # use bintrie::BinTrie;
    /// let mut trie = BinTrie::new();
    /// // Note that the item, the key, and the lookup key all obey the
    /// // unsafe requirements.
    /// trie.insert(5, |_| 0, |_, _| 0);
    /// assert_eq!(trie.items().collect::<Vec<u32>>(), vec![5]);
    /// ```
    #[inline(always)]
    pub fn insert<K, F>(&mut self, item: u32, key: K, lookup: F)
    where
        K: Fn(u32) -> usize,
        F: Fn(u32, u32) -> usize,
    {
        assert!(item & HIGH == 0);
        unsafe {
            self.insert_unchecked(
                item,
                |n| {
                    let out = key(n);
                    assert!(out < 16);
                    out
                },
                |item, group| {
                    let out = lookup(item, group);
                    assert!(out < 16);
                    out
                },
            );
        }
    }

    /// Inserts a number that does not have the most significant bit set.
    ///
    /// This version is unsafe because it doesn't verify that the output
    /// of `K` and `F` are below `16`. It also doesn't verify that the
    /// `item` doesn't have its most significant bit set. Ensure these
    /// conditions are met before calling this. It still asserts
    /// that there aren't too many internal nodes.
    ///
    /// `K(n)` - A function that provides the `n`th group of `4` bits for the
    ///    key.
    /// `F(item, n)` - A function that must be able to look up the nth group
    ///    of `4` bits from a previously inserted `u32`.
    ///
    /// ```
    /// # use bintrie::BinTrie;
    /// let mut trie = BinTrie::new();
    /// // Note that the item, the key, and the lookup key all obey the
    /// // unsafe requirements.
    /// unsafe {
    ///     trie.insert_unchecked(5, |_| 0, |_, _| 0);
    /// }
    /// assert_eq!(trie.items().collect::<Vec<u32>>(), vec![5]);
    /// ```
    #[inline(always)]
    pub unsafe fn insert_unchecked<K, F>(&mut self, item: u32, key: K, lookup: F)
    where
        K: Fn(u32) -> usize,
        F: Fn(u32, u32) -> usize,
    {
        let mut index = 0;
        for i in 0..self.depth - 1 {
            let position = key(i);
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
                    return;
                }
                // Leaf node encountered.
                m if m & HIGH != 0 => {
                    // Make an empty node.
                    let mut new_internal = Internal::default();
                    // Add the existing `m` to its proper location.
                    *new_internal.0.get_unchecked_mut(lookup(m & !HIGH, i + 1)) = m;
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
                }
                // Internal node encountered.
                m => {
                    // Move to the internal node.
                    index = m as usize;
                }
            }
        }

        // For the last bit we only handle the case that we can insert it.
        // The group position is `depth - 1`.
        let position = key(self.depth - 1);
        // Check if it is a leaf node.
        if *self
            .internals
            .get_unchecked(index)
            .0
            .get_unchecked(position)
            == 0
        {
            // Insert the item in the empty spot, making sure to set
            // its most significant bit to indicate it is a leaf.
            *self
                .internals
                .get_unchecked_mut(index)
                .0
                .get_unchecked_mut(position) = item | HIGH;
        }
    }

    /// Get an iterator over the items added to the trie.
    ///
    /// ```
    /// # use bintrie::BinTrie;
    /// let mut trie = BinTrie::new();
    /// trie.insert(3, |_| 0, |_, _| 0);
    /// let mut items = trie.items();
    /// assert_eq!(trie.items().collect::<Vec<u32>>(), vec![3]);
    /// ```
    pub fn items<'a>(&'a self) -> impl Iterator<Item = u32> + 'a {
        Iter::new(self)
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
