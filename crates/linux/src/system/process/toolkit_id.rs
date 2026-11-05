/// The toolkit an application reports on its root, when it answered.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ToolkitId {
    pub(crate) name: Option<String>,
    pub(crate) version: Option<String>,
    /// The application is a native Wayland client of a Hyprland session, so
    /// its toolkit can only report positions relative to its own window.
    pub(crate) window_relative: bool,
}
