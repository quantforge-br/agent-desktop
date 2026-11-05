use crate::system::shell::light::Light;
use crate::tree::AtspiElement;

/// The wrapper the center lays around the notifications of one application.
/// A group of several shows only its newest card until `toggle` expands it;
/// an expanded group shows every card and a `collapse` control.
#[derive(Debug, Clone)]
pub(crate) struct Group {
    pub(crate) wrapper: AtspiElement,
    pub(crate) toggle: Option<Light>,
    pub(crate) collapse: Option<Light>,
    pub(crate) expanded: bool,
}

impl Group {
    /// Whether the group holds several notifications and shows only the
    /// newest of them.
    pub(crate) fn hides_cards(&self) -> bool {
        !self.expanded && self.toggle.as_ref().is_some_and(|toggle| toggle.showing)
    }
}
