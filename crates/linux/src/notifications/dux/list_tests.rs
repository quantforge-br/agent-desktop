use agent_desktop_core::{ErrorCode, InteractionPolicy, NotificationFilter};

use super::list_in;
use crate::notifications::dux::fake::shell;
use crate::notifications::dux::fake_nodes::{deadline, note};

fn everything() -> NotificationFilter {
    NotificationFilter {
        app: None,
        text: None,
        limit: None,
    }
}

#[test]
fn every_notification_is_listed_with_its_index_body_and_actions() {
    let mut chat = note(2, "Chat", "ping");
    chat.body = "are you there";
    chat.actions = vec!["Reply"];
    let world = shell(vec![note(1, "Mail", "a"), chat, note(3, "Mail", "b")], true);

    let listed = list_in(
        &world,
        &everything(),
        InteractionPolicy::headless(),
        deadline(),
    )
    .expect("listed");

    let shown: Vec<(usize, &str, Option<&str>, usize)> = listed
        .iter()
        .map(|info| {
            (
                info.index,
                info.title.as_str(),
                info.body.as_deref(),
                info.actions.len(),
            )
        })
        .collect();
    assert_eq!(
        shown,
        vec![
            (1, "a", None, 0),
            (2, "b", None, 0),
            (3, "ping", Some("are you there"), 1)
        ]
    );
}

#[test]
fn a_closed_center_is_shown_to_be_read_and_closed_again() {
    let world = shell(vec![note(1, "Mail", "a")], false);

    let listed = list_in(
        &world,
        &everything(),
        InteractionPolicy::headed(),
        deadline(),
    )
    .expect("listed");

    assert_eq!(listed.len(), 1);
    assert!(!world.is_open());
    assert_eq!(*world.log.borrow(), vec!["ipc open", "ipc close"]);
}

#[test]
fn a_closed_center_is_refused_when_the_policy_forbids_taking_the_focus() {
    let world = shell(vec![note(1, "Mail", "a")], false);

    let error = list_in(
        &world,
        &everything(),
        InteractionPolicy::headless(),
        deadline(),
    )
    .expect_err("denied");

    assert_eq!(error.code, ErrorCode::PolicyDenied);
    assert!(world.log.borrow().is_empty());
}

#[test]
fn the_filter_keeps_the_unfiltered_indexes() {
    let world = shell(vec![note(1, "Mail", "a"), note(2, "Chat", "b")], true);
    let only_chat = NotificationFilter {
        app: Some("chat".to_owned()),
        text: None,
        limit: None,
    };

    let listed = list_in(
        &world,
        &only_chat,
        InteractionPolicy::headless(),
        deadline(),
    )
    .expect("listed");

    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].index, 2);
}

#[test]
fn groups_the_center_collapsed_are_expanded_for_the_read_and_collapsed_again() {
    let world = shell(vec![note(1, "Mail", "a"), note(2, "Mail", "b")], true);

    let listed = list_in(
        &world,
        &everything(),
        InteractionPolicy::headless(),
        deadline(),
    )
    .expect("listed");

    assert_eq!(listed.len(), 2);
    assert!(!world.is_expanded("Mail"));
}
