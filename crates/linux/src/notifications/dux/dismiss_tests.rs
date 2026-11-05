use agent_desktop_core::{
    DeliverySemantics, DismissNotificationRequest, ErrorCode, InteractionPolicy,
    NotificationIdentity,
};

use super::dismiss_in;
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
    identity: &'a NotificationIdentity,
    policy: InteractionPolicy,
) -> DismissNotificationRequest<'a> {
    DismissNotificationRequest {
        index,
        app_filter: None,
        identity,
        policy,
    }
}

fn mail_and_chat() -> Vec<crate::notifications::dux::fake_nodes::Note> {
    vec![
        note(1, "Mail", "a"),
        note(2, "Mail", "b"),
        note(3, "Mail", "c"),
        note(4, "Chat", "d"),
    ]
}

#[test]
fn one_notification_of_a_group_is_dismissed_and_its_neighbours_stay() {
    let world = shell(mail_and_chat(), false);
    let who = identity("Mail", "b");

    let info = dismiss_in(
        &world,
        &world.signals(),
        request(2, &who, InteractionPolicy::headed()),
        &lease(),
    )
    .expect("dismissed");

    assert_eq!(
        (info.app_name.as_str(), info.title.as_str(), info.index),
        ("Mail", "b", 2)
    );
    assert_eq!(world.titles(), vec!["a", "c", "d"]);
    assert!(!world.is_open());
    assert!(!world.is_expanded("Mail"));
}

#[test]
fn only_the_dismiss_control_of_the_target_is_pressed() {
    let world = shell(mail_and_chat(), true);
    let who = identity("Chat", "d");

    dismiss_in(
        &world,
        &world.signals(),
        request(4, &who, InteractionPolicy::headed()),
        &lease(),
    )
    .expect("dismissed");

    let pressed = world.pressed();
    assert!(
        pressed.contains(&"press /g-Chat/lead.dismiss".to_owned()),
        "{pressed:?}"
    );
    assert!(
        !pressed.iter().any(|line| line.contains("/g-Chat/dismiss")),
        "{pressed:?}"
    );
}

#[test]
fn a_changed_identity_is_refused_before_anything_is_pressed() {
    let world = shell(mail_and_chat(), true);
    let who = identity("Mail", "not-b");

    let error = dismiss_in(
        &world,
        &world.signals(),
        request(2, &who, InteractionPolicy::headed()),
        &lease(),
    )
    .expect_err("moved");

    assert_eq!(error.code, ErrorCode::NotificationNotFound);
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    assert_eq!(world.titles().len(), 4);
    assert!(
        !world.pressed().iter().any(|line| line.contains("dismiss")),
        "{:?}",
        world.pressed()
    );
}

#[test]
fn an_index_past_the_list_is_not_found() {
    let world = shell(mail_and_chat(), true);
    let who = identity("Mail", "a");

    let error = dismiss_in(
        &world,
        &world.signals(),
        request(9, &who, InteractionPolicy::headed()),
        &lease(),
    )
    .expect_err("absent");

    assert_eq!(error.code, ErrorCode::NotificationNotFound);
}

#[test]
fn the_app_filter_must_match_the_notification_at_the_index() {
    let world = shell(mail_and_chat(), true);
    let who = identity("Mail", "a");
    let mut asked = request(1, &who, InteractionPolicy::headed());
    asked.app_filter = Some("Chat");

    let error = dismiss_in(&world, &world.signals(), asked, &lease()).expect_err("moved");

    assert_eq!(error.code, ErrorCode::NotificationNotFound);
    assert_eq!(world.titles().len(), 4);
}

#[test]
fn a_press_that_changes_nothing_is_reported_as_delivered_but_unverified() {
    let world = shell(mail_and_chat(), true);
    world.quirks(Quirks {
        inert_dismiss: true,
        silent_service: true,
        ..Quirks::default()
    });
    let who = identity("Chat", "d");

    let error = dismiss_in(
        &world,
        &world.signals(),
        request(4, &who, InteractionPolicy::headed()),
        &lease(),
    )
    .expect_err("no effect");

    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
}

#[test]
fn a_press_that_also_removes_another_notification_is_a_failure() {
    let world = shell(mail_and_chat(), true);
    world.quirks(Quirks {
        dismiss_takes_neighbour: true,
        ..Quirks::default()
    });
    let who = identity("Chat", "d");

    let error = dismiss_in(
        &world,
        &world.signals(),
        request(4, &who, InteractionPolicy::headed()),
        &lease(),
    )
    .expect_err("too much");

    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
}

#[test]
fn the_cards_leaving_proves_the_dismissal_when_no_signal_could_be_heard() {
    let world = shell(mail_and_chat(), true);
    let mut signals = world.signals();
    signals.fails = true;
    let who = identity("Chat", "d");

    let info = dismiss_in(
        &world,
        &signals,
        request(4, &who, InteractionPolicy::headed()),
        &lease(),
    )
    .expect("dismissed");

    assert_eq!(info.title, "d");
}

#[test]
fn a_center_that_cannot_be_read_after_the_press_leaves_the_delivery_in_doubt() {
    let world = shell(vec![note(1, "Mail", "a"), note(4, "Chat", "d")], true);
    world.quirks(Quirks {
        unreadable_after_press: true,
        ..Quirks::default()
    });
    let who = identity("Chat", "d");

    let error = dismiss_in(
        &world,
        &world.signals(),
        request(2, &who, InteractionPolicy::headed()),
        &lease(),
    )
    .expect_err("unread");

    assert_eq!(
        error.disposition,
        DeliverySemantics::delivered_unverified(),
        "{error:?}"
    );
}

#[test]
fn a_headless_policy_is_refused_before_the_center_is_touched() {
    let world = shell(mail_and_chat(), false);
    let who = identity("Chat", "d");

    let error = dismiss_in(
        &world,
        &world.signals(),
        request(4, &who, InteractionPolicy::headless()),
        &lease(),
    )
    .expect_err("denied");

    assert_eq!(error.code, ErrorCode::PolicyDenied);
    assert!(world.log.borrow().is_empty());
}

#[test]
fn the_card_a_two_card_group_lays_out_again_is_not_a_bystander() {
    let world = shell(vec![note(1, "Mail", "a"), note(2, "Mail", "b")], false);
    world.regroups.set(true);
    let who = identity("Mail", "b");

    let info = dismiss_in(
        &world,
        &world.signals(),
        request(2, &who, InteractionPolicy::headed()),
        &lease(),
    )
    .expect("dismissed");

    assert_eq!(info.title, "b");
    assert_eq!(world.titles(), vec!["a"]);
}
