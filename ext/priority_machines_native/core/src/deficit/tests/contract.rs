//! Refusals, honoured repeats, and what an empty round records.

use super::*;

#[test]
fn a_repeated_deactivation_is_honoured_and_changes_nothing() {
    let mut scheduler = DeficitRoundRobin::new([units(10), units(1)]);
    assert_eq!(scheduler.select(&[Some(units(2)); 2]), Some(0));
    assert_eq!(scheduler.deactivate(0), Ok(Deactivation::Deactivated));
    let before = scheduler.clone();
    assert_eq!(scheduler.deactivate(0), Ok(Deactivation::Idle));
    assert_eq!(scheduler, before);

    // Lane 1 is under the cursor but unpaid and without credit: not a visit.
    assert_eq!(scheduler.deactivate(1), Ok(Deactivation::Idle));
    assert_eq!(scheduler, before);

    // An only lane that holds no credit and has not been paid gives up nothing.
    let mut single = DeficitRoundRobin::new([units(5)]);
    let before = single.clone();
    assert_eq!(single.deactivate(0), Ok(Deactivation::Idle));
    assert_eq!(single, before);
    assert_eq!(single.select(&[Some(units(2))]), Some(0));
    assert_eq!(single.deactivate(0), Ok(Deactivation::Deactivated));
    assert_eq!(single, before);
}

#[test]
fn an_empty_round_clears_credit_ends_the_visit_and_keeps_the_cursor() {
    let mut scheduler = DeficitRoundRobin::new([units(4), units(9)]);
    assert_eq!(scheduler.select(&[Some(units(3)); 2]), Some(0));
    assert_eq!(scheduler.select(&[Some(units(3)); 2]), Some(1));
    assert_eq!(scheduler.deficits, [1, 6]);
    assert_eq!((scheduler.cursor, scheduler.needs_quantum), (1, false));

    let mut expected = scheduler.clone();
    expected.deficits = [0; 2];
    expected.needs_quantum = true;
    assert_eq!(scheduler.select(&[None; 2]), None);
    assert_eq!(scheduler, expected);

    // Nothing is left to record: the next empty round changes nothing.
    assert_eq!(scheduler.select(&[None; 2]), None);
    assert_eq!(scheduler, expected);

    // Lane 1 is still next, and it is paid a fresh quantum, not the old credit.
    assert_eq!(scheduler.select(&[Some(units(3)); 2]), Some(1));
    assert_eq!(scheduler.deficits, [0, 6]);
}

#[test]
fn randomized_trace_matches_textbook_drr() {
    let mut rng = Rng(0x4215_0000_4215_0001);
    let mut actual = DeficitRoundRobin::new([units(3), units(7)]);
    let mut reference = actual.clone();
    let (mut tally, mut empty) = (Tally::default(), 0);
    for _ in 0..20_000 {
        let before = actual.clone();
        if rng.below(4) == 0 {
            let lane = rng.index(4);
            let outcome = actual.deactivate(lane);
            assert_eq!(outcome, reference_deactivate(&mut reference, lane));
            tally.record(outcome, &actual, &before);
        } else {
            let heads = core::array::from_fn(|_| {
                let draw = rng.below(57);
                (!draw.is_multiple_of(3)).then(|| units(1 + draw % 19))
            });
            let selected = actual.select(&heads);
            assert_eq!(selected, reference_select(&mut reference, heads));
            assert_eq!(selected.is_none(), heads.iter().all(Option::is_none));
            empty += u32::from(selected.is_none());
        }
        assert_eq!(actual, reference);
    }
    assert!(tally.refusals > 500 && tally.repeats > 500 && empty > 500);
}

#[test]
fn deactivating_lanes_that_hold_nothing_never_moves_the_cursor() {
    let mut scheduler = DeficitRoundRobin::new([units(3), units(7)]);
    let fresh = scheduler.clone();
    for lane in [0, 0, 1, 0, 1, 1] {
        assert_eq!(scheduler.deactivate(lane), Ok(Deactivation::Idle));
        assert_eq!(scheduler, fresh);
    }
    assert_eq!(scheduler.select(&[Some(units(1)); 2]), Some(0));
}

#[test]
fn deactivating_an_unpaid_lane_drops_its_credit_and_keeps_its_place() {
    let mut scheduler = DeficitRoundRobin::new([units(2), units(1)]);
    // Lane 0 banks 2 against a head of 5, lane 1 is served, lane 0 is next.
    assert_eq!(scheduler.select(&[Some(units(5)), Some(units(1))]), Some(1));
    assert_eq!(scheduler.deactivate(1), Ok(Deactivation::Deactivated));
    assert_eq!((scheduler.cursor, scheduler.needs_quantum), (0, true));
    assert_eq!(scheduler.deficits, [2, 0]);

    // Its visit has not been paid: the credit goes, the place stays.
    assert_eq!(scheduler.deactivate(0), Ok(Deactivation::Deactivated));
    assert_eq!((scheduler.cursor, scheduler.needs_quantum), (0, true));
    assert_eq!(scheduler.deficits, [0, 0]);
    let before = scheduler.clone();
    assert_eq!(scheduler.deactivate(0), Ok(Deactivation::Idle));
    assert_eq!(scheduler, before);
    assert_eq!(scheduler.select(&[Some(units(2)), Some(units(1))]), Some(0));
}
