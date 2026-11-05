use agent_desktop_core::Deadline;

use super::Windowing;
use crate::system::hyprland::HyprInstance;
use crate::system::hyprland::hypr_client::HyprClient;
use crate::system::hyprland::hypr_layer::HyprLayer;
use crate::system::hyprland::hypr_monitor::HyprMonitor;
use crate::system::window::hypr_window::hypr_window_key::WindowKey;
use crate::system::window::hypr_window::hypr_window_match::match_client;
use crate::system::x11::stacking::stacked_window::StackedWindow;
use crate::system::x11::stacking::stacking_snapshot::StackingSnapshot;
use crate::system::x11::window::desktop_placement::DesktopPlacement;
use crate::system::x11::window::window_flags::WindowFlags;
use crate::system::x11::window::window_kind::WindowKind;
use crate::system::x11::{FrameExtents, FrameFacts, X11Client};
use crate::tree::placement::hypr_frame::extents_of;

/// What Hyprland reports for one hit test, read once.
pub(crate) struct HyprScene<'a> {
    pub(crate) clients: &'a [HyprClient],
    pub(crate) monitors: &'a [HyprMonitor],
    pub(crate) layers: &'a [HyprLayer],
}

fn stable_id(client: &HyprClient) -> Option<u32> {
    u32::from_str_radix(&client.stable_id, 16).ok()
}

fn layer_rank(client: &HyprClient) -> u8 {
    if client.state.fullscreen != 0 {
        2
    } else {
        u8::from(client.state.floating)
    }
}

impl HyprScene<'_> {
    fn shown(&self, client: &HyprClient) -> bool {
        client.shown_on(self.monitors)
    }

    fn stacked(&self, client: &HyprClient, target: u64) -> Option<StackedWindow> {
        let shown = self.shown(client);
        if !shown && client.address != target {
            return None;
        }
        Some(StackedWindow {
            id: stable_id(client)?,
            flags: WindowFlags {
                hidden: !shown,
                above: false,
                mapped: client.state.mapped,
            },
            desktop: DesktopPlacement::Sticky,
            kind: WindowKind::Normal,
            client: extents_of(&client.frame)?,
            extents: FrameExtents::default(),
            shadow: (0, 0, 0, 0),
        })
    }

    /// The shell surfaces drawn above the target: overlay surfaces always,
    /// top surfaces unless the target is fullscreen on their monitor, which
    /// Hyprland draws over them. Their input regions are not reported, so a
    /// surface takes the pointer wherever it is drawn.
    fn covering_layers(&self, target: &HyprClient) -> Vec<(u32, &HyprLayer)> {
        let target_monitor = self
            .monitors
            .iter()
            .find(|monitor| monitor.id == target.state.monitor)
            .map(|monitor| monitor.name.as_str());
        self.layers
            .iter()
            .filter(|layer| match layer.level {
                HyprLayer::OVERLAY => true,
                HyprLayer::TOP => {
                    target.state.fullscreen == 0 || target_monitor != Some(layer.monitor.as_str())
                }
                _ => false,
            })
            .enumerate()
            .map(|(index, layer)| (u32::MAX - u32::try_from(index).unwrap_or(0), layer))
            .collect()
    }

    /// The stacking order around `target` in the shape the hit test reads:
    /// the windows on screen bottom to top, tiled below floating below
    /// fullscreen and each kind in Hyprland's window order, and the shell
    /// surfaces above them as override-redirect windows.
    pub(crate) fn stacking(&self, target: &HyprClient) -> Option<(u32, StackingSnapshot)> {
        let mut ordered: Vec<&HyprClient> = self.clients.iter().collect();
        ordered.sort_by_key(|client| layer_rank(client));
        let stack: Vec<StackedWindow> = ordered
            .into_iter()
            .filter_map(|client| self.stacked(client, target.address))
            .collect();
        let layers = self.covering_layers(target);
        let receiving = layers
            .iter()
            .filter_map(|(id, layer)| Some((*id, extents_of(&layer.rect)?)))
            .collect();
        let names = layers
            .iter()
            .map(|(id, layer)| (*id, layer.namespace.clone()))
            .collect();
        Some((
            stable_id(target)?,
            StackingSnapshot {
                current_desktop: None,
                stack,
                override_redirect_above: Vec::new(),
                override_redirect_receiving: receiving,
                override_redirect_names: names,
            },
        ))
    }

    /// The clients as covers are named from.
    pub(crate) fn named(&self) -> Vec<X11Client> {
        self.clients
            .iter()
            .filter_map(|client| {
                Some(X11Client {
                    id: stable_id(client)?,
                    pid: Some(client.owner.pid),
                    title: client.owner.title.clone(),
                    extents: None,
                    bounds: extents_of(&client.frame),
                    hidden: !self.shown(client),
                    shadow: (0, 0, 0, 0),
                })
            })
            .collect()
    }
}

/// The windowing of a frame under Hyprland, or `None` off Hyprland, for a
/// frame without one client, or when Hyprland does not answer in time.
pub(crate) fn hypr_windowing(frame: &FrameFacts, deadline: Deadline) -> Option<Windowing> {
    let instance = HyprInstance::detect()?;
    let clients = instance.clients(deadline).ok()?;
    let monitors = instance.monitors(deadline).ok()?;
    let layers = instance.layers(deadline).ok()?;
    let scene = HyprScene {
        clients: &clients,
        monitors: &monitors,
        layers: &layers,
    };
    let key = WindowKey::of_frame(frame.pid, &frame.title);
    let target = match_client(&clients, key).ok()?;
    let (client, snapshot) = scene.stacking(target)?;
    Some(Windowing {
        client,
        snapshot,
        clients: scene.named(),
        coordinate_origin: None,
        descent_root: None,
    })
}

#[cfg(test)]
#[path = "hypr_windowing_tests.rs"]
mod tests;
