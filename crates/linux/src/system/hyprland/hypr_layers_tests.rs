use agent_desktop_core::Rect;

use super::parse_layers;
use crate::system::hyprland::hypr_layer::HyprLayer;

const LAYERS: &str = r#"{
  "DP-5": {"levels": {
    "0": [{"address":"0x1","x":0,"y":0,"w":3440,"h":1440,"namespace":"quickshell","pid":3050}],
    "1": [],
    "2": [{"address":"0x2","x":0,"y":0,"w":3440,"h":49,"namespace":"dms:bar","pid":3050}],
    "3": []
  }}
}"#;

#[test]
fn every_surface_keeps_its_monitor_level_and_rectangle() {
    let layers = parse_layers(LAYERS).unwrap();
    assert_eq!(layers.len(), 2);
    let bar = layers
        .iter()
        .find(|layer| layer.namespace == "dms:bar")
        .unwrap();
    assert_eq!(bar.level, HyprLayer::TOP);
    assert_eq!(bar.monitor, "DP-5");
    assert_eq!(bar.pid, 3050);
    assert_eq!(
        bar.rect,
        Rect {
            x: 0.0,
            y: 0.0,
            width: 3440.0,
            height: 49.0
        }
    );
}

#[test]
fn a_reply_that_is_not_a_monitor_map_is_refused() {
    assert!(parse_layers("[]").is_err());
    assert!(parse_layers(r#"{"DP-5": {"levels": {"top": []}}}"#).is_err());
}
