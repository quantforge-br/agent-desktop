use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode};
use serde_json::json;

use super::hypr_command::HyprCommand;
use super::hypr_instance::HyprInstance;

fn lua_string(text: &str) -> String {
    let mut quoted = String::with_capacity(text.len() + 2);
    quoted.push('"');
    for byte in text.bytes() {
        match byte {
            b'"' => quoted.push_str("\\\""),
            b'\\' => quoted.push_str("\\\\"),
            0x20..=0x7e => quoted.push(char::from(byte)),
            other => quoted.push_str(&format!("\\{other:03}")),
        }
    }
    quoted.push('"');
    quoted
}

fn toggle(on: bool) -> &'static str {
    if on { "set" } else { "unset" }
}

/// The Lua dispatcher call Hyprland's `dispatch` request evaluates for
/// `command` on the window at `address`.
pub(crate) fn lua_call(address: u64, command: &HyprCommand) -> String {
    let window = lua_string(&format!("address:0x{address:x}"));
    match command {
        HyprCommand::Focus => format!("hl.dsp.focus({{ window = {window} }})"),
        HyprCommand::Float(on) => format!(
            "hl.dsp.window.float({{ action = \"{}\", window = {window} }})",
            toggle(*on)
        ),
        HyprCommand::MoveTo { x, y } => {
            format!("hl.dsp.window.move({{ x = {x}, y = {y}, window = {window} }})")
        }
        HyprCommand::ResizeTo { width, height } => {
            format!("hl.dsp.window.resize({{ x = {width}, y = {height}, window = {window} }})")
        }
        HyprCommand::Maximize(on) => format!(
            "hl.dsp.window.fullscreen({{ mode = \"maximized\", action = \"{}\", window = {window} }})",
            toggle(*on)
        ),
        HyprCommand::Fullscreen(on) => format!(
            "hl.dsp.window.fullscreen({{ mode = \"fullscreen\", action = \"{}\", window = {window} }})",
            toggle(*on)
        ),
        HyprCommand::ToWorkspace(workspace) => format!(
            "hl.dsp.window.move({{ workspace = {}, follow = false, window = {window} }})",
            lua_string(workspace)
        ),
        HyprCommand::Close => format!("hl.dsp.window.close({{ window = {window} }})"),
    }
}

fn legacy_dialect(reply: &str) -> AdapterError {
    AdapterError::new(
        ErrorCode::PlatformNotSupported,
        "This Hyprland does not evaluate Lua dispatcher calls",
    )
    .with_suggestion(
        "Run Hyprland 0.57 or newer, whose dispatch request takes hl.dsp calls; \
         older dispatcher syntax is not measured",
    )
    .with_platform_detail(reply.trim().to_owned())
    .with_details(json!({ "compositor": "hyprland", "cause": "hyprland_dispatch_dialect" }))
    .with_disposition(DeliverySemantics::not_delivered())
}

fn refused(call: &str, reply: &str) -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionFailed,
        "Hyprland refused the window dispatcher",
    )
    .with_platform_detail(reply.trim().to_owned())
    .with_details(json!({ "compositor": "hyprland", "dispatch": call }))
    .with_disposition(DeliverySemantics::not_delivered())
}

/// Reads a `dispatch` reply. Hyprland answers `ok` even for a window that
/// no longer exists, so success says only that the call was accepted;
/// every caller reads the effect back.
pub(crate) fn judge(call: &str, reply: &str) -> Result<(), AdapterError> {
    let reply = reply.trim();
    if reply == "ok" {
        Ok(())
    } else if reply.starts_with("Invalid dispatcher") {
        Err(legacy_dialect(reply))
    } else {
        Err(refused(call, reply))
    }
}

impl HyprInstance {
    pub(crate) fn dispatch(
        &self,
        address: u64,
        command: &HyprCommand,
        deadline: Deadline,
    ) -> Result<(), AdapterError> {
        let call = lua_call(address, command);
        let reply = self.request(&format!("dispatch {call}"), deadline)?;
        judge(&call, &reply)
    }
}

#[cfg(test)]
#[path = "hypr_dispatch_tests.rs"]
mod tests;
