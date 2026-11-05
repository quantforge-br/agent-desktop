use agent_desktop_core::{
    DeliverySemantics, ErrorCode, InteractionPolicy, NotificationActionRequest,
    NotificationIdentity,
};

use super::action_in;
use crate::notifications::dux::fake::shell;
use crate::notifications::dux::fake_nodes::{lease, note};
use crate::notifications::dux::fake_quirks::Quirks;

fn identity(app: &str, title: &str) -> NotificationIdentity {
    NotificationIdentity {
        expected_app: Some(app.to_owned()),
        expected_title: Some(title.to_owned()),
    }
}

fn request<'a>(
    index: usize,
    action: &'a str,
    identity: &'a NotificationIdentity,
) -> NotificationActionRequest<'a> {
    NotificationActionRequest {
        index,
        identity,
        action_name: action,
        policy: InteractionPolicy::headed(),
    }
}

fn with_actions() -> Vec<crate::notifications::dux::fake_nodes::Note> {
    let mut chat = note(2, "Chat", "ping");
    chat.actions = vec!["Reply", "Mute"];
    vec![note(1, "Mail", "a"), chat]
}

#[test]
fn the_named_action_is_pressed_and_the_services_signal_is_the_proof() {
    let world = shell(with_actions(), false);
    let who = identity("Chat", "ping");

    let result =
        action_in(&world, &world.signals(), request(2, "mute", &who), &lease()).expect("invoked");

    let shown = serde_json::to_value(&result).expect("json");
    assert_eq!(
        shown["details"]["evidence"],
        serde_json::json!("action_invoked")
    );
    assert_eq!(
        shown["disposition"]["delivery"],
        serde_json::json!("delivered_unverified")
    );
    assert!(
        world
            .pressed()
            .contains(&"press /g-Chat/lead.act1".to_owned()),
        "{:?}",
        world.pressed()
    );
}

#[test]
fn the_center_the_shell_closed_after_the_action_is_not_reopened_or_failed() {
    let world = shell(with_actions(), false);
    let who = identity("Chat", "ping");

    action_in(
        &world,
        &world.signals(),
        request(2, "Reply", &who),
        &lease(),
    )
    .expect("invoked");

    assert!(!world.is_open());
    assert_eq!(world.titles(), vec!["a"]);
}

#[test]
fn an_action_the_notification_does_not_offer_is_refused_with_the_ones_it_has() {
    let world = shell(with_actions(), true);
    let who = identity("Chat", "ping");

    let error = action_in(
        &world,
        &world.signals(),
        request(2, "Delete", &who),
        &lease(),
    )
    .expect_err("unoffered");

    assert_eq!(error.code, ErrorCode::ActionNotSupported);
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    assert_eq!(
        error.details.expect("details")["available_actions"],
        serde_json::json!(["Reply", "Mute"])
    );
    assert_eq!(world.titles().len(), 2);
}

#[test]
fn a_press_the_service_never_reports_is_delivered_but_unverified() {
    let world = shell(with_actions(), true);
    let mut signals = world.signals();
    signals.fails = true;
    let who = identity("Chat", "ping");

    let error =
        action_in(&world, &signals, request(2, "Reply", &who), &lease()).expect_err("silent");

    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
}

#[test]
fn a_changed_identity_is_refused_without_a_press() {
    let world = shell(with_actions(), true);
    let who = identity("Chat", "other");

    let error = action_in(
        &world,
        &world.signals(),
        request(2, "Reply", &who),
        &lease(),
    )
    .expect_err("moved");

    assert_eq!(error.code, ErrorCode::NotificationNotFound);
    assert!(world.pressed().is_empty());
}

#[test]
fn an_action_works_on_a_notification_inside_an_expanded_group() {
    let mut first = note(1, "Chat", "one");
    first.actions = vec!["Reply"];
    let mut second = note(2, "Chat", "two");
    second.actions = vec!["Reply"];
    let world = shell(vec![first, second], true);
    world.quirks(Quirks::default());
    let who = identity("Chat", "two");

    action_in(
        &world,
        &world.signals(),
        request(2, "Reply", &who),
        &lease(),
    )
    .expect("invoked");

    assert_eq!(world.titles(), vec!["one"]);
}
