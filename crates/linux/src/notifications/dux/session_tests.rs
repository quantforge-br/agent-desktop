use agent_desktop_core::{Deadline, DeliverySemantics, ErrorCode, InteractionPolicy};

use super::with_center;
use crate::notifications::dux::fake::shell;
use crate::notifications::dux::fake_nodes::note;
use crate::notifications::dux::fake_quirks::Quirks;

fn deadline() -> Deadline {
    Deadline::after(900).expect("deadline")
}

#[test]
fn a_closed_center_is_shown_for_the_call_and_closed_again() {
    let world = shell(vec![note(1, "Mail", "a")], false);

    let titles = with_center(&world, deadline(), InteractionPolicy::headed(), |_| {
        Ok(world.titles())
    })
    .expect("ran");

    assert_eq!(titles, vec!["a"]);
    assert!(!world.is_open());
    assert_eq!(*world.log.borrow(), vec!["ipc open", "ipc close"]);
}

#[test]
fn a_center_whose_tree_tears_while_it_closes_is_still_proved_closed() {
    let world = shell(vec![note(1, "Mail", "a")], false);
    world.tears_on_close(3);

    with_center(&world, deadline(), InteractionPolicy::headed(), |_| Ok(())).expect("closed");

    assert!(!world.is_open());
}

#[test]
fn a_center_that_was_open_is_left_open() {
    let world = shell(vec![note(1, "Mail", "a")], true);

    with_center(&world, deadline(), InteractionPolicy::headed(), |_| Ok(())).expect("ran");

    assert!(world.is_open());
    assert!(world.log.borrow().is_empty());
}

#[test]
fn a_closed_center_is_not_shown_without_permission_to_take_the_focus() {
    let world = shell(vec![note(1, "Mail", "a")], false);

    let error = with_center(
        &world,
        deadline(),
        InteractionPolicy::headless(),
        |_| Ok(()),
    )
    .expect_err("denied");

    assert_eq!(error.code, ErrorCode::PolicyDenied);
    assert!(world.log.borrow().is_empty());
}

#[test]
fn an_open_center_is_read_without_permission_to_take_the_focus() {
    let world = shell(vec![note(1, "Mail", "a")], true);

    assert!(
        with_center(
            &world,
            deadline(),
            InteractionPolicy::headless(),
            |_| Ok(())
        )
        .is_ok()
    );
}

#[test]
fn a_shell_that_accepts_the_request_but_shows_nothing_is_reported_with_its_answer() {
    let world = shell(vec![note(1, "Mail", "a")], false);
    world.quirks(Quirks {
        never_shows: true,
        ..Quirks::default()
    });

    let error = with_center(
        &world,
        Deadline::after(300).expect("deadline"),
        InteractionPolicy::headed(),
        |_| Ok(()),
    )
    .expect_err("never shown");

    assert_eq!(error.code, ErrorCode::WindowNotFound);
    let details = error.details.expect("details");
    assert_eq!(
        details["ipc_reply"],
        serde_json::json!("NOTIFICATION_MODAL_OPEN_SUCCESS")
    );
    assert_eq!(details["list_restore"], serde_json::json!("closed"));
}

#[test]
fn a_failing_body_still_closes_the_center_it_opened() {
    let world = shell(vec![note(1, "Mail", "a")], false);

    let error = with_center::<()>(&world, deadline(), InteractionPolicy::headed(), |_| {
        Err(
            agent_desktop_core::AdapterError::new(ErrorCode::ActionFailed, "boom")
                .with_disposition(DeliverySemantics::not_delivered()),
        )
    })
    .expect_err("failed");

    assert_eq!(error.message, "boom");
    assert!(!world.is_open());
    assert_eq!(
        error.details.expect("details")["list_restore"],
        serde_json::json!("closed")
    );
}
