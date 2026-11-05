use super::capture_progress::CaptureProgress;
use super::foreign_toplevel::ForeignToplevel;
use super::wayland_global::WaylandGlobal;
use super::wayland_output::WaylandOutput;

/// Everything the compositor has told this client: its globals, the
/// outputs and toplevels it listed, the latest completed sync, whether the
/// seat has a pointer, and the progress of an image capture.
#[derive(Debug, Default)]
pub(crate) struct WaylandState {
    pub(crate) globals: Vec<WaylandGlobal>,
    pub(crate) outputs: Vec<WaylandOutput>,
    pub(crate) toplevels: Vec<ForeignToplevel>,
    pub(crate) toplevels_listed: bool,
    pub(crate) synced: u32,
    pub(crate) seat_pointer: Option<bool>,
    pub(crate) capture: CaptureProgress,
}
