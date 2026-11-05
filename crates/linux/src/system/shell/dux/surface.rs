use agent_desktop_core::{AdapterError, DeliverySemantics, ErrorCode, SnapshotSurface};

use super::header::tab_header_start;

/// What the dux-shell can present, in its own terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Surface {
    NotificationCenter,
    ControlCenter,
    Launcher,
    Dock,
    Bar,
}

/// The IPC function that shows a surface, and the handler that offers it.
pub(crate) struct Opener {
    pub(crate) target: &'static str,
    pub(crate) open: &'static str,
}

const OFFERED: &str = "Use notification-center (action-center), quick-settings, spotlight, dock (taskbar) or system-tray";

impl Surface {
    pub(crate) const ALL: [Self; 5] = [
        Self::NotificationCenter,
        Self::ControlCenter,
        Self::Launcher,
        Self::Dock,
        Self::Bar,
    ];

    /// The layer-shell namespace the shell maps the surface under.
    pub(crate) fn namespace(self) -> &'static str {
        match self {
            Self::NotificationCenter => "dms:notification-center-modal",
            Self::ControlCenter => "dms:control-center",
            Self::Launcher => "dms:spotlight",
            Self::Dock => "dms:dock",
            Self::Bar => "dms:bar",
        }
    }

    pub(crate) fn title(self) -> &'static str {
        match self {
            Self::NotificationCenter => "Notification center",
            Self::ControlCenter => "Control center",
            Self::Launcher => "Launcher",
            Self::Dock => "Dock",
            Self::Bar => "Top bar",
        }
    }

    pub(crate) fn opener(self) -> Option<Opener> {
        match self {
            Self::NotificationCenter => Some(Opener {
                target: "notifications",
                open: "open",
            }),
            Self::ControlCenter => Some(Opener {
                target: "control-center",
                open: "open",
            }),
            Self::Launcher => Some(Opener {
                target: "launcher",
                open: "open",
            }),
            Self::Dock => Some(Opener {
                target: "dock",
                open: "reveal",
            }),
            Self::Bar => None,
        }
    }

    /// Whether the surface's accessibility window holds content. The bar's
    /// window holds none: the tray in it is read through the tray protocol.
    pub(crate) fn has_content(self) -> bool {
        self != Self::Bar
    }

    /// Whether a window with these child roles can be this surface's window.
    /// The notification center opens with its tab header, which no other
    /// window of the shell has; the launcher is the full-screen window that
    /// does not, since both cover the whole output.
    pub(crate) fn recognises(self, roles: &[u32]) -> bool {
        match self {
            Self::NotificationCenter => tab_header_start(roles).is_some(),
            Self::Launcher => tab_header_start(roles).is_none(),
            Self::ControlCenter | Self::Dock | Self::Bar => true,
        }
    }

    /// The snapshot surfaces that name this surface.
    pub(crate) fn snapshots(self) -> &'static [SnapshotSurface] {
        match self {
            Self::NotificationCenter => &[
                SnapshotSurface::NotificationCenter,
                SnapshotSurface::ActionCenter,
            ],
            Self::ControlCenter => &[SnapshotSurface::QuickSettings],
            Self::Launcher => &[SnapshotSurface::Spotlight],
            Self::Dock => &[SnapshotSurface::Dock, SnapshotSurface::Taskbar],
            Self::Bar => &[SnapshotSurface::SystemTray],
        }
    }
}

fn refusal(message: &str) -> AdapterError {
    AdapterError::new(ErrorCode::InvalidArgs, message)
        .with_suggestion(OFFERED)
        .with_disposition(DeliverySemantics::not_delivered())
}

/// The dux-shell surface a requested surface names, or the refusal saying
/// what the shell has instead.
pub(crate) fn of(requested: SnapshotSurface) -> Result<Surface, AdapterError> {
    match requested {
        SnapshotSurface::ActionCenter | SnapshotSurface::NotificationCenter => {
            Ok(Surface::NotificationCenter)
        }
        SnapshotSurface::QuickSettings => Ok(Surface::ControlCenter),
        SnapshotSurface::Spotlight => Ok(Surface::Launcher),
        SnapshotSurface::Taskbar | SnapshotSurface::Dock => Ok(Surface::Dock),
        SnapshotSurface::SystemTray => Ok(Surface::Bar),
        SnapshotSurface::StartMenu => Err(refusal(
            "The dux-shell has no start menu; its launcher is the spotlight surface",
        )),
        SnapshotSurface::MenuBarExtras | SnapshotSurface::Menubar => Err(refusal(
            "The dux-shell has no global menu bar; its status items are the system-tray surface",
        )),
        SnapshotSurface::SystemTrayOverflow => Err(refusal(
            "The dux-shell's tray has no overflow popup; read the system-tray surface instead",
        )),
        other => Err(refusal(&format!(
            "'{}' is not a shell surface the dux-shell has",
            other.as_str()
        ))),
    }
}

#[cfg(test)]
#[path = "surface_tests.rs"]
mod tests;
