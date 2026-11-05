use wayland_client::protocol::wl_output::WlOutput;

/// One bound `wl_output` and the connector name its `name` event gave.
#[derive(Debug, Clone)]
pub(crate) struct WaylandOutput {
    pub(crate) proxy: WlOutput,
    pub(crate) name: Option<String>,
}
