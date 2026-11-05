/// The fact that identified a process as the dux-shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Proof {
    /// The quickshell process loads a configuration directory that holds the
    /// shell's own entry point.
    ConfigDirectory,
    /// The quickshell process is the child of a running `dms` command.
    DmsParent,
}
