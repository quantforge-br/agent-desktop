/// One window dispatcher this adapter sends to Hyprland. Positions and
/// sizes are in the logical layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum HyprCommand {
    Focus,
    Float(bool),
    MoveTo { x: i64, y: i64 },
    ResizeTo { width: i64, height: i64 },
    Maximize(bool),
    Fullscreen(bool),
    ToWorkspace(String),
    Close,
}
