use std::cell::RefCell;

use agent_desktop_core::{ActionStep, AdapterError, DeliverySemantics, ErrorCode};

use super::{Popup, choose_row};
use crate::actions::select::select_pointer::Pointer;
use crate::tree::AtspiElement;

#[derive(Default)]
struct Keys {
    sent: RefCell<Vec<String>>,
}

impl Pointer for Keys {
    fn click(&self, _: &AtspiElement) -> Result<Vec<ActionStep>, AdapterError> {
        self.sent.borrow_mut().push("click".to_owned());
        Ok(vec![ActionStep::succeeded("VirtualPointer.click")])
    }

    fn press(&self, key: &str) -> Result<Vec<ActionStep>, AdapterError> {
        self.sent.borrow_mut().push(key.to_owned());
        Ok(vec![ActionStep::succeeded("VirtualKeyboard.press_key")])
    }
}

struct Combo {
    shows: bool,
    expanded: bool,
    opens: bool,
    row_current: bool,
    commits: bool,
}

impl Popup for Combo {
    fn shows_wanted(&mut self) -> Option<bool> {
        Some(self.shows)
    }

    fn expanded(&mut self) -> Option<bool> {
        Some(self.expanded)
    }

    fn await_open(&mut self) -> bool {
        self.opens
    }

    fn await_row_current(&mut self) -> bool {
        self.row_current
    }

    fn await_wanted(&mut self) -> Option<bool> {
        Some(self.commits)
    }
}

fn closed() -> Combo {
    Combo {
        shows: false,
        expanded: false,
        opens: true,
        row_current: true,
        commits: true,
    }
}

fn combo() -> AtspiElement {
    AtspiElement::new(":1.2", "/combo", 7)
}

#[test]
fn a_closed_combo_is_opened_walked_to_the_row_and_committed_and_verified() {
    let keys = Keys::default();
    let result = choose_row(&mut closed(), &keys, &combo(), 2).unwrap();
    assert_eq!(
        *keys.sent.borrow(),
        ["click", "home", "down", "down", "return"]
    );
    assert_eq!(
        result.disposition(),
        DeliverySemantics::delivered_verified()
    );
}

#[test]
fn an_open_popup_is_not_clicked_again() {
    let keys = Keys::default();
    let mut open = Combo {
        expanded: true,
        ..closed()
    };
    choose_row(&mut open, &keys, &combo(), 0).unwrap();
    assert_eq!(*keys.sent.borrow(), ["home", "return"]);
}

#[test]
fn a_choice_already_shown_sends_nothing() {
    let keys = Keys::default();
    let mut shown = Combo {
        shows: true,
        ..closed()
    };
    let result = choose_row(&mut shown, &keys, &combo(), 3).unwrap();
    assert!(keys.sent.borrow().is_empty());
    assert_eq!(result.disposition(), DeliverySemantics::not_delivered());
}

#[test]
fn a_row_that_never_becomes_current_is_escaped_and_never_committed() {
    let keys = Keys::default();
    let mut stuck = Combo {
        row_current: false,
        ..closed()
    };
    let error = choose_row(&mut stuck, &keys, &combo(), 1).unwrap_err();
    assert_eq!(*keys.sent.borrow(), ["click", "home", "down", "escape"]);
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(error.details.unwrap()["kind"], "popup_row_unreached");
}

#[test]
fn a_popup_that_never_opens_gets_no_keys() {
    let keys = Keys::default();
    let mut shut = Combo {
        opens: false,
        ..closed()
    };
    let error = choose_row(&mut shut, &keys, &combo(), 1).unwrap_err();
    assert_eq!(*keys.sent.borrow(), ["click", "click"]);
    assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
}

#[test]
fn a_commit_the_combo_does_not_show_stays_unverified() {
    let keys = Keys::default();
    let mut ignored = Combo {
        commits: false,
        ..closed()
    };
    let outcome = choose_row(&mut ignored, &keys, &combo(), 1);
    assert!(outcome.map_or(true, |result| result.disposition()
        != DeliverySemantics::delivered_verified()));
}
