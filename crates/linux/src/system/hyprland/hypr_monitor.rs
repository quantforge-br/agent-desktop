use agent_desktop_core::Rect;

/// One enabled output as Hyprland's `monitors` request lists it, with its
/// place in the logical layout that window positions and the pointer use.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct HyprMonitor {
    pub(crate) id: i64,
    pub(crate) name: String,
    pub(crate) logical: Rect,
    pub(crate) scale: f64,
    pub(crate) focused: bool,
    pub(crate) active_workspace: String,
    pub(crate) special_workspace: Option<String>,
}

impl HyprMonitor {
    /// Whether a toplevel on `workspace` is on this monitor's screen now.
    pub(crate) fn shows(&self, workspace: &str) -> bool {
        self.active_workspace == workspace || self.special_workspace.as_deref() == Some(workspace)
    }
}
