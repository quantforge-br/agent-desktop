//! A Wayland protocol client of the session compositor, for the globals
//! the compositor offers to clients directly: virtual input devices, the
//! foreign toplevel list and image copy capture. It speaks the wire
//! protocol in Rust and links no `libwayland`.

pub(crate) mod capture_dispatch;
pub(crate) mod capture_progress;
pub(crate) mod core_dispatch;
pub(crate) mod foreign_toplevel;
pub(crate) mod frame_status;
pub(crate) mod wayland_global;
pub(crate) mod wayland_link;
pub(crate) mod wayland_output;
pub(crate) mod wayland_state;

pub(crate) use wayland_link::WaylandLink;
