use agent_desktop_core::Rect;

use super::super::hypr_fake::{FakeHypr, PID, client, rect, window};
use super::super::hypr_window_op::MINIMIZED_WORKSPACE;
use super::HyprView;
use crate::system::hyprland::hypr_layer::HyprLayer;
use crate::system::registry::listed_window::ListedWindow;
use crate::system::x11::FrameFacts;
use crate::tree::Extents;

fn listed(title: &str, focused: bool) -> ListedWindow {
    let mut info = window(title);
    info.state.is_focused = focused;
    ListedWindow {
        app: 0,
        info,
        active: focused,
        modal: false,
        dialog: false,
        frame: FrameFacts {
            pid: 0,
            title: title.into(),
            extents: None,
            global_position: false,
            size_identity: false,
        },
        sheet: false,
        unshown: false,
    }
}

fn view(clients: Vec<crate::system::hyprland::hypr_client::HyprClient>, active: u64) -> HyprView {
    let fake = FakeHypr::with(clients);
    HyprView {
        clients: fake.clients.into_inner(),
        layers: Vec::new(),
        monitors: fake.monitors,
        active: Some(active),
    }
}

#[test]
fn the_compositor_answers_place_focus_and_visibility() {
    let mut parked = client(0x2, "Mail", rect(10.0, 10.0, 300.0, 200.0));
    parked.state.workspace = MINIMIZED_WORKSPACE.into();
    let mut elsewhere = client(0x3, "Chat", rect(0.0, 0.0, 300.0, 200.0));
    elsewhere.state.workspace = "9".into();
    let mut shown = client(0x1, "Doc", rect(40.0, 60.0, 800.0, 600.0));
    shown.state.workspace = "3".into();
    let view = view(vec![shown, parked, elsewhere], 0x1);
    let mut windows = vec![
        listed("Mail", true),
        listed("Doc", true),
        listed("Chat", false),
    ];
    view.annotate(&mut windows);
    let [mail, doc, chat] = &windows[..] else {
        panic!("three windows");
    };
    assert_eq!(doc.info.bounds, Some(rect(40.0, 60.0, 800.0, 600.0)));
    assert!(doc.info.state.is_focused && !mail.info.state.is_focused);
    assert_eq!(doc.info.state.visible, Some(true));
    assert_eq!(mail.info.state.minimized, Some(true));
    assert_eq!(mail.info.state.visible, Some(false));
    assert_eq!(chat.info.state.minimized, Some(false));
    assert_eq!(chat.info.state.visible, Some(false));
    assert_eq!(HyprView::focused(&windows), Some(1));
}

#[test]
fn a_window_without_a_single_client_is_left_unplaced_and_unfocused() {
    let twins = vec![
        client(0x1, "Doc", rect(0.0, 0.0, 10.0, 10.0)),
        client(0x2, "Doc", rect(50.0, 0.0, 10.0, 10.0)),
    ];
    let view = view(twins, 0x1);
    let mut windows = vec![listed("Doc", true)];
    view.annotate(&mut windows);
    assert_eq!(windows[0].info.bounds, None);
    assert!(!windows[0].info.state.is_focused);
}

#[test]
fn the_active_client_is_not_focused_while_its_frame_has_lost_active() {
    let view = view(vec![client(0x1, "Doc", rect(0.0, 0.0, 10.0, 10.0))], 0x1);
    let mut windows = vec![listed("Doc", false)];
    view.annotate(&mut windows);
    assert_eq!(windows[0].info.bounds, Some(rect(0.0, 0.0, 10.0, 10.0)));
    assert_eq!(HyprView::focused(&windows), None);
}

#[test]
fn two_same_titled_windows_are_told_apart_by_the_active_frame() {
    let twins = || {
        vec![
            client(0x1, "Doc", rect(0.0, 0.0, 10.0, 10.0)),
            client(0x2, "Doc", rect(50.0, 0.0, 10.0, 10.0)),
        ]
    };
    let mut windows = vec![listed("Doc", false), listed("Doc", true)];
    view(twins(), 0x2).annotate(&mut windows);
    assert_eq!(windows[0].info.bounds, Some(rect(0.0, 0.0, 10.0, 10.0)));
    assert_eq!(windows[1].info.bounds, Some(rect(50.0, 0.0, 10.0, 10.0)));
    assert_eq!(HyprView::focused(&windows), Some(1));
    let mut unproven = vec![listed("Doc", false), listed("Doc", false)];
    view(twins(), 0x2).annotate(&mut unproven);
    assert!(unproven.iter().all(|window| window.info.bounds.is_none()));
}

#[test]
fn shell_frames_beside_its_one_toplevel_take_neither_its_place_nor_its_focus() {
    let view = view(
        vec![client(0x1, "What's New", rect(280.0, 62.0, 720.0, 720.0))],
        0x1,
    );
    let mut windows = vec![
        listed("", false),
        listed("What's New", true),
        listed("", false),
    ];
    view.annotate(&mut windows);
    assert_eq!(HyprView::focused(&windows), Some(1));
    assert_eq!(
        windows[1].info.bounds,
        Some(rect(280.0, 62.0, 720.0, 720.0))
    );
    for frame in [&windows[0], &windows[2]] {
        assert!(!frame.info.state.is_focused);
        assert_eq!(frame.info.bounds, None);
    }
}

#[test]
fn a_lone_window_takes_its_process_single_client_whatever_its_title() {
    let view = view(
        vec![client(0x1, "Notes - draft", rect(0.0, 0.0, 300.0, 200.0))],
        0x1,
    );
    let mut windows = vec![listed("Notes", true)];
    view.annotate(&mut windows);
    assert_eq!(HyprView::focused(&windows), Some(0));
    assert_eq!(windows[0].info.bounds, Some(rect(0.0, 0.0, 300.0, 200.0)));
}

#[test]
fn a_process_is_hidden_only_when_no_window_of_it_is_shown() {
    let mut parked = client(0x2, "Mail", rect(10.0, 10.0, 300.0, 200.0));
    parked.state.workspace = MINIMIZED_WORKSPACE.into();
    let mut shown = client(0x1, "Doc", rect(40.0, 60.0, 800.0, 600.0));
    shown.state.workspace = "3".into();
    let pid = parked.owner.pid;
    assert!(view(vec![parked.clone()], 0x2).hides_every_window_of(pid));
    assert!(!view(vec![parked, shown], 0x2).hides_every_window_of(pid));
    assert!(!view(Vec::new(), 0x2).hides_every_window_of(pid));
}

fn shell_frame(width: i32, height: i32) -> ListedWindow {
    let mut frame = listed("", false);
    frame.frame.extents = Some(Extents {
        x: 0,
        y: 0,
        width,
        height,
    });
    frame
}

fn layer(namespace: &str, rect: Rect) -> HyprLayer {
    HyprLayer {
        monitor: "DP-1".into(),
        level: HyprLayer::TOP,
        namespace: namespace.into(),
        pid: PID,
        rect,
    }
}

#[test]
fn a_dock_with_no_client_is_placed_and_named_by_its_layer_not_by_its_window_relative_origin() {
    let mut view = view(Vec::new(), 0x1);
    view.layers = vec![
        layer("dms:dock", rect(0.0, 726.0, 1280.0, 74.0)),
        layer("dms:bar", rect(0.0, 0.0, 1280.0, 44.0)),
    ];
    let mut windows = vec![shell_frame(1280, 74), shell_frame(1280, 44)];
    view.annotate(&mut windows);
    assert_eq!(windows[0].info.bounds, Some(rect(0.0, 726.0, 1280.0, 74.0)));
    assert_eq!(windows[0].info.title, "Dock");
    assert_eq!(windows[1].info.bounds, Some(rect(0.0, 0.0, 1280.0, 44.0)));
    assert_eq!(windows[1].info.title, "Top bar");
}

#[test]
fn a_layer_two_frames_could_both_be_is_not_guessed() {
    let mut view = view(Vec::new(), 0x1);
    view.layers = vec![layer("dms:dock", rect(0.0, 726.0, 1280.0, 74.0))];
    let mut windows = vec![shell_frame(1280, 74), shell_frame(1280, 74)];
    view.annotate(&mut windows);
    assert!(windows.iter().all(|window| window.info.bounds.is_none()));
    assert!(windows.iter().all(|window| window.info.title.is_empty()));
}

#[test]
fn a_sibling_frame_with_no_readable_size_leaves_the_layer_unclaimed() {
    let mut view = view(Vec::new(), 0x1);
    view.layers = vec![layer("dms:dock", rect(0.0, 726.0, 1280.0, 74.0))];
    let mut unread = shell_frame(1280, 74);
    unread.frame.extents = None;
    let mut windows = vec![shell_frame(1280, 74), unread];
    view.annotate(&mut windows);
    assert!(windows[0].info.bounds.is_none());
}

#[test]
fn a_layer_the_shell_does_not_name_places_its_window_but_leaves_the_title() {
    let mut view = view(Vec::new(), 0x1);
    view.layers = vec![layer("quickshell", rect(0.0, 0.0, 1280.0, 800.0))];
    let mut windows = vec![shell_frame(1280, 800)];
    view.annotate(&mut windows);
    assert_eq!(windows[0].info.bounds, Some(rect(0.0, 0.0, 1280.0, 800.0)));
    assert!(windows[0].info.title.is_empty());
}
