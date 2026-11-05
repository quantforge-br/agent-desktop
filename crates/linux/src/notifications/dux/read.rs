use std::time::{Duration, Instant};

use agent_desktop_core::{AdapterError, Deadline, ErrorCode};
use serde_json::json;

use super::center::Center;
use super::restore::Restore;
use super::world::DuxWorld;
use crate::system::shell::light::same_object;
use crate::tree::AtspiElement;

const SETTLE: Duration = Duration::from_millis(3_000);
const POLL: Duration = Duration::from_millis(100);
const READ_TRIES: usize = 12;
const MAX_STEPS: usize = 24;

/// The changes a read made to the center to see every notification: the
/// groups it expanded and the tab it switched away from.
#[derive(Debug, Default)]
pub(crate) struct Changes {
    expanded: Vec<AtspiElement>,
    reselect: Option<AtspiElement>,
}

fn closed() -> AdapterError {
    AdapterError::new(
        ErrorCode::WindowNotFound,
        "The notification center closed while it was read",
    )
    .with_suggestion("Retry the command")
}

fn center_of(world: &dyn DuxWorld, deadline: Deadline) -> Result<Center, AdapterError> {
    world.center(deadline)?.ok_or_else(closed)
}

/// Reads until `done` holds or the settle window ends, and answers with the
/// last reading either way; the caller decides what a window that ended means.
fn settle(
    world: &dyn DuxWorld,
    deadline: Deadline,
    done: impl Fn(&Center) -> bool,
) -> Result<Center, AdapterError> {
    let started = Instant::now();
    loop {
        let center = center_of(world, deadline)?;
        if done(&center) || started.elapsed() >= SETTLE || deadline.is_expired() {
            return Ok(center);
        }
        std::thread::sleep(POLL);
    }
}

/// Whether the group is expanded, `None` once the center no longer holds it.
fn expansion(center: &Center, wrapper: &AtspiElement) -> Option<bool> {
    center
        .groups
        .iter()
        .find(|group| same_object(&group.wrapper, wrapper))
        .map(|group| group.expanded)
}

/// Reads the center showing the current list with every group of it
/// expanded, so each notification has a card of its own. What was changed to
/// get there is recorded in `changes` for `undo`. A group can only show as
/// many cards as the shell lays out, so the reading may hold fewer cards than
/// the header counts; `is_complete` says so.
pub(crate) fn read_expanded(
    world: &dyn DuxWorld,
    deadline: Deadline,
    changes: &mut Changes,
) -> Result<Center, AdapterError> {
    let mut center = center_of(world, deadline)?;
    for _ in 0..MAX_STEPS {
        if !center.shows_current() {
            let Some(current) = center.tabs.first().map(|tab| tab.radio.element.clone()) else {
                break;
            };
            let shown = center.tabs.iter().find(|tab| tab.checked);
            changes.reselect = shown.map(|tab| tab.radio.element.clone());
            world.press(&current, deadline)?;
            center = settle(world, deadline, Center::shows_current)?;
            continue;
        }
        let Some(group) = center.groups.iter().find(|group| group.hides_cards()) else {
            break;
        };
        let (Some(toggle), wrapper) = (group.toggle.clone(), group.wrapper.clone()) else {
            break;
        };
        world.press(&toggle.element, deadline)?;
        changes.expanded.push(wrapper.clone());
        center = settle(world, deadline, |now| {
            expansion(now, &wrapper) != Some(false)
        })?;
        if expansion(&center, &wrapper) == Some(false) {
            break;
        }
    }
    Ok(center)
}

fn incomplete(center: &Center) -> AdapterError {
    AdapterError::new(
        ErrorCode::Internal,
        "The notification center could only be read in part",
    )
    .with_suggestion(
        "Retry the command; the center shows a limited number of notifications per application",
    )
    .with_details(json!({
        "expected": center.expected,
        "read": center.cards.len(),
    }))
}

/// `read_expanded` read again while the tree settles, until it holds every
/// notification the header counts or the tries run out. A center the shell
/// has just shown counts its notifications before it lays out their cards,
/// so a first reading can hold fewer cards than there are.
pub(crate) fn read_settled(
    world: &dyn DuxWorld,
    deadline: Deadline,
    changes: &mut Changes,
) -> Result<Center, AdapterError> {
    let mut tries = 0;
    loop {
        let center = read_expanded(world, deadline, changes)?;
        tries += 1;
        if center.is_complete() || tries >= READ_TRIES || deadline.is_expired() {
            return Ok(center);
        }
        std::thread::sleep(POLL);
    }
}

/// `read_settled` that insists on every notification the header counts; a
/// center that never reaches its count is an error, never a shorter list.
pub(crate) fn read_complete(
    world: &dyn DuxWorld,
    deadline: Deadline,
    changes: &mut Changes,
) -> Result<Center, AdapterError> {
    let center = read_settled(world, deadline, changes)?;
    if center.is_complete() {
        Ok(center)
    } else {
        Err(incomplete(&center))
    }
}

/// Puts back what `read_expanded` changed: the expanded groups collapse and
/// the tab that was on show is selected again. A center that has closed has
/// nothing left to restore. A reading that fails while a group folds is
/// taken again until the settle window ends.
pub(crate) fn undo(world: &dyn DuxWorld, changes: &Changes) -> Restore {
    let Ok(deadline) = Deadline::detached_after(4_000) else {
        return Restore::Unverified;
    };
    let mut state = Restore::Adopted;
    for wrapper in &changes.expanded {
        state = state.merge(collapse(world, wrapper, deadline));
    }
    if let Some(tab) = &changes.reselect {
        let pressed = world.press(tab, deadline);
        if pressed.is_err() && matches!(world.center(deadline), Ok(Some(_))) {
            state = Restore::Unverified;
        }
    }
    state
}

fn collapse(world: &dyn DuxWorld, wrapper: &AtspiElement, deadline: Deadline) -> Restore {
    let Ok(Some(center)) = world.center(deadline) else {
        return Restore::Adopted;
    };
    let control = center
        .groups
        .iter()
        .find(|group| same_object(&group.wrapper, wrapper) && group.expanded)
        .and_then(|group| group.collapse.clone());
    let Some(control) = control else {
        return Restore::Adopted;
    };
    if world.press(&control.element, deadline).is_err() {
        return Restore::Unverified;
    }
    let started = Instant::now();
    loop {
        match world.center(deadline) {
            Ok(None) => return Restore::Adopted,
            Ok(Some(now)) if expansion(&now, wrapper) != Some(true) => return Restore::Adopted,
            _ if started.elapsed() >= SETTLE || deadline.is_expired() => {
                return Restore::Unverified;
            }
            _ => std::thread::sleep(POLL),
        }
    }
}

#[cfg(test)]
#[path = "read_tests.rs"]
mod tests;
