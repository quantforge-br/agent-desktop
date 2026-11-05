use agent_desktop_core::Rect;

use super::hypr_client_owner::HyprClientOwner;
use super::hypr_client_state::HyprClientState;
use super::hypr_monitor::HyprMonitor;

/// One toplevel as Hyprland's `clients` request lists it. The list order is
/// Hyprland's window order, which is its paint order within one layer.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct HyprClient {
    pub(crate) address: u64,
    pub(crate) stable_id: String,
    pub(crate) owner: HyprClientOwner,
    pub(crate) frame: Rect,
    pub(crate) state: HyprClientState,
    pub(crate) focus_history: i64,
}

impl HyprClient {
    /// Whether a monitor shows this client now: it is pinned, or one of
    /// `monitors` shows its workspace.
    pub(crate) fn shown_on(&self, monitors: &[HyprMonitor]) -> bool {
        self.state.pinned
            || monitors
                .iter()
                .any(|monitor| monitor.shows(&self.state.workspace))
    }
}
