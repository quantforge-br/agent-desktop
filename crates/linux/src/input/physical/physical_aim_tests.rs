use agent_desktop_core::{Point, Rect};

use super::visible_centre;

fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

#[test]
fn an_element_past_the_bottom_of_its_monitor_is_aimed_at_its_shown_part() {
    let monitor = rect(0.0, 0.0, 1280.0, 800.0);
    let point = visible_centre(rect(100.0, 700.0, 200.0, 212.0), &[monitor]);
    assert_eq!(point, Point { x: 200.0, y: 750.0 });
}

#[test]
fn an_element_across_two_monitors_is_aimed_on_the_one_showing_more_of_it() {
    let left = rect(0.0, 0.0, 1000.0, 800.0);
    let right = rect(1000.0, 0.0, 1000.0, 800.0);
    let point = visible_centre(rect(900.0, 100.0, 400.0, 100.0), &[left, right]);
    assert_eq!(
        point,
        Point {
            x: 1150.0,
            y: 150.0
        }
    );
}

#[test]
fn an_element_no_display_shows_keeps_its_own_centre() {
    let monitor = rect(0.0, 0.0, 1280.0, 800.0);
    let off = rect(2000.0, 100.0, 40.0, 20.0);
    assert_eq!(
        visible_centre(off, &[monitor]),
        Point {
            x: 2020.0,
            y: 110.0
        }
    );
    assert_eq!(
        visible_centre(off, &[]),
        Point {
            x: 2020.0,
            y: 110.0
        }
    );
}
