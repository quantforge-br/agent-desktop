use crate::input::wayland::virtual_input::virtual_send;
use crate::system::display::display_server::DisplayServer;
use crate::tree::properties::node_read::COMPONENT_INTERFACE;
use crate::tree::vocabulary::role_numbers::{
    DOCUMENT_FRAME, DOCUMENT_WEB, LIST, SCROLL_PANE, VIEWPORT,
};

const TABLE: u32 = 55;
const TREE: u32 = 65;
const TREE_TABLE: u32 = 66;

const WHEEL_TARGETS: [u32; 8] = [
    SCROLL_PANE,
    VIEWPORT,
    LIST,
    TABLE,
    TREE,
    TREE_TABLE,
    DOCUMENT_FRAME,
    DOCUMENT_WEB,
];

/// Whether a node can take a wheel at its centre as a scroll: its role moves
/// content under a wheel, it has bounds to aim at, and the session has a
/// pointer leg.
pub(crate) fn eligible(role: u32, interfaces: &[String], session: &DisplayServer) -> bool {
    wheel_target(role, interfaces)
        && has_wheel_leg(session, || virtual_send::compositor().is_some())
}

/// Whether the session has a pointer leg to aim a wheel with: an X11 display,
/// or a Wayland compositor whose virtual pointer this adapter drives, as
/// Hyprland's is. Any other Wayland compositor has none.
pub(crate) fn has_wheel_leg(
    session: &DisplayServer,
    virtual_pointer: impl FnOnce() -> bool,
) -> bool {
    match session {
        DisplayServer::X11(_) => true,
        DisplayServer::Wayland => virtual_pointer(),
        DisplayServer::Unavailable(_) => false,
    }
}

/// The part of eligibility a node decides alone: a role that moves content
/// under a wheel and a `Component` interface to take bounds from.
pub(crate) fn wheel_target(role: u32, interfaces: &[String]) -> bool {
    WHEEL_TARGETS.contains(&role) && interfaces.iter().any(|name| name == COMPONENT_INTERFACE)
}

#[cfg(test)]
#[path = "scroll_capability_tests.rs"]
mod tests;
