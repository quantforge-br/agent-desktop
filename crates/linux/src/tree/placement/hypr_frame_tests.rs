use agent_desktop_core::Rect;

use super::extents_of;
use crate::tree::Extents;

#[test]
fn layout_rectangles_round_to_whole_pixels_and_overflow_is_no_place() {
    let rect = Rect {
        x: -20.4,
        y: 1440.6,
        width: 500.0,
        height: 616.5,
    };
    assert_eq!(
        extents_of(&rect),
        Some(Extents {
            x: -20,
            y: 1441,
            width: 500,
            height: 617,
        })
    );
    assert_eq!(extents_of(&Rect { x: 1e12, ..rect }), None);
}
