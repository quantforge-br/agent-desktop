use agent_desktop_core::{AdapterError, InteractionPolicy};

pub(super) fn require_headed(policy: InteractionPolicy) -> Result<(), AdapterError> {
    if policy.is_headed() {
        return Ok(());
    }
    Err(AdapterError::policy_denied_for_policy(
        "Changing notifications shows the notification center and takes the keyboard focus, which needs headed mode",
        policy,
    )
    .with_suggestion("Pass --headed to let the command use the notification center"))
}
