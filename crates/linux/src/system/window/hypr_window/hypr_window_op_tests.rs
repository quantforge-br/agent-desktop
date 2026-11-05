use agent_desktop_core::{Deadline, DeliverySemantics, ErrorCode, WindowOp};

use super::super::hypr_fake::{FakeHypr, client, rect, window};
use super::{MINIMIZED_WORKSPACE, window_op};
use crate::system::hyprland::hypr_command::HyprCommand;

fn floating(title: &str) -> FakeHypr {
    let mut one = client(0x10, title, rect(200.0, 50.0, 500.0, 600.0));
    one.state.floating = true;
    FakeHypr::with(vec![one])
}

#[test]
fn a_resize_keeps_the_window_where_it_was_although_hyprland_keeps_its_centre() {
    let world = floating("Doc");
    let report = window_op(
        &world,
        &window("Doc"),
        &WindowOp::Resize {
            width: 300.0,
            height: 400.0,
        },
    )
    .unwrap();
    assert_eq!(world.only().frame, rect(200.0, 50.0, 300.0, 400.0));
    assert_eq!(
        report.disposition(),
        DeliverySemantics::delivered_verified()
    );
    let observed = &report.result.details.as_ref().unwrap()["observed"];
    assert_eq!(observed["width"], 300.0);
    assert_eq!(observed["x"], 200.0);
}

#[test]
fn a_tiled_or_maximized_window_is_released_to_floating_before_it_moves() {
    let mut tiled = client(0x10, "Doc", rect(0.0, 0.0, 1920.0, 1080.0));
    tiled.state.fullscreen = 1;
    let world = FakeHypr::with(vec![tiled]);
    window_op(&world, &window("Doc"), &WindowOp::Move { x: 40.0, y: 60.0 }).unwrap();
    assert_eq!(
        *world.sent.borrow(),
        vec![
            HyprCommand::Maximize(false),
            HyprCommand::Float(true),
            HyprCommand::MoveTo { x: 40, y: 60 },
        ]
    );
    let moved = world.only();
    assert_eq!((moved.frame.x, moved.frame.y), (40.0, 60.0));
    assert_eq!(moved.state.fullscreen, 0);
}

#[test]
fn minimize_parks_the_window_and_restore_returns_it_to_the_shown_workspace() {
    let world = floating("Doc");
    window_op(&world, &window("Doc"), &WindowOp::Minimize).unwrap();
    assert_eq!(world.only().state.workspace, MINIMIZED_WORKSPACE);
    window_op(&world, &window("Doc"), &WindowOp::Restore).unwrap();
    assert_eq!(world.only().state.workspace, "3");
    assert_eq!(world.active.get(), Some(0x10));
}

#[test]
fn maximize_is_verified_and_restore_clears_it() {
    let world = floating("Doc");
    let report = window_op(&world, &window("Doc"), &WindowOp::Maximize).unwrap();
    assert_eq!(
        report.disposition(),
        DeliverySemantics::delivered_verified()
    );
    assert_eq!(world.only().state.fullscreen, 1);
    window_op(&world, &window("Doc"), &WindowOp::Restore).unwrap();
    assert_eq!(world.only().state.fullscreen, 0);
}

#[test]
fn an_accepted_move_that_changes_nothing_is_a_mismatch_with_what_was_seen() {
    let mut world = floating("Doc");
    world.ignored = vec![HyprCommand::MoveTo { x: 900, y: 10 }];
    world.deadline = Deadline::detached_after(150).unwrap();
    let error = window_op(
        &world,
        &window("Doc"),
        &WindowOp::Move { x: 900.0, y: 10.0 },
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
    let details = error.details.unwrap();
    assert_eq!(details["cause"], "readback_mismatch");
    assert_eq!(details["observed"]["x"], 200.0);
    assert_eq!(details["requested"]["x"], 900);
}

#[test]
fn a_window_of_another_process_instance_is_stale_and_nothing_is_sent() {
    let mut world = floating("Doc");
    world.instance = "boot:2";
    let error = window_op(&world, &window("Doc"), &WindowOp::Minimize).unwrap_err();
    assert_eq!(error.code, ErrorCode::StaleRef);
    assert!(world.sent.borrow().is_empty());
}

#[test]
fn a_window_without_a_client_is_not_found_before_anything_is_sent() {
    let world = floating("Other");
    world.clients.borrow_mut()[0].owner.pid = 1;
    let error = window_op(&world, &window("Doc"), &WindowOp::Maximize).unwrap_err();
    assert_eq!(error.code, ErrorCode::WindowNotFound);
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    assert!(world.sent.borrow().is_empty());
}

#[test]
fn restoring_a_maximized_tile_leaves_it_tiled() {
    let mut tiled = client(0x10, "Doc", rect(0.0, 0.0, 1920.0, 1080.0));
    tiled.state.fullscreen = 1;
    let world = FakeHypr::with(vec![tiled]);
    window_op(&world, &window("Doc"), &WindowOp::Restore).unwrap();
    assert_eq!(*world.sent.borrow(), vec![HyprCommand::Maximize(false)]);
    let restored = world.only();
    assert_eq!(restored.state.fullscreen, 0);
    assert!(!restored.state.floating);
}

#[test]
fn a_move_onto_another_monitor_joins_the_workspace_that_monitor_shows() {
    let mut world = floating("Doc");
    let mut right = world.monitors[0].clone();
    right.id = 1;
    right.name = "HDMI-1".into();
    right.logical = rect(1920.0, 0.0, 1920.0, 1080.0);
    right.focused = false;
    right.active_workspace = "5".into();
    world.monitors.push(right);
    window_op(
        &world,
        &window("Doc"),
        &WindowOp::Move {
            x: 2000.0,
            y: 100.0,
        },
    )
    .unwrap();
    assert_eq!(
        *world.sent.borrow(),
        vec![
            HyprCommand::ToWorkspace("5".into()),
            HyprCommand::MoveTo { x: 2000, y: 100 },
        ]
    );
    assert_eq!(world.only().state.workspace, "5");
    world.sent.borrow_mut().clear();
    window_op(
        &world,
        &window("Doc"),
        &WindowOp::Move {
            x: 2100.0,
            y: 100.0,
        },
    )
    .unwrap();
    assert_eq!(
        *world.sent.borrow(),
        vec![HyprCommand::MoveTo { x: 2100, y: 100 }]
    );
}
