use agent_desktop_core::{Rect, WindowInfo};

/// What is known of a listed window for finding its Hyprland client.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WindowKey<'a> {
    pub(crate) pid: u32,
    pub(crate) title: &'a str,
    pub(crate) bounds: Option<&'a Rect>,
    /// Whether no other listed window belongs to the process. Only a lone
    /// window may take its process's single client whatever its title: a
    /// shell that also lists its panels as frames has more frames than
    /// toplevels.
    pub(crate) alone: bool,
}

impl<'a> WindowKey<'a> {
    /// The key of a window already placed by a listing, whose bounds are its
    /// client's frame when the listing matched one.
    pub(crate) fn of(window: &'a WindowInfo) -> Self {
        Self {
            pid: window.pid.get(),
            title: &window.title,
            bounds: window.bounds.as_ref(),
            alone: true,
        }
    }

    /// The key of one accessibility frame known only by its process and
    /// title. An untitled frame is never taken as alone: a shell's panel
    /// frames have no title and no toplevel, so its one titled window must
    /// not lend them its client.
    pub(crate) fn of_frame(pid: u32, title: &'a str) -> Self {
        Self {
            pid,
            title,
            bounds: None,
            alone: !title.is_empty(),
        }
    }
}
