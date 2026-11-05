use agent_desktop_core::Rect;

use super::close_id;
use crate::system::hyprland::hypr_client::HyprClient;
use crate::system::hyprland::hypr_client_owner::HyprClientOwner;
use crate::system::hyprland::hypr_client_state::HyprClientState;

fn with_stable_id(stable_id: &str) -> HyprClient {
    HyprClient {
        address: 0x5643_f6b9_1d70,
        stable_id: stable_id.into(),
        owner: HyprClientOwner {
            pid: 7,
            class: "app".into(),
            title: "Doc".into(),
            xwayland: false,
        },
        frame: Rect {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
        },
        state: HyprClientState {
            workspace: "1".into(),
            monitor: 0,
            floating: false,
            fullscreen: 0,
            mapped: true,
            hidden: false,
            pinned: false,
        },
        focus_history: 0,
    }
}

#[test]
fn the_hexadecimal_stable_id_names_the_close_target() {
    assert_eq!(close_id(&with_stable_id("18000002")), Some(0x1800_0002));
}

#[test]
fn a_stable_id_wider_than_32_bits_or_not_hexadecimal_names_none() {
    assert_eq!(close_id(&with_stable_id("1180000002")), None);
    assert_eq!(close_id(&with_stable_id("window")), None);
}
