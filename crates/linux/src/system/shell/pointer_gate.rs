use agent_desktop_core::{AdapterError, Deadline};

use crate::input::backend::{Backend, backend};
use crate::input::wayland::keysym_send::pointer_refusal;
use crate::input::wayland::virtual_input::virtual_send;
use crate::system::bus::session_bus::SessionBus;
use crate::system::{owner_process_id, within};

const MUTTER_REMOTE_DESKTOP: &str = "org.gnome.Mutter.RemoteDesktop";

fn pointer_service_present(deadline: Deadline) -> bool {
    let Ok(connection) = SessionBus::get(deadline) else {
        return false;
    };
    within(deadline, async {
        Ok(owner_process_id(&connection, MUTTER_REMOTE_DESKTOP)
            .await
            .is_ok())
    })
    .unwrap_or(false)
}

/// The refusal for a click the session cannot deliver, decided before any
/// panel is opened: a Wayland session delivers shell clicks through
/// Mutter's RemoteDesktop service or Hyprland's virtual pointer, and every
/// other compositor offers neither.
pub(super) fn gate(
    backend: Result<Backend, AdapterError>,
    pointer_service: impl FnOnce() -> bool,
) -> Result<(), AdapterError> {
    match backend {
        Ok(Backend::Wayland) if !pointer_service() => {
            Err(pointer_refusal("physical pointer input on the shell"))
        }
        _ => Ok(()),
    }
}

/// Fails fast when a click on a shell control cannot be delivered, so a
/// mutation never opens a panel it then cannot press in.
pub(crate) fn require_pointer_capability(deadline: Deadline) -> Result<(), AdapterError> {
    gate(backend(), || {
        virtual_send::compositor().is_some() || pointer_service_present(deadline)
    })
}

#[cfg(test)]
#[path = "pointer_gate_tests.rs"]
mod tests;
