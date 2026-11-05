use agent_desktop_core::{DismissAllNotificationsRequest, ErrorCode, InteractionPolicy};

use super::dismiss_all_in;
use crate::notifications::dux::fake::shell;
use crate::notifications::dux::fake_nodes::{lease, note};
use crate::notifications::dux::fake_quirks::Quirks;

fn request(app: Option<&str>) -> DismissAllNotificationsRequest<'_> {
    DismissAllNotificationsRequest {
        app_filter: app,
        policy: InteractionPolicy::headed(),
    }
}

fn mixed() -> Vec<crate::notifications::dux::fake_nodes::Note> {
    vec![
        note(1, "Mail", "a"),
        note(2, "Chat", "b"),
        note(3, "Mail", "c"),
        note(4, "Files", "d"),
        note(5, "Mail", "e"),
    ]
}

#[test]
fn every_notification_is_dismissed_one_card_at_a_time() {
    let world = shell(mixed(), false);

    let (dismissed, failures) =
        dismiss_all_in(&world, &world.signals(), request(None), &lease()).expect("swept");

    assert_eq!(dismissed.len(), 5);
    assert!(failures.is_empty(), "{failures:?}");
    assert!(world.titles().is_empty());
    assert!(!world.is_open());
    assert!(
        world
            .pressed()
            .iter()
            .all(|line| !line.ends_with("/dismiss") || line.contains("lead")),
        "{:?}",
        world.pressed()
    );
}

#[test]
fn the_app_filter_dismisses_only_that_apps_notifications() {
    let world = shell(mixed(), true);

    let (dismissed, failures) =
        dismiss_all_in(&world, &world.signals(), request(Some("mail")), &lease()).expect("swept");

    let mut titles: Vec<&str> = dismissed.iter().map(|info| info.title.as_str()).collect();
    titles.sort_unstable();
    assert_eq!(titles, vec!["a", "c", "e"]);
    assert!(failures.is_empty(), "{failures:?}");
    assert_eq!(world.titles(), vec!["b", "d"]);
}

#[test]
fn the_report_names_each_notification_by_the_index_it_had_when_listed() {
    let world = shell(mixed(), true);

    let (dismissed, _) =
        dismiss_all_in(&world, &world.signals(), request(Some("Mail")), &lease()).expect("swept");

    let mut indexes: Vec<usize> = dismissed.iter().map(|info| info.index).collect();
    indexes.sort_unstable();
    assert_eq!(indexes, vec![1, 2, 3]);
}

#[test]
fn a_center_that_opens_before_it_lays_out_its_cards_is_swept_once_they_appear() {
    let world = shell(mixed(), false);
    world.lays_out_after(3);

    let (dismissed, failures) =
        dismiss_all_in(&world, &world.signals(), request(Some("mail")), &lease()).expect("swept");

    assert_eq!(dismissed.len(), 3, "{failures:?}");
    assert!(failures.is_empty(), "{failures:?}");
    assert_eq!(world.titles(), vec!["b", "d"]);
}

#[test]
fn an_empty_center_dismisses_nothing_and_presses_nothing() {
    let world = shell(Vec::new(), false);

    let (dismissed, failures) =
        dismiss_all_in(&world, &world.signals(), request(None), &lease()).expect("swept");

    assert!(dismissed.is_empty() && failures.is_empty());
    assert!(world.pressed().is_empty());
}

#[test]
fn a_card_that_does_not_react_is_reported_and_stops_the_sweep() {
    let world = shell(mixed(), true);
    world.quirks(Quirks {
        inert_dismiss: true,
        ..Quirks::default()
    });
    let (dismissed, failures) =
        dismiss_all_in(&world, &world.signals(), request(None), &lease()).expect("swept");

    assert!(dismissed.is_empty());
    assert_eq!(failures.len(), 2, "{failures:?}");
    assert!(
        failures[1].contains("4 more notifications were not attempted"),
        "{failures:?}"
    );
    assert_eq!(world.titles().len(), 5);
}

#[test]
fn a_service_that_reports_fewer_dismissals_than_cards_that_left_is_a_failure() {
    let world = shell(vec![note(1, "Mail", "a"), note(2, "Chat", "b")], true);
    world.quirks(Quirks {
        silent_service: true,
        ..Quirks::default()
    });

    let (dismissed, failures) =
        dismiss_all_in(&world, &world.signals(), request(None), &lease()).expect("swept");

    assert_eq!(dismissed.len(), 2);
    assert_eq!(failures.len(), 1);
    assert!(failures[0].contains("0 dismissals for 2"), "{failures:?}");
}

#[test]
fn a_group_larger_than_the_center_lays_out_is_cleared_over_repeated_readings() {
    let world = shell(
        vec![
            note(1, "Mail", "a"),
            note(2, "Mail", "b"),
            note(3, "Mail", "c"),
            note(4, "Mail", "d"),
        ],
        true,
    );
    world.quirks(Quirks {
        expand_limit: Some(2),
        ..Quirks::default()
    });

    let (dismissed, failures) =
        dismiss_all_in(&world, &world.signals(), request(None), &lease()).expect("swept");

    assert_eq!(dismissed.len(), 4);
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn a_headless_policy_is_refused() {
    let world = shell(mixed(), true);
    let asked = DismissAllNotificationsRequest {
        app_filter: None,
        policy: InteractionPolicy::headless(),
    };

    let error = dismiss_all_in(&world, &world.signals(), asked, &lease()).expect_err("denied");

    assert_eq!(error.code, ErrorCode::PolicyDenied);
}
