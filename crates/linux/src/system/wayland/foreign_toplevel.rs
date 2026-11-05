use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_handle_v1::ExtForeignToplevelHandleV1;

/// One toplevel the compositor lists through `ext_foreign_toplevel_list_v1`.
/// `identifier` is the compositor's stable id for it; Hyprland prints the
/// same id as a client's `stableId`.
#[derive(Debug, Clone)]
pub(crate) struct ForeignToplevel {
    pub(crate) handle: ExtForeignToplevelHandleV1,
    pub(crate) identifier: String,
    pub(crate) title: String,
    pub(crate) app_id: String,
    pub(crate) closed: bool,
}
