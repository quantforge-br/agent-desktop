/// One global the compositor's registry advertises.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WaylandGlobal {
    pub(crate) name: u32,
    pub(crate) interface: String,
    pub(crate) version: u32,
}
