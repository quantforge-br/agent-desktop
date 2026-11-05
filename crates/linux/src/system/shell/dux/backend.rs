use agent_desktop_core::Deadline;

use super::proof::Proof;
use super::recognise::recognise;
use super::{
    config_dirs,
    procs::{LiveProcs, Procs},
};
use crate::system::bus::session_bus::SessionBus;
use crate::system::{NAME_HAS_NO_OWNER, SERVICE_UNKNOWN, owner_process_id, within};

const NOTIFICATIONS: &str = "org.freedesktop.Notifications";

/// The dux-shell process of this session and the fact that identified it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DuxShell {
    pub(crate) pid: u32,
    pub(crate) proof: Proof,
}

/// Which shell this session runs, decided from what the buses and `/proc`
/// say. GNOME Shell wins whenever its bus name has an owner; the dux-shell is
/// the quickshell process that owns the notification service; anything else
/// is left to the GNOME rules, which refuse it as they always did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Backend {
    Gnome,
    Dux(DuxShell),
    Unrecognised,
}

pub(crate) fn classify(
    gnome_owner: Option<u32>,
    notification_owner: Option<u32>,
    recognise: impl FnOnce(u32) -> Option<Proof>,
) -> Backend {
    if gnome_owner.is_some() {
        return Backend::Gnome;
    }
    notification_owner
        .and_then(|pid| recognise(pid).map(|proof| DuxShell { pid, proof }))
        .map_or(Backend::Unrecognised, Backend::Dux)
}

async fn owner(connection: &zbus::Connection, name: &str) -> Option<u32> {
    match owner_process_id(connection, name).await {
        Ok(pid) => Some(pid),
        Err(zbus::Error::MethodError(fault, _, _))
            if matches!(fault.as_str(), NAME_HAS_NO_OWNER | SERVICE_UNKNOWN) =>
        {
            None
        }
        Err(_) => None,
    }
}

/// The backend of the running session. A session bus that cannot be read
/// leaves the backend unrecognised, so the GNOME rules report it themselves.
pub(crate) fn select(deadline: Deadline) -> Backend {
    let Ok(session) = SessionBus::get(deadline) else {
        return Backend::Unrecognised;
    };
    let owners = within(deadline, async {
        Ok((
            owner(&session, super::super::identity::SHELL_BUS_NAME).await,
            owner(&session, NOTIFICATIONS).await,
        ))
    });
    let Ok((gnome, notifications)) = owners else {
        return Backend::Unrecognised;
    };
    let me = LiveProcs.user(std::process::id());
    classify(gnome, notifications, |pid| {
        recognise(&LiveProcs, pid, me?, &config_dirs())
    })
}

#[cfg(test)]
#[path = "backend_tests.rs"]
mod tests;
