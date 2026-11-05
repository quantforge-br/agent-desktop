//! Hyprland's request socket: the instance detection that ties a Hyprland
//! to this session's Wayland socket, the client and monitor records its
//! JSON requests answer with, and the Lua window dispatchers.

pub(crate) mod hypr_client;
pub(crate) mod hypr_client_owner;
pub(crate) mod hypr_client_state;
pub(crate) mod hypr_command;
pub(crate) mod hypr_dispatch;
pub(crate) mod hypr_instance;
pub(crate) mod hypr_layer;
pub(crate) mod hypr_layers;
pub(crate) mod hypr_monitor;
pub(crate) mod hypr_positions;
pub(crate) mod hypr_read;

pub(crate) use hypr_instance::HyprInstance;
