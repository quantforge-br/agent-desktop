use std::time::{Duration, Instant};

use agent_desktop_core::{
    AdapterError, Deadline, DeliverySemantics, ErrorCode, InteractionPolicy, Rect, WindowInfo,
    WindowState,
};
use serde_json::json;

use super::frame::Frame;
use super::ipc::Ipc;
use super::layer::Layer;
use super::surface::Surface;
use crate::system::process::process_facts;
use crate::system::window::window_id;

const POLL: Duration = Duration::from_millis(80);
const SHOW_SETTLE: Duration = Duration::from_millis(4_000);

/// The shell as the presentation rules see it: what the compositor has
/// mapped, what the application's windows are, and the IPC. A test
/// substitutes a fixed shell.
pub(crate) trait Stage {
    fn pid(&self) -> u32;
    fn layers(&self, deadline: Deadline) -> Result<Vec<Layer>, AdapterError>;
    /// The application's name and its top-level windows.
    fn frames(&self, deadline: Deadline) -> Result<(String, Vec<Frame>), AdapterError>;
    fn ipc(&self) -> &dyn Ipc;
    fn pause(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
}

fn closed(surface: Surface, detail: &str) -> AdapterError {
    AdapterError::new(
        ErrorCode::WindowNotFound,
        format!("The shell's {} surface is closed", surface.title()),
    )
    .with_suggestion("Run 'open-system-surface' for this surface before observing it")
    .with_details(json!({ "surface": surface.title(), "detail": detail }))
}

/// The window that presents the surface now. The compositor says the surface
/// is mapped (a layer of the shell process under the surface's namespace),
/// and the one window of the shell's application that has the layer's size
/// and holds content is the surface's window; no other pairing is guessed.
pub(crate) fn identify<'a>(
    surface: Surface,
    pid: u32,
    layers: &[Layer],
    frames: &'a [Frame],
) -> Result<&'a Frame, AdapterError> {
    let layer = layers
        .iter()
        .find(|layer| layer.namespace == surface.namespace() && layer.pid == pid)
        .ok_or_else(|| closed(surface, "the compositor maps no such surface"))?;
    let mut found = frames.iter().filter(|frame| {
        frame.showing
            && frame.width == layer.width
            && frame.height == layer.height
            && (frame.children > 0 || !surface.has_content())
            && surface.recognises(&frame.roles)
    });
    match (found.next(), found.next()) {
        (Some(frame), None) => Ok(frame),
        (None, _) => Err(closed(
            surface,
            "no accessibility window has the surface's size",
        )),
        (Some(_), Some(_)) => Err(AdapterError::ambiguous_target(format!(
            "Several accessibility windows look like the shell's {} surface",
            surface.title()
        ))),
    }
}

fn info(
    surface: Surface,
    pid: u32,
    app: String,
    frame: &Frame,
) -> Result<WindowInfo, AdapterError> {
    let id = window_id::encode(frame.element.bus_name(), frame.element.path())
        .ok_or_else(|| AdapterError::internal("The shell window has no window id"))?;
    Ok(WindowInfo {
        id,
        title: surface.title().to_owned(),
        app,
        pid: pid.into(),
        process_instance: process_facts::read(pid).map(|facts| facts.instance),
        bounds: Some(Rect {
            x: 0.0,
            y: 0.0,
            width: frame.width as f64,
            height: frame.height as f64,
        }),
        state: WindowState {
            is_focused: false,
            accessible: true,
            minimized: Some(false),
            visible: Some(true),
        },
    })
}

/// The window presenting the surface, when it is presented.
pub(crate) fn resolve_with(
    stage: &dyn Stage,
    surface: Surface,
    deadline: Deadline,
) -> Result<WindowInfo, AdapterError> {
    let layers = stage.layers(deadline)?;
    let (app, frames) = stage.frames(deadline)?;
    let frame = identify(surface, stage.pid(), &layers, &frames)?;
    info(surface, stage.pid(), app, frame)
}

fn progress(error: AdapterError, opened_by_call: bool, reply: Option<&str>) -> AdapterError {
    let mut details = match error.details.clone() {
        Some(serde_json::Value::Object(map)) => map,
        Some(other) => {
            let mut map = serde_json::Map::new();
            map.insert("cause".to_owned(), other);
            map
        }
        None => serde_json::Map::new(),
    };
    details.insert("surface_opened_by_call".to_owned(), json!(opened_by_call));
    if let Some(reply) = reply {
        details.insert("ipc_reply".to_owned(), json!(reply));
    }
    error.with_details(serde_json::Value::Object(details))
}

fn denied(surface: Surface, policy: InteractionPolicy) -> AdapterError {
    AdapterError::policy_denied_for_policy(
        format!(
            "Showing the shell's {} surface changes what is on screen and takes the focus",
            surface.title()
        ),
        policy,
    )
    .with_suggestion("Pass --headed to let the command show the surface")
}

fn unoffered(surface: Surface, target: &str) -> AdapterError {
    AdapterError::new(
        ErrorCode::PlatformNotSupported,
        format!(
            "This dux-shell build offers no IPC to show its {} surface",
            surface.title()
        ),
    )
    .with_suggestion("Show the surface by hand; it can then be observed")
    .with_details(json!({ "ipc": "handler_missing", "target": target }))
    .with_disposition(DeliverySemantics::not_delivered())
}

/// Presents the surface and answers with its window. A surface that is
/// already presented is returned as found. Otherwise the shell's own IPC
/// shows it (never a pointer click), and the window is the one the compositor
/// and the accessibility tree agree on afterwards. The bar has no opener: it
/// is observed where it is.
pub(crate) fn open_with(
    stage: &dyn Stage,
    surface: Surface,
    policy: InteractionPolicy,
    deadline: Deadline,
) -> Result<WindowInfo, AdapterError> {
    let present = resolve_with(stage, surface, deadline);
    let Some(opener) = surface.opener() else {
        return present;
    };
    match present {
        Err(error) if error.code == ErrorCode::WindowNotFound => {}
        settled => return settled,
    }
    if !policy.allow_focus_steal {
        return Err(progress(denied(surface, policy), false, None));
    }
    let ipc = stage.ipc();
    if !ipc.exposes(opener.target, opener.open, deadline)? {
        return Err(progress(unoffered(surface, opener.target), false, None));
    }
    let reply = ipc.call(opener.target, opener.open, deadline)?;
    let started = Instant::now();
    loop {
        match resolve_with(stage, surface, deadline) {
            Ok(found) => return Ok(found),
            Err(error)
                if error.code != ErrorCode::WindowNotFound
                    || started.elapsed() >= SHOW_SETTLE
                    || deadline.is_expired() =>
            {
                return Err(progress(error, true, Some(&reply)));
            }
            Err(_) => stage.pause(POLL),
        }
    }
}

#[cfg(test)]
#[path = "present_tests.rs"]
mod tests;
