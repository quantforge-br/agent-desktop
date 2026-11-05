use agent_desktop_core::{
    AdapterError, DismissNotificationRequest, InteractionLease, NotificationInfo,
};

use super::card::Card;
use super::center::Center;
use super::locate::{locate, pressable};
use super::policy::require_headed;
use super::read::read_complete;
use super::session::with_center;
use super::verify::{REMOVAL_SETTLE, bystanders, dismissals, is_gone, settle_read};
use super::world::DuxWorld;
use crate::notifications::signal::{SETTLE_BUDGET, Signal, SignalSource};
use crate::notifications::target::Wanted;
use crate::notifications::verify::pressed_without_effect;

/// Removes exactly the notification at the index through its own dismiss
/// control, and proves it twice over: the card's object left the center while
/// no other card did, and the notification service reported the close as a
/// dismissal. A press whose effect shows in neither is a failure that says it
/// was delivered.
pub(crate) fn dismiss_in(
    world: &dyn DuxWorld,
    signals: &dyn SignalSource,
    request: DismissNotificationRequest<'_>,
    lease: &InteractionLease,
) -> Result<NotificationInfo, AdapterError> {
    require_headed(request.policy)?;
    let wanted = Wanted {
        index: request.index,
        app_filter: request.app_filter,
        identity: request.identity,
    };
    let deadline = lease.deadline();
    with_center(world, deadline, request.policy, |changes| {
        let before = read_complete(world, deadline, changes)?;
        let target = locate(&before, wanted)?.clone();
        let control = pressable(&target.dismiss, "dismiss control", wanted.index)?;
        let subscription = signals.subscribe(deadline).ok();
        world.press(&control.element, deadline)?;
        let after = settle_read(world, deadline, REMOVAL_SETTLE, |now| {
            now.is_none_or(|center| is_gone(center, &target))
        })?;
        let seen = subscription
            .map(|held| held.collect(SETTLE_BUDGET))
            .unwrap_or_default();
        confirm(&before, after.as_ref(), &target, &seen)?;
        Ok(target.info)
    })
}

fn confirm(
    before: &Center,
    after: Option<&Center>,
    target: &Card,
    seen: &[Signal],
) -> Result<(), AdapterError> {
    let closed = dismissals(seen);
    let others = after.map_or(0, |center| bystanders(before, center, target).len());
    if others > 0 || closed > 1 {
        return Err(pressed_without_effect(
            "The dismiss control was pressed and more than the target notification left",
            "Run list-notifications to see what the center holds now",
        ));
    }
    let gone = after.is_some_and(|center| is_gone(center, target));
    if gone || closed == 1 {
        return Ok(());
    }
    Err(pressed_without_effect(
        "The dismiss control was pressed but the notification stayed in the center",
        "Run list-notifications; a notification that stays may not be dismissable",
    ))
}

#[cfg(test)]
#[path = "dismiss_tests.rs"]
mod tests;
