//! Physical input through the compositor's virtual devices:
//! `zwp_virtual_keyboard_v1` for keys and `zwlr_virtual_pointer_v1` for
//! the pointer, as Hyprland offers them to its clients.

pub(crate) mod layout_box;
pub(crate) mod virtual_devices;
pub(crate) mod virtual_key_home;
pub(crate) mod virtual_key_session;
pub(crate) mod virtual_key_table;
pub(crate) mod virtual_send;
pub(crate) mod virtual_sink;
