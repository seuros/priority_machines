use super::*;

#[test]
fn only_an_index_below_n_is_a_lane() {
    assert_eq!(Lane::<3>::new(2).map(Lane::get), Ok(2));
    assert_eq!(Lane::<3>::new(2).map(usize::from), Ok(2));
    assert_eq!(Lane::<3>::new(3), Err(Refusal::UnknownLane));
    assert_eq!(Lane::<3>::new(usize::MAX), Err(Refusal::UnknownLane));
    assert_eq!(Lane::<0>::new(0), Err(Refusal::UnknownLane));
}
