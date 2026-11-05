use std::io::{ErrorKind, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode};
use serde_json::json;

const MAX_REPLY_BYTES: usize = 16 * 1024 * 1024;
const MINIMUM_SLICE: Duration = Duration::from_millis(1);

/// One running Hyprland compositor, addressed by its request socket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HyprInstance {
    socket: PathBuf,
}

fn valid_signature(signature: &str) -> bool {
    !signature.is_empty()
        && signature
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn lock_socket_name(lock: &Path) -> Option<String> {
    let text = std::fs::read_to_string(lock).ok()?;
    let mut lines = text.lines();
    lines.next()?;
    let name = lines.next()?.trim();
    (!name.is_empty()).then(|| name.to_owned())
}

fn display_name(wayland_display: &str) -> &str {
    Path::new(wayland_display)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(wayland_display)
}

fn non_empty(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

impl HyprInstance {
    #[cfg(test)]
    pub(crate) fn at(socket: PathBuf) -> Self {
        Self { socket }
    }

    /// The Hyprland instance of this session: the one named by
    /// `HYPRLAND_INSTANCE_SIGNATURE` whose lock file says it serves the
    /// session's `WAYLAND_DISPLAY`. A signature inherited from another
    /// session, whose instance serves a different Wayland socket, is not it.
    /// Unit test builds never detect one, so no test reaches the compositor
    /// of the desktop it runs in.
    pub(crate) fn detect() -> Option<Self> {
        if cfg!(test) {
            return None;
        }
        Self::find(
            &non_empty("XDG_RUNTIME_DIR")?,
            &non_empty("HYPRLAND_INSTANCE_SIGNATURE")?,
            &non_empty("WAYLAND_DISPLAY")?,
        )
    }

    pub(crate) fn find(runtime_dir: &str, signature: &str, wayland_display: &str) -> Option<Self> {
        if !valid_signature(signature) {
            return None;
        }
        let directory = Path::new(runtime_dir).join("hypr").join(signature);
        let served = lock_socket_name(&directory.join("hyprland.lock"))?;
        if served != display_name(wayland_display) {
            return None;
        }
        let socket = directory.join(".socket.sock");
        socket.exists().then_some(Self { socket })
    }

    /// Sends one request and returns Hyprland's whole reply. Hyprland
    /// answers each connection once and closes it.
    pub(crate) fn request(
        &self,
        command: &str,
        deadline: Deadline,
    ) -> Result<String, AdapterError> {
        let slice = deadline.remaining();
        if slice.is_zero() {
            return Err(silent(command, deadline));
        }
        let mut stream = UnixStream::connect(&self.socket).map_err(|error| unreachable(&error))?;
        let slice = deadline.remaining().max(MINIMUM_SLICE);
        stream
            .set_write_timeout(Some(slice))
            .and_then(|()| stream.set_read_timeout(Some(slice)))
            .map_err(|error| unreachable(&error))?;
        stream
            .write_all(command.as_bytes())
            .map_err(|error| io_failure(command, &error, deadline))?;
        let mut reply = Vec::new();
        let mut chunk = [0_u8; 64 * 1024];
        loop {
            let read = match stream.read(&mut chunk) {
                Ok(read) => read,
                Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                Err(error) => return Err(io_failure(command, &error, deadline)),
            };
            if read == 0 {
                break;
            }
            if reply.len() + read > MAX_REPLY_BYTES {
                return Err(AdapterError::internal(
                    "Hyprland's IPC reply is larger than this tool reads",
                )
                .with_details(json!({ "request": command, "limit_bytes": MAX_REPLY_BYTES })));
            }
            reply.extend_from_slice(&chunk[..read]);
            if deadline.is_expired() {
                return Err(silent(command, deadline));
            }
        }
        String::from_utf8(reply).map_err(|_| {
            AdapterError::internal("Hyprland's IPC reply is not UTF-8")
                .with_details(json!({ "request": command }))
        })
    }
}

fn unreachable(error: &std::io::Error) -> AdapterError {
    AdapterError::new(
        ErrorCode::PlatformNotSupported,
        "Hyprland's IPC socket did not accept the connection",
    )
    .with_suggestion("Run inside the Hyprland session whose HYPRLAND_INSTANCE_SIGNATURE is set")
    .with_platform_detail(error.to_string())
    .with_details(json!({ "compositor": "hyprland", "hyprland_ipc": "unreachable" }))
    .with_disposition(DeliverySemantics::not_delivered())
}

fn silent(command: &str, deadline: Deadline) -> AdapterError {
    AdapterError::new(ErrorCode::Timeout, "Hyprland's IPC did not answer in time").with_details(
        json!({
            "compositor": "hyprland",
            "hyprland_ipc": "silent",
            "request": command,
            "timeout_ms": deadline.timeout_ms(),
            "retryable": true,
        }),
    )
}

fn io_failure(command: &str, error: &std::io::Error, deadline: Deadline) -> AdapterError {
    match error.kind() {
        ErrorKind::WouldBlock | ErrorKind::TimedOut => silent(command, deadline),
        _ => AdapterError::new(ErrorCode::ActionFailed, "Hyprland's IPC request failed")
            .with_platform_detail(error.to_string())
            .with_details(json!({ "compositor": "hyprland", "request": command })),
    }
}

#[cfg(test)]
#[path = "hypr_instance_tests.rs"]
mod tests;
