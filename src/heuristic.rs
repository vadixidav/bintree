/// The `Heuristic` chooses which of the `16` groups to explore next.
/// This is useful for searching spaces for nearest neighbors because you can
/// check the nearest bits first.
///
/// This is cloned right before entering a `group`, so it is expected that
/// `choose` update the state of the `Heuristic`.
pub trait Heuristic: Clone {
    type Iter: Iterator<Item = usize>;

    /// This is passed the `group` (guaranteed to be less than `16`).
    fn enter(&mut self, group: usize);

    /// Must return an iterator which returns values below `16`, otherwise panics.
    fn iter(&self) -> Self::Iter;
}

/// This is the same as `Heuristic` except that the returned group indices
/// are unchecked. It is therefore unsafe to implement. See the documentation
/// for `Heuristic`.
pub unsafe trait UncheckedHeuristic: Clone {
    type UncheckedIter: Iterator<Item = usize>;
    /// This is passed the `group` (guaranteed to be less than `16`).
    fn enter_unchecked(&mut self, group: usize);

    /// Must return an iterator which returns values below `16`, otherwise panics.
    fn iter_unchecked(&self) -> Self::UncheckedIter;
}

unsafe impl<T> UncheckedHeuristic for T
where
    T: Heuristic,
{
    type UncheckedIter = std::iter::Inspect<<Self as Heuristic>::Iter, fn(&usize)>;

    #[inline(always)]
    fn enter_unchecked(&mut self, group: usize) {
        // Needs no special checks.
        self.enter(group);
    }

    #[inline(always)]
    fn iter_unchecked(&self) -> Self::UncheckedIter {
        self.iter().inspect(|&g| assert!(g < 16))
    }
}

pub trait IntoHeuristic {
    type Heuristic: UncheckedHeuristic;

    fn into_heuristic(self) -> Self::Heuristic;
}

impl<H> IntoHeuristic for H
where
    H: UncheckedHeuristic,
{
    type Heuristic = Self;

    #[inline(always)]
    fn into_heuristic(self) -> Self {
        self
    }
}

/// Wrap a type with the bound `F: FnMut(usize) -> bool + Clone` and
/// this will implement `UncheckedHeuristic`. The function will be cloned
/// internally so that from the function's point of view it is being called
/// in the order it descends in. It is passed the group that is being entered
/// and returns whether or not it would like to enter.
#[derive(Clone)]
pub struct FnHeuristic<F>(pub F);

unsafe impl<F> UncheckedHeuristic for FnHeuristic<F>
where
    F: FnMut(usize) -> bool + Clone,
{
    type UncheckedIter = FnHeuristicIter<F>;

    #[inline(always)]
    fn enter_unchecked(&mut self, group: usize) {
        self.0(group);
    }

    #[inline(always)]
    fn iter_unchecked(&self) -> Self::UncheckedIter {
        FnHeuristicIter {
            f: self.0.clone(),
            iter: 0..16,
        }
    }
}

#[doc(hidden)]
pub struct FnHeuristicIter<F> {
    f: F,
    iter: std::ops::Range<usize>,
}

impl<F> Iterator for FnHeuristicIter<F>
where
    F: FnMut(usize) -> bool + Clone,
{
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let f = self.f.clone();
        (&mut self.iter).find(move |&n| (f.clone())(n))
    }
}
