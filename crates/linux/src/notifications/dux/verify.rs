use std::time::{Duration, Instant};

use agent_desktop_core::{AdapterError, Deadline};

use super::card::Card;
use super::center::Center;
use super::locate::find;
use super::world::DuxWorld;
use crate::notifications::signal::Signal;
use crate::notifications::verify::{pressed_without_effect, same_identity};
use crate::system::shell::light::same_object;

const POLL: Duration = Duration::from_millis(100);
pub(crate) const REMOVAL_SETTLE: Duration = Duration::from_millis(6_000);
pub(crate) const SWEEP_SETTLE: Duration = Duration::from_millis(2_000);
const DISMISSED: u32 = 2;

/// Whether the card is absent for a reason that proves it left: its group
/// shows every card, so a missing card is a removed one.
fn group_shows_all(center: &Center, card: &Card) -> bool {
    center
        .groups
        .iter()
        .find(|group| same_object(&group.wrapper, &card.group))
        .is_none_or(|group| !group.hides_cards())
}

/// Whether the reading proves the card left the center.
pub(crate) fn is_gone(after: &Center, card: &Card) -> bool {
    find(after, &card.node).is_none() && group_shows_all(after, card)
}

/// The cards of `before`, other than the target, that `after` no longer
/// holds: notifications that left while only the target should have. A card
/// whose object left but whose words `after` still shows is not one: a group
/// that drops to one card lays that card out again under a new object.
pub(crate) fn bystanders<'a>(before: &'a Center, after: &Center, target: &Card) -> Vec<&'a Card> {
    before
        .cards
        .iter()
        .filter(|card| {
            !same_object(&card.node, &target.node)
                && is_gone(after, card)
                && !still_shown(before, after, target, card)
        })
        .collect()
}

/// Whether `after` still shows as many cards with the words of `card` as
/// `before` held besides the target.
fn still_shown(before: &Center, after: &Center, target: &Card, card: &Card) -> bool {
    let alike = |cards: &[Card]| {
        cards
            .iter()
            .filter(|other| same_identity(&other.info, &card.info))
            .count()
    };
    let expected =
        alike(&before.cards).saturating_sub(usize::from(same_identity(&target.info, &card.info)));
    alike(&after.cards) >= expected
}

/// How many notifications the service reported closed by a user's dismissal.
pub(crate) fn dismissals(signals: &[Signal]) -> usize {
    signals
        .iter()
        .filter(|signal| matches!(signal, Signal::Closed(_, DISMISSED)))
        .count()
}

pub(crate) fn invocations(signals: &[Signal]) -> usize {
    signals
        .iter()
        .filter(|signal| matches!(signal, Signal::ActionInvoked(_)))
        .count()
}

/// Reads the center until `done` holds or the window closes, answering with
/// the last reading; a center that has closed reads as `None`. An expired
/// window is an answer, not an error.
pub(crate) fn settle_read(
    world: &dyn DuxWorld,
    deadline: Deadline,
    window: Duration,
    done: impl Fn(Option<&Center>) -> bool,
) -> Result<Option<Center>, AdapterError> {
    let started = Instant::now();
    loop {
        let center = world.center(deadline).map_err(unconfirmed)?;
        if done(center.as_ref()) || started.elapsed() >= window || deadline.is_expired() {
            return Ok(center);
        }
        std::thread::sleep(POLL);
    }
}

/// The press went out, so a center that cannot be read afterwards leaves the
/// delivery in doubt and never turns it into a bare failure of the read.
pub(crate) fn unconfirmed(error: AdapterError) -> AdapterError {
    pressed_without_effect(
        "The control was pressed but the notification center could not be read afterwards",
        "Run list-notifications to see what the center holds now",
    )
    .with_platform_detail(error.message)
}

#[cfg(test)]
#[path = "verify_tests.rs"]
mod tests;
