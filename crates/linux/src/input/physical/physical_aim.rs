use agent_desktop_core::{AdapterError, DeliverySemantics, ErrorCode, Point, Rect};
use serde_json::json;

use crate::tree::hit_test::hit_guard::{admits, pixel};
use crate::tree::placement::Placement;
use crate::tree::properties::toolkit_trust::reports_global_position;

pub(crate) fn validate(
    point: &Point,
    placed: &Placement,
    has_container: bool,
) -> Result<(), AdapterError> {
    let inside = || {
        let node = placed.node?;
        let frame = placed.frame?;
        if placed.frame_iconified == Some(true) || placed.x11.is_some_and(|window| window.hidden) {
            return None;
        }
        let origin = placed.popup_origin.or_else(|| {
            let window = placed.x11?;
            let shadow = if reports_global_position(&placed.toolkit) {
                (0, 0, 0, 0)
            } else {
                window.shadow
            };
            Some((
                window.client_origin.0.checked_add(shadow.0)?,
                window.client_origin.1.checked_add(shadow.1)?,
            ))
        })?;
        let point = pixel(point.x - f64::from(origin.0), point.y - f64::from(origin.1))?;
        if !admits(node, point) || !admits(frame, point) {
            return None;
        }
        if has_container && !admits(placed.container?, point) {
            return None;
        }
        Some(())
    };
    inside().ok_or_else(|| {
        AdapterError::new(
            ErrorCode::StaleRef,
            "The pointer point is outside the target's proven visible area",
        )
        .with_suggestion("Reveal the target and find it again before sending pointer input")
        .with_details(json!({
            "kind": "pointer_target_not_visible",
            "physical_delivery_started": false,
        }))
        .with_disposition(DeliverySemantics::not_delivered())
    })
}

/// The point a pointer gesture without a proven point aims at: the centre of
/// the part of `rect` its largest overlap with one display shows. An element
/// taller than the monitor its window is on, as a window cropped by its
/// tiling compositor has, keeps a reachable aim. A rect no listed display
/// shows has no aim; with no display listed, the rect keeps its own centre.
pub(crate) fn visible_centre(rect: Rect, displays: &[Rect]) -> Option<Point> {
    let shown = if displays.is_empty() {
        rect
    } else {
        displays
            .iter()
            .filter_map(|display| overlap(&rect, display))
            .max_by(|left, right| {
                (left.width * left.height).total_cmp(&(right.width * right.height))
            })?
    };
    Some(Point {
        x: shown.x + shown.width / 2.0,
        y: shown.y + shown.height / 2.0,
    })
}

/// Whether a listed display shows `point`; with no display listed, every
/// point counts as shown.
pub(crate) fn is_shown(point: &Point, displays: &[Rect]) -> bool {
    displays.is_empty()
        || displays.iter().any(|display| {
            point.x >= display.x
                && point.x < display.x + display.width
                && point.y >= display.y
                && point.y < display.y + display.height
        })
}

fn overlap(rect: &Rect, display: &Rect) -> Option<Rect> {
    let x = rect.x.max(display.x);
    let y = rect.y.max(display.y);
    let right = (rect.x + rect.width).min(display.x + display.width);
    let bottom = (rect.y + rect.height).min(display.y + display.height);
    (right > x && bottom > y).then_some(Rect {
        x,
        y,
        width: right - x,
        height: bottom - y,
    })
}

#[cfg(test)]
#[path = "physical_aim_tests.rs"]
mod tests;
