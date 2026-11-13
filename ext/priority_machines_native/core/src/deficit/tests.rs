#![allow(clippy::expect_used)] // Failed test assumptions should panic.

use super::*;

mod contract;

fn units(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("test units are positive")
}

#[test]
fn quantum_pays_for_multiple_heads_in_one_visit() {
    let mut scheduler = DeficitRoundRobin::new([units(4), units(2)]);
    let heads = [Some(units(2)); 2];
    let turns: [_; 6] = core::array::from_fn(|_| scheduler.select(&heads));
    assert_eq!(
        turns,
        [Some(0), Some(0), Some(1), Some(0), Some(0), Some(1)],
    );
}

#[test]
fn residual_credit_survives_an_unaffordable_head() {
    let mut scheduler = DeficitRoundRobin::new([units(3), units(1)]);
    let heads = [Some(units(2)), Some(units(1))];
    assert_eq!(scheduler.select(&heads), Some(0));
    assert_eq!(scheduler.deficits, [1, 0]);
    assert_eq!(scheduler.select(&heads), Some(1));
    assert_eq!(scheduler.select(&heads), Some(0));
    assert_eq!(scheduler.select(&heads), Some(0));
}

#[test]
fn empty_lanes_lose_credit_even_when_not_visited() {
    let mut scheduler = DeficitRoundRobin::new([units(4), units(4)]);
    assert_eq!(scheduler.select(&[Some(units(3)); 2]), Some(0));
    assert_eq!(scheduler.select(&[Some(units(3)); 2]), Some(1));
    assert_eq!(scheduler.deficits, [1, 1]);
    assert_eq!(scheduler.select(&[None, Some(units(1))]), Some(1));
    assert_eq!(scheduler.deficits, [0, 0]);
}

#[test]
fn deactivation_prevents_credit_leaking_into_a_refill() {
    let mut scheduler = DeficitRoundRobin::new([units(10), units(1)]);
    assert_eq!(scheduler.select(&[Some(units(2)); 2]), Some(0));
    assert_eq!(scheduler.deficits[0], 8);
    assert_eq!(scheduler.deactivate(0), Ok(Deactivation::Deactivated));
    assert_eq!(scheduler.deficits[0], 0);
    assert_eq!(scheduler.select(&[Some(units(2)), Some(units(1))]), Some(1));

    let before = scheduler.clone();
    assert_eq!(scheduler.deactivate(2), Err(Refusal::UnknownLane));
    assert_eq!(scheduler.deactivate(usize::MAX), Err(Refusal::UnknownLane));
    assert_eq!(scheduler, before);
}

#[test]
fn very_large_costs_skip_billions_of_empty_rounds() {
    let mut scheduler = DeficitRoundRobin::new([units(1)]);
    for _ in 0..3 {
        assert_eq!(scheduler.select(&[Some(units(u32::MAX))]), Some(0));
        assert_eq!(scheduler.deficits, [0]);
    }
}

#[test]
fn maximum_quantum_and_cost_do_not_overflow() {
    let mut scheduler = DeficitRoundRobin::new([units(u32::MAX), units(u32::MAX)]);
    let heads = [Some(units(u32::MAX - 1)), Some(units(u32::MAX))];
    for _ in 0..100 {
        assert_eq!(scheduler.select(&heads), Some(0));
        assert_eq!(scheduler.select(&heads), Some(1));
    }
    assert_eq!(scheduler.deficits, [100, 0]);
}

#[test]
fn idle_and_zero_lane_schedulers_select_nothing() {
    let mut scheduler = DeficitRoundRobin::new([units(5), units(2)]);
    assert_eq!(scheduler.select(&[Some(units(2)); 2]), Some(0));
    for _ in 0..10 {
        assert_eq!(scheduler.select(&[None; 2]), None);
        assert_eq!(scheduler.deficits, [0; 2]);
    }
    assert_eq!(scheduler.select(&[Some(units(5)), None]), Some(0));
    assert_eq!(scheduler.deficits, [0; 2]);

    let mut empty = DeficitRoundRobin::new([]);
    assert_eq!(empty.select(&[]), None);
    assert_eq!(empty.deactivate(0), Err(Refusal::UnknownLane));
}

#[test]
fn shares_cost_rather_than_dispatch_counts() {
    let mut scheduler = DeficitRoundRobin::new([units(6), units(3)]);
    let heads = [Some(units(3)), Some(units(1))];
    let mut spent = [0; 2];
    for _ in 0..500 {
        let lane = scheduler.select(&heads).expect("both lanes are ready");
        spent[lane] += heads[lane].expect("selected lane is ready").get();
    }
    assert_eq!(spent, [600, 300]);
}

/// Textbook deactivation: drop the credit, and end the visit only if it has
/// been paid; `Idle` when the lane had neither.
fn reference_deactivate(
    scheduler: &mut DeficitRoundRobin<2>,
    lane: usize,
) -> Result<Deactivation, Refusal> {
    if lane >= 2 {
        return Err(Refusal::UnknownLane);
    }
    let credit = core::mem::take(&mut scheduler.deficits[lane]);
    let paid = lane == scheduler.cursor && !scheduler.needs_quantum;
    if paid {
        scheduler.cursor = (scheduler.cursor + 1) % 2;
        scheduler.needs_quantum = true;
    }
    Ok(if credit == 0 && !paid {
        Deactivation::Idle
    } else {
        Deactivation::Deactivated
    })
}

/// Deliberately slow textbook visits, without arithmetic round skipping.
fn reference_select(
    scheduler: &mut DeficitRoundRobin<2>,
    heads: [Option<NonZeroU32>; 2],
) -> Option<usize> {
    for (lane, head) in heads.iter().enumerate() {
        if head.is_none() {
            scheduler.deficits[lane] = 0;
        }
    }
    if heads.iter().all(Option::is_none) {
        for _ in 0..2 {
            scheduler.cursor = (scheduler.cursor + 1) % 2;
        }
        scheduler.needs_quantum = true;
        return None;
    }
    loop {
        let lane = scheduler.cursor;
        if let Some(cost) = heads[lane] {
            if scheduler.needs_quantum {
                scheduler.deficits[lane] += u64::from(scheduler.quanta[lane].get());
                scheduler.needs_quantum = false;
            }
            if scheduler.deficits[lane] >= u64::from(cost.get()) {
                scheduler.deficits[lane] -= u64::from(cost.get());
                return Some(lane);
            }
        }
        scheduler.cursor = (scheduler.cursor + 1) % 2;
        scheduler.needs_quantum = true;
    }
}

#[test]
fn optimized_round_skipping_matches_textbook_drr() {
    for first_quantum in 1..=4 {
        for second_quantum in 1..=4 {
            for first_cost in 1..=8 {
                for second_cost in 1..=8 {
                    let mut actual =
                        DeficitRoundRobin::new([units(first_quantum), units(second_quantum)]);
                    let mut reference = actual.clone();
                    for step in 0..40 {
                        let heads = [
                            (step % 7 != 0).then(|| units(first_cost)),
                            (step % 5 != 0).then(|| units(second_cost)),
                        ];
                        assert_eq!(
                            actual.select(&heads),
                            reference_select(&mut reference, heads)
                        );
                        assert_eq!(actual, reference);
                    }
                }
            }
        }
    }
}

#[test]
fn changing_head_costs_match_textbook_drr() {
    let mut actual = DeficitRoundRobin::new([units(3), units(7)]);
    let mut reference = actual.clone();
    for step in 0..1_000 {
        let heads = [
            (step % 11 != 0).then(|| units(1 + step % 17)),
            (step % 13 != 0).then(|| units(1 + (step * 3) % 19)),
        ];
        if step % 23 == 0 {
            assert_eq!(
                actual.deactivate(0),
                reference_deactivate(&mut reference, 0)
            );
        }
        assert_eq!(
            actual.select(&heads),
            reference_select(&mut reference, heads)
        );
        assert_eq!(actual, reference);
    }
}
