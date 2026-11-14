use super::*;

/// A scripted trace: each ready view with the lane it must select.
type Steps = &'static [(&'static [bool], Option<usize>)];

#[test]
fn scripted_selections_follow_the_cursor() {
    const ALL3: &[bool] = &[true; 3];
    const ALL4: &[bool] = &[true; 4];
    let cases: [(Steps, usize); 4] = [
        // Turns in order, wrapping.
        (
            &[
                (ALL3, Some(0)),
                (ALL3, Some(1)),
                (ALL3, Some(2)),
                (ALL3, Some(0)),
                (ALL3, Some(1)),
            ],
            2,
        ),
        // Unready lanes are skipped; the search resumes after the pick.
        (
            &[
                (&[false, true, true], Some(1)),
                (&[true, true, false], Some(0)),
            ],
            1,
        ),
        // Nothing ready, or no lanes at all, moves nothing.
        (
            &[
                (&[true, true], Some(0)),
                (&[false, false], None),
                (&[], None),
            ],
            1,
        ),
        // The lane count may shrink (the cursor wraps) or grow between calls.
        (
            &[
                (ALL4, Some(0)),
                (ALL4, Some(1)),
                (ALL4, Some(2)),
                (&[true; 2], Some(1)),
                (&[true; 5], Some(0)),
            ],
            1,
        ),
    ];
    for (steps, cursor) in cases {
        let mut scheduler = RoundRobin::new();
        for &(ready, expected) in steps {
            assert_eq!(scheduler.select(ready), expected, "{ready:?}");
        }
        assert_eq!(scheduler.cursor(), cursor);
    }
}
