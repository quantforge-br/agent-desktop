use agent_desktop_core::{ErrorCode, Rect};

use super::localized;
use crate::input::plan::Plan;
use crate::input::step::Step;
use crate::input::wayland::virtual_input::layout_box::LayoutBox;

fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn plan(steps: Vec<Step>) -> Plan {
    Plan {
        steps,
        release_origin: None,
    }
}

fn monitors() -> Vec<Rect> {
    vec![
        rect(0.0, 0.0, 3440.0, 1440.0),
        rect(3440.0, 180.0, 1920.0, 1080.0),
    ]
}

#[test]
fn pointer_positions_become_offsets_in_the_layout_box() {
    let monitors = vec![
        rect(-1920.0, 0.0, 1920.0, 1080.0),
        rect(0.0, 0.0, 2560.0, 1440.0),
    ];
    let layout = LayoutBox::around(&monitors).unwrap();
    let plans = localized(
        vec![plan(vec![
            Step::MoveTo { x: -10, y: 5 },
            Step::Button {
                code: 1,
                down: true,
            },
        ])],
        &layout,
        &monitors,
    )
    .unwrap();
    assert_eq!(plans[0].steps[0], Step::MoveTo { x: 1910, y: 5 });
    assert_eq!(
        plans[0].steps[1],
        Step::Button {
            code: 1,
            down: true
        }
    );
}

#[test]
fn a_point_in_the_box_but_on_no_monitor_is_refused_before_delivery() {
    let monitors = monitors();
    let layout = LayoutBox::around(&monitors).unwrap();
    let error = localized(
        vec![plan(vec![Step::MoveTo { x: 4000, y: 50 }])],
        &layout,
        &monitors,
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidArgs);
    assert_eq!(error.details.unwrap()["physical_delivery_started"], false);
}
