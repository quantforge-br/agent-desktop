use agent_desktop_core::ErrorCode;

use super::parse;
use crate::notifications::dux::fake::shell;
use crate::notifications::dux::fake_nodes::element;
use crate::notifications::dux::fake_nodes::{Described, Tree, deadline, node, note};
use crate::notifications::dux::world::DuxWorld;
use crate::system::shell::dux::details::Details;
use crate::system::shell::search::walk;
use crate::tree::vocabulary::role_numbers::{FILLER, FRAME, LIST_ITEM, PUSH_BUTTON, RADIO_BUTTON};

fn center_of(tree: Tree) -> Result<super::Center, agent_desktop_core::AdapterError> {
    let walked = walk(&Described(tree), &element("/f"), deadline(), &|_| false).expect("walked");
    parse(&walked, &|_| Ok(Details::default()))
}

fn header(tree: &mut Tree) {
    node(tree, None, "/f", FRAME, "", true);
    node(tree, Some("/f"), "/t0", RADIO_BUTTON, "Current (1)", true);
    node(tree, Some("/f"), "/t1", RADIO_BUTTON, "History (4)", true);
    for at in 0..3 {
        node(tree, Some("/f"), &format!("/h{at}"), PUSH_BUTTON, "", true);
    }
}

#[test]
fn cards_are_numbered_in_layout_order_across_groups() {
    let world = shell(
        vec![
            note(1, "Mail", "a"),
            note(2, "Chat", "b"),
            note(3, "Mail", "c"),
        ],
        true,
    );
    world.expand("Mail");

    let center = world.center(deadline()).expect("read").expect("open");

    let seen: Vec<(usize, String)> = center
        .cards
        .iter()
        .map(|card| {
            (
                card.info.index,
                format!("{}/{}", card.info.app_name, card.info.title),
            )
        })
        .collect();
    assert_eq!(
        seen,
        vec![
            (1, "Mail/a".to_owned()),
            (2, "Mail/c".to_owned()),
            (3, "Chat/b".to_owned())
        ]
    );
}

#[test]
fn a_collapsed_group_shows_only_its_newest_card_and_says_it_hides_the_rest() {
    let world = shell(vec![note(1, "Mail", "a"), note(2, "Mail", "b")], true);

    let center = world.center(deadline()).expect("read").expect("open");

    assert_eq!(center.cards.len(), 1);
    assert!(center.groups[0].hides_cards());
    assert_eq!(center.expected, Some(2));
    assert!(!center.is_complete());
}

#[test]
fn a_single_notification_group_hides_nothing() {
    let world = shell(vec![note(1, "Mail", "a")], true);

    let center = world.center(deadline()).expect("read").expect("open");

    assert!(!center.groups[0].hides_cards());
    assert!(center.is_complete());
}

#[test]
fn action_buttons_between_the_header_controls_and_the_dismissal_are_the_actions() {
    let mut with_actions = note(1, "Chat", "ping");
    with_actions.actions = vec!["Reply", "Mute"];
    let world = shell(vec![with_actions], true);

    let center = world.center(deadline()).expect("read").expect("open");

    assert_eq!(center.cards[0].info.actions, vec!["Reply", "Mute"]);
    assert_eq!(
        center.cards[0]
            .action_named("mute")
            .map(|button| button.name.as_str()),
        Some("Mute")
    );
}

#[test]
fn history_entries_are_not_current_notifications() {
    let mut tree = Tree::new();
    header(&mut tree);
    node(&mut tree, Some("/f"), "/old", FILLER, "", true);
    node(
        &mut tree,
        Some("/old"),
        "/old/li",
        LIST_ITEM,
        "Old. gone",
        true,
    );

    let center = center_of(tree).expect("parsed");

    assert!(center.cards.is_empty());
}

#[test]
fn a_card_without_a_dismissal_control_is_refused_as_unproven() {
    let mut tree = Tree::new();
    header(&mut tree);
    node(&mut tree, Some("/f"), "/g", FILLER, "", true);
    node(&mut tree, Some("/g"), "/g/lead", LIST_ITEM, "Mail. a", true);
    node(
        &mut tree,
        Some("/g/lead"),
        "/g/lead/b0",
        PUSH_BUTTON,
        "x",
        true,
    );
    node(&mut tree, Some("/g"), "/g/dismiss", PUSH_BUTTON, "", true);
    node(&mut tree, Some("/g"), "/g/collapse", PUSH_BUTTON, "", false);
    node(&mut tree, Some("/g"), "/g/menu", FILLER, "", true);

    let error = center_of(tree).expect_err("refused");

    assert_eq!(error.code, ErrorCode::ActionNotSupported);
    assert_eq!(
        error
            .details
            .as_ref()
            .and_then(|details| details.get("kind")),
        Some(&serde_json::json!("notification_center_unproven"))
    );
}

#[test]
fn a_list_holder_in_an_unknown_arrangement_is_refused_rather_than_skipped() {
    let mut tree = Tree::new();
    header(&mut tree);
    node(&mut tree, Some("/f"), "/g", FILLER, "", true);
    node(&mut tree, Some("/g"), "/g/a", LIST_ITEM, "Mail. a", true);
    node(&mut tree, Some("/g"), "/g/b", LIST_ITEM, "Mail. b", true);

    let error = center_of(tree).expect_err("refused");

    assert_eq!(error.code, ErrorCode::ActionNotSupported);
}

#[test]
fn a_frame_without_the_tab_header_is_not_a_center() {
    let mut tree = Tree::new();
    node(&mut tree, None, "/f", FRAME, "", true);
    node(&mut tree, Some("/f"), "/b", PUSH_BUTTON, "x", true);

    assert!(center_of(tree).is_err());
}
