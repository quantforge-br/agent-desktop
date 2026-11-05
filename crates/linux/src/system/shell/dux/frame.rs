use crate::tree::AtspiElement;

/// One top-level accessibility window of the shell's application.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Frame {
    pub(crate) element: AtspiElement,
    pub(crate) width: i64,
    pub(crate) height: i64,
    pub(crate) showing: bool,
    pub(crate) children: usize,
    pub(crate) roles: Vec<u32>,
}
