use agent_desktop_core::{AdapterError, Deadline, Point, Rect};
use serde_json::{Value, json};

use super::hypr_client::HyprClient;
use super::hypr_client_owner::HyprClientOwner;
use super::hypr_client_state::HyprClientState;
use super::hypr_instance::HyprInstance;
use super::hypr_monitor::HyprMonitor;

fn malformed(request: &str, what: &str) -> AdapterError {
    AdapterError::internal(format!("Hyprland's {request} reply has no usable {what}"))
        .with_details(json!({ "compositor": "hyprland", "request": request }))
}

fn parse_json(request: &str, text: &str) -> Result<Value, AdapterError> {
    serde_json::from_str(text).map_err(|_| malformed(request, "JSON"))
}

fn text(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn flag(value: &Value, key: &str) -> bool {
    value.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn pair(value: &Value, key: &str) -> Option<(f64, f64)> {
    let items = value.get(key)?.as_array()?;
    match items.as_slice() {
        [x, y] => Some((x.as_f64()?, y.as_f64()?)),
        _ => None,
    }
}

fn workspace_name(value: &Value, key: &str) -> String {
    value
        .get(key)
        .map(|workspace| text(workspace, "name"))
        .unwrap_or_default()
}

/// Reads a Hyprland window address such as `0x5643f6b91d70`.
pub(crate) fn parse_address(address: &str) -> Option<u64> {
    let digits = address.strip_prefix("0x")?;
    u64::from_str_radix(digits, 16).ok()
}

fn client_of(value: &Value) -> Option<HyprClient> {
    let address = parse_address(value.get("address")?.as_str()?)?;
    let (x, y) = pair(value, "at")?;
    let (width, height) = pair(value, "size")?;
    Some(HyprClient {
        address,
        stable_id: text(value, "stableId"),
        owner: HyprClientOwner {
            pid: u32::try_from(value.get("pid")?.as_i64()?).ok()?,
            class: text(value, "class"),
            title: text(value, "title"),
            xwayland: flag(value, "xwayland"),
        },
        frame: Rect {
            x,
            y,
            width,
            height,
        },
        state: HyprClientState {
            workspace: workspace_name(value, "workspace"),
            monitor: value.get("monitor").and_then(Value::as_i64).unwrap_or(-1),
            floating: flag(value, "floating"),
            fullscreen: value
                .get("fullscreen")
                .and_then(Value::as_u64)
                .and_then(|mode| u8::try_from(mode).ok())
                .unwrap_or(0),
            mapped: flag(value, "mapped"),
            hidden: flag(value, "hidden"),
            pinned: flag(value, "pinned"),
        },
        focus_history: value
            .get("focusHistoryID")
            .and_then(Value::as_i64)
            .unwrap_or(i64::MAX),
    })
}

pub(crate) fn parse_clients(reply: &str) -> Result<Vec<HyprClient>, AdapterError> {
    let value = parse_json("clients", reply)?;
    let items = value
        .as_array()
        .ok_or_else(|| malformed("clients", "list"))?;
    items
        .iter()
        .map(|item| client_of(item).ok_or_else(|| malformed("clients", "client record")))
        .collect()
}

fn logical_rect(value: &Value) -> Option<(Rect, f64)> {
    let scale = value.get("scale")?.as_f64().filter(|scale| *scale > 0.0)?;
    let width = value.get("width")?.as_f64()?;
    let height = value.get("height")?.as_f64()?;
    let rotated = value.get("transform").and_then(Value::as_i64).unwrap_or(0) % 2 == 1;
    let (width, height) = if rotated {
        (height, width)
    } else {
        (width, height)
    };
    Some((
        Rect {
            x: value.get("x")?.as_f64()?,
            y: value.get("y")?.as_f64()?,
            width: width / scale,
            height: height / scale,
        },
        scale,
    ))
}

fn monitor_of(value: &Value) -> Option<HyprMonitor> {
    let (logical, scale) = logical_rect(value)?;
    let special = workspace_name(value, "specialWorkspace");
    Some(HyprMonitor {
        id: value.get("id")?.as_i64()?,
        name: text(value, "name"),
        logical,
        scale,
        focused: flag(value, "focused"),
        active_workspace: workspace_name(value, "activeWorkspace"),
        special_workspace: (!special.is_empty()).then_some(special),
    })
}

pub(crate) fn parse_monitors(reply: &str) -> Result<Vec<HyprMonitor>, AdapterError> {
    let value = parse_json("monitors", reply)?;
    let items = value
        .as_array()
        .ok_or_else(|| malformed("monitors", "list"))?;
    items
        .iter()
        .map(|item| monitor_of(item).ok_or_else(|| malformed("monitors", "monitor record")))
        .collect()
}

pub(crate) fn parse_active(reply: &str) -> Result<Option<u64>, AdapterError> {
    let value = parse_json("activewindow", reply)?;
    match value.get("address") {
        None => Ok(None),
        Some(address) => address
            .as_str()
            .and_then(parse_address)
            .map(Some)
            .ok_or_else(|| malformed("activewindow", "address")),
    }
}

pub(crate) fn parse_cursor(reply: &str) -> Result<Point, AdapterError> {
    let value = parse_json("cursorpos", reply)?;
    let read = |key: &str| value.get(key).and_then(Value::as_f64);
    match (read("x"), read("y")) {
        (Some(x), Some(y)) => Ok(Point { x, y }),
        _ => Err(malformed("cursorpos", "position")),
    }
}

impl HyprInstance {
    pub(crate) fn clients(&self, deadline: Deadline) -> Result<Vec<HyprClient>, AdapterError> {
        parse_clients(&self.request("j/clients", deadline)?)
    }

    pub(crate) fn monitors(&self, deadline: Deadline) -> Result<Vec<HyprMonitor>, AdapterError> {
        parse_monitors(&self.request("j/monitors", deadline)?)
    }

    /// The address of the toplevel holding Hyprland's focus, if any.
    pub(crate) fn active_address(&self, deadline: Deadline) -> Result<Option<u64>, AdapterError> {
        parse_active(&self.request("j/activewindow", deadline)?)
    }

    /// The pointer's position in the logical layout.
    pub(crate) fn cursor(&self, deadline: Deadline) -> Result<Point, AdapterError> {
        parse_cursor(&self.request("j/cursorpos", deadline)?)
    }
}

#[cfg(test)]
#[path = "hypr_read_tests.rs"]
mod tests;
