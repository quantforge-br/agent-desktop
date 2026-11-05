use agent_desktop_core::{
    AdapterError, DismissAllNotificationsRequest, InteractionLease, NotificationInfo,
};

use super::card::Card;
use super::center::Center;
use super::policy::require_headed;
use super::read::{read_expanded, read_settled};
use super::session::with_center;
use super::verify::{SWEEP_SETTLE, dismissals, is_gone, settle_read};
use super::world::DuxWorld;
use crate::notifications::list::matches_app;
use crate::notifications::signal::{SETTLE_BUDGET, SignalSource};
use crate::notifications::verify::same_identity;
use crate::system::shell::light::same_object;
use crate::tree::AtspiElement;

type Outcome = (Vec<NotificationInfo>, Vec<String>);

const MAX_PRESSES: usize = 100;

/// Clears the center, or only the notifications of one app, one card at a
/// time. The first reading waits for the cards the header counts, so a
/// center still laying out is not taken for an empty one. Each press goes
/// to a card of a fresh reading, because the cards are
/// rebuilt as their neighbours leave, and counts as a dismissal only when that
/// card left or the number of matching cards fell. No control that acts on
/// the whole list is pressed. A card that does not react stops the sweep, since
/// the cards after it would only wait out the same window. The service's dismissal signals are counted
/// against the dismissals reported.
pub(crate) fn dismiss_all_in(
    world: &dyn DuxWorld,
    signals: &dyn SignalSource,
    request: DismissAllNotificationsRequest<'_>,
    lease: &InteractionLease,
) -> Result<Outcome, AdapterError> {
    require_headed(request.policy)?;
    let deadline = lease.deadline();
    with_center(world, deadline, request.policy, |changes| {
        let subscription = signals.subscribe(deadline).ok();
        let mut sweep = Sweep::default();
        let mut listed: Option<Vec<NotificationInfo>> = None;
        for _ in 0..MAX_PRESSES {
            let center = if listed.is_none() {
                read_settled(world, deadline, changes)?
            } else {
                read_expanded(world, deadline, changes)?
            };
            let here = matching(&center, request.app_filter);
            let initial =
                listed.get_or_insert_with(|| here.iter().map(|card| card.info.clone()).collect());
            let Some(card) = here.iter().find(|card| !sweep.is_stuck(&card.node)) else {
                break;
            };
            let removed = press(world, card, here.len(), request.app_filter, deadline)?;
            match removed {
                Ok(()) => {
                    let index = original_index(initial, &card.info);
                    sweep.dismissed.push(NotificationInfo {
                        index,
                        ..card.info.clone()
                    });
                }
                Err(why) => {
                    sweep.stick(card, why);
                    let untried = here.len() - 1;
                    if untried > 0 {
                        sweep.failures.push((
                            0,
                            format!(
                                "the sweep stopped; {untried} more notifications were not attempted"
                            ),
                        ));
                    }
                    break;
                }
            }
        }
        let seen = subscription.map(|held| held.collect(SETTLE_BUDGET));
        if let Some(seen) = seen.filter(|seen| dismissals(seen) < sweep.dismissed.len()) {
            sweep.failures.push((
                0,
                format!(
                    "the notification service reported {} dismissals for {} notifications that left",
                    dismissals(&seen),
                    sweep.dismissed.len()
                ),
            ));
        }
        Ok(sweep.outcome())
    })
}

#[derive(Default)]
struct Sweep {
    dismissed: Vec<NotificationInfo>,
    failures: Vec<(usize, String)>,
    stuck: Vec<AtspiElement>,
}

impl Sweep {
    fn is_stuck(&self, node: &AtspiElement) -> bool {
        self.stuck.iter().any(|seen| same_object(seen, node))
    }

    fn stick(&mut self, card: &Card, why: String) {
        self.stuck.push(card.node.clone());
        self.failures.push((card.info.index, why));
    }

    fn outcome(self) -> Outcome {
        (
            self.dismissed,
            self.failures
                .into_iter()
                .map(|(index, message)| format!("#{index}: {message}"))
                .collect(),
        )
    }
}

fn matching<'a>(center: &'a Center, app: Option<&str>) -> Vec<&'a Card> {
    center
        .cards
        .iter()
        .filter(|card| app.is_none_or(|name| matches_app(&card.info, name)))
        .collect()
}

/// The index the notification had in the first reading of this call, so the
/// report names the entries as the caller listed them.
fn original_index(initial: &mut Vec<NotificationInfo>, info: &NotificationInfo) -> usize {
    match initial.iter().position(|first| same_identity(first, info)) {
        Some(at) => initial.remove(at).index,
        None => info.index,
    }
}

/// Presses the card's dismiss control. The inner answer says whether the
/// press took effect; the outer error is a center that could not be read.
fn press(
    world: &dyn DuxWorld,
    card: &Card,
    before: usize,
    app: Option<&str>,
    deadline: agent_desktop_core::Deadline,
) -> Result<Result<(), String>, AdapterError> {
    if !card.dismiss.showing {
        return Ok(Err("shows no dismiss control".to_owned()));
    }
    if let Err(error) = world.press(&card.dismiss.element, deadline) {
        return Ok(Err(error.message));
    }
    let left = |now: Option<&Center>| {
        now.is_none_or(|center| is_gone(center, card) || matching(center, app).len() < before)
    };
    let after = settle_read(world, deadline, SWEEP_SETTLE, left)?;
    if left(after.as_ref()) {
        Ok(Ok(()))
    } else {
        Ok(Err("stayed in the center after its dismiss".to_owned()))
    }
}

#[cfg(test)]
#[path = "dismiss_all_tests.rs"]
mod tests;
