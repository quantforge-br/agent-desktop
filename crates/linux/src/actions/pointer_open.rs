use agent_desktop_core::{
    ActionResult, ActionStep, AdapterError, DeliverySemantics, ErrorCode, StepMechanism,
};
use serde_json::json;

use super::popup_verify::NEEDS_INPUT_SERIAL;

const SEMANTIC_ROUTE: &str = "Action.DoAction";

fn needs_input_serial(error: &AdapterError) -> bool {
    let Some(details) = error.details.as_ref() else {
        return false;
    };
    let named = |key: &str| {
        details
            .get(key)
            .is_some_and(|value| value == NEEDS_INPUT_SERIAL)
    };
    match error.code {
        ErrorCode::ActionNotSupported => named("kind"),
        ErrorCode::PolicyDenied => named("cause"),
        _ => false,
    }
}

fn pointer_unavailable(error: &AdapterError) -> bool {
    error.code == ErrorCode::ActionNotSupported
        && error.disposition == DeliverySemantics::not_delivered()
}

/// A headed expand whose semantic opener was refused because the toolkit
/// shows the popup only for a user input serial clicks the opener instead,
/// then gives the popup its settle time through `opened`. A click that lands
/// while the window is still taking keyboard focus opens the popup and loses
/// it to the activation, so a click that shows no popup is sent once more.
/// The clicks are ordinary pointer deliveries, so the expanded state core
/// reads afterwards is what verifies them. A session with no pointer to aim
/// keeps the semantic refusal.
pub(super) fn expand_or_click(
    semantic: impl FnOnce() -> Result<ActionResult, AdapterError>,
    physical: impl Fn() -> Result<ActionResult, AdapterError>,
    opened: impl Fn() -> bool,
) -> Result<ActionResult, AdapterError> {
    let refusal = match semantic() {
        Err(refusal) if needs_input_serial(&refusal) => refusal,
        other => return other,
    };
    match physical() {
        Ok(mut result) => {
            if !opened() {
                let again = physical().map_err(|error| {
                    error.with_disposition(DeliverySemantics::delivered_unverified())
                })?;
                result.steps.extend(again.steps);
                opened();
            }
            "expand".clone_into(&mut result.action);
            result.steps.insert(
                0,
                ActionStep::skipped(SEMANTIC_ROUTE).with_mechanism(StepMechanism::SemanticApi),
            );
            result.details.get_or_insert_with(|| json!({}))["semantic_refusal"] = json!({
                "code": refusal.code,
                "message": refusal.message,
                "details": refusal.details,
            });
            Ok(result)
        }
        Err(error) if pointer_unavailable(&error) => Err(refusal),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
#[path = "pointer_open_tests.rs"]
mod tests;
