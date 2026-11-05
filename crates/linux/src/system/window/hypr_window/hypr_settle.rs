use std::time::{Duration, Instant};

use agent_desktop_core::{AdapterError, WindowInfo};
use serde_json::Value;

use super::hypr_window_op::{delivered, mismatch, vanished};
use super::hypr_world::HyprWorld;
use crate::system::hyprland::hypr_client::HyprClient;
use crate::system::window::window_op::window_op_verdict::POSITION_TOLERANCE;

const SETTLE: Duration = Duration::from_millis(1_000);
const POLL: Duration = Duration::from_millis(25);
const QUIET: Duration = Duration::from_millis(100);

fn read(
    world: &impl HyprWorld,
    win: &WindowInfo,
    address: u64,
) -> Result<HyprClient, AdapterError> {
    let clients = world.clients().map_err(delivered)?;
    match clients.into_iter().find(|client| client.address == address) {
        Some(client) if client.owner.pid == win.pid.get() => Ok(client),
        _ => Err(vanished()),
    }
}

/// Reads the client at `address` until `holds` accepts it or the settle
/// budget ends, then checks the client still belongs to the window's process.
pub(super) fn settle(
    world: &impl HyprWorld,
    win: &WindowInfo,
    address: u64,
    asked: &Value,
    holds: impl Fn(&HyprClient) -> bool,
) -> Result<HyprClient, AdapterError> {
    let limit = world.deadline().capped(SETTLE);
    loop {
        let client = read(world, win, address)?;
        if holds(&client) {
            return Ok(client);
        }
        if limit.is_expired() {
            return Err(mismatch(asked, &client));
        }
        std::thread::sleep(POLL.min(limit.remaining()));
    }
}

/// Like [`settle`], but a client may still answer the compositor's
/// configure with its own minimum size after the frame first reads as
/// asked. The frame counts only once it has read as asked, without a miss,
/// for a full quiet period, all within one settle budget that a drift does
/// not renew. A budget that ends first, whatever the last reads showed, ends
/// in the mismatch with the latest observation.
pub(super) fn settle_steady(
    world: &impl HyprWorld,
    win: &WindowInfo,
    address: u64,
    asked: &Value,
    holds: impl Fn(&HyprClient) -> bool,
) -> Result<HyprClient, AdapterError> {
    let limit = world.deadline().capped(SETTLE);
    let mut holding_since: Option<Instant> = None;
    loop {
        let client = read(world, win, address)?;
        if holds(&client) {
            let since = *holding_since.get_or_insert_with(Instant::now);
            if since.elapsed() >= QUIET {
                return Ok(client);
            }
        } else {
            holding_since = None;
        }
        if limit.is_expired() {
            return Err(mismatch(asked, &client));
        }
        std::thread::sleep(POLL.min(limit.remaining()));
    }
}

/// Whether a measured coordinate is within the position tolerance of the
/// requested one.
pub(super) fn near(wanted: i64, seen: f64) -> bool {
    (wanted as f64 - seen).abs() <= f64::from(POSITION_TOLERANCE)
}

#[cfg(test)]
#[path = "hypr_settle_tests.rs"]
mod tests;
