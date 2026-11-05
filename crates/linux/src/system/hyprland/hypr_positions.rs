use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use agent_desktop_core::Deadline;

use super::HyprInstance;
use super::hypr_client::HyprClient;

const READ_BUDGET_MS: u64 = 500;

static ANSWERS: LazyLock<Mutex<HashMap<u32, bool>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// Whether a process under Hyprland draws through Wayland rather than
/// Xwayland, judged from its clients: only an Xwayland client knows where it
/// is on the screen.
pub(crate) fn draws_natively(clients: &[HyprClient], pid: u32) -> bool {
    !clients
        .iter()
        .any(|client| client.owner.pid == pid && client.owner.xwayland)
}

/// Whether the accessibility positions process `pid` reports are relative to
/// its own window because it is a native Wayland client of this session's
/// Hyprland. Any other session, and a Hyprland that does not answer, leave
/// the toolkit's own position model in place. One answer per process is
/// kept for the life of the command.
pub(crate) fn window_relative(pid: u32) -> bool {
    if let Some(known) = ANSWERS.lock().ok().and_then(|map| map.get(&pid).copied()) {
        return known;
    }
    let Some(instance) = HyprInstance::detect() else {
        return false;
    };
    let Ok(deadline) = Deadline::detached_after(READ_BUDGET_MS) else {
        return false;
    };
    let Ok(clients) = instance.clients(deadline) else {
        return false;
    };
    let native = draws_natively(&clients, pid);
    if let Ok(mut map) = ANSWERS.lock() {
        map.insert(pid, native);
    }
    native
}

#[cfg(test)]
#[path = "hypr_positions_tests.rs"]
mod tests;
