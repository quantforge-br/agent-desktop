//! Windows on Hyprland: the client each listed window maps to, the place,
//! focus and visibility the compositor reports for it, and the window
//! operations sent through Hyprland's dispatchers and read back from its
//! client list.

#[cfg(test)]
mod hypr_fake;
pub(crate) mod hypr_focus;
pub(crate) mod hypr_listing;
pub(crate) mod hypr_settle;
pub(crate) mod hypr_unmatched;
pub(crate) mod hypr_window_key;
pub(crate) mod hypr_window_match;
pub(crate) mod hypr_window_op;
pub(crate) mod hypr_world;
