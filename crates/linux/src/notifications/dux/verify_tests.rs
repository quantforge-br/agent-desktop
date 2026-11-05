use super::{bystanders, dismissals, invocations, is_gone};
use crate::notifications::dux::fake::shell;
use crate::notifications::dux::fake_nodes::{deadline, note};
use crate::notifications::dux::fake_quirks::Quirks;
use crate::notifications::dux::world::DuxWorld;
use crate::notifications::signal::Signal;

#[test]
fn only_user_dismissals_count_as_dismissals() {
    let seen = [
        Signal::Closed(1, 2),
        Signal::Closed(2, 1),
        Signal::Closed(3, 3),
        Signal::ActionInvoked(4),
    ];

    assert_eq!(dismissals(&seen), 1);
    assert_eq!(invocations(&seen), 1);
}

#[test]
fn a_card_missing_from_a_fully_laid_out_group_has_left() {
    let before = shell(vec![note(1, "Mail", "a"), note(2, "Chat", "b")], true);
    let first = before.center(deadline()).expect("read").expect("open");
    let after = shell(vec![note(2, "Chat", "b")], true);
    let later = after.center(deadline()).expect("read").expect("open");

    assert!(is_gone(&later, &first.cards[0]));
    assert!(!is_gone(&later, &first.cards[1]));
}

#[test]
fn a_card_missing_from_a_group_that_hides_cards_proves_nothing() {
    let world = shell(vec![note(1, "Mail", "a"), note(2, "Mail", "b")], true);
    world.expand("Mail");
    let expanded = world.center(deadline()).expect("read").expect("open");
    let target = expanded.cards[1].clone();
    world.collapse("Mail");
    let collapsed = world.center(deadline()).expect("read").expect("open");

    assert!(!is_gone(&collapsed, &target));
}

#[test]
fn neighbours_that_left_with_the_target_are_bystanders() {
    let world = shell(
        vec![
            note(1, "Mail", "a"),
            note(2, "Chat", "b"),
            note(3, "Files", "c"),
        ],
        true,
    );
    world.quirks(Quirks {
        dismiss_takes_neighbour: true,
        ..Quirks::default()
    });
    let before = world.center(deadline()).expect("read").expect("open");
    let target = before.cards[0].clone();
    world
        .press(&target.dismiss.element, deadline())
        .expect("pressed");
    let after = world.center(deadline()).expect("read").expect("open");

    let left: Vec<&str> = bystanders(&before, &after, &target)
        .iter()
        .map(|card| card.info.title.as_str())
        .collect();

    assert_eq!(left, vec!["b"]);
}
