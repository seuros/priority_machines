use crate::Refusal;

/// A lane index proven to be one of `N` lanes.
///
/// [`crate::WeightedRoundRobin::select`] hands one back, so an adapter that
/// keeps it can call [`crate::WeightedRoundRobin::deactivate`] without an
/// index the scheduler could refuse. Any other index is checked once, by
/// [`Lane::new`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Lane<const N: usize>(
    /// Below `N`. Inside the crate it is built directly only from a
    /// scheduler's cursor, which is always one of its lanes.
    pub(crate) usize,
);

impl<const N: usize> Lane<N> {
    /// The lane at `index`.
    ///
    /// # Errors
    ///
    /// [`Refusal::UnknownLane`] for an index of `N` or more.
    pub const fn new(index: usize) -> Result<Self, Refusal> {
        if index < N {
            Ok(Self(index))
        } else {
            Err(Refusal::UnknownLane)
        }
    }

    /// The index, below `N`.
    #[must_use]
    pub const fn get(self) -> usize {
        self.0
    }
}

impl<const N: usize> From<Lane<N>> for usize {
    fn from(lane: Lane<N>) -> Self {
        lane.get()
    }
}

#[cfg(test)]
mod tests;
