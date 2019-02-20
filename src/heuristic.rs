/// The `Heuristic` chooses which side to explore next.
///
/// This is not useful for finding perfect nearest neighbors because
/// it can take a path first that eliminates another better match in
/// another branch. This will work well to find things of a particular
/// distance, which is useful for discrete nearest neighbor searches
/// in a given radius. It is also useful to find all things within a
/// given radius, but the outputs will only be approximately ordered with
/// respect to distance.
///
/// This is cloned right before entering a `side`, so it is expected that
/// `enter` updates the state of the `Heuristic`.
pub trait Heuristic: Clone {
    type Iter: Iterator<Item = bool>;

    /// This is passed the `side`.
    fn enter(&mut self, side: bool);

    /// Must return an iterator which returns values below `16`, otherwise panics.
    fn iter(&self) -> Self::Iter;
}

pub trait IntoHeuristic {
    type Heuristic: Heuristic;

    fn into_heuristic(self) -> Self::Heuristic;
}

impl<H> IntoHeuristic for H
where
    H: Heuristic,
{
    type Heuristic = Self;

    #[inline(always)]
    fn into_heuristic(self) -> Self {
        self
    }
}

/// Chooses whether to enter a path or not.
///
/// Wrap a type with the bound `F: FnMut(bool) -> bool + Clone` and
/// this will implement `Heuristic`. The function will be cloned
/// internally so that from the function's point of view it is being called
/// in the order it descends in. It is passed the side that is being entered
/// and returns whether or not it would like to enter.
///
/// This is useful when looking for items with a discrete distance.
#[derive(Clone)]
pub struct FilterHeuristic<F>(pub F);

impl<F> Heuristic for FilterHeuristic<F>
where
    F: FnMut(bool) -> bool + Clone,
{
    type Iter = FilterHeuristicIter<F>;

    #[inline(always)]
    fn enter(&mut self, side: bool) {
        self.0(side);
    }

    #[inline(always)]
    fn iter(&self) -> Self::Iter {
        FilterHeuristicIter {
            f: self.0.clone(),
            iter: [false, true].iter(),
        }
    }
}

#[doc(hidden)]
pub struct FilterHeuristicIter<F> {
    f: F,
    iter: std::slice::Iter<'static, bool>,
}

impl<F> Iterator for FilterHeuristicIter<F>
where
    F: FnMut(bool) -> bool + Clone,
{
    type Item = bool;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        let f = self.f.clone();
        (&mut self.iter).cloned().find(move |&n| (f.clone())(n))
    }
}

/// Chooses paths to search down.
///
/// Wrap a type with the bound `F: FnMut(bool) -> bool + Clone` and
/// this will implement `Heuristic`. The second argument has to be the first
/// choice. The function will be cloned internally so that from the function's
/// point of view it is being called in the order it descends in. It is passed
/// the side that is being entered and returns which side it would like to
/// enter next.
///
/// This is not particularly useful for most applications, but if you want
/// to search different halves of a binary tree first, this is correct.
/// This could be used to make an approximate nearest-neighbor (ANN) solution,
/// but the quality of the match would then be fixed and depend on which bits
/// differed between two matches (more significant bits differing would throw
/// it out).
#[derive(Clone)]
pub struct SearchHeuristic<F>(pub F, pub bool);

impl<F> Heuristic for SearchHeuristic<F>
where
    F: FnMut(bool) -> bool + Clone,
{
    type Iter = std::iter::Cloned<std::slice::Iter<'static, bool>>;

    #[inline(always)]
    fn enter(&mut self, side: bool) {
        self.1 = self.0(side);
    }

    #[inline(always)]
    fn iter(&self) -> Self::Iter {
        if self.1 {
            [true, false].iter().cloned()
        } else {
            [false, true].iter().cloned()
        }
    }
}
