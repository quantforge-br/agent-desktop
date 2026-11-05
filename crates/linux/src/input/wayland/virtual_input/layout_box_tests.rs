use agent_desktop_core::Rect;

use super::LayoutBox;

fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

#[test]
fn the_box_spans_monitors_with_offsets_and_a_negative_origin() {
    let layout = LayoutBox::around(&[
        rect(0.0, 0.0, 3440.0, 1440.0),
        rect(3440.0, 180.0, 1920.0, 1080.0),
        rect(-1280.0, -100.0, 1280.0, 800.0),
    ])
    .unwrap();
    assert_eq!((layout.x, layout.y), (-1280.0, -100.0));
    assert_eq!((layout.width, layout.height), (6640, 1540));
    assert_eq!(layout.local(0, 0), Some((1280, 100)));
    assert_eq!(layout.local(-1280, -100), Some((0, 0)));
    assert_eq!(layout.local(5360, 0), None);
}

#[test]
fn no_monitor_is_no_box() {
    assert_eq!(LayoutBox::around(&[]), None);
}
