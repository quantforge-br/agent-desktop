use agent_desktop_core::{AdapterError, Deadline};

use super::hypr_unmatched::Unmatched;
use super::hypr_window_key::WindowKey;
use super::hypr_window_match::match_client;
use super::hypr_window_op::MINIMIZED_WORKSPACE;
use crate::system::hyprland::HyprInstance;
use crate::system::hyprland::hypr_client::HyprClient;
use crate::system::hyprland::hypr_layer::HyprLayer;
use crate::system::hyprland::hypr_monitor::HyprMonitor;
use crate::system::registry::listed_window::ListedWindow;
use crate::system::shell::dux::surface::Surface;
use crate::system::window::listed_twins::twins_of;
use crate::system::x11::twin_tie::broken_tie;

/// Hyprland's view of the session, read once per listing.
pub(crate) struct HyprView {
    pub(crate) clients: Vec<HyprClient>,
    pub(crate) layers: Vec<HyprLayer>,
    pub(crate) monitors: Vec<HyprMonitor>,
    pub(crate) active: Option<u64>,
}

impl HyprView {
    pub(crate) fn read(instance: &HyprInstance, deadline: Deadline) -> Result<Self, AdapterError> {
        Ok(Self {
            clients: instance.clients(deadline)?,
            layers: instance.layers(deadline).unwrap_or_default(),
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
    /// Hyprland client. Same-titled windows of one process are told apart
    /// the way `broken_tie` allows: the frame the toolkit reports active is
    /// Hyprland's active client and, with two of them, the other frame is
    /// the other client. A window is focused when it is Hyprland's active
    /// client and its frame carries `ACTIVE`: a shell layer that takes the
    /// keyboard, such as a launcher, leaves the active client in place but
    /// takes `ACTIVE` from its frame. A window with no single client keeps
    /// what the toolkit said but is never the focused one. A window with no
    /// client that is one layer-shell surface of the shell takes that
    /// surface's place and, when the shell names it, its title.
    pub(crate) fn annotate(&self, windows: &mut [ListedWindow]) {
        let clients: Vec<Option<&HyprClient>> = (0..windows.len())
            .map(|index| self.client_of(windows, index))
            .collect();
        let layers: Vec<Option<&HyprLayer>> = (0..windows.len())
            .map(|index| self.layer_of(windows, index))
            .collect();
        for ((window, client), layer) in windows.iter_mut().zip(clients).zip(layers) {
            let Some(client) = client else {
                window.info.state.is_focused = false;
                if let Some(layer) = layer {
                    window.info.bounds = Some(layer.rect);
                    if let Some(surface) = Surface::of_namespace(&layer.namespace) {
                        window.info.title = surface.title().to_owned();
                    }
                }
                continue;
            };
            let minimized = client.state.workspace == MINIMIZED_WORKSPACE;
            window.info.bounds = Some(client.frame);
            window.info.state.is_focused = self.active == Some(client.address) && window.active;
            window.info.state.minimized = Some(minimized);
            window.info.state.visible = Some(!minimized && client.shown_on(&self.monitors));
        }
    }

    /// The layer-shell surface a client-less window is, by the rule
    /// [`HyprLayer::placing`] shares with the snapshot and live placement. A
    /// bar or dock is a layer, so the compositor's client list has no entry
    /// for it.
    fn layer_of(&self, windows: &[ListedWindow], index: usize) -> Option<&HyprLayer> {
        let window = windows.get(index)?;
        let pid = window.info.pid.get();
        let size = window.frame.extents?;
        let size = (size.width, size.height);
        HyprLayer::placing(&self.layers, pid, size, || {
            HyprLayer::frames_of_size(
                windows
                    .iter()
                    .filter(|other| other.info.pid.get() == pid)
                    .map(|other| other.frame.extents.map(|e| (e.width, e.height))),
                size,
            )
        })
    }

    fn client_of(&self, windows: &[ListedWindow], index: usize) -> Option<&HyprClient> {
        let window = windows.get(index)?;
        let pid = window.info.pid.get();
        let key = WindowKey {
            pid,
            title: &window.info.title,
            bounds: None,
            alone: windows
                .iter()
                .filter(|listed| listed.info.pid.get() == pid)
                .count()
                == 1,
        };
        match match_client(&self.clients, key) {
            Ok(client) => Some(client),
            Err(Unmatched::Missing) => None,
            Err(Unmatched::Several) => {
                let twins: Vec<&HyprClient> = self
                    .clients
                    .iter()
                    .filter(|client| {
                        client.state.mapped
                            && client.owner.pid == pid
                            && client.owner.title == window.info.title
                    })
                    .collect();
                let ids: Vec<u64> = twins.iter().map(|client| client.address).collect();
                let address = broken_tie(&ids, self.active, &twins_of(windows, index)?)?;
                twins.into_iter().find(|client| client.address == address)
            }
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
