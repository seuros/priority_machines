use super::*;

#[test]
fn selects_the_first_earliest_eligible_deadline() {
    let at = |secs| Some(Duration::from_secs(secs));
    let cases: [(&[Option<Duration>], Option<usize>); 6] = [
        (&[at(9), None, at(3), at(7)], Some(2)),
        // Equal deadlines choose the first index.
        (&[None, at(0), at(0)], Some(1)),
        (&[], None),
        (&[None; 3], None),
        // Extremes need no arithmetic.
        (&[Some(Duration::MAX), at(0)], Some(1)),
        (&[Some(Duration::MAX)], Some(0)),
    ];
    for (deadlines, expected) in cases {
        assert_eq!(earliest_deadline(deadlines), expected, "{deadlines:?}");
    }
}
