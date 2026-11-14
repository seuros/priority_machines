//! Helpers shared by the crate's test modules.

use core::{fmt::Debug, num::NonZeroU32};

use crate::{Deactivation, Refusal};

/// A positive test amount; zero is a broken test assumption.
pub fn nonzero(value: u32) -> NonZeroU32 {
    let Some(value) = NonZeroU32::new(value) else {
        panic!("test amounts are positive");
    };
    value
}

/// xorshift64: a fixed seed keeps a trace reproducible.
pub struct Rng(pub u64);

impl Rng {
    /// The next draw, in `0..bound`.
    pub fn below(&mut self, bound: u32) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        let Ok(draw) = u32::try_from(self.0 >> 40) else {
            panic!("24 bits fit");
        };
        draw % bound
    }
}

impl Rng {
    /// The next draw as an index, in `0..bound`.
    pub fn index(&mut self, bound: u32) -> usize {
        let Ok(index) = usize::try_from(self.below(bound)) else {
            panic!("a u32 index fits a usize");
        };
        index
    }
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
