/// Where a toplevel sits in Hyprland's layout and how it is shown.
/// `fullscreen` is Hyprland's internal mode: 0 none, 1 maximized,
/// 2 fullscreen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HyprClientState {
    pub(crate) workspace: String,
    pub(crate) monitor: i64,
    pub(crate) floating: bool,
    pub(crate) fullscreen: u8,
    pub(crate) mapped: bool,
    pub(crate) hidden: bool,
    pub(crate) pinned: bool,
}
