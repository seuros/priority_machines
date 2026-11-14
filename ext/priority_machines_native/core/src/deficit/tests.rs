use super::*;
use crate::test_support::{Rng, Tally, nonzero as units};

mod contract;

/// Run `steps` against fresh quanta and answer the deficits they leave.
fn replay<const N: usize>(
    quanta: [u32; N],
    steps: &[([Option<u32>; N], Option<usize>)],
) -> [u64; N] {
    let mut scheduler = DeficitRoundRobin::new(quanta.map(units));
    for (index, (heads, expected)) in steps.iter().enumerate() {
        let heads = heads.map(|head| head.map(units));
        assert_eq!(
            scheduler.select(&heads),
            *expected,
            "{quanta:?} step {index}"
        );
    }
    scheduler.deficits
}

#[test]
fn scripted_selections_carry_credit_between_visits() {
    // One quantum pays for several heads in one visit.
    let two = [Some(2); 2];
    let multiple = [0, 0, 1, 0, 0, 1].map(|lane| (two, Some(lane)));
    assert_eq!(replay([4, 2], &multiple), [0, 0]);
    // Residual credit survives an unaffordable head.
    let heads = [Some(2), Some(1)];
    let residual = [0, 1, 0, 0].map(|lane| (heads, Some(lane)));
    assert_eq!(replay([3, 1], &residual), [0, 0]);
    // Empty lanes lose their credit even when the round does not visit them.
    let empty = [
        ([Some(3); 2], Some(0)),
        ([Some(3); 2], Some(1)),
        ([None, Some(1)], Some(1)),
    ];
    assert_eq!(replay([4, 4], &empty), [0, 0]);
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
fn extreme_quanta_and_costs_do_not_overflow() {
    // A cost of u32::MAX against a quantum of one skips billions of rounds.
    let mut scheduler = DeficitRoundRobin::new([units(1)]);
    for _ in 0..3 {
        assert_eq!(scheduler.select(&[Some(units(u32::MAX))]), Some(0));
        assert_eq!(scheduler.deficits, [0]);
    }
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
        let Some(lane) = scheduler.select(&heads) else {
            panic!("both lanes are ready");
        };
        spent[lane] += heads[lane].map_or(0, NonZeroU32::get);
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
