use agent_desktop_core::{Deadline, ErrorCode};

use crate::system::hyprland::HyprInstance;
use crate::system::hyprland::hypr_client::HyprClient;
use crate::system::hyprland::hypr_command::HyprCommand;
use crate::system::x11::control::ControlRead;
use crate::system::x11::control::control_error::ControlError;
use crate::system::x11::window::toplevel::Toplevel;
use crate::system::x11::window::window_kind::WindowKind;

/// The 32-bit window id a close request names a Hyprland client by: its
/// stable id, which Hyprland prints in hexadecimal.
pub(crate) fn close_id(client: &HyprClient) -> Option<u32> {
    u32::from_str_radix(&client.stable_id, 16).ok()
}

fn owned(clients: Vec<HyprClient>, pid: u32) -> impl Iterator<Item = HyprClient> {
    clients
        .into_iter()
        .filter(move |client| client.state.mapped && client.owner.pid == pid)
}

/// The mapped Hyprland clients of process `pid` as close targets. Hyprland
/// reports no window type, so every client is a normal window; a prompt a
/// close raises is found through the accessibility tree instead.
pub(crate) fn toplevels_of(
    instance: &HyprInstance,
    pid: u32,
    deadline: Deadline,
) -> ControlRead<Vec<Toplevel>> {
    match instance.clients(deadline) {
        Ok(clients) => ControlRead::Ready(
            owned(clients, pid)
                .filter_map(|client| {
                    Some(Toplevel {
                        id: close_id(&client)?,
                        title: client.owner.title,
                        kind: WindowKind::Normal,
                        transient: false,
                    })
                })
                .collect(),
        ),
        Err(_) => ControlRead::Unavailable,
    }
}

fn control_error(code: ErrorCode) -> ControlError {
    match code {
        ErrorCode::Timeout => ControlError::TimedOut,
        ErrorCode::PlatformNotSupported => ControlError::NoDisplay,
        _ => ControlError::SendFailed,
    }
}

/// Asks Hyprland to close the client of process `pid` whose stable id is
/// `window`, the request a window's close button sends.
pub(crate) fn request_close(
    instance: &HyprInstance,
    pid: u32,
    window: u32,
    deadline: Deadline,
) -> Result<(), ControlError> {
    let clients = instance
        .clients(deadline)
        .map_err(|error| control_error(error.code))?;
    let client = owned(clients, pid)
        .find(|client| close_id(client) == Some(window))
        .ok_or(ControlError::BadWindow)?;
    instance
        .dispatch(client.address, &HyprCommand::Close, deadline)
        .map_err(|error| control_error(error.code))
}

#[cfg(test)]
#[path = "hypr_close_tests.rs"]
mod tests;
