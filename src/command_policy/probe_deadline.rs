//! The deadline of the permission probe a command pays before its own
//! deadline starts.

use agent_desktop_core::{AdapterError, DEFAULT_OPERATION_TIMEOUT_MS, Deadline};

use crate::cli::Commands;

/// The least time the probe gets. A healthy bus answers in milliseconds, so
/// a command with a very short timeout still reaches it.
const MIN_PROBE_MS: u64 = 1_000;

/// The probe is held to the command's own timeout, kept between
/// [`MIN_PROBE_MS`] and the standard operation deadline. A bus that accepts
/// the connection and never answers then ends a short command within its
/// timeout plus a second, not after the standard five seconds.
pub(crate) fn permission_probe_deadline(cmd: &Commands) -> Result<Deadline, AdapterError> {
    Deadline::after(
        own_timeout_ms(cmd).map_or(DEFAULT_OPERATION_TIMEOUT_MS, |timeout_ms| {
            timeout_ms.clamp(MIN_PROBE_MS, DEFAULT_OPERATION_TIMEOUT_MS)
        }),
    )
}

/// The timeout the command runs under, when it declares one. Zero is the
/// flags' "do not wait" value, so it bounds nothing.
fn own_timeout_ms(cmd: &Commands) -> Option<u64> {
    let declared = match cmd {
        Commands::Snapshot(args) => Some(
            args.timeout_ms
                .unwrap_or(agent_desktop_core::snapshot::DEFAULT_SNAPSHOT_TIMEOUT_MS),
        ),
        Commands::Find(args) => args.timeout_ms,
        Commands::Click(args)
        | Commands::DoubleClick(args)
        | Commands::TripleClick(args)
        | Commands::RightClick(args)
        | Commands::Clear(args)
        | Commands::Focus(args)
        | Commands::Toggle(args)
        | Commands::Check(args)
        | Commands::Uncheck(args)
        | Commands::Expand(args)
        | Commands::Collapse(args)
        | Commands::ScrollTo(args) => Some(args.timeout_ms),
        Commands::Type(args) => Some(args.effective_timeout_ms()),
        Commands::SetValue(args) => Some(args.timeout_ms),
        Commands::Select(args) => Some(args.timeout_ms),
        Commands::Scroll(args) => Some(args.timeout_ms),
        Commands::Hover(args) => Some(args.timeout_ms),
        Commands::Drag(args) => Some(args.timeout_ms),
        Commands::Wait(args) if args.mode.ms.is_none() => Some(args.timeout),
        Commands::Launch(args) => Some(args.timeout),
        Commands::ListWindows(args) => args.timeout_ms,
        Commands::ListApps(args) => args.timeout_ms,
        Commands::Batch(args) => Some(args.timeout_ms),
        Commands::Wait(_)
        | Commands::Screenshot(_)
        | Commands::Get(_)
        | Commands::Is(_)
        | Commands::Press(_)
        | Commands::KeyDown(_)
        | Commands::KeyUp(_)
        | Commands::MouseMove(_)
        | Commands::MouseClick(_)
        | Commands::MouseDown(_)
        | Commands::MouseUp(_)
        | Commands::MouseWheel(_)
        | Commands::CloseApp(_)
        | Commands::ListDisplays
        | Commands::FocusWindow(_)
        | Commands::ResizeWindow(_)
        | Commands::MoveWindow(_)
        | Commands::Minimize(_)
        | Commands::Maximize(_)
        | Commands::Restore(_)
        | Commands::ListSurfaces(_)
        | Commands::OpenSystemSurface(_)
        | Commands::ListNotifications(_)
        | Commands::DismissNotification(_)
        | Commands::DismissAllNotifications(_)
        | Commands::NotificationAction(_)
        | Commands::ClipboardGet(_)
        | Commands::ClipboardSet(_)
        | Commands::ClipboardClear
        | Commands::Status
        | Commands::Permissions(_)
        | Commands::Version
        | Commands::Skills(_)
        | Commands::Session(_)
        | Commands::CursorOverlay(_)
        | Commands::Trace(_) => None,
    };
    declared.filter(|timeout_ms| *timeout_ms > 0)
}

#[cfg(test)]
#[path = "probe_deadline_tests.rs"]
mod tests;
