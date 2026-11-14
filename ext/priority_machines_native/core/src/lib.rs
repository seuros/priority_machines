#![no_std]
#![doc = include_str!("../README.md")]

mod deficit;
mod edf;
mod refusal;
mod round_robin;
#[cfg(test)]
mod test_support;
mod weighted;

pub use deficit::DeficitRoundRobin;
pub use edf::earliest_deadline;
pub use refusal::{Deactivation, Refusal};
pub use round_robin::RoundRobin;
pub use weighted::WeightedRoundRobin;

/// The lane after `lane` among `lanes`, wrapping to zero.
const fn next_lane(lane: usize, lanes: usize) -> usize {
    if lane + 1 == lanes { 0 } else { lane + 1 }
}
