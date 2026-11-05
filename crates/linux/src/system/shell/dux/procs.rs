use std::path::Path;

use crate::system::process::{proc_stat, process_facts};

/// What the recognition rule reads about processes and the file system, so a
/// test can stand in a described session.
pub(crate) trait Procs {
    /// The file name of the executable the process runs.
    fn executable(&self, pid: u32) -> Option<String>;
    fn argv(&self, pid: u32) -> Option<Vec<String>>;
    fn parent(&self, pid: u32) -> Option<u32>;
    fn user(&self, pid: u32) -> Option<u32>;
    fn is_file(&self, path: &Path) -> bool;
}

/// `/proc` and the file system of this machine.
pub(crate) struct LiveProcs;

impl Procs for LiveProcs {
    fn executable(&self, pid: u32) -> Option<String> {
        process_facts::read(pid)?.exe_basename
    }

    fn argv(&self, pid: u32) -> Option<Vec<String>> {
        let raw = std::fs::read(format!("/proc/{pid}/cmdline")).ok()?;
        Some(
            raw.split(|byte| *byte == 0)
                .filter(|part| !part.is_empty())
                .map(|part| String::from_utf8_lossy(part).into_owned())
                .collect(),
        )
    }

    fn parent(&self, pid: u32) -> Option<u32> {
        proc_stat::read_stat(pid).ok()?.map(|stat| stat.parent)
    }

    fn user(&self, pid: u32) -> Option<u32> {
        use std::os::unix::fs::MetadataExt;
        std::fs::metadata(format!("/proc/{pid}"))
            .ok()
            .map(|metadata| metadata.uid())
    }

    fn is_file(&self, path: &Path) -> bool {
        path.is_file()
    }
}
