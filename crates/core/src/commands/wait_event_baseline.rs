use crate::commands::wait_event::parse_event_kind;
use crate::commands::wait_event_filter::window_process;
use crate::system::app_info::app_name_matches;
use crate::{
    AdapterError, AppError, Deadline, EventKind, PlatformAdapter, ProcessIdentity, SignalBaseline,
    SignalFilter,
};

/// What a `wait --event` entry names, as a batch reads it before the entry
/// ahead of the wait runs.
pub struct EventWaitTarget<'a> {
    pub event: &'a str,
    pub app: Option<&'a str>,
    pub window_id: Option<&'a str>,
}

/// Captures the baseline a following `wait --event` will diff against,
/// scoped the way that wait scopes itself. A name shared by several
/// application instances, with no window id that picks one, fails here, so a
/// batch can refuse to start the entry that precedes the wait.
pub fn capture_event_baseline(
    target: &EventWaitTarget<'_>,
    adapter: &dyn PlatformAdapter,
    deadline: Deadline,
) -> Result<SignalBaseline, AppError> {
    let requested = parse_event_kind(target.event)?;
    let launched = matches!(requested, EventKind::AppLaunched);
    let process = if launched {
        None
    } else {
        window_process(target.window_id, target.app, adapter, deadline)
    };
    let filter = SignalFilter {
        app: target.app.map(str::to_owned),
        process,
    };
    let baseline = adapter.capture_signal_baseline(&filter, deadline)?;
    if let Some(app) = target.app
        && !launched
    {
        validate_one_instance(&baseline, app, filter.process.as_ref())?;
    }
    Ok(baseline)
}

/// Fails when the captured baseline holds applications or windows of more
/// than one process under the name `app`, or of any process but `followed`
/// when the wait is scoped to one. The check reads the baseline, not the
/// filter, so an adapter that returns every matching instance despite a
/// process scope is caught before the entry ahead of the wait runs.
fn validate_one_instance(
    baseline: &SignalBaseline,
    app: &str,
    followed: Option<&ProcessIdentity>,
) -> Result<(), AppError> {
    let mut pids: Vec<u32> = baseline
        .apps
        .iter()
        .filter(|candidate| candidate.matches_identifier(app))
        .map(|candidate| candidate.pid.get())
        .chain(
            baseline
                .windows
                .iter()
                .filter(|window| app_name_matches(&window.app, app))
                .map(|window| window.pid.get()),
        )
        .collect();
    pids.sort_unstable();
    pids.dedup();
    let several = match followed {
        Some(process) => pids.iter().any(|pid| *pid != process.pid.get()),
        None => pids.len() > 1,
    };
    if several {
        return Err(AdapterError::ambiguous_target(format!(
            "Multiple application instances matched '{app}' in the event baseline"
        ))
        .into());
    }
    Ok(())
}

#[cfg(test)]
#[path = "wait_event_baseline_tests.rs"]
mod tests;
