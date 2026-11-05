use serde_json::json;

use crate::{AdapterError, ErrorCode, RefEntry};

/// The incomplete-and-retryable answer shared by both resolvers: a candidate
/// that could not be read is not a non-match. `RefEntry` evidence is
/// platform-neutral, and an unreadable candidate means the same thing
/// whichever adapter's search produced it, so the payload - message,
/// suggestion, and the `complete: false, retryable: true` stamp - is decided
/// once, here, instead of by two copies that can drift apart.
pub fn identity_unknown_error(entry: &RefEntry) -> AdapterError {
    AdapterError::new(
        ErrorCode::Timeout,
        "Strict resolution could not determine candidate identity from the live accessibility evidence",
    )
    .with_suggestion("Retry after the target application finishes updating its accessibility tree")
    .with_disposition(crate::DeliverySemantics::not_delivered())
    .with_details(json!({
        "kind": "resolution_identity_unknown",
        "role": entry.identity.role,
        "complete": false,
        "retryable": true,
    }))
}

/// A tokenless ref that was not proven unique when it was captured.
pub(crate) fn identity_not_unique(entry: &RefEntry) -> AdapterError {
    AdapterError::new(
        ErrorCode::AmbiguousTarget,
        "Saved ref was not proven unique when it was captured, so it cannot be re-identified safely",
    )
    .with_suggestion(
        "Either another element in the window matched it, or the window could not be read \
         whole within the snapshot timeout. For a large window, retake the snapshot with a \
         larger --timeout-ms. Identical twins are refused again by every snapshot; where \
         headed pointer input is available, an explicit --headed coordinate click can pick one.",
    )
    .with_details(json!({
        "kind": "identity_not_unique",
        "retryable": false,
        "role": entry.identity.role,
    }))
    .with_disposition(crate::DeliverySemantics::not_delivered())
}

/// Stamps the final incomplete diagnosis with `deadline_elapsed` so the
/// caller sees why the retries ran out, preserving the incomplete's own
/// details rather than discarding them for a bare `TIMEOUT`. Shared by both
/// resolvers' retry loops, which reach the same terminal shape when their own
/// deadline expires with an incomplete answer still on hand.
pub fn mark_deadline_elapsed(mut error: AdapterError) -> AdapterError {
    let mut details = error.details.take().unwrap_or_else(|| json!({}));
    if let Some(object) = details.as_object_mut() {
        object.insert("deadline_elapsed".into(), json!(true));
    } else {
        details = json!({
            "evidence": details,
            "deadline_elapsed": true,
        });
    }
    error.with_details(details)
}

/// A ref taken on a surface that is no longer open, such as a closed menu, is
/// stale: the surface's absence is the observed answer. The recovery is the
/// ordinary stale one, a fresh snapshot and a retry, with the surface reopened
/// first, because reopening it mints new refs rather than restoring this one.
pub(crate) fn stale_when_surface_closed(error: AdapterError) -> AdapterError {
    let surface = match (&error.code, error.details.as_ref()) {
        (crate::ErrorCode::ElementNotFound, Some(details))
            if details
                .get("kind")
                .is_some_and(|kind| kind == crate::SURFACE_ABSENT) =>
        {
            details
                .get("surface")
                .cloned()
                .unwrap_or(serde_json::Value::Null)
        }
        _ => return error,
    };
    AdapterError::stale_ref_because("The surface the ref was taken on is no longer open")
        .with_suggestion(
            "Reopen the surface through its opener, take a fresh snapshot of it, and use the new ref.",
        )
        .with_details(serde_json::json!({
            "kind": "surface_closed",
            "surface": surface,
            "complete": true,
        }))
}

#[cfg(test)]
#[path = "resolve_errors_tests.rs"]
mod tests;
