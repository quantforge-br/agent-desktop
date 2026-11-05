use agent_desktop_core::{AdapterError, Deadline, ErrorCode};
use serde_json::json;

use crate::system::shell::identity::pick_shell_app;
use crate::system::shell::registry;
use crate::system::{SharedBus, within};
use crate::tree::AtspiElement;

fn absent(detail: &str) -> AdapterError {
    AdapterError::new(
        ErrorCode::AppNotFound,
        "The dux-shell accessibility application is absent from this session",
    )
    .with_suggestion("Start the session with accessibility enabled so the shell registers with the AT-SPI registry")
    .with_details(json!({ "shell_tree": "absent", "detail": detail }))
}

/// The registry application of the shell process: the one application whose
/// process is the shell's. Its children are the shell's windows.
pub(crate) fn find_app(
    bus: &SharedBus,
    pid: u32,
    deadline: Deadline,
) -> Result<AtspiElement, AdapterError> {
    let accessibility = bus.get(deadline)?.inner().clone();
    let apps = within(deadline, registry::read(&accessibility))?;
    pick_shell_app(&apps, pid)
        .cloned()
        .ok_or_else(|| absent("no unique registry application identifies the shell process"))
}
