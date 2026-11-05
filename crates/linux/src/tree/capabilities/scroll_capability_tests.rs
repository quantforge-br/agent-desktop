use super::{eligible, has_wheel_leg};
use crate::system::display::display_cause::Cause;
use crate::system::display::display_server::DisplayServer;
use crate::system::display::display_unavailable::Unavailable;
use crate::tree::properties::node_read::COMPONENT_INTERFACE;
use crate::tree::vocabulary::role_numbers::{
    DOCUMENT_FRAME, DOCUMENT_WEB, LIST, PANEL, PUSH_BUTTON, SCROLL_PANE, VIEWPORT,
};

fn x11() -> DisplayServer {
    DisplayServer::X11(":0".to_owned())
}

fn component() -> Vec<String> {
    vec![COMPONENT_INTERFACE.to_owned()]
}

#[test]
fn wheel_eligibility_refuses_unsupported_routes() {
    let component = component();
    for role in [SCROLL_PANE, VIEWPORT, LIST, DOCUMENT_FRAME, DOCUMENT_WEB] {
        assert!(eligible(role, &component, &x11()), "role {role}");
    }
    for role in [PUSH_BUTTON, PANEL] {
        assert!(!eligible(role, &component, &x11()), "role {role}");
    }
    assert!(!eligible(SCROLL_PANE, &[], &x11()));
    assert!(!eligible(
        SCROLL_PANE,
        &["org.a11y.atspi.Action".to_owned()],
        &x11()
    ));
    assert!(!eligible(SCROLL_PANE, &component, &DisplayServer::Wayland));
    let unavailable = DisplayServer::Unavailable(Unavailable {
        cause: Cause::NotConfigured,
        detail: String::new(),
    });
    assert!(!eligible(SCROLL_PANE, &component, &unavailable));
}

#[test]
fn a_wayland_session_has_a_wheel_leg_only_through_a_virtual_pointer() {
    assert!(has_wheel_leg(&DisplayServer::Wayland, || true));
    assert!(!has_wheel_leg(&DisplayServer::Wayland, || false));
    assert!(has_wheel_leg(&x11(), || panic!(
        "X11 needs no virtual pointer"
    )));
    let unavailable = DisplayServer::Unavailable(Unavailable {
        cause: Cause::NotConfigured,
        detail: String::new(),
    });
    assert!(!has_wheel_leg(&unavailable, || true));
}
