use super::card::Card;
use super::group::Group;
use super::tab::Tab;
use crate::system::shell::light::Light;

/// The notification center as the shell's accessibility tree shows it.
#[derive(Debug, Clone)]
pub(crate) struct Center {
    pub(crate) frame: Light,
    pub(crate) tabs: Vec<Tab>,
    pub(crate) expected: Option<usize>,
    pub(crate) groups: Vec<Group>,
    pub(crate) cards: Vec<Card>,
}

impl Center {
    /// Whether the center is on screen: its header is showing.
    pub(crate) fn is_open(&self) -> bool {
        self.frame.showing && self.tabs.iter().any(|tab| tab.radio.showing)
    }

    /// The tab that lists the current notifications, the first one.
    pub(crate) fn current_tab(&self) -> Option<&Tab> {
        self.tabs.first()
    }

    /// Whether the list of current notifications is the one on show.
    pub(crate) fn shows_current(&self) -> bool {
        self.current_tab().is_none_or(|tab| tab.checked)
    }

    /// Whether the cards read are all the notifications the header counts.
    pub(crate) fn is_complete(&self) -> bool {
        self.expected.is_none_or(|count| count == self.cards.len())
    }
}
