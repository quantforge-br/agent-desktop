use agent_desktop_core::{AdapterError, Deadline};

use super::hypr_window_key::WindowKey;
use super::hypr_window_match::match_client;
use super::hypr_window_op::MINIMIZED_WORKSPACE;
use crate::system::hyprland::HyprInstance;
use crate::system::hyprland::hypr_client::HyprClient;
use crate::system::hyprland::hypr_monitor::HyprMonitor;
use crate::system::registry::listed_window::ListedWindow;

/// Hyprland's view of the session, read once per listing.
pub(crate) struct HyprView {
    pub(crate) clients: Vec<HyprClient>,
    pub(crate) monitors: Vec<HyprMonitor>,
    pub(crate) active: Option<u64>,
}

impl HyprView {
    pub(crate) fn read(instance: &HyprInstance, deadline: Deadline) -> Result<Self, AdapterError> {
        Ok(Self {
            clients: instance.clients(deadline)?,
            monitors: instance.monitors(deadline)?,
            active: instance.active_address(deadline)?,
        })
    }

    /// The view of the Hyprland this session runs under, or `None` on any
    /// other compositor or when Hyprland does not answer.
    pub(crate) fn of_session(deadline: Deadline) -> Option<Self> {
        Self::read(&HyprInstance::detect()?, deadline).ok()
    }

    /// Whether `pid` has mapped windows and Hyprland shows none of them.
    pub(crate) fn hides_every_window_of(&self, pid: u32) -> bool {
        let mut own = self
            .clients
            .iter()
            .filter(|client| client.owner.pid == pid && client.state.mapped)
            .peekable();
        own.peek().is_some() && own.all(|client| !client.shown_on(&self.monitors))
    }

    /// Gives each listed window the place, focus and visibility of its one
    /// Hyprland client. The compositor's active window is the only focus
    /// answer; a window with no single client keeps what the toolkit said
    /// but is never the focused one.
    pub(crate) fn annotate(&self, windows: &mut [ListedWindow]) {
        let pids: Vec<u32> = windows.iter().map(|window| window.info.pid.get()).collect();
        for window in windows {
            let pid = window.info.pid.get();
            let key = WindowKey {
                pid,
                title: &window.info.title,
                bounds: None,
                alone: pids.iter().filter(|listed| **listed == pid).count() == 1,
            };
            let Ok(client) = match_client(&self.clients, key) else {
                window.info.state.is_focused = false;
                continue;
            };
            let minimized = client.state.workspace == MINIMIZED_WORKSPACE;
            window.info.bounds = Some(client.frame);
            window.info.state.is_focused = self.active == Some(client.address);
            window.info.state.minimized = Some(minimized);
            window.info.state.visible = Some(!minimized && client.shown_on(&self.monitors));
        }
    }

    /// The index of the focused window after [`Self::annotate`].
    pub(crate) fn focused(windows: &[ListedWindow]) -> Option<usize> {
        windows
            .iter()
            .position(|window| window.info.state.is_focused)
    }
}

#[cfg(test)]
#[path = "hypr_listing_tests.rs"]
mod tests;
