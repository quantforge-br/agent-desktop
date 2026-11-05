use agent_desktop_core::{Rect, WindowInfo};

/// What is known of a listed window for finding its Hyprland client.
#[derive(Debug, Clone, Copy)]
pub(crate) struct WindowKey<'a> {
    pub(crate) pid: u32,
    pub(crate) title: &'a str,
    pub(crate) bounds: Option<&'a Rect>,
}

impl<'a> WindowKey<'a> {
    pub(crate) fn of(window: &'a WindowInfo) -> Self {
        Self {
            pid: window.pid.get(),
            title: &window.title,
            bounds: window.bounds.as_ref(),
        }
    }
}
