use agent_desktop_core::{Deadline, Rect};

use crate::system::hyprland::HyprInstance;
use crate::system::hyprland::hypr_client::HyprClient;
use crate::system::hyprland::hypr_monitor::HyprMonitor;
use crate::system::window::hypr_window::hypr_window_key::WindowKey;
use crate::system::window::hypr_window::hypr_window_match::match_client;
use crate::system::x11::window::window_x11_state::WindowX11State;
use crate::tree::Extents;

pub(crate) fn extents_of(rect: &Rect) -> Option<Extents> {
    let side = |value: f64| {
        let rounded = value.round();
        (rounded.abs() <= f64::from(i32::MAX)).then_some(rounded as i32)
    };
    Some(Extents {
        x: side(rect.x)?,
        y: side(rect.y)?,
        width: side(rect.width)?,
        height: side(rect.height)?,
    })
}

fn client_of(pid: u32, title: &str, deadline: Deadline) -> Option<(HyprClient, HyprInstance)> {
    let instance = HyprInstance::detect()?;
    let clients = instance.clients(deadline).ok()?;
    let key = WindowKey::of_frame(pid, title);
    let client = match_client(&clients, key).ok()?.clone();
    Some((client, instance))
}

/// Where a frame of process `pid` titled `title` sits under Hyprland: its
/// client's rectangle in the layout. A native Wayland toolkit's window
/// coordinates start at the client's window geometry, which excludes any
/// shadow the toolkit draws, so the rectangle's corner is their origin.
/// `None` off Hyprland and for a frame without one client.
pub(crate) fn hypr_frame(pid: u32, title: &str, deadline: Deadline) -> Option<Extents> {
    extents_of(&client_of(pid, title, deadline)?.0.frame)
}

/// What Hyprland says about the client of a frame, in the shape the live
/// placement reads: its origin, no shadow, and hidden while no monitor
/// shows its workspace, which is where a minimized window is parked.
pub(crate) fn hypr_window_state(
    pid: u32,
    title: &str,
    deadline: Deadline,
) -> Option<WindowX11State> {
    let (client, instance) = client_of(pid, title, deadline)?;
    let monitors: Vec<HyprMonitor> = instance.monitors(deadline).ok()?;
    let frame = extents_of(&client.frame)?;
    let shown = client.state.pinned
        || monitors
            .iter()
            .any(|monitor| monitor.shows(&client.state.workspace));
    Some(WindowX11State {
        client_id: u32::from_str_radix(&client.stable_id, 16).unwrap_or_default(),
        hidden: !shown,
        client_origin: (frame.x, frame.y),
        shadow: (0, 0, 0, 0),
    })
}

#[cfg(test)]
#[path = "hypr_frame_tests.rs"]
mod tests;
