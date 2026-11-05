use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline, ErrorCode, PermissionReport, PermissionState};

use crate::system::SharedBus;
use crate::system::bus::probe_registry;
use crate::system::display::route::{Route, current};
use crate::system::x11::capture::grab::{DisplayGrabber, Grabber};

const CAPTURE_PROBE_SLICE: Duration = Duration::from_millis(1_000);
/// Every command that needs a permission report pays this probe before its
/// own deadline starts, so a hung Hyprland must not cost it a second. A
/// healthy one answers a monitor request in a few milliseconds.
const HYPRLAND_PROBE_SLICE: Duration = Duration::from_millis(250);

pub(crate) fn report(
    bus: &SharedBus,
    deadline: Deadline,
) -> Result<PermissionReport, AdapterError> {
    crate::input::x11::stale_release::sweep();
    let capture = if hyprland_answers(deadline.capped(HYPRLAND_PROBE_SLICE)) {
        CaptureAvailability::Reachable
    } else {
        capture_availability(
            current(),
            &DisplayGrabber,
            deadline.capped(CAPTURE_PROBE_SLICE),
        )
    };
    let probed = bus
        .get(deadline)
        .and_then(|connection| probe_registry(bus, &connection, deadline));
    let report = report_from_probe(probed, capture)?;
    deadline.ensure_remaining()?;
    Ok(report)
}

/// Whether this session's Hyprland answers a monitor request. Its capture
/// route copies frames with `ext-image-copy-capture-v1` and asks nobody, so
/// capture needs no permission there.
fn hyprland_answers(deadline: Deadline) -> bool {
    crate::system::hyprland::HyprInstance::detect()
        .is_some_and(|instance| instance.monitors(deadline).is_ok())
}

/// Whether the accessibility registry answers on the session bus, which is
/// what every snapshot surface reads through.
pub(crate) fn accessibility_answers(bus: &SharedBus, deadline: Deadline) -> bool {
    bus.get(deadline)
        .and_then(|connection| probe_registry(bus, &connection, deadline))
        .is_ok()
}

/// Whether the session's capture route answered. An X11 display or a
/// Hyprland that answers inside the deadline is `Reachable`; every other
/// case, GNOME Wayland included, is `Unknown` because nothing was learned
/// about the route.
pub(crate) enum CaptureAvailability {
    Reachable,
    Unknown,
}

/// Probes the X11 display with one bounded request. A Wayland session, a
/// missing display and a display that does not answer are all `Unknown`; no
/// portal is called.
pub(crate) fn capture_availability(
    route: Result<Route, AdapterError>,
    grabber: &dyn Grabber,
    deadline: Deadline,
) -> CaptureAvailability {
    match route {
        Ok(Route::X11(_)) if grabber.compositing(deadline).is_ok() => {
            CaptureAvailability::Reachable
        }
        _ => CaptureAvailability::Unknown,
    }
}

/// An unreachable bus is the accessibility permission being unavailable, so it
/// is reported as `Denied` with the cause-specific recovery step. A timeout or
/// an unexpected bus error stays an error: nothing was learned about the bus.
pub(crate) fn report_from_probe(
    probed: Result<(), AdapterError>,
    capture: CaptureAvailability,
) -> Result<PermissionReport, AdapterError> {
    let accessibility = match probed {
        Ok(()) => PermissionState::Granted,
        Err(error) if error.code == ErrorCode::PlatformNotSupported => PermissionState::Denied {
            suggestion: error
                .suggestion
                .unwrap_or_else(|| "Start the AT-SPI accessibility bus and retry".into()),
        },
        Err(error) => return Err(error),
    };
    Ok(PermissionReport {
        accessibility,
        screen_recording: match capture {
            CaptureAvailability::Reachable => PermissionState::NotRequired,
            CaptureAvailability::Unknown => PermissionState::Unknown,
        },
        automation: PermissionState::NotRequired,
    })
}

#[cfg(test)]
#[path = "permissions_tests.rs"]
mod tests;
