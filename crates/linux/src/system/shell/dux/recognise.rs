use std::path::{Path, PathBuf};

use super::procs::Procs;
use super::proof::Proof;

const QUICKSHELL_EXECUTABLES: [&str; 2] = ["quickshell", "qs"];
const SHELL_ENTRY: &str = "DMSShell.qml";
const SHELL_ROOT: &str = "shell.qml";
const DMS_EXECUTABLE: &str = "dms";
const DMS_RUN: &str = "run";

/// The directories quickshell was told to load: the value after `-p`/`--path`,
/// or the named configuration under each configuration directory for
/// `-c`/`--config`.
pub(super) fn config_candidates(argv: &[String], config_dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut words = argv.iter().skip(1);
    while let Some(word) = words.next() {
        match word.as_str() {
            "-p" | "--path" => return words.next().map(PathBuf::from).into_iter().collect(),
            "-c" | "--config" => {
                let Some(name) = words.next() else {
                    return Vec::new();
                };
                return config_dirs
                    .iter()
                    .map(|dir| dir.join("quickshell").join(name))
                    .collect();
            }
            _ => {}
        }
    }
    Vec::new()
}

fn holds_shell(procs: &dyn Procs, directory: &Path) -> bool {
    let directory = if procs.is_file(directory) {
        directory.parent().unwrap_or(directory)
    } else {
        directory
    };
    procs.is_file(&directory.join(SHELL_ENTRY)) && procs.is_file(&directory.join(SHELL_ROOT))
}

fn dms_parent(procs: &dyn Procs, pid: u32, user: u32) -> bool {
    let Some(parent) = procs.parent(pid) else {
        return false;
    };
    let runs = procs
        .argv(parent)
        .is_some_and(|argv| argv.get(1).is_some_and(|word| word.as_str() == DMS_RUN));
    procs.executable(parent).as_deref() == Some(DMS_EXECUTABLE)
        && procs.user(parent) == Some(user)
        && runs
}

/// Whether the process is the dux-shell and why. The process must be a
/// quickshell run by the session's user; it is the dux-shell when it loads a
/// configuration directory that holds the shell's entry point, or when the
/// `dms` command it was started by is still its parent. The shell's own
/// environment variables and the desktop name never count.
pub(crate) fn recognise(
    procs: &dyn Procs,
    pid: u32,
    me: u32,
    config_dirs: &[PathBuf],
) -> Option<Proof> {
    let executable = procs.executable(pid)?;
    if !QUICKSHELL_EXECUTABLES.contains(&executable.as_str()) || procs.user(pid) != Some(me) {
        return None;
    }
    let loaded = procs
        .argv(pid)
        .map(|argv| config_candidates(&argv, config_dirs))
        .unwrap_or_default();
    if loaded.iter().any(|directory| holds_shell(procs, directory)) {
        return Some(Proof::ConfigDirectory);
    }
    dms_parent(procs, pid, me).then_some(Proof::DmsParent)
}

#[cfg(test)]
#[path = "recognise_tests.rs"]
mod tests;
