use agent_desktop_core::{
    ActionResult, AdapterError, Deadline, DismissAllNotificationsRequest,
    DismissNotificationRequest, InteractionLease, InteractionPolicy, NotificationActionRequest,
    NotificationFilter, NotificationInfo,
};

use crate::system::SharedBus;
use crate::system::shell::dux::backend::{self, Backend};

mod action;
mod close;
mod dismiss;
mod dismiss_all;
mod dux;
mod extract;
mod list;
mod session;
mod signal;
mod target;
mod verify;
mod world;

#[cfg(test)]
mod fake;

#[cfg(test)]
mod backend_live_tests;
#[cfg(test)]
mod live_tests;

pub(crate) fn list(
    bus: &SharedBus,
    filter: &NotificationFilter,
    policy: InteractionPolicy,
    deadline: Deadline,
    lease: Option<&InteractionLease>,
) -> Result<Vec<NotificationInfo>, AdapterError> {
    match backend::select(deadline) {
        Backend::Dux(shell) => dux::list(bus, shell, filter, policy, deadline),
        _ => list::list(bus, filter, policy, deadline, lease),
    }
}

pub(crate) fn dismiss(
    bus: &SharedBus,
    request: DismissNotificationRequest<'_>,
    lease: &InteractionLease,
) -> Result<NotificationInfo, AdapterError> {
    match backend::select(lease.deadline()) {
        Backend::Dux(shell) => dux::dismiss(bus, shell, request, lease),
        _ => dismiss::dismiss(bus, request, lease),
    }
}

pub(crate) fn dismiss_all(
    bus: &SharedBus,
    request: DismissAllNotificationsRequest<'_>,
    lease: &InteractionLease,
) -> Result<(Vec<NotificationInfo>, Vec<String>), AdapterError> {
    match backend::select(lease.deadline()) {
        Backend::Dux(shell) => dux::dismiss_all(bus, shell, request, lease),
        _ => dismiss_all::dismiss_all(bus, request, lease),
    }
}

pub(crate) fn action(
    bus: &SharedBus,
    request: NotificationActionRequest<'_>,
    lease: &InteractionLease,
) -> Result<ActionResult, AdapterError> {
    match backend::select(lease.deadline()) {
        Backend::Dux(shell) => dux::action(bus, shell, request, lease),
        _ => action::action(bus, request, lease),
    }
}

/// Whether the session's notification service belongs to the shell the
/// session runs. The dux-shell is its own notification service by the
/// recognition rule; every other session keeps the GNOME rules.
pub(crate) fn check_support(deadline: Deadline) -> Result<(), AdapterError> {
    match backend::select(deadline) {
        Backend::Dux(_) => Ok(()),
        _ => crate::system::shell::detect::require_daemon(deadline),
    }
}
