use std::sync::atomic::{AtomicUsize, Ordering};

use agent_desktop_core::{
    AdapterError, AppInfo, CommandContext, Deadline, InteractionLease, OperationReport,
    OperationTarget, PermissionReport, ProcessId, SignalBaseline, SignalCompleteness, SignalFilter,
    WindowInfo, WindowState,
};
use serde_json::json;

use super::execute;
use crate::cli_args::batch::BatchArgs;

struct TwoDialogs {
    clears: AtomicUsize,
    honours_process: bool,
}

fn app(pid: u32) -> AppInfo {
    AppInfo {
        name: "zenity".into(),
        pid: ProcessId::new(pid),
        bundle_id: None,
        process_instance: Some(format!("boot:{pid}")),
        presentation: None,
    }
}

fn window(id: &str, pid: u32) -> WindowInfo {
    WindowInfo {
        id: id.into(),
        title: id.into(),
        app: "zenity".into(),
        pid: ProcessId::new(pid),
        process_instance: Some(format!("boot:{pid}")),
        bounds: None,
        state: WindowState::default(),
    }
}

impl agent_desktop_core::ObservationOps for TwoDialogs {
    fn list_windows(
        &self,
        _filter: &agent_desktop_core::WindowFilter,
        _deadline: Deadline,
    ) -> Result<Vec<WindowInfo>, AdapterError> {
        Ok(vec![window("w-a", 10), window("w-b", 11)])
    }

    fn list_apps(&self, _deadline: Deadline) -> Result<Vec<AppInfo>, AdapterError> {
        Ok(vec![app(10), app(11)])
    }
}

impl agent_desktop_core::ActionOps for TwoDialogs {}

impl agent_desktop_core::InputOps for TwoDialogs {
    fn clear_clipboard(&self, _lease: &InteractionLease) -> Result<OperationReport, AdapterError> {
        self.clears.fetch_add(1, Ordering::SeqCst);
        Ok(OperationReport::delivered_unverified(
            "test",
            OperationTarget::Clipboard,
        ))
    }
}

impl agent_desktop_core::SystemOps for TwoDialogs {
    fn acquire_interaction_lease(
        &self,
        deadline: Deadline,
    ) -> Result<InteractionLease, AdapterError> {
        InteractionLease::guarded(deadline, ())
    }

    fn capture_signal_baseline(
        &self,
        filter: &SignalFilter,
        _deadline: Deadline,
    ) -> Result<SignalBaseline, AdapterError> {
        let apps = [app(10), app(11)]
            .into_iter()
            .filter(|app| {
                !self.honours_process || filter.process.as_ref().is_none_or(|p| p.pid == app.pid)
            })
            .collect();
        Ok(SignalBaseline {
            windows: Vec::new(),
            apps,
            surfaces: Vec::new(),
            completeness: SignalCompleteness::complete(),
        })
    }
}

fn run(wait_args: serde_json::Value) -> (serde_json::Value, usize) {
    run_with(wait_args, true)
}

fn run_with(wait_args: serde_json::Value, honours_process: bool) -> (serde_json::Value, usize) {
    let adapter = TwoDialogs {
        clears: AtomicUsize::new(0),
        honours_process,
    };
    let args = BatchArgs {
        commands_json: json!([
            {"command": "clipboard-clear", "args": {}},
            {"command": "wait", "args": wait_args}
        ])
        .to_string(),
        stop_on_error: false,
        timeout_ms: 60_000,
    };
    let value = execute(
        args,
        &adapter,
        &PermissionReport::default(),
        &CommandContext::default(),
    )
    .unwrap();
    (value, adapter.clears.load(Ordering::SeqCst))
}

#[test]
fn a_window_id_lets_the_wait_name_one_of_two_same_named_processes() {
    let (value, clears) = run(json!({
        "event": "window-closed", "app": "zenity", "window_id": "w-b", "timeout": 100
    }));
    assert_eq!(clears, 1, "{value}");
    assert_eq!(value["results"][0]["ok"], true, "{value}");
    assert_eq!(value["results"][1]["error"]["code"], "TIMEOUT", "{value}");
    assert!(value.get("stopped").is_none(), "{value}");
}

#[test]
fn two_same_named_processes_without_a_window_id_stop_the_batch_before_the_action() {
    let (value, clears) = run(json!({
        "event": "window-closed", "app": "zenity", "timeout": 100
    }));
    assert_eq!(clears, 0, "{value}");
    assert_eq!(value["results"][0]["execution"], "not_started", "{value}");
    assert_eq!(
        value["results"][0]["error"]["code"], "AMBIGUOUS_TARGET",
        "{value}"
    );
    assert_eq!(
        value["stopped"]["reason"], "pre_action_baseline_failed",
        "{value}"
    );
}

#[test]
fn an_adapter_that_returns_every_instance_despite_the_scope_still_stops_the_batch_before_the_action()
 {
    let (value, clears) = run_with(
        json!({
            "event": "window-closed", "app": "zenity", "window_id": "w-b", "timeout": 100
        }),
        false,
    );
    assert_eq!(clears, 0, "{value}");
    assert_eq!(value["results"][0]["execution"], "not_started", "{value}");
    assert_eq!(
        value["stopped"]["reason"], "pre_action_baseline_failed",
        "{value}"
    );
}
