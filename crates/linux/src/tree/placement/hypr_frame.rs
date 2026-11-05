use agent_desktop_core::{Deadline, Rect};

use crate::system::hyprland::HyprInstance;
use crate::system::hyprland::hypr_client::HyprClient;
use crate::system::hyprland::hypr_layer::HyprLayer;
use crate::system::hyprland::hypr_monitor::HyprMonitor;
use crate::system::window::hypr_window::hypr_window_key::WindowKey;
use crate::system::window::hypr_window::hypr_window_match::match_client;
use crate::system::x11::FrameFacts;
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

fn client_in(
    instance: &HyprInstance,
    facts: &FrameFacts,
    deadline: Deadline,
) -> Option<HyprClient> {
    let clients = instance.clients(deadline).ok()?;
    let key = WindowKey::of_frame(facts.pid, &facts.title);
    match_client(&clients, key).ok().cloned()
}

/// Where a frame with no client of its own is drawn: the one layer-shell
/// surface of its process that has the frame's size, when no other
/// top-level frame of the process has that size ([`HyprLayer::placing`];
/// `frames_of_size` counts them). A shell bar or dock is a layer, not a
/// client, and its toolkit reports window-relative coordinates, so the
/// layer's corner is their origin.
pub(crate) fn layer_place(
    layers: &[HyprLayer],
    facts: &FrameFacts,
    frames_of_size: impl FnOnce((i32, i32)) -> Option<usize>,
) -> Option<Extents> {
    let size = facts.extents?;
    let size = (size.width, size.height);
    let layer = HyprLayer::placing(layers, facts.pid, size, || frames_of_size(size))?;
    extents_of(&layer.rect)
}

fn drawn_layer(
    instance: &HyprInstance,
    facts: &FrameFacts,
    frames_of_size: impl FnOnce((i32, i32)) -> Option<usize>,
    deadline: Deadline,
) -> Option<Extents> {
    layer_place(&instance.layers(deadline).ok()?, facts, frames_of_size)
}

/// Where a frame of process `pid` titled `title` sits under Hyprland: its
/// client's rectangle in the layout. A native Wayland toolkit's window
/// coordinates start at the client's window geometry, which excludes any
/// shadow the toolkit draws, so the rectangle's corner is their origin. A
/// frame with no client is placed by its layer-shell surface. `None` off
/// Hyprland and for a frame placed by neither.
pub(crate) fn hypr_frame(
    facts: &FrameFacts,
    frames_of_size: impl FnOnce((i32, i32)) -> Option<usize>,
    deadline: Deadline,
) -> Option<Extents> {
    let instance = HyprInstance::detect()?;
    match client_in(&instance, facts, deadline) {
        Some(client) => extents_of(&client.frame),
        None => drawn_layer(&instance, facts, frames_of_size, deadline),
    }
}

/// What Hyprland says about the client of a frame, in the shape the live
/// placement reads: its origin, no shadow, and hidden while no monitor
/// shows its workspace, which is where a minimized window is parked. A frame
/// with no client but a layer-shell surface of its own is shown at that
/// surface's corner.
pub(crate) fn hypr_window_state(
    facts: &FrameFacts,
    frames_of_size: impl FnOnce((i32, i32)) -> Option<usize>,
    deadline: Deadline,
) -> Option<WindowX11State> {
    let instance = HyprInstance::detect()?;
    let Some(client) = client_in(&instance, facts, deadline) else {
        let drawn = drawn_layer(&instance, facts, frames_of_size, deadline)?;
        return Some(WindowX11State {
            client_id: 0,
            hidden: false,
            client_origin: (drawn.x, drawn.y),
            shadow: (0, 0, 0, 0),
        });
    };
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
