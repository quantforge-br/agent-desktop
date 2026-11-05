//! The dux-shell (a DankMaterialShell fork running on quickshell): how it is
//! recognised and how its IPC and accessibility application are reached.

use std::path::PathBuf;

pub(crate) mod app;
pub(crate) mod backend;
pub(crate) mod details;
pub(crate) mod entry;
pub(crate) mod frame;
pub(crate) mod header;
pub(crate) mod ipc;
pub(crate) mod layer;
pub(crate) mod layers;
pub(crate) mod present;
pub(crate) mod procs;
pub(crate) mod proof;
pub(crate) mod recognise;
pub(crate) mod stage;
pub(crate) mod surface;

const DEFAULT_SYSTEM_CONFIG: &str = "/etc/xdg";

/// The directories quickshell looks its named configurations up in.
pub(crate) fn config_dirs() -> Vec<PathBuf> {
    let user = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")));
    let system = std::env::var_os("XDG_CONFIG_DIRS")
        .map(|dirs| std::env::split_paths(&dirs).collect::<Vec<_>>())
        .unwrap_or_else(|| vec![PathBuf::from(DEFAULT_SYSTEM_CONFIG)]);
    user.into_iter().chain(system).collect()
}
