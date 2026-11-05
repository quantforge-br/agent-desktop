use std::cell::Cell;
use std::cell::RefCell;

use agent_desktop_core::{ActionResult, AdapterError, DeliverySemantics, ErrorCode};
use serde_json::json;

use super::expand_or_click;

fn native_refusal() -> Result<ActionResult, AdapterError> {
    Err(
        AdapterError::new(ErrorCode::ActionNotSupported, "needs a user input serial")
            .with_details(json!({ "kind": "native_popup_requires_input" }))
            .with_disposition(DeliverySemantics::not_delivered()),
    )
}

#[test]
fn an_opener_qt_shows_only_for_input_is_clicked_and_reported_as_the_expand() {
    let result = expand_or_click(
        native_refusal,
        || Ok(ActionResult::delivered_unverified("click")),
        || true,
    )
    .unwrap();
    assert_eq!(result.action, "expand");
    assert_eq!(
        result.details.unwrap()["semantic_refusal"]["details"]["kind"],
        "native_popup_requires_input"
    );
    assert_eq!(result.steps[0].label(), "Action.DoAction");
}

#[test]
fn any_other_semantic_answer_stands_and_no_pointer_moves() {
    let clicked = Cell::new(false);
    let leaf = || {
        Err(AdapterError::new(ErrorCode::ActionNotSupported, "leaf")
            .with_details(json!({ "expandable": false }))
            .with_disposition(DeliverySemantics::not_delivered()))
    };
    let error = expand_or_click(
        leaf,
        || {
            clicked.set(true);
            Ok(ActionResult::delivered_unverified("click"))
        },
        || true,
    )
    .unwrap_err();
    assert_eq!(error.message, "leaf");
    assert!(!clicked.get());
}

#[test]
fn a_session_without_an_aimable_pointer_keeps_the_semantic_refusal() {
    let error = expand_or_click(
        native_refusal,
        || {
            Err(
                AdapterError::new(ErrorCode::ActionNotSupported, "no pointer")
                    .with_disposition(DeliverySemantics::not_delivered()),
            )
        },
        || true,
    )
    .unwrap_err();
    assert_eq!(
        error.details.unwrap()["kind"],
        "native_popup_requires_input"
    );
}

#[test]
fn the_headless_policy_refusal_of_a_native_popup_is_also_clicked_open() {
    let policy = || {
        Err(
            AdapterError::new(ErrorCode::PolicyDenied, "takes a pointer click")
                .with_details(
                    json!({ "kind": "pointer_required", "cause": "native_popup_requires_input" }),
                )
                .with_disposition(DeliverySemantics::not_delivered()),
        )
    };
    let result = expand_or_click(
        policy,
        || Ok(ActionResult::delivered_unverified("click")),
        || true,
    )
    .unwrap();
    assert_eq!(result.action, "expand");
}

#[test]
fn a_click_that_shows_no_popup_is_sent_once_more_and_never_a_third_time() {
    let clicks = Cell::new(0);
    let reads = RefCell::new(vec![false, false]);
    let result = expand_or_click(
        native_refusal,
        || {
            clicks.set(clicks.get() + 1);
            Ok(ActionResult::delivered_unverified("click").with_steps(vec![
                agent_desktop_core::ActionStep::succeeded("VirtualPointer.click"),
            ]))
        },
        || reads.borrow_mut().pop().unwrap_or(true),
    )
    .unwrap();
    assert_eq!(clicks.get(), 2);
    assert_eq!(result.steps.len(), 3);
}
