use agent_desktop_core::{AdapterError, Deadline, ImageBuffer, ScreenshotTarget};

use super::capture_source::CaptureSource;
use super::display::select;
use super::image_copy;
use crate::system::display::display_hyprland::hyprland_displays;
use crate::system::hyprland::HyprInstance;
use crate::system::window::hypr_window::hypr_window_key::WindowKey;
use crate::system::window::hypr_window::hypr_window_match::match_client;

/// Captures `target` under Hyprland with `ext-image-copy-capture-v1`: a
/// display by its connector name, a window by the foreign toplevel whose
/// identifier is the window's Hyprland stable id. No portal dialog is
/// involved and the pointer is not painted.
pub(crate) fn capture_hyprland(
    instance: &HyprInstance,
    target: &ScreenshotTarget,
    deadline: Deadline,
) -> Result<ImageBuffer, AdapterError> {
    match target {
        ScreenshotTarget::ExactWindow(window) => {
            let clients = instance.clients(deadline)?;
            let client = match_client(&clients, WindowKey::of(window))
                .map_err(|unmatched| unmatched.into_error())?;
            let scale = instance
                .monitors(deadline)?
                .iter()
                .find(|monitor| monitor.id == client.state.monitor)
                .map_or(1.0, |monitor| monitor.scale);
            let source = CaptureSource::Toplevel(client.stable_id.clone());
            image_copy::capture(&source, scale, deadline)
        }
        target => {
            let displays = hyprland_displays(instance, deadline)?;
            let (_, display) = select(&displays, target)?;
            image_copy::capture(&CaptureSource::Output(display.id), display.scale, deadline)
        }
    }
}
