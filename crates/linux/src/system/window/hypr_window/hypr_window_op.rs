use std::time::Duration;

use agent_desktop_core::{
    AdapterError, DeliverySemantics, ErrorCode, OperationReport, Rect, WindowInfo, WindowOp,
};
use serde_json::{Value, json};

use super::hypr_window_key::WindowKey;
use super::hypr_window_match::match_client;
use super::hypr_world::HyprWorld;
use crate::system::hyprland::hypr_client::HyprClient;
use crate::system::hyprland::hypr_command::HyprCommand;
use crate::system::process::process_lookup::Lookup;
use crate::system::window::window_op::window_op_verdict::{POSITION_TOLERANCE, requested};
use crate::system::window::window_report::window_readback;
use crate::system::x11::control::control_send::WindowWrite;

/// The special workspace a minimized window is parked on. Hyprland has no
/// minimized state; a window on this workspace is listed as minimized.
pub(crate) const MINIMIZED_WORKSPACE: &str = "special:minimized";

const SETTLE: Duration = Duration::from_millis(1_000);
const POLL: Duration = Duration::from_millis(25);

pub(super) fn stale(message: &str) -> AdapterError {
    AdapterError::stale_ref_because(message).with_disposition(DeliverySemantics::not_delivered())
}

pub(super) fn owned_before(world: &impl HyprWorld, win: &WindowInfo) -> Result<(), AdapterError> {
    let Some(instance) = win.process_instance.as_deref() else {
        return Err(stale(
            "A window operation needs the process instance the window was listed with",
        ));
    };
    match world.lookup(win.pid.get()) {
        Lookup::Found(facts) if facts.instance != instance => Err(stale(
            "The window belongs to a different process instance or the process has exited",
        )),
        Lookup::Gone => Err(stale(
            "The window belongs to a different process instance or the process has exited",
        )),
        _ => Ok(()),
    }
}

fn rect_json(rect: &Rect) -> Value {
    json!({ "x": rect.x, "y": rect.y, "width": rect.width, "height": rect.height })
}

fn observed(client: &HyprClient) -> Value {
    let mut seen = rect_json(&client.frame);
    seen["workspace"] = json!(client.state.workspace);
    seen["floating"] = json!(client.state.floating);
    seen["fullscreen"] = json!(client.state.fullscreen);
    seen
}

pub(super) fn vanished() -> AdapterError {
    AdapterError::new(
        ErrorCode::WindowNotFound,
        "The window disappeared or its owner changed during the operation",
    )
    .with_suggestion("Run 'list-windows' to find the window again")
    .with_disposition(DeliverySemantics::delivered_unverified())
}

fn mismatch(requested: &Value, client: &HyprClient) -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionFailed,
        "Hyprland left the window other than requested",
    )
    .with_suggestion(
        "A window rule or the window's size limits constrained the request; read the held state from 'list-windows'",
    )
    .with_details(json!({
        "cause": "readback_mismatch",
        "compositor": "hyprland",
        "requested": requested,
        "observed": observed(client),
    }))
    .with_disposition(DeliverySemantics::delivered_unverified())
}

fn delivered(error: AdapterError) -> AdapterError {
    if error.disposition == DeliverySemantics::not_delivered() {
        error.with_disposition(DeliverySemantics::uncertain())
    } else {
        error
    }
}

pub(super) fn locate(world: &impl HyprWorld, win: &WindowInfo) -> Result<HyprClient, AdapterError> {
    let clients = world.clients()?;
    match_client(&clients, WindowKey::of(win))
        .cloned()
        .map_err(|unmatched| unmatched.into_error())
}

/// Reads the client at `address` until `holds` accepts it or the settle
/// budget ends, then checks the client still belongs to the window's process.
fn settle(
    world: &impl HyprWorld,
    win: &WindowInfo,
    address: u64,
    asked: &Value,
    holds: impl Fn(&HyprClient) -> bool,
) -> Result<HyprClient, AdapterError> {
    let limit = world.deadline().capped(SETTLE);
    loop {
        let clients = world.clients().map_err(delivered)?;
        let Some(client) = clients.into_iter().find(|client| client.address == address) else {
            return Err(vanished());
        };
        if client.owner.pid != win.pid.get() {
            return Err(vanished());
        }
        if holds(&client) {
            return Ok(client);
        }
        if limit.is_expired() {
            return Err(mismatch(asked, &client));
        }
        std::thread::sleep(POLL.min(limit.remaining()));
    }
}

fn near(wanted: i64, seen: f64) -> bool {
    (wanted as f64 - seen).abs() <= f64::from(POSITION_TOLERANCE)
}

fn send(
    world: &impl HyprWorld,
    client: &HyprClient,
    command: &HyprCommand,
    started: bool,
) -> Result<(), AdapterError> {
    world
        .send(client.address, command)
        .map_err(|error| if started { delivered(error) } else { error })
}

/// Takes a maximized or fullscreen window back to the layout it left: a
/// tiled window returns to its tile, a floating one to its frame.
fn release_fullscreen(
    world: &impl HyprWorld,
    win: &WindowInfo,
    client: HyprClient,
    started: bool,
) -> Result<HyprClient, AdapterError> {
    if client.state.fullscreen == 0 {
        return Ok(client);
    }
    let command = if client.state.fullscreen == 1 {
        HyprCommand::Maximize(false)
    } else {
        HyprCommand::Fullscreen(false)
    };
    send(world, &client, &command, started)?;
    let asked = json!({ "fullscreen": 0 });
    settle(world, win, client.address, &asked, |c| {
        c.state.fullscreen == 0
    })
}

/// Brings a maximized, fullscreen or tiled window to a plain floating one,
/// which is the only kind Hyprland moves and sizes freely.
fn release_layout(
    world: &impl HyprWorld,
    win: &WindowInfo,
    client: HyprClient,
) -> Result<(HyprClient, bool), AdapterError> {
    let mut started = client.state.fullscreen != 0;
    let mut current = release_fullscreen(world, win, client, false)?;
    if !current.state.floating {
        send(world, &current, &HyprCommand::Float(true), started)?;
        started = true;
        let asked = json!({ "floating": true });
        current = settle(world, win, current.address, &asked, |c| c.state.floating)?;
    }
    Ok((current, started))
}

fn place(
    world: &impl HyprWorld,
    win: &WindowInfo,
    client: HyprClient,
    write: &WindowWrite,
) -> Result<Option<Value>, AdapterError> {
    let WindowWrite::MoveResize {
        x,
        y,
        width,
        height,
    } = *write
    else {
        return Ok(None);
    };
    let (mut client, mut started) = release_layout(world, win, client)?;
    let x = x.map_or(client.frame.x.round() as i64, i64::from);
    let y = y.map_or(client.frame.y.round() as i64, i64::from);
    let extent = (
        width.map_or(client.frame.width, f64::from),
        height.map_or(client.frame.height, f64::from),
    );
    if let Some(workspace) = arrival_workspace(world, &client, (x, y), extent)? {
        send(
            world,
            &client,
            &HyprCommand::ToWorkspace(workspace.clone()),
            started,
        )?;
        started = true;
        let asked = json!({ "workspace": workspace });
        client = settle(world, win, client.address, &asked, |c| {
            c.state.workspace == workspace
        })?;
    }
    if let (Some(width), Some(height)) = (width, height) {
        let size = HyprCommand::ResizeTo {
            width: i64::from(width),
            height: i64::from(height),
        };
        send(world, &client, &size, started)?;
        started = true;
    }
    send(world, &client, &HyprCommand::MoveTo { x, y }, started)?;
    let asked = json!({ "x": x, "y": y, "width": width, "height": height });
    let done = settle(world, win, client.address, &asked, |c| {
        near(x, c.frame.x)
            && near(y, c.frame.y)
            && width.is_none_or(|w| near(i64::from(w), c.frame.width))
            && height.is_none_or(|h| near(i64::from(h), c.frame.height))
    })?;
    Ok(width.map(|_| rect_json(&done.frame)))
}

/// The workspace a window moved to `origin` must join: the one shown by the
/// monitor holding the moved window's centre, when that is not the monitor
/// it is on. Hyprland draws a window past its monitor's edge but routes the
/// pointer there to the other monitor's workspace.
fn arrival_workspace(
    world: &impl HyprWorld,
    client: &HyprClient,
    origin: (i64, i64),
    extent: (f64, f64),
) -> Result<Option<String>, AdapterError> {
    let centre = (
        origin.0 as f64 + extent.0 / 2.0,
        origin.1 as f64 + extent.1 / 2.0,
    );
    let monitors = world.monitors()?;
    let holder = monitors.iter().find(|monitor| {
        let area = monitor.logical;
        centre.0 >= area.x
            && centre.1 >= area.y
            && centre.0 < area.x + area.width
            && centre.1 < area.y + area.height
    });
    Ok(holder
        .filter(|monitor| monitor.id != client.state.monitor)
        .map(|monitor| monitor.active_workspace.clone()))
}

fn home_workspace(world: &impl HyprWorld, client: &HyprClient) -> Result<String, AdapterError> {
    let monitors = world.monitors()?;
    monitors
        .iter()
        .find(|monitor| monitor.id == client.state.monitor)
        .or_else(|| monitors.iter().find(|monitor| monitor.focused))
        .map(|monitor| monitor.active_workspace.clone())
        .ok_or_else(|| {
            AdapterError::new(
                ErrorCode::ActionFailed,
                "Hyprland lists no monitor to restore the window onto",
            )
            .with_disposition(DeliverySemantics::not_delivered())
        })
}

pub(super) fn restore(
    world: &impl HyprWorld,
    win: &WindowInfo,
    client: HyprClient,
) -> Result<(), AdapterError> {
    let mut current = client;
    let mut started = false;
    if current.state.workspace == MINIMIZED_WORKSPACE {
        let home = home_workspace(world, &current)?;
        send(
            world,
            &current,
            &HyprCommand::ToWorkspace(home.clone()),
            started,
        )?;
        started = true;
        let asked = json!({ "workspace": home });
        current = settle(world, win, current.address, &asked, |c| {
            c.state.workspace != MINIMIZED_WORKSPACE
        })?;
        send(world, &current, &HyprCommand::Focus, started)?;
    }
    release_fullscreen(world, win, current, started)?;
    Ok(())
}

fn dispatch(
    world: &impl HyprWorld,
    win: &WindowInfo,
    op: &WindowOp,
) -> Result<Option<Value>, AdapterError> {
    let write = requested(op)?;
    owned_before(world, win)?;
    if world.stopped(win.pid.get()) {
        return Err(AdapterError::app_unresponsive(&win.app)
            .with_disposition(DeliverySemantics::not_delivered()));
    }
    let client = locate(world, win)?;
    match write {
        Some(WindowWrite::Iconify) => {
            let parking = HyprCommand::ToWorkspace(MINIMIZED_WORKSPACE.to_owned());
            send(world, &client, &parking, false)?;
            let asked = json!({ "workspace": MINIMIZED_WORKSPACE });
            settle(world, win, client.address, &asked, |c| {
                c.state.workspace == MINIMIZED_WORKSPACE
            })
            .map(|_| None)
        }
        Some(WindowWrite::SetMaximized(_)) => {
            send(world, &client, &HyprCommand::Maximize(true), false)?;
            let asked = json!({ "fullscreen": 1 });
            settle(world, win, client.address, &asked, |c| {
                c.state.fullscreen == 1
            })
            .map(|_| None)
        }
        Some(write) => place(world, win, client, &write),
        None => restore(world, win, client).map(|()| None),
    }
}

/// Moves, sizes, minimizes, maximizes or restores `win` through Hyprland's
/// dispatchers, answering only after the client list shows the effect.
pub(crate) fn window_op(
    world: &impl HyprWorld,
    win: &WindowInfo,
    op: &WindowOp,
) -> Result<OperationReport, AdapterError> {
    let dispatched = dispatch(world, win, op);
    let seen = dispatched.as_ref().ok().cloned().flatten();
    let mut report =
        OperationReport::from_window_dispatch(win, op, window_readback(op), dispatched.map(drop))?;
    if let (Some(seen), Some(Value::Object(details))) = (seen, report.result.details.as_mut()) {
        details.insert("observed".to_owned(), seen);
    }
    Ok(report)
}

#[cfg(test)]
#[path = "hypr_window_op_tests.rs"]
mod tests;
