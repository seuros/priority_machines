use super::*;

#[test]
fn takes_turns_in_order() {
    let mut scheduler = RoundRobin::new();
    let turns: [_; 5] = core::array::from_fn(|_| scheduler.select(&[true; 3]));
    assert_eq!(turns, [Some(0), Some(1), Some(2), Some(0), Some(1)]);
}

#[test]
fn skips_unready_lanes_and_resumes_after_the_pick() {
    let mut scheduler = RoundRobin::new();
    assert_eq!(scheduler.select(&[false, true, true]), Some(1));
    assert_eq!(scheduler.select(&[true, true, false]), Some(0));
    assert_eq!(scheduler.cursor(), 1);
}

#[test]
fn nothing_ready_moves_nothing() {
    let mut scheduler = RoundRobin::new();
    assert_eq!(scheduler.select(&[true, true]), Some(0));
    assert_eq!(scheduler.select(&[false, false]), None);
    assert_eq!(scheduler.select(&[]), None);
    assert_eq!(scheduler.cursor(), 1);
}

#[test]
fn lane_count_may_change_between_calls() {
    let mut scheduler = RoundRobin::new();
    assert_eq!(scheduler.select(&[true; 4]), Some(0));
    assert_eq!(scheduler.select(&[true; 4]), Some(1));
    assert_eq!(scheduler.select(&[true; 4]), Some(2));
    // Shrunk: the cursor wraps instead of indexing out of range.
    assert_eq!(scheduler.select(&[true; 2]), Some(1));
    // Grown: continues from where it was.
    assert_eq!(scheduler.select(&[true; 5]), Some(0));
}

#[test]
fn every_lane_is_served_once_per_cycle() {
    let mut scheduler = RoundRobin::new();
    let mut counts = [0; 7];
    for _ in 0..70 {
        counts[scheduler.select(&[true; 7]).unwrap_or(usize::MAX)] += 1;
    }
    assert_eq!(counts, [10; 7]);
}
