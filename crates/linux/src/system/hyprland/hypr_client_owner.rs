/// The process and identity strings Hyprland records for a toplevel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HyprClientOwner {
    pub(crate) pid: u32,
    pub(crate) class: String,
    pub(crate) title: String,
    pub(crate) xwayland: bool,
}
