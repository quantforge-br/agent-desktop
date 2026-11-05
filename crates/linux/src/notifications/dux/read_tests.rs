use super::{Changes, read_complete, read_expanded, undo};
use crate::notifications::dux::fake::shell;
use crate::notifications::dux::fake_nodes::{deadline, note};
use crate::notifications::dux::fake_quirks::Quirks;
use crate::notifications::dux::restore::Restore;

fn mail_group() -> Vec<crate::notifications::dux::fake_nodes::Note> {
    vec![
        note(1, "Mail", "a"),
        note(2, "Mail", "b"),
        note(3, "Mail", "c"),
        note(4, "Chat", "d"),
    ]
}

#[test]
fn collapsed_groups_are_expanded_so_every_notification_has_a_card() {
    let world = shell(mail_group(), true);
    let mut changes = Changes::default();

    let center = read_complete(&world, deadline(), &mut changes).expect("read");

    let titles: Vec<&str> = center
        .cards
        .iter()
        .map(|card| card.info.title.as_str())
        .collect();
    assert_eq!(titles, vec!["a", "b", "c", "d"]);
    assert!(world.is_expanded("Mail"));
}

#[test]
fn undo_collapses_exactly_the_groups_the_read_expanded() {
    let world = shell(mail_group(), true);
    world.expand("Chat");
    let mut changes = Changes::default();
    read_complete(&world, deadline(), &mut changes).expect("read");

    let restore = undo(&world, &changes);

    assert_eq!(restore, Restore::Adopted);
    assert!(!world.is_expanded("Mail"));
    assert!(world.is_expanded("Chat"));
    assert_eq!(
        world.pressed(),
        vec!["press /g-Mail/lead.toggle", "press /g-Mail/collapse"]
    );
}

#[test]
fn the_history_tab_is_switched_away_from_and_selected_again() {
    let world = shell(vec![note(1, "Mail", "a")], true);
    world.selects_history();
    let mut changes = Changes::default();

    let center = read_complete(&world, deadline(), &mut changes).expect("read");
    assert_eq!(center.cards.len(), 1);
    assert!(!world.on_history());

    undo(&world, &changes);
    assert!(world.on_history());
    assert_eq!(world.pressed(), vec!["press /tab0", "press /tab1"]);
}

#[test]
fn a_center_that_shows_fewer_cards_than_it_counts_is_an_error_not_a_shorter_list() {
    let world = shell(mail_group(), true);
    world.quirks(Quirks {
        expand_limit: Some(2),
        ..Quirks::default()
    });
    let mut changes = Changes::default();

    let error = read_complete(
        &world,
        agent_desktop_core::Deadline::after(3_000).expect("deadline"),
        &mut changes,
    )
    .expect_err("incomplete");

    let details = error.details.expect("details");
    assert_eq!(details["expected"], serde_json::json!(4));
    assert_eq!(details["read"], serde_json::json!(3));
}

#[test]
fn the_tolerant_read_reports_what_is_laid_out_and_says_it_is_incomplete() {
    let world = shell(mail_group(), true);
    world.quirks(Quirks {
        expand_limit: Some(2),
        ..Quirks::default()
    });
    let mut changes = Changes::default();

    let center = read_expanded(&world, deadline(), &mut changes).expect("read");

    assert_eq!(center.cards.len(), 3);
    assert!(!center.is_complete());
}

#[test]
fn a_center_that_closes_during_the_read_leaves_nothing_to_restore() {
    let world = shell(mail_group(), true);
    let mut changes = Changes::default();
    read_complete(&world, deadline(), &mut changes).expect("read");
    world.close_now();

    assert_eq!(undo(&world, &changes), Restore::Adopted);
}
