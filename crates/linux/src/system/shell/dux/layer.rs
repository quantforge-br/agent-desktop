/// A layer-shell surface the compositor has mapped, with the process that owns
/// it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Layer {
    pub(crate) namespace: String,
    pub(crate) x: i64,
    pub(crate) y: i64,
    pub(crate) width: i64,
    pub(crate) height: i64,
    pub(crate) pid: u32,
}
