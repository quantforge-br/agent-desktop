use agent_desktop_core::Rect;

use super::HyprScene;
use crate::system::hyprland::hypr_client::HyprClient;
use crate::system::hyprland::hypr_client_owner::HyprClientOwner;
use crate::system::hyprland::hypr_client_state::HyprClientState;
use crate::system::hyprland::hypr_layer::HyprLayer;
use crate::system::hyprland::hypr_monitor::HyprMonitor;
use crate::tree::hit_test::hit_occlusion::{Occlusion, occlusion};

fn rect(x: f64, y: f64, width: f64, height: f64) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn client(id: u32, workspace: &str, floating: bool, frame: Rect) -> HyprClient {
    HyprClient {
        address: u64::from(id),
        stable_id: format!("{id:x}"),
        owner: HyprClientOwner {
            pid: id,
            class: "app".into(),
            title: format!("w{id}"),
            xwayland: false,
        },
        frame,
        state: HyprClientState {
            workspace: workspace.into(),
            monitor: 0,
            floating,
            fullscreen: 0,
            mapped: true,
            hidden: false,
            pinned: false,
        },
        focus_history: 0,
    }
}

fn monitors() -> Vec<HyprMonitor> {
    vec![HyprMonitor {
        id: 0,
        name: "DP-1".into(),
        logical: rect(0.0, 0.0, 1920.0, 1080.0),
        scale: 1.0,
        focused: true,
        active_workspace: "1".into(),
        special_workspace: None,
    }]
}

fn bar() -> HyprLayer {
    HyprLayer {
        monitor: "DP-1".into(),
        level: HyprLayer::TOP,
        namespace: "dms:bar".into(),
        pid: 99,
        rect: rect(0.0, 0.0, 1920.0, 40.0),
    }
}

fn verdict(clients: &[HyprClient], layers: &[HyprLayer], point: (i32, i32)) -> Occlusion {
    let monitors = monitors();
    let scene = HyprScene {
        clients,
        monitors: &monitors,
        layers,
    };
    let (id, snapshot) = scene.stacking(&clients[0]).unwrap();
    occlusion(&snapshot, id, point)
}

#[test]
fn a_floating_window_listed_before_a_tiled_target_still_covers_it() {
    let target = client(0x10, "1", false, rect(0.0, 40.0, 1920.0, 1040.0));
    let dialog = client(0x11, "1", true, rect(100.0, 100.0, 300.0, 200.0));
    let clients = [target, dialog];
    assert!(matches!(
        verdict(&clients, &[], (150, 150)),
        Occlusion::CoveredBy { window: 0x11, .. }
    ));
    assert_eq!(verdict(&clients, &[], (900, 900)), Occlusion::Clear);
}

#[test]
fn a_window_on_a_workspace_no_monitor_shows_covers_nothing() {
    let target = client(0x10, "1", true, rect(0.0, 40.0, 800.0, 600.0));
    let elsewhere = client(0x11, "7", true, rect(0.0, 40.0, 800.0, 600.0));
    assert_eq!(
        verdict(&[target, elsewhere], &[], (10, 50)),
        Occlusion::Clear
    );
}

#[test]
fn a_bar_takes_the_pointer_unless_the_target_is_fullscreen_beneath_it() {
    let target = client(0x10, "1", true, rect(0.0, 0.0, 800.0, 600.0));
    assert!(matches!(
        verdict(std::slice::from_ref(&target), &[bar()], (10, 10)),
        Occlusion::CoveredBy { .. }
    ));
    let mut fullscreen = target;
    fullscreen.state.fullscreen = 2;
    assert_eq!(verdict(&[fullscreen], &[bar()], (10, 10)), Occlusion::Clear);
}

#[test]
fn a_target_no_monitor_shows_is_not_judged() {
    let parked = client(
        0x10,
        "special:minimized",
        true,
        rect(0.0, 0.0, 800.0, 600.0),
    );
    assert_eq!(verdict(&[parked], &[], (10, 10)), Occlusion::Unknown);
}
