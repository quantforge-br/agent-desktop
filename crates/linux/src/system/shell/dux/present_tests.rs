use std::cell::{Cell, RefCell};
use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode, InteractionPolicy};

use super::{Stage, identify, open_with, resolve_with};
use crate::system::shell::dux::frame::Frame;
use crate::system::shell::dux::ipc::Ipc;
use crate::system::shell::dux::layer::Layer;
use crate::system::shell::dux::surface::Surface;
use crate::tree::AtspiElement;
use crate::tree::vocabulary::role_numbers::{FILLER, PUSH_BUTTON, RADIO_BUTTON};

const PID: u32 = 50;

fn layer(namespace: &str, width: i64, height: i64) -> Layer {
    Layer {
        namespace: namespace.to_owned(),
        width,
        height,
        pid: PID,
    }
}

fn frame(id: u32, width: i64, height: i64, roles: Vec<u32>) -> Frame {
    Frame {
        element: AtspiElement::new(":1.2", format!("/org/a11y/atspi/accessible/{id}"), PID),
        width,
        height,
        showing: true,
        children: roles.len(),
        roles,
    }
}

fn tabs() -> Vec<u32> {
    vec![
        RADIO_BUTTON,
        RADIO_BUTTON,
        PUSH_BUTTON,
        PUSH_BUTTON,
        PUSH_BUTTON,
    ]
}

struct Shell {
    layers: RefCell<Vec<Layer>>,
    frames: RefCell<Vec<Frame>>,
    on_open: RefCell<Option<(Layer, Frame)>>,
    offers: bool,
    calls: RefCell<Vec<String>>,
    opened: Cell<bool>,
}

impl Shell {
    fn new(layers: Vec<Layer>, frames: Vec<Frame>) -> Self {
        Self {
            layers: RefCell::new(layers),
            frames: RefCell::new(frames),
            on_open: RefCell::new(None),
            offers: true,
            calls: RefCell::new(Vec::new()),
            opened: Cell::new(false),
        }
    }

    fn opening(self, layer: Layer, frame: Frame) -> Self {
        *self.on_open.borrow_mut() = Some((layer, frame));
        self
    }
}

impl Ipc for Shell {
    fn call(&self, target: &str, function: &str, _: Deadline) -> Result<String, AdapterError> {
        self.calls.borrow_mut().push(format!("{target} {function}"));
        if let Some((layer, frame)) = self.on_open.borrow_mut().take() {
            self.layers.borrow_mut().push(layer);
            self.frames.borrow_mut().push(frame);
            self.opened.set(true);
        }
        Ok("OPEN_SUCCESS".to_owned())
    }

    fn listing(&self, _: Deadline) -> Result<String, AdapterError> {
        if !self.offers {
            return Ok(String::new());
        }
        Ok("target notifications\n  function open(): string\ntarget control-center\n  function open(): string\ntarget launcher\n  function open(): string\ntarget dock\n  function reveal(): string\n".to_owned())
    }
}

impl Stage for Shell {
    fn pid(&self) -> u32 {
        PID
    }

    fn layers(&self, _: Deadline) -> Result<Vec<Layer>, AdapterError> {
        Ok(self.layers.borrow().clone())
    }

    fn frames(&self, _: Deadline) -> Result<(String, Vec<Frame>), AdapterError> {
        Ok(("quickshell".to_owned(), self.frames.borrow().clone()))
    }

    fn ipc(&self) -> &dyn Ipc {
        self
    }

    fn pause(&self, _: Duration) {}
}

fn deadline() -> Deadline {
    Deadline::after(2_000).expect("deadline")
}

#[test]
fn the_window_with_the_mapped_layers_size_and_content_presents_the_surface() {
    let layers = [layer("dms:dock", 1280, 74), layer("quickshell", 1280, 720)];
    let frames = [
        frame(1, 1280, 720, vec![]),
        frame(2, 1280, 74, vec![FILLER, PUSH_BUTTON]),
    ];

    let found = identify(Surface::Dock, PID, &layers, &frames).expect("identified");

    assert!(found.element.path().ends_with("/2"));
}

#[test]
fn a_surface_the_compositor_does_not_map_is_closed() {
    let layers = [layer("dms:dock", 1280, 74)];
    let frames = [frame(3, 712, 720, vec![FILLER])];

    let error = identify(Surface::ControlCenter, PID, &layers, &frames).expect_err("closed");

    assert_eq!(error.code, ErrorCode::WindowNotFound);
}

#[test]
fn a_layer_of_another_process_is_not_the_shells_surface() {
    let mut foreign = layer("dms:dock", 1280, 74);
    foreign.pid = 99;
    let frames = [frame(2, 1280, 74, vec![FILLER])];

    assert!(identify(Surface::Dock, PID, &[foreign], &frames).is_err());
}

#[test]
fn a_mapped_surface_with_no_window_of_its_size_is_not_guessed_at() {
    let layers = [layer("dms:control-center", 712, 720)];
    let frames = [frame(3, 600, 720, vec![FILLER])];

    assert_eq!(
        identify(Surface::ControlCenter, PID, &layers, &frames)
            .expect_err("none")
            .code,
        ErrorCode::WindowNotFound
    );
}

#[test]
fn two_windows_that_fit_equally_are_reported_as_ambiguous() {
    let layers = [layer("dms:control-center", 712, 720)];
    let frames = [
        frame(3, 712, 720, vec![FILLER]),
        frame(4, 712, 720, vec![FILLER]),
    ];

    assert_eq!(
        identify(Surface::ControlCenter, PID, &layers, &frames)
            .expect_err("two")
            .code,
        ErrorCode::AmbiguousTarget
    );
}

#[test]
fn the_launcher_and_the_notification_center_share_a_size_and_are_told_apart_by_content() {
    let layers = [
        layer("dms:spotlight", 1280, 720),
        layer("dms:notification-center-modal", 1280, 720),
    ];
    let frames = [
        frame(7, 1280, 720, tabs()),
        frame(8, 1280, 720, vec![FILLER, FILLER]),
    ];

    let center = identify(Surface::NotificationCenter, PID, &layers, &frames).expect("center");
    let launcher = identify(Surface::Launcher, PID, &layers, &frames).expect("launcher");

    assert!(center.element.path().ends_with("/7"));
    assert!(launcher.element.path().ends_with("/8"));
}

#[test]
fn a_childless_full_output_window_is_never_a_surface_with_content() {
    let layers = [layer("dms:spotlight", 1280, 720)];
    let frames = [frame(1, 1280, 720, vec![])];

    assert!(identify(Surface::Launcher, PID, &layers, &frames).is_err());
}

#[test]
fn the_bar_is_found_without_content() {
    let layers = [layer("dms:bar", 1280, 44)];
    let frames = [frame(5, 1280, 44, vec![])];

    assert!(identify(Surface::Bar, PID, &layers, &frames).is_ok());
}

#[test]
fn resolving_reports_the_window_with_the_applications_name_and_the_surface_title() {
    let shell = Shell::new(
        vec![layer("dms:dock", 1280, 74)],
        vec![frame(2, 1280, 74, vec![FILLER])],
    );

    let window = resolve_with(&shell, Surface::Dock, deadline()).expect("resolved");

    assert_eq!(
        (window.app.as_str(), window.title.as_str()),
        ("quickshell", "Dock")
    );
    assert!(window.id.starts_with("w-"));
    assert_eq!(window.pid.get(), PID);
}

#[test]
fn a_surface_already_on_screen_is_returned_without_a_call_to_the_shell() {
    let shell = Shell::new(
        vec![layer("dms:control-center", 712, 720)],
        vec![frame(3, 712, 720, vec![FILLER])],
    );

    open_with(
        &shell,
        Surface::ControlCenter,
        InteractionPolicy::headless(),
        deadline(),
    )
    .expect("adopted");

    assert!(shell.calls.borrow().is_empty());
}

#[test]
fn a_closed_surface_is_shown_by_the_shells_ipc_and_answers_with_the_window_that_appeared() {
    let shell = Shell::new(Vec::new(), Vec::new()).opening(
        layer("dms:control-center", 712, 720),
        frame(3, 712, 720, vec![FILLER]),
    );

    let window = open_with(
        &shell,
        Surface::ControlCenter,
        InteractionPolicy::headed(),
        deadline(),
    )
    .expect("opened");

    assert_eq!(*shell.calls.borrow(), vec!["control-center open"]);
    assert!(window.id.ends_with("3"));
}

#[test]
fn the_dock_is_asked_to_reveal_itself() {
    let shell = Shell::new(Vec::new(), Vec::new()).opening(
        layer("dms:dock", 1280, 74),
        frame(2, 1280, 74, vec![FILLER]),
    );

    open_with(
        &shell,
        Surface::Dock,
        InteractionPolicy::headed(),
        deadline(),
    )
    .expect("opened");

    assert_eq!(*shell.calls.borrow(), vec!["dock reveal"]);
}

#[test]
fn showing_a_closed_surface_needs_permission_to_take_the_focus() {
    let shell = Shell::new(Vec::new(), Vec::new());

    let error = open_with(
        &shell,
        Surface::Launcher,
        InteractionPolicy::headless(),
        deadline(),
    )
    .expect_err("denied");

    assert_eq!(error.code, ErrorCode::PolicyDenied);
    assert!(shell.calls.borrow().is_empty());
}

#[test]
fn a_shell_build_without_the_handler_is_a_platform_limit_and_nothing_is_called() {
    let mut shell = Shell::new(Vec::new(), Vec::new());
    shell.offers = false;

    let error = open_with(
        &shell,
        Surface::Launcher,
        InteractionPolicy::headed(),
        deadline(),
    )
    .expect_err("unoffered");

    assert_eq!(error.code, ErrorCode::PlatformNotSupported);
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    assert!(shell.calls.borrow().is_empty());
}

#[test]
fn a_shell_that_accepts_the_call_but_never_shows_the_surface_says_it_opened_nothing_visible() {
    let shell = Shell::new(Vec::new(), Vec::new());
    let short = Deadline::after(300).expect("deadline");

    let error = open_with(
        &shell,
        Surface::Launcher,
        InteractionPolicy::headed(),
        short,
    )
    .expect_err("never shown");

    assert_eq!(error.code, ErrorCode::WindowNotFound);
    let details = error.details.expect("details");
    assert_eq!(details["surface_opened_by_call"], serde_json::json!(true));
    assert_eq!(details["ipc_reply"], serde_json::json!("OPEN_SUCCESS"));
}

#[test]
fn the_bar_cannot_be_opened_and_is_only_found() {
    let shell = Shell::new(Vec::new(), Vec::new());

    let error = open_with(
        &shell,
        Surface::Bar,
        InteractionPolicy::headed(),
        deadline(),
    )
    .expect_err("absent");

    assert_eq!(error.code, ErrorCode::WindowNotFound);
    assert!(shell.calls.borrow().is_empty());
}
