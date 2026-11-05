use std::sync::Mutex;

use super::*;
use crate::adapter::{ActionOps, InputOps, ObservationOps, SystemOps, WindowFilter};
use crate::{
    AdapterError, AppInfo, ErrorCode, ProcessId, SignalCompleteness, WindowInfo, WindowState,
};

struct TwoDialogs {
    windows_listed: bool,
    honours_process: bool,
    apps_listed: bool,
    filters: Mutex<Vec<SignalFilter>>,
}

fn dialog_app(pid: u32) -> AppInfo {
    AppInfo {
        name: "zenity".into(),
        pid: ProcessId::new(pid),
        bundle_id: None,
        process_instance: Some(format!("boot:{pid}")),
        presentation: None,
    }
}

fn dialog_window(id: &str, pid: u32) -> WindowInfo {
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

impl TwoDialogs {
    fn new(windows_listed: bool) -> Self {
        Self {
            windows_listed,
            honours_process: true,
            apps_listed: true,
            filters: Mutex::new(Vec::new()),
        }
    }
}

impl ObservationOps for TwoDialogs {
    fn list_windows(
        &self,
        _filter: &WindowFilter,
        _deadline: Deadline,
    ) -> Result<Vec<WindowInfo>, AdapterError> {
        if !self.windows_listed {
            return Ok(Vec::new());
        }
        Ok(vec![dialog_window("w-a", 10), dialog_window("w-b", 11)])
    }

    fn list_apps(&self, _deadline: Deadline) -> Result<Vec<AppInfo>, AdapterError> {
        Ok(vec![dialog_app(10), dialog_app(11)])
    }
}

impl ActionOps for TwoDialogs {}
impl InputOps for TwoDialogs {}

impl SystemOps for TwoDialogs {
    fn capture_signal_baseline(
        &self,
        filter: &SignalFilter,
        _deadline: Deadline,
    ) -> Result<SignalBaseline, AdapterError> {
        self.filters.lock().unwrap().push(filter.clone());
        let apps = if self.apps_listed {
            [dialog_app(10), dialog_app(11)]
                .into_iter()
                .filter(|app| {
                    !self.honours_process
                        || filter.process.as_ref().is_none_or(|p| p.pid == app.pid)
                })
                .collect()
        } else {
            Vec::new()
        };
        let windows = if self.honours_process {
            Vec::new()
        } else {
            vec![dialog_window("w-a", 10), dialog_window("w-b", 11)]
        };
        Ok(SignalBaseline {
            windows,
            apps,
            surfaces: Vec::new(),
            completeness: SignalCompleteness::complete(),
        })
    }
}

fn target<'a>(window_id: Option<&'a str>) -> EventWaitTarget<'a> {
    EventWaitTarget {
        event: "window-closed",
        app: Some("zenity"),
        window_id,
    }
}

fn deadline() -> Deadline {
    Deadline::detached_after(5_000).unwrap()
}

#[test]
fn a_name_shared_by_two_processes_fails_at_capture_without_a_window_id() {
    let adapter = TwoDialogs::new(true);
    let error = capture_event_baseline(&target(None), &adapter, deadline()).unwrap_err();
    let AppError::Adapter(error) = error else {
        panic!("expected an adapter error");
    };
    assert_eq!(error.code, ErrorCode::AmbiguousTarget);
}

#[test]
fn a_window_id_picks_the_one_instance_and_scopes_the_capture_to_it() {
    let adapter = TwoDialogs::new(true);
    let baseline = capture_event_baseline(&target(Some("w-b")), &adapter, deadline()).unwrap();
    assert_eq!(baseline.apps.len(), 1);
    assert_eq!(baseline.apps[0].pid.get(), 11);
    let filters = adapter.filters.lock().unwrap();
    assert_eq!(filters[0].process.as_ref().unwrap().pid, 11);
}

#[test]
fn a_window_id_the_list_does_not_hold_still_fails_an_ambiguous_name_at_capture() {
    let adapter = TwoDialogs::new(false);
    let error = capture_event_baseline(&target(Some("w-gone")), &adapter, deadline()).unwrap_err();
    let AppError::Adapter(error) = error else {
        panic!("expected an adapter error");
    };
    assert_eq!(error.code, ErrorCode::AmbiguousTarget);
}

#[test]
fn an_adapter_that_ignores_the_process_scope_is_caught_by_the_captured_baseline() {
    let mut adapter = TwoDialogs::new(true);
    adapter.honours_process = false;
    let error = capture_event_baseline(&target(Some("w-b")), &adapter, deadline()).unwrap_err();
    let AppError::Adapter(error) = error else {
        panic!("expected an adapter error");
    };
    assert_eq!(error.code, ErrorCode::AmbiguousTarget);
    let mut windows_only = TwoDialogs::new(true);
    windows_only.honours_process = false;
    windows_only.apps_listed = false;
    assert!(capture_event_baseline(&target(Some("w-b")), &windows_only, deadline()).is_err());
}

#[test]
fn an_app_launch_wait_does_not_need_one_instance() {
    let adapter = TwoDialogs::new(true);
    let launched = EventWaitTarget {
        event: "app-launched",
        app: Some("zenity"),
        window_id: None,
    };
    assert!(capture_event_baseline(&launched, &adapter, deadline()).is_ok());
}

#[test]
fn a_direct_wait_names_the_instance_its_window_id_belongs_to() {
    let adapter = TwoDialogs::new(true);
    let input = crate::commands::wait_event_input::EventWaitInput {
        event: "window-closed".into(),
        app: Some("zenity".into()),
        window_id: Some("w-a".into()),
        window_title: None,
        timeout_ms: 1_000,
    };
    let filter = crate::commands::wait_event_filter::signal_filter(
        &input,
        &EventKind::WindowClosed,
        None,
        &adapter,
        deadline(),
    )
    .unwrap();
    assert_eq!(filter.process.unwrap().pid, 10);
}
