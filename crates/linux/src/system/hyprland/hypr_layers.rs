use agent_desktop_core::{AdapterError, Deadline, Rect};
use serde_json::{Value, json};

use super::HyprInstance;
use super::hypr_layer::HyprLayer;

fn malformed() -> AdapterError {
    AdapterError::internal("Hyprland's layers reply has no usable layer record")
        .with_details(json!({ "compositor": "hyprland", "request": "layers" }))
}

fn layer_of(monitor: &str, level: u8, value: &Value) -> Option<HyprLayer> {
    let number = |key: &str| value.get(key).and_then(Value::as_f64);
    Some(HyprLayer {
        monitor: monitor.to_owned(),
        level,
        namespace: value
            .get("namespace")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned(),
        pid: u32::try_from(value.get("pid")?.as_i64()?).ok()?,
        rect: Rect {
            x: number("x")?,
            y: number("y")?,
            width: number("w")?,
            height: number("h")?,
        },
    })
}

/// Reads the `layers` reply: monitors by name, each with its layers by
/// level number, each level a list of surfaces.
pub(crate) fn parse_layers(reply: &str) -> Result<Vec<HyprLayer>, AdapterError> {
    let value: Value = serde_json::from_str(reply).map_err(|_| malformed())?;
    let monitors = value.as_object().ok_or_else(malformed)?;
    let mut layers = Vec::new();
    for (monitor, entry) in monitors {
        let levels = entry
            .get("levels")
            .and_then(Value::as_object)
            .ok_or_else(malformed)?;
        for (level, surfaces) in levels {
            let level: u8 = level.parse().map_err(|_| malformed())?;
            for surface in surfaces.as_array().ok_or_else(malformed)? {
                layers.push(layer_of(monitor, level, surface).ok_or_else(malformed)?);
            }
        }
    }
    Ok(layers)
}

impl HyprInstance {
    pub(crate) fn layers(&self, deadline: Deadline) -> Result<Vec<HyprLayer>, AdapterError> {
        parse_layers(&self.request("j/layers", deadline)?)
    }
}

#[cfg(test)]
#[path = "hypr_layers_tests.rs"]
mod tests;
