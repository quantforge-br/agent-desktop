use agent_desktop_core::{AdapterError, DeliverySemantics, ErrorCode, NotificationInfo};
use serde_json::json;

use super::card::Card;
use super::center::Center;
use super::group::Group;
use super::name::{count_of, split};
use super::tab::Tab;
use crate::system::shell::dux::details::Details;
use crate::system::shell::dux::header::tab_header_start;
use crate::system::shell::light::Light;
use crate::system::shell::search::Walk;
use crate::tree::vocabulary::role_numbers::{FILLER, LIST_ITEM, PUSH_BUTTON, RADIO_BUTTON};

const CARD_HEADER_BUTTONS: usize = 3;
const WRAPPER_PREFIX: [u32; 3] = [LIST_ITEM, PUSH_BUTTON, PUSH_BUTTON];

type DetailsOf<'a> = &'a dyn Fn(&crate::tree::AtspiElement) -> Result<Details, AdapterError>;

pub(super) fn unproven(detail: &str) -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionNotSupported,
        "The shell's notification center has a layout this tool cannot prove",
    )
    .with_suggestion("Update agent-desktop; the dux-shell's notification center changed shape")
    .with_details(json!({ "kind": "notification_center_unproven", "detail": detail }))
    .with_disposition(DeliverySemantics::not_delivered())
}

fn roles_of(walk: &Walk, kids: &[usize]) -> Vec<u32> {
    kids.iter()
        .map(|kid| walk.visits[*kid].light.role)
        .collect()
}

fn tabs_of(walk: &Walk, kids: &[usize], details: DetailsOf<'_>) -> Result<Vec<Tab>, AdapterError> {
    let roles = roles_of(walk, kids);
    let start = tab_header_start(&roles).ok_or_else(|| unproven("no tab header"))?;
    kids[start..]
        .iter()
        .take_while(|kid| walk.visits[**kid].light.role == RADIO_BUTTON)
        .map(|kid| {
            let radio = walk.visits[*kid].light.clone();
            let checked = details(&radio.element)?.checked;
            Ok(Tab { radio, checked })
        })
        .collect()
}

fn buttons_of(walk: &Walk, card: usize) -> Vec<&Light> {
    walk.visits[card]
        .children
        .iter()
        .map(|child| &walk.visits[*child].light)
        .filter(|light| light.role == PUSH_BUTTON)
        .collect()
}

fn card_at(
    walk: &Walk,
    at: usize,
    group: &Light,
    details: DetailsOf<'_>,
) -> Result<Card, AdapterError> {
    let light = &walk.visits[at].light;
    let buttons = buttons_of(walk, at);
    if buttons.len() <= CARD_HEADER_BUTTONS {
        return Err(unproven(
            "a card holds fewer controls than its header and dismissal",
        ));
    }
    let dismiss = buttons[buttons.len() - 1].clone();
    let actions: Vec<Light> = buttons[CARD_HEADER_BUTTONS..buttons.len() - 1]
        .iter()
        .filter(|button| !button.name.trim().is_empty())
        .map(|button| (*button).clone())
        .collect();
    let (app_name, title) = split(&light.name);
    let body = details(&light.element)?.description;
    Ok(Card {
        info: NotificationInfo {
            index: 0,
            app_name,
            title,
            body: (!body.is_empty()).then_some(body),
            actions: actions.iter().map(|action| action.name.clone()).collect(),
        },
        node: light.element.clone(),
        dismiss,
        actions,
        group: group.element.clone(),
    })
}

/// Reads one group wrapper: its leading card, the group's own dismiss and
/// collapse controls, the nested cards of an expanded group, and the context
/// menu holder that closes the row.
fn group_at(
    walk: &Walk,
    at: usize,
    details: DetailsOf<'_>,
) -> Result<(Group, Vec<Card>), AdapterError> {
    let kids = &walk.visits[at].children;
    let wrapper = &walk.visits[at].light;
    let lead = kids[0];
    let nested = &kids[WRAPPER_PREFIX.len()..kids.len() - 1];
    let toggle = buttons_of(walk, lead)
        .first()
        .map(|button| (*button).clone());
    let collapse = Some(walk.visits[kids[2]].light.clone());
    let cards = if nested.is_empty() {
        vec![card_at(walk, lead, wrapper, details)?]
    } else {
        nested
            .iter()
            .map(|card| card_at(walk, *card, wrapper, details))
            .collect::<Result<_, _>>()?
    };
    let group = Group {
        wrapper: wrapper.element.clone(),
        toggle,
        collapse,
        expanded: !nested.is_empty(),
    };
    Ok((group, cards))
}

/// How a filler under the center relates to the current notifications: a
/// group wrapper (a card, two group controls, nested cards, a menu holder),
/// a history entry (a lone card), or no notification holder at all. A filler
/// that holds cards in any other arrangement is a layout this tool does not
/// know.
enum Holder {
    Group,
    History,
    Other,
}

fn holder(walk: &Walk, kid: usize) -> Result<Holder, AdapterError> {
    let visit = &walk.visits[kid];
    if visit.light.role != FILLER {
        return Ok(Holder::Other);
    }
    let roles = roles_of(walk, &visit.children);
    if !roles.contains(&LIST_ITEM) {
        return Ok(Holder::Other);
    }
    let shaped = roles.len() > WRAPPER_PREFIX.len()
        && roles.starts_with(&WRAPPER_PREFIX)
        && roles[roles.len() - 1] == FILLER
        && roles[WRAPPER_PREFIX.len()..roles.len() - 1]
            .iter()
            .all(|role| *role == LIST_ITEM);
    match (shaped, roles.as_slice()) {
        (true, _) => Ok(Holder::Group),
        (false, [LIST_ITEM]) => Ok(Holder::History),
        _ => Err(unproven(
            "a list holder arranges its cards in an unknown way",
        )),
    }
}

/// Reads the center from a walk of its frame: the tabs, the groups of current
/// notifications and their cards, numbered in the order they are laid out.
pub(crate) fn parse(walk: &Walk, details: DetailsOf<'_>) -> Result<Center, AdapterError> {
    let kids = &walk.visits[0].children;
    let tabs = tabs_of(walk, kids, details)?;
    let expected = tabs.first().and_then(|tab| count_of(&tab.radio.name));
    let (mut groups, mut cards) = (Vec::new(), Vec::new());
    for kid in kids {
        if matches!(holder(walk, *kid)?, Holder::Group) {
            let (group, found) = group_at(walk, *kid, details)?;
            groups.push(group);
            cards.extend(found);
        }
    }
    for (position, card) in cards.iter_mut().enumerate() {
        card.info.index = position + 1;
    }
    Ok(Center {
        frame: walk.visits[0].light.clone(),
        tabs,
        expected,
        groups,
        cards,
    })
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod tests;
