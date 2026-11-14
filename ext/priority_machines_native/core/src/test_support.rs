//! Helpers shared by the crate's test modules.

use core::{fmt::Debug, num::NonZeroU32};

use matryona_machines::testing::XorShift64;

use crate::{Deactivation, Refusal};

/// A positive test amount; zero is a broken test assumption.
pub fn nonzero(value: u32) -> NonZeroU32 {
    let Some(value) = NonZeroU32::new(value) else {
        panic!("test amounts are positive");
    };
    value
}

/// The next draw of `rng`, in `0..bound`.
pub fn below(rng: &mut XorShift64, bound: u32) -> u32 {
    let Ok(draw) = u32::try_from(rng.below(u64::from(bound))) else {
        panic!("a draw below a u32 fits a u32");
    };
    draw
}

/// The next draw of `rng` as an index, in `0..bound`.
pub fn index(rng: &mut XorShift64, bound: u32) -> usize {
    let Ok(index) = usize::try_from(below(rng, bound)) else {
        panic!("a u32 index fits a usize");
    };
    index
}

/// What a randomized run's deactivations answered. A refusal and an honoured
/// repeat must leave the scheduler exactly as it was; a deactivation must not.
#[derive(Default)]
pub struct Tally {
    pub refusals: u32,
    pub repeats: u32,
}

impl Tally {
    pub fn record<T: PartialEq + Debug>(
        &mut self,
        outcome: Result<Deactivation, Refusal>,
        after: &T,
        before: &T,
    ) {
        match outcome {
            Err(Refusal::UnknownLane) => {
                assert_eq!(after, before, "a refusal changed the scheduler");
                self.refusals += 1;
            }
            Ok(Deactivation::Idle) => {
                assert_eq!(after, before, "an honoured repeat changed the scheduler");
                self.repeats += 1;
            }
            Ok(Deactivation::Deactivated) => assert_ne!(after, before),
        }
    }
}
