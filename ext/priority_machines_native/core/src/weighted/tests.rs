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
    assert!(scheduler.deactivate(0));
    assert_eq!(scheduler.select(&[true; 2]), Some(1));
    let before = scheduler.clone();
    assert!(!scheduler.deactivate(2));
    assert_eq!(scheduler, before);
}

#[test]
fn zero_lanes_and_maximum_weight_are_supported() {
    let mut empty = WeightedRoundRobin::new([]);
    assert_eq!(empty.select(&[]), None);
    assert!(!empty.deactivate(0));

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
