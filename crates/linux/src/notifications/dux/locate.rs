use agent_desktop_core::{AdapterError, DeliverySemantics, ErrorCode};

use super::card::Card;
use super::center::Center;
use crate::notifications::list::matches_app;
use crate::notifications::target::{Wanted, moved};
use crate::system::shell::light::{Light, same_object};

/// The card at the index. The index is a position, never the identity: the
/// app filter and the expected identity are checked against what the read
/// found, and a mismatch means the list moved.
pub(crate) fn locate<'a>(center: &'a Center, wanted: Wanted<'_>) -> Result<&'a Card, AdapterError> {
    let card = center
        .cards
        .iter()
        .find(|card| card.info.index == wanted.index)
        .ok_or_else(|| AdapterError::notification_not_found(wanted.index))?;
    let app_ok = wanted
        .app_filter
        .is_none_or(|app| matches_app(&card.info, app));
    if !app_ok || !wanted.identity.matches(&card.info) {
        return Err(moved(wanted.index));
    }
    Ok(card)
}

/// The card with the object, in a fresh reading.
pub(crate) fn find<'a>(center: &'a Center, node: &crate::tree::AtspiElement) -> Option<&'a Card> {
    center
        .cards
        .iter()
        .find(|card| same_object(&card.node, node))
}

/// A control that is laid out and can be pressed, or the refusal naming it.
pub(crate) fn pressable<'a>(
    control: &'a Light,
    what: &str,
    index: usize,
) -> Result<&'a Light, AdapterError> {
    if control.showing {
        return Ok(control);
    }
    Err(AdapterError::new(
        ErrorCode::ActionFailed,
        format!("The notification at index {index} shows no {what} to press"),
    )
    .with_suggestion("Run list-notifications and retry with a freshly observed entry")
    .with_disposition(DeliverySemantics::not_delivered()))
}
