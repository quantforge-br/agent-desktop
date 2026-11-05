use std::time::{Duration, Instant};

use agent_desktop_core::{AdapterError, Deadline, ErrorCode, InteractionPolicy};
use serde_json::json;

use super::read::{Changes, undo};
use super::restore::{Restore, with_restore};
use super::world::DuxWorld;

const OPEN_SETTLE: Duration = Duration::from_millis(4_000);
const CLEANUP_MS: u64 = 4_000;
const CLOSE_SETTLE: Duration = Duration::from_millis(2_000);
const POLL: Duration = Duration::from_millis(80);

/// The notification center for one call. A center that was on screen is
/// adopted and left there; a center this call opened is closed on the way
/// out, and the close is checked by reading the center back.
pub(crate) struct Session<'a> {
    world: &'a dyn DuxWorld,
    opened: bool,
}

fn closed_without_permission(policy: InteractionPolicy) -> AdapterError {
    AdapterError::policy_denied_for_policy(
        "The notification center is closed, and showing it takes the keyboard focus",
        policy,
    )
    .with_suggestion(
        "Pass --headed to let the command show the center, or open it by hand and retry",
    )
}

fn never_shown(reply: &str) -> AdapterError {
    AdapterError::new(
        ErrorCode::WindowNotFound,
        "The shell accepted the request but no notification center appeared",
    )
    .with_suggestion("The shell may have routed the center to a surface this tool cannot read; open it by hand and retry")
    .with_details(json!({ "ipc_reply": reply }))
}

/// Whether the center reaches `open` inside `window`. The shell builds the
/// center's tree up and tears it down across several readings, so a reading
/// that fails mid-change is taken again; only the last one's error stands.
fn wait_for(
    world: &dyn DuxWorld,
    deadline: Deadline,
    window: Duration,
    open: bool,
) -> Result<bool, AdapterError> {
    let started = Instant::now();
    loop {
        let read = world.center(deadline);
        if read.as_ref().is_ok_and(|center| center.is_some() == open) {
            return Ok(true);
        }
        if started.elapsed() >= window || deadline.is_expired() {
            return read.map(|_| false);
        }
        std::thread::sleep(POLL);
    }
}

impl<'a> Session<'a> {
    pub(crate) fn begin(
        world: &'a dyn DuxWorld,
        deadline: Deadline,
        policy: InteractionPolicy,
    ) -> Result<Self, AdapterError> {
        if world.center(deadline)?.is_some() {
            return Ok(Self {
                world,
                opened: false,
            });
        }
        if !policy.allow_focus_steal {
            return Err(closed_without_permission(policy));
        }
        let reply = world.open(deadline)?;
        let session = Self {
            world,
            opened: true,
        };
        if wait_for(world, deadline, OPEN_SETTLE, true)? {
            return Ok(session);
        }
        let restore = session.restore();
        Err(with_restore(never_shown(&reply), restore))
    }

    /// Closes a center this call opened. The close runs on a deadline of its
    /// own: the call's deadline may be spent when it waited out its budget.
    pub(crate) fn restore(&self) -> Restore {
        if !self.opened {
            return Restore::Adopted;
        }
        let Ok(deadline) = Deadline::detached_after(CLEANUP_MS) else {
            return Restore::Unverified;
        };
        if self.world.close(deadline).is_err() {
            return Restore::Unverified;
        }
        match wait_for(self.world, deadline, CLOSE_SETTLE, false) {
            Ok(true) => Restore::Closed,
            _ => Restore::Unverified,
        }
    }

    /// Ends the call: the result stands when the center was left as found,
    /// and an unrestored center turns a success into a reported failure.
    pub(crate) fn finish<T>(
        self,
        result: Result<T, AdapterError>,
        undone: Restore,
    ) -> Result<T, AdapterError> {
        let restore = self.restore().merge(undone);
        match (result, restore) {
            (Ok(value), Restore::Adopted | Restore::Closed) => Ok(value),
            (Ok(_), Restore::Unverified) => Err(with_restore(
                AdapterError::new(
                    ErrorCode::ActionFailed,
                    "The call finished but the notification center it changed could not be restored",
                )
                .with_suggestion("Close the notification center by hand"),
                restore,
            )),
            (Err(error), restore) => Err(with_restore(error, restore)),
        }
    }
}

/// Runs `body` against the center: shown if it was closed, with the groups
/// the body expanded collapsed again afterwards, and closed again if this
/// call opened it.
pub(crate) fn with_center<T>(
    world: &dyn DuxWorld,
    deadline: Deadline,
    policy: InteractionPolicy,
    body: impl FnOnce(&mut Changes) -> Result<T, AdapterError>,
) -> Result<T, AdapterError> {
    let session = Session::begin(world, deadline, policy)?;
    let mut changes = Changes::default();
    let result = body(&mut changes);
    let undone = undo(world, &changes);
    session.finish(result, undone)
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
