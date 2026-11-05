use crate::system::shell::light::Light;

/// A tab of the center's header and whether it is the selected one.
#[derive(Debug, Clone)]
pub(crate) struct Tab {
    pub(crate) radio: Light,
    pub(crate) checked: bool,
}
