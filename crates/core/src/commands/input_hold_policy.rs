use crate::{AdapterError, AppError, DeliverySemantics, ErrorCode};

/// Refuses a held-input command before anything reaches the platform.
///
/// Each invocation is a separate process, so nothing could release a held key
/// or button after the call returns. Nothing is ever sent, which the
/// `not_delivered` disposition states so callers know a retry is safe.
pub(crate) fn reject(command: &str, replacement: &str) -> AppError {
    AdapterError::new(
        ErrorCode::ActionNotSupported,
        format!("{command} is unavailable in stateless mode"),
    )
    .with_details(serde_json::json!({ "raw_input_emitted": false }))
    .with_suggestion(format!(
        "Use the atomic {replacement} command; a held key or button cannot be guaranteed released between separate CLI invocations"
    ))
    .with_disposition(DeliverySemantics::not_delivered())
    .into()
}

#[cfg(test)]
#[path = "input_hold_policy_tests.rs"]
mod tests;
