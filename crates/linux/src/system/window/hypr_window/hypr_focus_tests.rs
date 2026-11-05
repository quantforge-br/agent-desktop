use agent_desktop_core::{Deadline, DeliverySemantics, ErrorCode};

use super::super::hypr_fake::{FakeHypr, client, rect, window};
use super::super::hypr_window_op::MINIMIZED_WORKSPACE;
use super::focus_window;
use crate::system::hyprland::hypr_command::HyprCommand;

#[test]
fn a_minimized_window_is_brought_back_and_focused() {
    let mut parked = client(0x20, "Doc", rect(0.0, 0.0, 400.0, 300.0));
    parked.state.workspace = MINIMIZED_WORKSPACE.into();
    let world = FakeHypr::with(vec![parked]);
    focus_window(&world, &window("Doc")).unwrap();
    assert_eq!(world.only().state.workspace, "3");
    assert_eq!(world.active.get(), Some(0x20));
}

#[test]
fn an_already_focused_window_sends_nothing() {
    let world = FakeHypr::with(vec![client(0x20, "Doc", rect(0.0, 0.0, 4.0, 3.0))]);
    world.active.set(Some(0x20));
    focus_window(&world, &window("Doc")).unwrap();
    assert!(world.sent.borrow().is_empty());
}

#[test]
fn focus_that_hyprland_accepts_but_does_not_take_is_reported_unverified() {
    let mut world = FakeHypr::with(vec![client(0x20, "Doc", rect(0.0, 0.0, 4.0, 3.0))]);
    world.ignored = vec![HyprCommand::Focus];
    world.active.set(Some(0x99));
    world.deadline = Deadline::detached_after(150).unwrap();
    let error = focus_window(&world, &window("Doc")).unwrap_err();
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
    assert_eq!(error.details.unwrap()["cause"], "focus_not_taken");
}
