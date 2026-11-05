use std::ffi::OsString;
use std::path::PathBuf;

use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode};
use serde_json::json;

use crate::input::clipboard::command_runner::{CommandRunner, ProcessRunner, RunOutcome, RunSpec};
use crate::input::clipboard::wayland::locate;

const COMMANDS: [&str; 2] = ["qs", "quickshell"];
const ERROR_PREFIX: &str = "ERROR";

/// The shell's IPC, addressed to one quickshell process. A call names the
/// handler and the function; the answer is the text the function returns.
pub(crate) trait Ipc {
    fn call(
        &self,
        target: &str,
        function: &str,
        deadline: Deadline,
    ) -> Result<String, AdapterError>;

    /// The process's own listing of its handlers and their functions.
    fn listing(&self, deadline: Deadline) -> Result<String, AdapterError>;

    /// Whether the process exposes the handler function.
    fn exposes(
        &self,
        target: &str,
        function: &str,
        deadline: Deadline,
    ) -> Result<bool, AdapterError> {
        let listing = self.listing(deadline)?;
        Ok(handler_functions(&listing, target).any(|name| name == function))
    }
}

/// IPC through the `qs ipc --pid` command, so a call can only reach the
/// process the detection named.
pub(crate) struct QuickshellIpc {
    pid: u32,
    search_path: Option<OsString>,
}

impl QuickshellIpc {
    pub(crate) fn of(pid: u32) -> Self {
        Self {
            pid,
            search_path: std::env::var_os("PATH"),
        }
    }

    fn program(&self) -> Result<PathBuf, AdapterError> {
        COMMANDS
            .iter()
            .find_map(|name| locate(name, self.search_path.as_deref()))
            .ok_or_else(missing_command)
    }

    fn run(
        &self,
        words: &[&str],
        target: &str,
        deadline: Deadline,
    ) -> Result<String, AdapterError> {
        let mut args = vec!["ipc".to_owned(), "--pid".to_owned(), self.pid.to_string()];
        args.extend(words.iter().map(|word| (*word).to_owned()));
        let spec = RunSpec {
            program: self.program()?,
            args,
            stdin: None,
            capture_stdout: true,
        };
        match (ProcessRunner { env: Vec::new() }).run(&spec, deadline) {
            RunOutcome::Exited {
                code: Some(0),
                stdout,
                ..
            } => Ok(String::from_utf8_lossy(&stdout).trim().to_owned()),
            RunOutcome::Exited { stderr, .. } => Err(refused(target, words, &stderr)),
            RunOutcome::TimedOut | RunOutcome::OverCap => Err(AdapterError::new(
                ErrorCode::Timeout,
                "The shell did not answer its IPC call in time",
            )
            .with_suggestion("Retry the command; the shell may be busy")
            .with_disposition(DeliverySemantics::delivered_unverified())),
            RunOutcome::NotStarted(reason) => Err(missing_command().with_platform_detail(reason)),
        }
    }
}

impl Ipc for QuickshellIpc {
    fn call(
        &self,
        target: &str,
        function: &str,
        deadline: Deadline,
    ) -> Result<String, AdapterError> {
        let reply = self.run(&["call", target, function], target, deadline)?;
        if reply.starts_with(ERROR_PREFIX) {
            return Err(refused(target, &[function], &reply));
        }
        Ok(reply)
    }

    fn listing(&self, deadline: Deadline) -> Result<String, AdapterError> {
        self.run(&["show"], "show", deadline)
    }
}

/// The function names a handler lists in the output of `qs ipc show`.
pub(super) fn handler_functions<'a>(
    listing: &'a str,
    target: &'a str,
) -> impl Iterator<Item = &'a str> {
    let mut inside = false;
    listing.lines().filter_map(move |line| {
        if let Some(name) = line.strip_prefix("target ") {
            inside = name.trim() == target;
            return None;
        }
        let signature = line.trim_start().strip_prefix("function ")?;
        inside.then(|| signature.split('(').next().unwrap_or(signature).trim())
    })
}

fn missing_command() -> AdapterError {
    AdapterError::new(
        ErrorCode::PlatformNotSupported,
        "The quickshell command is not installed, so the shell's IPC cannot be reached",
    )
    .with_suggestion("Install quickshell (it provides the qs command) and retry")
    .with_details(json!({ "ipc": "missing_command" }))
    .with_disposition(DeliverySemantics::not_delivered())
}

fn refused(target: &str, words: &[&str], answer: &str) -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionFailed,
        format!("The shell refused its '{target}' IPC call"),
    )
    .with_suggestion(
        "Check that the shell build offers this handler: run 'qs ipc --pid <pid> show'",
    )
    .with_details(json!({ "ipc": "refused", "target": target, "call": words, "answer": answer }))
    .with_disposition(DeliverySemantics::not_delivered())
}

#[cfg(test)]
#[path = "ipc_tests.rs"]
mod tests;
