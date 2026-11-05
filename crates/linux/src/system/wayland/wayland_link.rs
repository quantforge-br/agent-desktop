use std::io::ErrorKind;

use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode};
use rustix::event::{PollFd, PollFlags, Timespec, poll};
use serde_json::json;
use wayland_client::backend::WaylandError;
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_client::{Connection, Dispatch, EventQueue, Proxy, QueueHandle};

use super::wayland_global::WaylandGlobal;
use super::wayland_state::WaylandState;

/// One client connection to the session's Wayland compositor with its own
/// event queue. Every wait on the compositor is bounded by a deadline.
pub(crate) struct WaylandLink {
    connection: Connection,
    queue: EventQueue<WaylandState>,
    registry: WlRegistry,
    pub(crate) state: WaylandState,
}

fn unreachable(detail: impl std::fmt::Display) -> AdapterError {
    AdapterError::new(
        ErrorCode::PlatformNotSupported,
        "The Wayland compositor did not accept a client connection",
    )
    .with_suggestion("Run inside the Wayland session, with WAYLAND_DISPLAY naming its socket")
    .with_platform_detail(detail.to_string())
    .with_details(json!({ "cause": "wayland_unreachable", "physical_delivery_started": false }))
    .with_disposition(DeliverySemantics::not_delivered())
}

fn connection_lost(detail: impl std::fmt::Display) -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionFailed,
        "The Wayland connection failed while the compositor was answering",
    )
    .with_platform_detail(detail.to_string())
    .with_details(json!({ "cause": "wayland_connection" }))
}

fn silent(deadline: Deadline) -> AdapterError {
    AdapterError::new(
        ErrorCode::Timeout,
        "The Wayland compositor did not answer in time",
    )
    .with_details(json!({
        "cause": "wayland_silent",
        "timeout_ms": deadline.timeout_ms(),
        "retryable": true,
    }))
}

fn would_block(error: &WaylandError) -> bool {
    matches!(error, WaylandError::Io(io) if io.kind() == ErrorKind::WouldBlock)
}

impl WaylandLink {
    pub(crate) fn connect(deadline: Deadline) -> Result<Self, AdapterError> {
        Self::over(Connection::connect_to_env().map_err(unreachable)?, deadline)
    }

    /// Starts a client on an open connection and reads the registry.
    pub(crate) fn over(connection: Connection, deadline: Deadline) -> Result<Self, AdapterError> {
        let queue = connection.new_event_queue();
        let registry = connection.display().get_registry(&queue.handle(), ());
        let mut link = Self {
            connection,
            queue,
            registry,
            state: WaylandState::default(),
        };
        link.sync(deadline)?;
        Ok(link)
    }

    pub(crate) fn handle(&self) -> QueueHandle<WaylandState> {
        self.queue.handle()
    }

    pub(crate) fn global(&self, interface: &str) -> Option<&WaylandGlobal> {
        self.state
            .globals
            .iter()
            .find(|global| global.interface == interface)
    }

    /// Binds the first advertised global of `I` at no more than
    /// `max_version`, or `None` when the compositor does not offer it.
    pub(crate) fn bind<I, U>(&self, max_version: u32, data: U) -> Option<I>
    where
        I: Proxy + 'static,
        U: Send + Sync + 'static,
        WaylandState: Dispatch<I, U>,
    {
        let global = self.global(I::interface().name)?.clone();
        Some(self.bind_global(&global, max_version, data))
    }

    /// Binds one specific advertised global.
    pub(crate) fn bind_global<I, U>(&self, global: &WaylandGlobal, max_version: u32, data: U) -> I
    where
        I: Proxy + 'static,
        U: Send + Sync + 'static,
        WaylandState: Dispatch<I, U>,
    {
        self.registry.bind::<I, U, WaylandState>(
            global.name,
            global.version.min(max_version),
            &self.handle(),
            data,
        )
    }

    /// Sends a `wl_display.sync` and waits for its callback, so every
    /// request sent before it has been processed by the compositor.
    pub(crate) fn sync(&mut self, deadline: Deadline) -> Result<(), AdapterError> {
        let token = self.state.synced.wrapping_add(1);
        self.connection.display().sync(&self.handle(), token);
        self.dispatch_until(deadline, |state| state.synced == token)
    }

    pub(crate) fn flush(&self) -> Result<(), AdapterError> {
        self.connection.flush().map_err(connection_lost)
    }

    /// Dispatches compositor events until `done` holds for the state.
    pub(crate) fn dispatch_until(
        &mut self,
        deadline: Deadline,
        mut done: impl FnMut(&WaylandState) -> bool,
    ) -> Result<(), AdapterError> {
        loop {
            self.queue
                .dispatch_pending(&mut self.state)
                .map_err(connection_lost)?;
            if done(&self.state) {
                return Ok(());
            }
            self.queue.flush().map_err(connection_lost)?;
            let Some(guard) = self.queue.prepare_read() else {
                continue;
            };
            let remaining = deadline.remaining();
            if remaining.is_zero() {
                return Err(silent(deadline));
            }
            let timeout = Timespec::try_from(remaining).map_err(|_| silent(deadline))?;
            let ready = {
                let fd = guard.connection_fd();
                let mut fds = [PollFd::new(&fd, PollFlags::IN)];
                match poll(&mut fds, Some(&timeout)) {
                    Ok(ready) => ready,
                    Err(rustix::io::Errno::INTR) => continue,
                    Err(error) => return Err(connection_lost(error)),
                }
            };
            if ready == 0 {
                return Err(silent(deadline));
            }
            match guard.read() {
                Ok(_) => {}
                Err(error) if would_block(&error) => {}
                Err(error) => return Err(connection_lost(error)),
            }
        }
    }
}

#[cfg(test)]
#[path = "wayland_link_tests.rs"]
mod tests;
