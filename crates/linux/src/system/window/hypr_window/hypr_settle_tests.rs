use std::cell::Cell;
use std::time::{Duration, Instant};

use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode, WindowOp};

use super::super::hypr_fake::{FakeHypr, client, rect, window};
use super::super::hypr_window_op::window_op;
use super::super::hypr_world::HyprWorld;
use crate::system::hyprland::hypr_client::HyprClient;
use crate::system::hyprland::hypr_command::HyprCommand;
use crate::system::hyprland::hypr_monitor::HyprMonitor;
use crate::system::process::process_lookup::Lookup;

/// A compositor whose client answers a resize with its own minimum size
/// only after `lag` further client-list reads, the way a toolkit answers a
/// configure after the compositor has already recorded the request.
struct ClampingHypr {
    inner: FakeHypr,
    minimum: (f64, f64),
    lag: u32,
    reads: Cell<u32>,
    flicker: bool,
}

impl ClampingHypr {
    fn new(minimum: (f64, f64), lag: u32) -> Self {
        let mut one = client(0x10, "Doc", rect(200.0, 50.0, 500.0, 600.0));
        one.state.floating = true;
        let mut inner = FakeHypr::with(vec![one]);
        inner.deadline = Deadline::detached_after(300).unwrap();
        Self {
            inner,
            minimum,
            lag,
            reads: Cell::new(0),
            flicker: false,
        }
    }
}

impl HyprWorld for ClampingHypr {
    fn clients(&self) -> Result<Vec<HyprClient>, AdapterError> {
        let reads = self.reads.get();
        self.reads.set(reads + 1);
        if reads >= self.lag {
            for one in self.inner.clients.borrow_mut().iter_mut() {
                one.frame.width = one.frame.width.max(self.minimum.0);
                one.frame.height = one.frame.height.max(self.minimum.1);
            }
        }
        let mut seen = self.inner.clients()?;
        if self.flicker && reads % 2 == 1 {
            for one in &mut seen {
                one.frame.width += 40.0;
            }
        }
        Ok(seen)
    }

    fn monitors(&self) -> Result<Vec<HyprMonitor>, AdapterError> {
        self.inner.monitors()
    }

    fn active(&self) -> Result<Option<u64>, AdapterError> {
        self.inner.active()
    }

    fn send(&self, address: u64, command: &HyprCommand) -> Result<(), AdapterError> {
        if matches!(command, HyprCommand::ResizeTo { .. }) {
            self.reads.set(0);
        }
        self.inner.send(address, command)
    }

    fn lookup(&self, pid: u32) -> Lookup {
        self.inner.lookup(pid)
    }

    fn stopped(&self, pid: u32) -> bool {
        self.inner.stopped(pid)
    }

    fn deadline(&self) -> Deadline {
        self.inner.deadline()
    }
}

fn resize(world: &ClampingHypr, width: f64, height: f64) -> Result<(), AdapterError> {
    window_op(world, &window("Doc"), &WindowOp::Resize { width, height }).map(|_| ())
}

#[test]
fn a_resize_the_client_clamps_after_the_first_read_is_a_readback_mismatch() {
    let world = ClampingHypr::new((99.0, 69.0), 1);
    let error = resize(&world, 10.0, 10.0).unwrap_err();
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
    let details = error.details.unwrap();
    assert_eq!(details["cause"], "readback_mismatch");
    assert_eq!(details["observed"]["width"], 99.0);
    assert_eq!(details["observed"]["height"], 69.0);
    assert_eq!(details["requested"]["width"], 10);
}

#[test]
fn a_resize_the_client_honours_is_verified_with_the_held_size() {
    let world = ClampingHypr::new((99.0, 69.0), 1);
    let report = resize(&world, 300.0, 400.0);
    assert!(report.is_ok());
    assert_eq!(world.inner.only().frame.width, 300.0);
}

#[test]
fn a_size_that_keeps_flickering_ends_at_one_settle_budget_not_at_the_command_deadline() {
    let mut world = ClampingHypr::new((0.0, 0.0), 0);
    world.flicker = true;
    world.inner.deadline = Deadline::detached_after(5_000).unwrap();
    let started = Instant::now();
    let error = resize(&world, 300.0, 400.0).unwrap_err();
    assert!(started.elapsed() < Duration::from_millis(2_000));
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(error.details.unwrap()["cause"], "readback_mismatch");
}

#[test]
fn a_command_budget_shorter_than_the_quiet_period_never_verifies_a_matching_sample() {
    let mut world = ClampingHypr::new((0.0, 0.0), 0);
    world.inner.deadline = Deadline::detached_after(60).unwrap();
    let error = resize(&world, 300.0, 400.0).unwrap_err();
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
    let details = error.details.unwrap();
    assert_eq!(details["cause"], "readback_mismatch");
    assert_eq!(details["observed"]["width"], 300.0);
}
