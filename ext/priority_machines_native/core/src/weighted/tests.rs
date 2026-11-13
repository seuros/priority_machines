#![allow(clippy::expect_used)] // Failed test assumptions should panic.

use super::*;

fn weight(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("test weight is positive")
}

#[test]
fn serves_weighted_bursts() {
    let mut scheduler = WeightedRoundRobin::new([weight(3), weight(1), weight(2)]);
    let turns: [_; 12] = core::array::from_fn(|_| scheduler.select(&[true; 3]));
    assert_eq!(
        turns,
        [
            Some(0),
            Some(0),
            Some(0),
            Some(1),
            Some(2),
            Some(2),
            Some(0),
            Some(0),
            Some(0),
            Some(1),
            Some(2),
            Some(2),
        ],
    );
}

#[test]
fn skips_unready_lanes_and_forfeits_residual_turns() {
    let mut scheduler = WeightedRoundRobin::new([weight(3), weight(1)]);
    assert_eq!(scheduler.select(&[true, true]), Some(0));
    assert_eq!(scheduler.select(&[false, true]), Some(1));
    assert_eq!(scheduler.select(&[true, true]), Some(0));
    assert_eq!(scheduler.remaining, 2);
}

#[test]
fn idle_lanes_do_not_bank_turns() {
    let mut scheduler = WeightedRoundRobin::new([weight(2), weight(1)]);
    assert_eq!(scheduler.select(&[true, true]), Some(0));
    for _ in 0..10 {
        assert_eq!(scheduler.select(&[false, false]), None);
    }
    let turns: [_; 3] = core::array::from_fn(|_| scheduler.select(&[true; 2]));
    assert_eq!(turns, [Some(0), Some(0), Some(1)]);
}

#[test]
fn deactivation_relinquishes_a_partial_burst() {
    let mut scheduler = WeightedRoundRobin::new([weight(3), weight(1)]);
    assert_eq!(scheduler.select(&[true; 2]), Some(0));
    assert_eq!(scheduler.deactivate(0), Ok(Deactivation::Deactivated));
    assert_eq!(scheduler.select(&[true; 2]), Some(1));
    let before = scheduler.clone();
    assert_eq!(scheduler.deactivate(2), Err(Refusal::UnknownLane));
    assert_eq!(scheduler.deactivate(usize::MAX), Err(Refusal::UnknownLane));
    assert_eq!(scheduler, before);
}

#[test]
fn zero_lanes_and_maximum_weight_are_supported() {
    let mut empty = WeightedRoundRobin::new([]);
    assert_eq!(empty.select(&[]), None);
    assert_eq!(empty.deactivate(0), Err(Refusal::UnknownLane));

    let mut single = WeightedRoundRobin::new([weight(u32::MAX)]);
    assert_eq!(single.select(&[true]), Some(0));
    assert_eq!(single.remaining, u32::MAX - 1);
    assert_eq!(single.select(&[false]), None);
    assert_eq!(single.remaining, u32::MAX);
}

#[test]
fn full_cycles_match_weight_ratios() {
    for first in 1..=5 {
        for second in 1..=5 {
            let mut scheduler = WeightedRoundRobin::new([weight(first), weight(second)]);
            let mut counts = [0; 2];
            for _ in 0..10 * (first + second) {
                let lane = scheduler.select(&[true; 2]).expect("both lanes are ready");
                counts[lane] += 1;
            }
            assert_eq!(counts, [10 * first, 10 * second]);
        }
    }
}

#[test]
fn a_repeated_deactivation_is_honoured_and_changes_nothing() {
    let mut scheduler = WeightedRoundRobin::new([weight(3), weight(2)]);
    assert_eq!(scheduler.select(&[true; 2]), Some(0));
    assert_eq!(scheduler.deactivate(0), Ok(Deactivation::Deactivated));
    let before = scheduler.clone();
    assert_eq!(scheduler.deactivate(0), Ok(Deactivation::Idle));
    assert_eq!(scheduler, before);

    // An only lane with an untouched burst holds nothing to give up.
    let mut single = WeightedRoundRobin::new([weight(2)]);
    let before = single.clone();
    assert_eq!(single.deactivate(0), Ok(Deactivation::Idle));
    assert_eq!(single, before);
    assert_eq!(single.select(&[true]), Some(0));
    assert_eq!(single.deactivate(0), Ok(Deactivation::Deactivated));
    assert_eq!(single, before);
}

#[test]
fn nothing_ready_forfeits_the_burst_and_keeps_the_cursor() {
    let mut scheduler = WeightedRoundRobin::new([weight(3), weight(2)]);
    assert_eq!(scheduler.select(&[true; 2]), Some(0));
    assert_eq!(scheduler.select(&[false, true]), Some(1));
    let mut expected = scheduler.clone();
    expected.remaining = 2;
    assert_eq!(scheduler.select(&[false; 2]), None);
    assert_eq!(scheduler, expected);
    assert_eq!(scheduler.select(&[false; 2]), None);
    assert_eq!(scheduler, expected);
}

#[test]
fn refusals_describe_themselves() {
    extern crate std;
    use std::string::ToString;

    assert_eq!(
        Refusal::UnknownLane.to_string(),
        "lane index is not one of the scheduler's lanes"
    );
}

/// Textbook burst WRR, kept apart from the implementation's bookkeeping.
#[derive(Clone, PartialEq, Eq, Debug)]
struct Model {
    weights: [u32; 3],
    lane: usize,
    left: u32,
}

impl Model {
    const fn pass(&mut self) {
        self.lane = (self.lane + 1) % 3;
        self.left = self.weights[self.lane];
    }

    fn select(&mut self, ready: [bool; 3]) -> Option<usize> {
        for _ in 0..3 {
            let lane = self.lane;
            if ready[lane] {
                self.left -= 1;
                if self.left == 0 {
                    self.pass();
                }
                return Some(lane);
            }
            self.pass();
        }
        None
    }

    fn deactivate(&mut self, lane: usize) -> Result<Deactivation, Refusal> {
        if lane >= 3 {
            return Err(Refusal::UnknownLane);
        }
        // Mid-burst: under the cursor and already served a turn this visit.
        if lane != self.lane || self.left == self.weights[lane] {
            return Ok(Deactivation::Idle);
        }
        self.pass();
        Ok(Deactivation::Deactivated)
    }
}

/// xorshift64: a fixed seed keeps the trace reproducible.
fn next(state: &mut u64) -> usize {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    usize::try_from(*state >> 40).expect("24 bits fit")
}

#[test]
fn randomized_trace_matches_a_reference_model() {
    let mut seed = 0x4215_4215_4215_4215;
    let weights = [3, 1, 2];
    let mut scheduler = WeightedRoundRobin::new(weights.map(weight));
    let mut model = Model {
        weights,
        lane: 0,
        left: weights[0],
    };
    let (mut refusals, mut repeats) = (0, 0);
    for _ in 0..20_000 {
        let before = scheduler.clone();
        if next(&mut seed).is_multiple_of(4) {
            let lane = next(&mut seed) % 5;
            let outcome = scheduler.deactivate(lane);
            assert_eq!(outcome, model.deactivate(lane));
            match outcome {
                Err(Refusal::UnknownLane) => {
                    assert_eq!(scheduler, before);
                    refusals += 1;
                }
                Ok(Deactivation::Idle) => {
                    assert_eq!(scheduler, before);
                    repeats += 1;
                }
                Ok(Deactivation::Deactivated) => assert_ne!(scheduler, before),
            }
        } else {
            let bits = next(&mut seed);
            let ready = core::array::from_fn(|lane| bits >> lane & 1 == 1);
            assert_eq!(scheduler.select(&ready), model.select(ready));
        }
        assert_eq!(scheduler.cursor, model.lane);
        assert_eq!(scheduler.remaining, model.left);
    }
    assert!(refusals > 500 && repeats > 500);
}

#[test]
fn deactivating_lanes_that_hold_nothing_never_moves_the_cursor() {
    let mut scheduler = WeightedRoundRobin::new([weight(3), weight(2)]);
    let fresh = scheduler.clone();
    for lane in [0, 0, 1, 0, 1, 1] {
        assert_eq!(scheduler.deactivate(lane), Ok(Deactivation::Idle));
        assert_eq!(scheduler, fresh);
    }
    // The lane under the cursor kept its place and its whole burst.
    let turns: [_; 3] = core::array::from_fn(|_| scheduler.select(&[true; 2]));
    assert_eq!(turns, [Some(0); 3]);
}
