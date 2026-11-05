use std::time::Duration;

use agent_desktop_core::{AdapterError, DeliverySemantics, ErrorCode, WindowInfo};
use serde_json::json;

use super::hypr_window_op::{MINIMIZED_WORKSPACE, locate, owned_before, restore};
use super::hypr_world::HyprWorld;
use crate::system::hyprland::hypr_command::HyprCommand;

const SETTLE: Duration = Duration::from_millis(1_000);
const POLL: Duration = Duration::from_millis(25);

fn not_focused(active: Option<u64>) -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionFailed,
        "Hyprland accepted the focus request but another window stayed active",
    )
    .with_suggestion(
        "A fullscreen window or a window rule may hold the focus; run 'list-windows' to see the focused window",
    )
    .with_details(json!({
        "cause": "focus_not_taken",
        "compositor": "hyprland",
        "active_address": active.map(|address| format!("0x{address:x}")),
    }))
    .with_disposition(DeliverySemantics::delivered_unverified())
}

/// Focuses `win` through Hyprland: a window parked on the minimized
/// workspace is brought back first, and the call answers once Hyprland
/// reports the window active.
pub(crate) fn focus_window(world: &impl HyprWorld, win: &WindowInfo) -> Result<(), AdapterError> {
    owned_before(world, win)?;
    if world.stopped(win.pid.get()) {
        return Err(AdapterError::app_unresponsive(&win.app)
            .with_disposition(DeliverySemantics::not_delivered()));
    }
    let client = locate(world, win)?;
    if world.active()? == Some(client.address) {
        return Ok(());
    }
    if client.state.workspace == MINIMIZED_WORKSPACE {
        restore(world, win, client.clone())?;
    } else {
        world.send(client.address, &HyprCommand::Focus)?;
    }
    let limit = world.deadline().capped(SETTLE);
    loop {
        let active = world
            .active()
            .map_err(|error| error.with_disposition(DeliverySemantics::delivered_unverified()))?;
        if active == Some(client.address) {
            return Ok(());
        }
        if limit.is_expired() {
            return Err(not_focused(active));
        }
        std::thread::sleep(POLL.min(limit.remaining()));
    }
}

#[cfg(test)]
#[path = "hypr_focus_tests.rs"]
mod tests;
