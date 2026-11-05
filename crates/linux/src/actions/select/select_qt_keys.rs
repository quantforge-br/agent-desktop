use std::time::Duration;

use agent_desktop_core::{ActionResult, ActionStep, AdapterError, DeliverySemantics, ErrorCode};
use serde_json::json;

use super::select_pointer::{Pointer, already_satisfied, by_pointer_result};
use super::select_row::click_row;
use super::select_search::Hit;
use super::select_verify::read_verification;
use super::{SELECT_SETTLE, children_of, container_target, handle_of, parent_of};
use crate::actions::click::{POLL_STEP, observed_of, poll_until};
use crate::actions::context::Target;
use crate::actions::disclosure::await_expanded;
use crate::actions::popup_verify::shows_native_qt_popups;
use crate::tree::AtspiElement;
use crate::tree::capabilities::action_read::read_node_view;
use crate::tree::vocabulary::role_numbers::COMBO_BOX;
use crate::tree::vocabulary::state_bits::{EXPANDED, FOCUSED};

const ROW_SETTLE: Duration = Duration::from_millis(1000);

/// What choosing a row by keys needs to read from the combo and its popup.
pub(super) trait Popup {
    fn shows_wanted(&mut self) -> Option<bool>;
    fn expanded(&mut self) -> Option<bool>;
    fn await_open(&mut self) -> bool;
    fn await_row_current(&mut self) -> bool;
    fn await_wanted(&mut self) -> Option<bool>;
}

fn unreadable_list() -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionFailed,
        "The combo's popup list could not be read, so its row cannot be counted",
    )
    .with_details(json!({ "kind": "popup_list_unread", "physical_delivery_started": false }))
    .with_disposition(DeliverySemantics::not_delivered())
}

fn stopped(message: &str, kind: &str, steps: &[ActionStep]) -> AdapterError {
    AdapterError::new(ErrorCode::ActionFailed, message.to_owned())
        .with_suggestion("Take a fresh snapshot: the combo keeps the value it showed before")
        .with_details(json!({ "kind": kind, "steps": steps }))
        .with_disposition(DeliverySemantics::delivered_unverified())
}

fn after_input(error: AdapterError) -> AdapterError {
    if error.disposition == DeliverySemantics::not_delivered() {
        error.with_disposition(DeliverySemantics::uncertain())
    } else {
        error
    }
}

/// Chooses row `index` of a combo's popup: a click on `combo` opens it when it
/// is closed, with one more click when the first one's popup was lost to the
/// window taking focus, `Home` and `Down` move the popup's current row to the choice,
/// and `Return` commits it only once that row reads current. A popup that
/// never opens, or a row that never becomes current, is closed with `Escape`
/// and nothing is committed.
pub(super) fn choose_row(
    popup: &mut dyn Popup,
    pointer: &dyn Pointer,
    combo: &AtspiElement,
    index: usize,
) -> Result<ActionResult, AdapterError> {
    if popup.shows_wanted() == Some(true) {
        return Ok(already_satisfied());
    }
    let mut steps = Vec::new();
    if popup.expanded() != Some(true) {
        steps.extend(pointer.click(combo)?);
        if !popup.await_open() {
            steps.extend(pointer.click(combo).map_err(after_input)?);
        }
        if !popup.await_open() {
            return Err(stopped(
                "The combo's popup did not open after the click",
                "popup_not_observed",
                &steps,
            ));
        }
    }
    steps.extend(pointer.press("home").map_err(after_input)?);
    for _ in 0..index {
        steps.extend(pointer.press("down").map_err(after_input)?);
    }
    if !popup.await_row_current() {
        steps.extend(pointer.press("escape").unwrap_or_default());
        return Err(stopped(
            "The popup's current row did not reach the choice, so the popup was closed without committing",
            "popup_row_unreached",
            &steps,
        ));
    }
    steps.extend(pointer.press("return").map_err(after_input)?);
    by_pointer_result(steps, observed_of(popup.await_wanted()))
}

struct LivePopup<'a> {
    target: &'a Target<'a>,
    row: &'a AtspiElement,
    wanted: &'a str,
}

impl Popup for LivePopup<'_> {
    fn shows_wanted(&mut self) -> Option<bool> {
        read_verification(self.target.bus, self.target, self.row, self.wanted, None)
    }

    fn expanded(&mut self) -> Option<bool> {
        self.target.view().ok()?.state(EXPANDED)
    }

    fn await_open(&mut self) -> bool {
        await_expanded(self.target)
    }

    fn await_row_current(&mut self) -> bool {
        let handle = handle_of(self.row);
        let read = || {
            read_node_view(self.target.bus, &handle, self.target.deadline)
                .ok()
                .and_then(|view| view.state(FOCUSED))
        };
        poll_until(
            self.target.deadline,
            ROW_SETTLE,
            POLL_STEP,
            read,
            |current| *current == Some(true),
        ) == Some(true)
    }

    fn await_wanted(&mut self) -> Option<bool> {
        poll_until(
            self.target.deadline,
            SELECT_SETTLE,
            POLL_STEP,
            || read_verification(self.target.bus, self.target, self.row, self.wanted, None),
            |verified| *verified == Some(true),
        )
    }
}

fn row_index(
    target: &Target<'_>,
    list: &AtspiElement,
    row: &AtspiElement,
) -> Result<usize, AdapterError> {
    children_of(target.bus, list, None, target.deadline)
        .map_err(|_| unreadable_list())?
        .iter()
        .position(|child| child.bus_name() == row.bus_name() && child.path() == row.path())
        .ok_or_else(unreadable_list)
}

/// Selects a Qt combo's choice with the pointer. A popup drawn on X11 is
/// clicked at its row. A native Wayland popup places itself where no other
/// client can read and reports its rows inside its own surface, so its row is
/// reached with the keys the popup's list takes instead. A target that is a
/// choice row of the combo opens and verifies through the combo that owns it.
pub(in crate::actions) fn select_qt_combo(
    target: &Target<'_>,
    hit: &Hit<AtspiElement>,
    display_value: &str,
    pointer: &dyn Pointer,
) -> Result<ActionResult, AdapterError> {
    if !shows_native_qt_popups(target) {
        return click_row(target, hit, display_value, pointer);
    }
    let row = hit.row.as_ref().unwrap_or(&hit.item);
    let list = match hit.container.as_ref() {
        Some(container) => container.node.clone(),
        None => parent_of(target.bus, row, target.deadline)?.ok_or_else(unreadable_list)?,
    };
    let index = row_index(target, &list, row)?;
    let owner = (target.view()?.role() != Some(COMBO_BOX))
        .then(|| target.element.ancestry().combo_owner())
        .flatten()
        .map(|owner| handle_of(&owner));
    let owner_target = owner
        .as_ref()
        .map(|handle| container_target(target, handle))
        .transpose()?;
    let combo = owner_target.as_ref().unwrap_or(target);
    let mut popup = LivePopup {
        target: combo,
        row,
        wanted: display_value,
    };
    choose_row(&mut popup, pointer, combo.element, index)
}

#[cfg(test)]
#[path = "select_qt_keys_tests.rs"]
mod tests;
