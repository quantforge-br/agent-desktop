use agent_desktop_core::NotificationInfo;

use crate::system::shell::light::Light;
use crate::tree::AtspiElement;

/// One notification the center shows, with the controls that act on it. The
/// `node` is the card's own accessibility object, the identity a mutation
/// proves its effect against.
#[derive(Debug, Clone)]
pub(crate) struct Card {
    pub(crate) info: NotificationInfo,
    pub(crate) node: AtspiElement,
    pub(crate) dismiss: Light,
    pub(crate) actions: Vec<Light>,
    pub(crate) group: AtspiElement,
}

impl Card {
    pub(crate) fn action_named(&self, name: &str) -> Option<&Light> {
        let wanted = name.to_lowercase();
        self.actions
            .iter()
            .find(|button| button.name.to_lowercase() == wanted)
    }
}
