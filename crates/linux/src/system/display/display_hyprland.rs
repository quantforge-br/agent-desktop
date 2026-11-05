use agent_desktop_core::{AdapterError, Deadline, DisplayInfo};

use crate::system::hyprland::HyprInstance;
use crate::system::hyprland::hypr_monitor::HyprMonitor;

fn holds_layout_origin(monitor: &HyprMonitor) -> bool {
    let bounds = &monitor.logical;
    bounds.x <= 0.0
        && bounds.y <= 0.0
        && bounds.x + bounds.width > 0.0
        && bounds.y + bounds.height > 0.0
}

/// Hyprland's monitors as displays in its logical layout. Hyprland has no
/// primary output, so the monitor holding the layout origin is primary,
/// and the lowest monitor id is primary when no monitor holds it.
pub(crate) fn from_hyprland(monitors: Vec<HyprMonitor>) -> Vec<DisplayInfo> {
    let primary = monitors
        .iter()
        .find(|monitor| holds_layout_origin(monitor))
        .or_else(|| monitors.iter().min_by_key(|monitor| monitor.id))
        .map(|monitor| monitor.id);
    let mut displays: Vec<(bool, i64, DisplayInfo)> = monitors
        .into_iter()
        .map(|monitor| {
            let is_primary = Some(monitor.id) == primary;
            (
                is_primary,
                monitor.id,
                DisplayInfo {
                    id: monitor.name,
                    bounds: monitor.logical,
                    is_primary,
                    scale: monitor.scale,
                },
            )
        })
        .collect();
    displays.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
    displays
        .into_iter()
        .map(|(_, _, display)| display)
        .collect()
}

pub(crate) fn hyprland_displays(
    instance: &HyprInstance,
    deadline: Deadline,
) -> Result<Vec<DisplayInfo>, AdapterError> {
    let monitors = instance.monitors(deadline)?;
    if monitors.is_empty() {
        return Err(AdapterError::internal("Hyprland listed no enabled monitor")
            .with_suggestion("Enable an output in Hyprland, then retry"));
    }
    Ok(from_hyprland(monitors))
}

#[cfg(test)]
#[path = "display_hyprland_tests.rs"]
mod tests;
