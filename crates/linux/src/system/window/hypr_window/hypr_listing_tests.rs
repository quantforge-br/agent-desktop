use super::super::hypr_fake::{FakeHypr, client, rect, window};
use super::super::hypr_window_op::MINIMIZED_WORKSPACE;
use super::HyprView;
use crate::system::registry::listed_window::ListedWindow;
use crate::system::x11::FrameFacts;

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
        listed("Doc", false),
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
