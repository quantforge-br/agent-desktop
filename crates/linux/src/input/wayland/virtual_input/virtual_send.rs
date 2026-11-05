use agent_desktop_core::{
    AdapterError, Deadline, DeliverySemantics, ErrorCode, InteractionLease, Rect,
};
use serde_json::json;

use super::layout_box::LayoutBox;
use super::virtual_key_session::VirtualKeySession;
use super::virtual_key_table::VirtualKeyTable;
use super::virtual_sink::VirtualSink;
use crate::input::delivery::{Delivery, Gate};
use crate::input::keyboard::keymap::Keymap;
use crate::input::keyboard::keysym_step::KeysymStep;
use crate::input::plan::Plan;
use crate::input::step::Step;
use crate::input::wayland::keysym_send::run_chunks;
use crate::input::x11::session::{CallEnv, run_journaled};
use crate::input::x11::worker_slot;
use crate::system::hyprland::HyprInstance;

/// The compositor whose virtual input devices this adapter drives: Hyprland,
/// the one whose virtual keyboard and pointer were measured.
pub(crate) fn compositor() -> Option<HyprInstance> {
    HyprInstance::detect()
}

/// The keymap the key and pointer planners read for a virtual delivery.
pub(crate) fn planner_keymap() -> Keymap {
    VirtualKeyTable::default().keymap()
}

fn before_delivery(mut error: AdapterError) -> AdapterError {
    error.details.get_or_insert_with(|| json!({}))["physical_delivery_started"] = false.into();
    error.with_disposition(DeliverySemantics::not_delivered())
}

fn outside_monitors(x: i32, y: i32) -> AdapterError {
    AdapterError::new(
        ErrorCode::InvalidArgs,
        "A pointer position lies outside every monitor",
    )
    .with_details(json!({ "x": x, "y": y, "physical_delivery_started": false }))
    .with_suggestion("Use a point inside one of the bounds 'list-displays' reports")
    .with_disposition(DeliverySemantics::not_delivered())
}

fn on_a_monitor(monitors: &[Rect], x: i32, y: i32) -> bool {
    let (x, y) = (f64::from(x), f64::from(y));
    monitors.iter().any(|rect| {
        x >= rect.x && y >= rect.y && x < rect.x + rect.width && y < rect.y + rect.height
    })
}

/// The plans with every pointer position moved into the layout box, after
/// refusing any position no monitor shows.
pub(crate) fn localized(
    plans: Vec<Plan>,
    layout: &LayoutBox,
    monitors: &[Rect],
) -> Result<Vec<Plan>, AdapterError> {
    plans
        .into_iter()
        .map(|mut plan| {
            for step in &mut plan.steps {
                if let Step::MoveTo { x, y } = step {
                    if !on_a_monitor(monitors, *x, *y) {
                        return Err(outside_monitors(*x, *y));
                    }
                    let (local_x, local_y) = layout
                        .local(*x, *y)
                        .ok_or_else(|| outside_monitors(*x, *y))?;
                    *x = local_x;
                    *y = local_y;
                }
            }
            Ok(plan)
        })
        .collect()
}

/// Runs planned pointer and key steps through Hyprland's virtual devices
/// with the same worker, gate and release guarantees as XTEST delivery.
pub(crate) fn run_plans(
    instance: HyprInstance,
    plans: Vec<Plan>,
    gate: Gate<'_>,
    lease: &InteractionLease,
) -> Result<Delivery, AdapterError> {
    let deadline = lease.deadline();
    let monitors: Vec<Rect> = instance
        .monitors(deadline)
        .map_err(before_delivery)?
        .into_iter()
        .map(|monitor| monitor.logical)
        .collect();
    let layout = LayoutBox::around(&monitors).ok_or_else(|| {
        before_delivery(AdapterError::internal(
            "Hyprland reported no usable monitor layout",
        ))
    })?;
    let plans = localized(plans, &layout, &monitors)?;
    run_journaled(
        move || VirtualSink::open(instance, layout, deadline),
        plans,
        gate,
        CallEnv {
            deadline,
            journal: None,
            slot: worker_slot::shared(),
            release_on_signal: true,
            lease: Some(lease),
        },
    )
}

/// Sends keysym chunks through a virtual keyboard whose keymap holds every
/// keysym of the delivery.
pub(crate) fn send_keysyms(
    chunks: &[&[KeysymStep]],
    gate: Gate<'_>,
    deadline: Deadline,
) -> Result<Delivery, AdapterError> {
    let keysyms = chunks
        .iter()
        .flat_map(|chunk| chunk.iter().map(|key| key.keysym));
    let mut session = VirtualKeySession::new(keysyms);
    run_chunks(&mut session, chunks, gate, deadline)
}

#[cfg(test)]
#[path = "virtual_send_tests.rs"]
mod tests;
