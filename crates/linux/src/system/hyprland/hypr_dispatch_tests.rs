use agent_desktop_core::ErrorCode;

use super::{judge, lua_call};
use crate::system::hyprland::hypr_command::HyprCommand;

#[test]
fn window_calls_name_the_exact_address() {
    assert_eq!(
        lua_call(0xabc, &HyprCommand::MoveTo { x: -10, y: 20 }),
        "hl.dsp.window.move({ x = -10, y = 20, window = \"address:0xabc\" })"
    );
    assert_eq!(
        lua_call(0xabc, &HyprCommand::Maximize(false)),
        "hl.dsp.window.fullscreen({ mode = \"maximized\", action = \"unset\", window = \"address:0xabc\" })"
    );
}

#[test]
fn a_workspace_name_cannot_break_out_of_its_lua_string() {
    let call = lua_call(
        1,
        &HyprCommand::ToWorkspace("a\" }) hl.dsp.exit() --\n".to_owned()),
    );
    assert_eq!(
        call,
        "hl.dsp.window.move({ workspace = \"a\\\" }) hl.dsp.exit() --\\010\", follow = false, window = \"address:0x1\" })"
    );
}

#[test]
fn only_ok_is_an_accepted_dispatch() {
    assert!(judge("call", "ok\n").is_ok());
    let error = judge("call", "error: [string ...]:1: attempt to call a nil value").unwrap_err();
    assert_eq!(error.code, ErrorCode::ActionFailed);
}

#[test]
fn a_hyprland_without_lua_dispatchers_is_a_named_platform_refusal() {
    let error = judge("call", "Invalid dispatcher").unwrap_err();
    assert_eq!(error.code, ErrorCode::PlatformNotSupported);
    assert_eq!(error.details.unwrap()["cause"], "hyprland_dispatch_dialect");
}
