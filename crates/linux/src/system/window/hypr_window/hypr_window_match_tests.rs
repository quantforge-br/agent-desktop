use agent_desktop_core::Rect;

use super::super::hypr_unmatched::Unmatched;
use super::super::hypr_window_key::WindowKey;
use super::match_client;
use crate::system::hyprland::hypr_client::HyprClient;
use crate::system::hyprland::hypr_client_owner::HyprClientOwner;
use crate::system::hyprland::hypr_client_state::HyprClientState;

fn client(address: u64, pid: u32, title: &str, x: f64) -> HyprClient {
    HyprClient {
        address,
        stable_id: format!("{address:x}"),
        owner: HyprClientOwner {
            pid,
            class: "app".into(),
            title: title.into(),
            xwayland: false,
        },
        frame: Rect {
            x,
            y: 10.0,
            width: 300.0,
            height: 200.0,
        },
        state: HyprClientState {
            workspace: "1".into(),
            monitor: 0,
            floating: true,
            fullscreen: 0,
            mapped: true,
            hidden: false,
            pinned: false,
        },
        focus_history: 0,
    }
}

fn key<'a>(pid: u32, title: &'a str, bounds: Option<&'a Rect>) -> WindowKey<'a> {
    WindowKey { pid, title, bounds }
}

#[test]
fn the_titled_client_of_the_process_is_the_window() {
    let clients = [client(1, 7, "Notes", 0.0), client(2, 7, "Mail", 0.0)];
    assert_eq!(
        match_client(&clients, key(7, "Mail", None))
            .unwrap()
            .address,
        2
    );
}

#[test]
fn a_single_client_answers_for_its_process_whatever_its_title() {
    let clients = [
        client(1, 7, "Notes - draft", 0.0),
        client(2, 8, "Mail", 0.0),
    ];
    assert_eq!(
        match_client(&clients, key(7, "Notes", None))
            .unwrap()
            .address,
        1
    );
}

#[test]
fn no_client_of_the_process_is_missing_and_unmapped_ones_do_not_count() {
    let mut hidden = client(1, 7, "Notes", 0.0);
    hidden.state.mapped = false;
    let clients = [hidden, client(2, 8, "Notes", 0.0)];
    assert_eq!(
        match_client(&clients, key(7, "Notes", None)),
        Err(Unmatched::Missing)
    );
}

#[test]
fn twins_are_told_apart_by_bounds_or_left_ambiguous() {
    let clients = [client(1, 7, "Notes", 0.0), client(2, 7, "Notes", 500.0)];
    let bounds = Rect {
        x: 501.0,
        y: 10.0,
        width: 300.0,
        height: 200.0,
    };
    assert_eq!(
        match_client(&clients, key(7, "Notes", Some(&bounds)))
            .unwrap()
            .address,
        2
    );
    assert_eq!(
        match_client(&clients, key(7, "Notes", None)),
        Err(Unmatched::Several)
    );
    let elsewhere = Rect { x: 900.0, ..bounds };
    assert_eq!(
        match_client(&clients, key(7, "Notes", Some(&elsewhere))),
        Err(Unmatched::Several)
    );
}

#[test]
fn several_untitled_matches_are_not_resolved_by_the_lone_client_rule() {
    let clients = [client(1, 7, "A", 0.0), client(2, 7, "B", 0.0)];
    assert_eq!(
        match_client(&clients, key(7, "C", None)),
        Err(Unmatched::Missing)
    );
}
