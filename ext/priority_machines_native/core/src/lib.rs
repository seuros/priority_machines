#![no_std]
#![cfg_attr(not(test), deny(clippy::indexing_slicing))]
#![doc = include_str!("../README.md")]

mod deficit;
mod edf;
mod lane;
mod refusal;
mod round_robin;
#[cfg(test)]
mod test_support;
mod weighted;

pub use deficit::{DeficitRoundRobin, Head};
pub use edf::earliest_deadline;
pub use lane::Lane;
pub use refusal::{Deactivation, Refusal};
pub use round_robin::RoundRobin;
pub use weighted::{Ready, WeightedRoundRobin};

/// The lane after `lane` among `lanes`, wrapping to zero.
///
/// Every caller passes `lane < lanes`, so `lane + 1` cannot overflow.
const fn next_lane(lane: usize, lanes: usize) -> usize {
    if lane + 1 == lanes { 0 } else { lane + 1 }
}

/// The entry at `index`, `None` past the end. `slice::get` is not a const
/// fn yet; this is, so const methods read without indexing.
const fn entry<T>(items: &[T], index: usize) -> Option<&T> {
    match items.split_at_checked(index) {
        Some((_, [item, ..])) => Some(item),
        _ => None,
    }
}

/// [`entry`], mutable.
const fn entry_mut<T>(items: &mut [T], index: usize) -> Option<&mut T> {
    match items.split_at_mut_checked(index) {
        Some((_, [item, ..])) => Some(item),
        _ => None,
    }
}
