use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode};
use serde_json::{Value, json};

use super::layer::Layer;
use crate::system::hyprland::HyprInstance;

const REQUEST: &str = "j/layers";

fn malformed() -> AdapterError {
    AdapterError::internal("Hyprland's layers reply has no usable layer list")
        .with_details(json!({ "compositor": "hyprland", "request": "layers" }))
}

fn layer_of(value: &Value) -> Option<Layer> {
    Some(Layer {
        namespace: value.get("namespace")?.as_str()?.to_owned(),
        x: value.get("x")?.as_i64()?,
        y: value.get("y")?.as_i64()?,
        width: value.get("w")?.as_i64()?,
        height: value.get("h")?.as_i64()?,
        pid: u32::try_from(value.get("pid")?.as_i64()?).ok()?,
    })
}

/// The layers of a `layers` reply, over every output and level.
pub(super) fn parse(reply: &str) -> Result<Vec<Layer>, AdapterError> {
    let value: Value = serde_json::from_str(reply).map_err(|_| malformed())?;
    let outputs = value.as_object().ok_or_else(malformed)?;
    let mut layers = Vec::new();
    for output in outputs.values() {
        let levels = output
            .get("levels")
            .and_then(Value::as_object)
            .ok_or_else(malformed)?;
        for level in levels.values() {
            for item in level.as_array().ok_or_else(malformed)? {
                layers.push(layer_of(item).ok_or_else(malformed)?);
            }
        }
    }
    Ok(layers)
}

fn no_compositor() -> AdapterError {
    AdapterError::new(
        ErrorCode::PlatformNotSupported,
        "The compositor's layer list is not available, so the shell's surfaces cannot be told apart",
    )
    .with_suggestion("Run the shell under Hyprland, whose layer list identifies the shell's surfaces")
    .with_details(json!({ "compositor": "unsupported" }))
    .with_disposition(DeliverySemantics::not_delivered())
}

/// The layers the compositor has mapped now.
pub(crate) fn read(deadline: Deadline) -> Result<Vec<Layer>, AdapterError> {
    let instance = HyprInstance::detect().ok_or_else(no_compositor)?;
    parse(&instance.request(REQUEST, deadline)?)
}

#[cfg(test)]
#[path = "layers_tests.rs"]
mod tests;
