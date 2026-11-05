use agent_desktop_core::Rect;

use super::from_hyprland;
use crate::system::hyprland::hypr_monitor::HyprMonitor;

fn monitor(id: i64, name: &str, x: f64, y: f64, scale: f64) -> HyprMonitor {
    HyprMonitor {
        id,
        name: name.to_owned(),
        logical: Rect {
            x,
            y,
            width: 1920.0 / scale,
            height: 1080.0 / scale,
        },
        scale,
        focused: false,
        active_workspace: "1".to_owned(),
        special_workspace: None,
    }
}

#[test]
fn the_monitor_at_the_layout_origin_is_primary_and_sorts_first() {
    let displays = from_hyprland(vec![
        monitor(0, "HDMI-A-2", 1920.0, 180.0, 1.0),
        monitor(1, "DP-5", 0.0, 0.0, 1.5),
    ]);
    assert_eq!(displays[0].id, "DP-5");
    assert!(displays[0].is_primary);
    assert_eq!(displays[0].bounds.width, 1280.0);
    assert_eq!(displays[0].scale, 1.5);
    assert!(!displays[1].is_primary);
}

#[test]
fn without_a_monitor_at_the_origin_the_lowest_id_is_primary() {
    let displays = from_hyprland(vec![
        monitor(3, "DP-1", 100.0, 0.0, 1.0),
        monitor(2, "DP-2", 2020.0, 0.0, 1.0),
    ]);
    assert_eq!(displays[0].id, "DP-2");
    assert!(displays[0].is_primary);
    assert_eq!(
        displays.iter().filter(|display| display.is_primary).count(),
        1
    );
}
