//! Notifications on the dux-shell: its notification center is read and acted
//! on through the shell's accessibility tree, shown through the shell's own
//! IPC, and every effect is confirmed against the notification service.

use agent_desktop_core::{
    ActionResult, AdapterError, Deadline, DismissAllNotificationsRequest,
    DismissNotificationRequest, InteractionLease, InteractionPolicy, NotificationActionRequest,
    NotificationFilter, NotificationInfo,
};

mod action;
mod card;
mod center;
mod dismiss;
mod dismiss_all;
mod group;
mod locate;
mod name;
mod parse;
mod policy;
mod read;
mod restore;
mod session;
mod tab;
mod verify;
mod world;

#[cfg(test)]
mod fake;
#[cfg(test)]
mod fake_nodes;
#[cfg(test)]
mod fake_quirks;
#[cfg(test)]
mod fake_signals;
#[cfg(test)]
#[path = "list_tests.rs"]
mod list_tests;

use super::list::apply_filter;
use super::signal::BusSignals;
use crate::system::SharedBus;
use crate::system::shell::dux::backend::DuxShell;
use world::LiveDuxWorld;

/// Lists the current notifications. The center must be on screen to be read:
/// a closed center is shown first when the policy allows the focus change,
/// and closed again afterwards.
pub(crate) fn list(
    bus: &SharedBus,
    shell: DuxShell,
    filter: &NotificationFilter,
    policy: InteractionPolicy,
    deadline: Deadline,
) -> Result<Vec<NotificationInfo>, AdapterError> {
    list_in(&LiveDuxWorld::new(bus, shell), filter, policy, deadline)
}

pub(super) fn list_in(
    world: &dyn world::DuxWorld,
    filter: &NotificationFilter,
    policy: InteractionPolicy,
    deadline: Deadline,
) -> Result<Vec<NotificationInfo>, AdapterError> {
    session::with_center(world, deadline, policy, |changes| {
        let center = read::read_complete(world, deadline, changes)?;
        let all: Vec<NotificationInfo> =
            center.cards.iter().map(|card| card.info.clone()).collect();
        Ok(apply_filter(all, filter))
    })
}

pub(crate) fn dismiss(
    bus: &SharedBus,
    shell: DuxShell,
    request: DismissNotificationRequest<'_>,
    lease: &InteractionLease,
) -> Result<NotificationInfo, AdapterError> {
    dismiss::dismiss_in(&LiveDuxWorld::new(bus, shell), &BusSignals, request, lease)
}

pub(crate) fn dismiss_all(
    bus: &SharedBus,
    shell: DuxShell,
    request: DismissAllNotificationsRequest<'_>,
    lease: &InteractionLease,
) -> Result<(Vec<NotificationInfo>, Vec<String>), AdapterError> {
    dismiss_all::dismiss_all_in(&LiveDuxWorld::new(bus, shell), &BusSignals, request, lease)
}

pub(crate) fn action(
    bus: &SharedBus,
    shell: DuxShell,
    request: NotificationActionRequest<'_>,
    lease: &InteractionLease,
) -> Result<ActionResult, AdapterError> {
    action::action_in(&LiveDuxWorld::new(bus, shell), &BusSignals, request, lease)
}
