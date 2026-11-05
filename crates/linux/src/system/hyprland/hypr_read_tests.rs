use super::{parse_active, parse_address, parse_clients, parse_cursor, parse_monitors};

const MONITORS: &str = r#"[
  {"id": 1, "name": "DP-5", "width": 3440, "height": 1440, "x": 0, "y": 0, "scale": 1.25,
   "transform": 0, "focused": true,
   "activeWorkspace": {"id": 1, "name": "1"}, "specialWorkspace": {"id": 0, "name": ""}},
  {"id": 0, "name": "HDMI-A-2", "width": 1920, "height": 1080, "x": 2752, "y": 180, "scale": 1.0,
   "transform": 1, "focused": false,
   "activeWorkspace": {"id": 10, "name": "10"}, "specialWorkspace": {"id": -98, "name": "special:claude"}}
]"#;

const CLIENTS: &str = r#"[
  {"address": "0x5643f6b91d70", "mapped": true, "hidden": false, "at": [495, 185], "size": [500, 600],
   "workspace": {"id": 1, "name": "1"}, "floating": true, "monitor": 1,
   "class": "org.gnome.Calculator", "title": "Calculadora", "pid": 1273229, "xwayland": false,
   "pinned": false, "fullscreen": 1, "focusHistoryID": 0, "stableId": "18000002"}
]"#;

#[test]
fn monitor_bounds_are_logical_after_scale_and_rotation() {
    let monitors = parse_monitors(MONITORS).unwrap();
    assert_eq!(monitors[0].logical.width, 2752.0);
    assert_eq!(monitors[0].logical.height, 1152.0);
    assert_eq!(monitors[1].logical.width, 1080.0);
    assert_eq!(monitors[1].logical.height, 1920.0);
    assert_eq!(monitors[1].logical.x, 2752.0);
}

#[test]
fn a_monitor_shows_its_active_and_its_open_special_workspace_only() {
    let monitors = parse_monitors(MONITORS).unwrap();
    assert_eq!(monitors[0].special_workspace, None);
    assert!(monitors[1].shows("10"));
    assert!(monitors[1].shows("special:claude"));
    assert!(!monitors[1].shows("1"));
}

#[test]
fn a_client_keeps_its_address_frame_and_layout_state() {
    let clients = parse_clients(CLIENTS).unwrap();
    let client = &clients[0];
    assert_eq!(client.address, 0x5643_f6b9_1d70);
    assert_eq!(client.owner.pid, 1_273_229);
    assert_eq!((client.frame.x, client.frame.width), (495.0, 500.0));
    assert_eq!(client.state.fullscreen, 1);
    assert!(client.state.floating);
    assert_eq!(client.stable_id, "18000002");
}

#[test]
fn a_client_record_without_an_address_fails_the_whole_read() {
    let reply = r#"[{"at": [0, 0], "size": [1, 1], "pid": 1}]"#;
    assert!(parse_clients(reply).is_err());
    assert!(parse_clients("not json").is_err());
}

#[test]
fn an_empty_active_window_reply_means_nothing_is_focused() {
    assert_eq!(parse_active("{}").unwrap(), None);
    assert_eq!(parse_active(r#"{"address": "0x10"}"#).unwrap(), Some(0x10));
    assert!(parse_active(r#"{"address": "10"}"#).is_err());
}

#[test]
fn addresses_need_the_hex_prefix() {
    assert_eq!(parse_address("0xff"), Some(255));
    assert_eq!(parse_address("ff"), None);
}

#[test]
fn the_cursor_position_is_read_in_layout_coordinates() {
    let point = parse_cursor(r#"{"x": 745, "y": 485}"#).unwrap();
    assert_eq!((point.x, point.y), (745.0, 485.0));
    assert!(parse_cursor(r#"{"x": 1}"#).is_err());
}
