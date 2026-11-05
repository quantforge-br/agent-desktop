use agent_desktop_core::{
    AdapterError, Deadline, DeliverySemantics, ErrorCode, InteractionLease, InteractionPolicy,
    ObservationRequest, ObservationRoot, ObservedTree, SnapshotSurface, WindowInfo,
};
use serde_json::json;

use super::app::find_app;
use super::backend::DuxShell;
use super::present::{Stage, open_with, resolve_with};
use super::recognise::is_quickshell;
use super::stage::LiveStage;
use super::surface::{Surface, of};
use crate::system::bus::session_bus::SessionBus;
use crate::system::shell::tray_tree;
use crate::system::window::window_id;
use crate::system::{SharedBus, owner_process_id, within};

const WATCHER: &str = "org.kde.StatusNotifierWatcher";

fn foreign_tray(owner: Option<u32>) -> AdapterError {
    let (message, state) = match owner {
        Some(_) => (
            "The system tray of this session is not the shell's own",
            "foreign_watcher",
        ),
        None => (
            "No StatusNotifierWatcher is running in this session, so the shell's tray cannot be read",
            "no_watcher",
        ),
    };
    AdapterError::new(ErrorCode::PlatformNotSupported, message)
    .with_suggestion("Start the shell's status notifier host, or read the tray with a command that reads the tray protocol directly")
    .with_details(json!({ "tray": state, "watcher_pid": owner }))
    .with_disposition(DeliverySemantics::not_delivered())
}

/// Requires the StatusNotifier watcher to be the shell's own, so the tray the
/// protocol shows is the tray of the shell's bar.
fn require_own_tray(shell: DuxShell, deadline: Deadline) -> Result<(), AdapterError> {
    crate::system::tray::require_watcher(deadline)?;
    let connection = SessionBus::get(deadline)?;
    let owner = within(deadline, async {
        Ok(owner_process_id(&connection, WATCHER)
            .await
            .map_err(|error| error.to_string()))
    })?;
    match owner {
        Ok(pid) if pid == shell.pid => Ok(()),
        Ok(pid) => Err(foreign_tray(Some(pid))),
        Err(why) => Err(foreign_tray(None).with_platform_detail(why)),
    }
}

fn backed(shell: DuxShell, surface: Surface, deadline: Deadline) -> Result<(), AdapterError> {
    match surface {
        Surface::Bar => require_own_tray(shell, deadline),
        _ => Ok(()),
    }
}

/// The window presenting the requested surface now.
pub(crate) fn resolve(
    bus: &SharedBus,
    shell: DuxShell,
    requested: SnapshotSurface,
    deadline: Deadline,
) -> Result<WindowInfo, AdapterError> {
    let surface = of(requested)?;
    backed(shell, surface, deadline)?;
    resolve_with(&LiveStage::new(bus, shell), surface, deadline)
}

/// Presents the requested surface and answers with its window.
pub(crate) fn open(
    bus: &SharedBus,
    shell: DuxShell,
    requested: SnapshotSurface,
    policy: InteractionPolicy,
    lease: &InteractionLease,
) -> Result<WindowInfo, AdapterError> {
    let surface = of(requested)?;
    backed(shell, surface, lease.deadline())?;
    open_with(
        &LiveStage::new(bus, shell),
        surface,
        policy,
        lease.deadline(),
    )
}

fn offered(stage: &dyn Stage, surface: Surface, deadline: Deadline) -> bool {
    let present = stage.layers(deadline).is_ok_and(|layers| {
        layers
            .iter()
            .any(|layer| layer.namespace == surface.namespace() && layer.pid == stage.pid())
    });
    match surface.opener() {
        Some(opener) => {
            present
                || stage
                    .ipc()
                    .exposes(opener.target, opener.open, deadline)
                    .unwrap_or(false)
        }
        None => present,
    }
}

/// The surfaces the shell can show or has on screen: those it offers IPC for
/// or maps, and for the bar, those whose tray is the shell's own.
pub(crate) fn supported(
    bus: &SharedBus,
    shell: DuxShell,
    deadline: Deadline,
) -> Vec<SnapshotSurface> {
    let stage = LiveStage::new(bus, shell);
    Surface::ALL
        .into_iter()
        .filter(|surface| backed(shell, *surface, deadline).is_ok())
        .filter(|surface| offered(&stage, *surface, deadline))
        .flat_map(|surface| surface.snapshots().iter().copied())
        .collect()
}

fn stale(message: &str) -> AdapterError {
    AdapterError::new(ErrorCode::StaleRef, message)
        .with_suggestion("Run 'snapshot' to refresh, then retry with the new ref")
}

/// Whether `window` was taken from a dux-shell, this one or one that has
/// since restarted, rather than chosen by the application path for a request
/// that named no window: that path knows no shell surface and offers another
/// application's window.
fn taken_from_a_shell(shell: DuxShell, window: &WindowInfo) -> bool {
    window.pid.get() == shell.pid || is_quickshell(&window.app)
}

/// Observes a shell surface window: a window taken from the shell must still
/// be the one that presents the surface, any other names the surface the
/// shell presents now, and the tray is read through its protocol because the
/// bar's own window holds no content.
pub(crate) fn observe(
    bus: &SharedBus,
    shell: DuxShell,
    window: &WindowInfo,
    request: &ObservationRequest,
) -> Result<ObservedTree, AdapterError> {
    let surface = of(request.surface)?;
    let current = resolve(bus, shell, request.surface, request.deadline)?;
    let window = if taken_from_a_shell(shell, window) {
        window
    } else {
        &current
    };
    let (bus_name, _) = window_id::decode(&window.id)?;
    let app = find_app(bus, shell.pid, request.deadline)?;
    if app.bus_name() != bus_name {
        return Err(stale("The shell was restarted since this window was taken"));
    }
    if current.id != window.id {
        return Err(stale(
            "The shell surface changed before it could be observed",
        ));
    }
    if surface == Surface::Bar {
        let view = crate::system::tray::view::read(request.deadline)?;
        return tray_tree::logical_tree(&view, window, request.surface);
    }
    let mut inner = *request;
    inner.surface = SnapshotSurface::Window;
    crate::tree::observe_tree(bus, ObservationRoot::Window(window), &inner)
}

#[cfg(test)]
#[path = "entry_tests.rs"]
mod tests;
