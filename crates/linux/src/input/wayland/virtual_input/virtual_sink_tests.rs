use super::nudge_before;

#[test]
fn a_move_onto_the_resting_point_passes_one_pixel_beside_it_first() {
    assert_eq!(nudge_before((10, 20), Some((10, 20)), 100), Some((11, 20)));
    assert_eq!(nudge_before((99, 20), Some((99, 20)), 100), Some((98, 20)));
}

#[test]
fn a_move_elsewhere_or_from_an_unread_cursor_goes_straight_there() {
    assert_eq!(nudge_before((10, 20), Some((10, 21)), 100), None);
    assert_eq!(nudge_before((10, 20), None, 100), None);
}
