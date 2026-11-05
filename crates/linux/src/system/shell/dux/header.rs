use crate::tree::vocabulary::role_numbers::{PUSH_BUTTON, RADIO_BUTTON};

const MAX_TABS: usize = 2;
const HEADER_BUTTONS: usize = 3;

/// Where the notification center's tab radios start among the children of a
/// window: one or two radio buttons followed by at least three push buttons.
/// The longer radio run of the history filters is followed by lists, never by
/// buttons, and no other window of the shell opens with this arrangement.
pub(crate) fn tab_header_start(roles: &[u32]) -> Option<usize> {
    (0..roles.len()).find(|start| {
        let radios = roles[*start..]
            .iter()
            .take_while(|role| **role == RADIO_BUTTON)
            .count();
        let before_radio = *start == 0 || roles[start - 1] != RADIO_BUTTON;
        let buttons = roles[start + radios..]
            .iter()
            .take_while(|role| **role == PUSH_BUTTON)
            .count();
        before_radio && (1..=MAX_TABS).contains(&radios) && buttons >= HEADER_BUTTONS
    })
}

#[cfg(test)]
#[path = "header_tests.rs"]
mod tests;
