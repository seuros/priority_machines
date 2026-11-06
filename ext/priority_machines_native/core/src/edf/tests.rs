use super::*;

#[test]
fn selects_earliest_eligible_deadline() {
    assert_eq!(
        earliest_deadline(&[
            Some(Duration::from_secs(9)),
            None,
            Some(Duration::from_secs(3)),
            Some(Duration::from_secs(7)),
        ]),
        Some(2),
    );
}

#[test]
fn equal_deadlines_choose_first_index() {
    assert_eq!(
        earliest_deadline(&[None, Some(Duration::ZERO), Some(Duration::ZERO)]),
        Some(1),
    );
}

#[test]
fn empty_and_ineligible_inputs_select_nothing() {
    assert_eq!(earliest_deadline(&[]), None);
    assert_eq!(earliest_deadline(&[None; 3]), None);
}

#[test]
fn deadline_extremes_do_not_require_arithmetic() {
    assert_eq!(
        earliest_deadline(&[Some(Duration::MAX), Some(Duration::ZERO)]),
        Some(1),
    );
    assert_eq!(earliest_deadline(&[Some(Duration::MAX)]), Some(0));
}
