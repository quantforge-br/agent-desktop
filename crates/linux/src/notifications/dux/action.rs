use agent_desktop_core::{
    ActionResult, AdapterError, DeliverySemantics, ErrorCode, InteractionLease,
    NotificationActionRequest,
};
use serde_json::json;

use super::card::Card;
use super::locate::{locate, pressable};
use super::policy::require_headed;
use super::read::read_complete;
use super::session::with_center;
use super::verify::invocations;
use super::world::DuxWorld;
use crate::notifications::signal::{SETTLE_BUDGET, SignalSource};
use crate::notifications::target::Wanted;
use crate::notifications::verify::pressed_without_effect;
use crate::system::shell::light::Light;

/// Presses the named action button of the notification at the index. The
/// shell closes the center once an action runs, so the center cannot prove
/// the effect; the proof is the notification service's own `ActionInvoked`
/// signal, subscribed before the press. A press that raised no such signal is
/// a failure that says it was delivered, and the application's own effect is
/// never claimed.
pub(crate) fn action_in(
    world: &dyn DuxWorld,
    signals: &dyn SignalSource,
    request: NotificationActionRequest<'_>,
    lease: &InteractionLease,
) -> Result<ActionResult, AdapterError> {
    require_headed(request.policy)?;
    let wanted = Wanted {
        index: request.index,
        app_filter: None,
        identity: request.identity,
    };
    let (deadline, name) = (lease.deadline(), request.action_name);
    with_center(world, deadline, request.policy, |changes| {
        let center = read_complete(world, deadline, changes)?;
        let card = locate(&center, wanted)?;
        let button = pressable(offered(card, name)?, "action button", wanted.index)?;
        let subscription = signals.subscribe(deadline).ok();
        world.press(&button.element, deadline)?;
        let seen = subscription
            .map(|held| held.collect(SETTLE_BUDGET))
            .unwrap_or_default();
        let proof = match invocations(&seen) {
            0 => {
                return Err(pressed_without_effect(
                    format!(
                        "The '{name}' button was pressed but the notification service reported no action"
                    ),
                    "Run list-notifications; the notification may keep its actions",
                ));
            }
            1 => "action_invoked",
            _ => "action_invoked_among_several",
        };
        Ok(ActionResult::delivered_unverified(name).with_details(json!({ "evidence": proof })))
    })
}

fn offered<'a>(card: &'a Card, name: &str) -> Result<&'a Light, AdapterError> {
    card.action_named(name).ok_or_else(|| {
        AdapterError::new(
            ErrorCode::ActionNotSupported,
            format!("The notification does not offer an action named '{name}'"),
        )
        .with_suggestion("Pick one of the notification's available actions")
        .with_details(json!({ "available_actions": card.info.actions }))
        .with_disposition(DeliverySemantics::not_delivered())
    })
}

#[cfg(test)]
#[path = "action_tests.rs"]
mod tests;
