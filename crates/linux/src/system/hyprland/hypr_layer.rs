use agent_desktop_core::Rect;

/// One layer-shell surface as Hyprland's `layers` request lists it: a bar,
/// dock, notification or other shell surface. `level` is the layer, from 0
/// for the background to 3 for the overlay; levels 2 and 3 draw above
/// windows.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct HyprLayer {
    pub(crate) monitor: String,
    pub(crate) level: u8,
    pub(crate) namespace: String,
    pub(crate) pid: u32,
    pub(crate) rect: Rect,
}

impl HyprLayer {
    pub(crate) const TOP: u8 = 2;
    pub(crate) const OVERLAY: u8 = 3;
}
