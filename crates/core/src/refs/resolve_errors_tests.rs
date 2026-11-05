//! Pins the shared resolver-error payload byte-for-byte against the two
//! platform copies these constructors replaced
//! (`crates/windows/src/tree/resolve/resolve_search.rs::identity_unknown_error`,
//! `crates/macos/src/tree/resolve/resolve_errors.rs::identity_unknown`, and both
//! platforms' `mark_deadline_elapsed`, captured before deletion). Renaming or
//! dropping a key here is a behaviour change for both adapters at once.

use super::*;

fn entry_with_role(role: &str) -> RefEntry {
    crate::adapter::minimal_ref_entry(role, None)
}

#[test]
fn identity_unknown_error_pins_the_shared_payload() {
    let entry = entry_with_role("button");
    let error = identity_unknown_error(&entry);

    assert_eq!(error.code, ErrorCode::Timeout);
    assert_eq!(error.disposition, crate::DeliverySemantics::not_delivered());
    assert_eq!(
        error.message,
        "Strict resolution could not determine candidate identity from the live accessibility evidence"
    );
    assert_eq!(
        error.suggestion.as_deref(),
        Some("Retry after the target application finishes updating its accessibility tree")
    );
    assert_eq!(
        error.details,
        Some(json!({
            "kind": "resolution_identity_unknown",
            "role": "button",
            "complete": false,
            "retryable": true,
        }))
    );
    assert!(error.is_explicitly_retryable());
    assert!(error.permits_retry_by_default());
}

#[test]
fn mark_deadline_elapsed_stamps_an_object_details_payload() {
    let error = AdapterError::new(ErrorCode::AppUnresponsive, "incomplete").with_details(json!({
        "kind": "resolution_identity_unknown",
        "complete": false,
        "retryable": true,
    }));

    let stamped = mark_deadline_elapsed(error);

    assert_eq!(
        stamped.details,
        Some(json!({
            "kind": "resolution_identity_unknown",
            "complete": false,
            "retryable": true,
            "deadline_elapsed": true,
        }))
    );
}

#[test]
fn mark_deadline_elapsed_wraps_a_non_object_details_payload() {
    let error =
        AdapterError::new(ErrorCode::Timeout, "bare").with_details(json!("evidence-string"));

    let stamped = mark_deadline_elapsed(error);

    assert_eq!(
        stamped.details,
        Some(json!({
            "evidence": "evidence-string",
            "deadline_elapsed": true,
        }))
    );
}

#[test]
fn mark_deadline_elapsed_fills_missing_details() {
    let error = AdapterError::new(ErrorCode::Timeout, "bare");

    let stamped = mark_deadline_elapsed(error);

    assert_eq!(stamped.details, Some(json!({ "deadline_elapsed": true })));
}

#[test]
fn a_closed_surface_makes_its_ref_stale_and_other_errors_pass_through() {
    let closed = AdapterError::new(crate::ErrorCode::ElementNotFound, "no open menu")
        .with_details(serde_json::json!({ "kind": crate::SURFACE_ABSENT, "surface": "menu" }));
    let stale = stale_when_surface_closed(closed);
    assert_eq!(stale.code, crate::ErrorCode::StaleRef);
    assert_eq!(stale.disposition, crate::DeliverySemantics::not_delivered());
    assert!(stale.permits_retry_by_default());
    let details = stale.details.expect("stale evidence");
    assert_eq!(details["kind"], "surface_closed");
    assert_eq!(details["surface"], "menu");
    let missing = AdapterError::new(crate::ErrorCode::ElementNotFound, "element missing");
    assert_eq!(
        stale_when_surface_closed(missing).code,
        crate::ErrorCode::ElementNotFound
    );
}
