use std::ffi::OsString;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use agent_desktop_core::{Deadline, DeliverySemantics, ErrorCode};

use super::{Ipc, QuickshellIpc, handler_functions};

const SCRIPT: &str = r#"#!/bin/sh
case "$*" in ready) exit 0;; esac
echo "$@" >> "$0.args"
case "$*" in
  *"ipc --pid 4242 show"*) printf 'target notifications\n  function open(): string\n  function close(): string\ntarget dock\n  function reveal(): string\n';;
  *"call notifications open"*) echo NOTIFICATION_MODAL_OPEN_SUCCESS;;
  *"call dock hide"*) echo "ERROR: Function not found";;
  *"call broken"*) echo "Target not found" >&2; exit 3;;
esac
"#;

struct Bin(PathBuf);

impl Bin {
    fn with(name: &str, script: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos());
        let dir = std::env::temp_dir().join(format!("ad-ipc-{}-{nanos}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let program = dir.join(name);
        std::fs::write(&program, script).expect("script");
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).expect("mode");
        let ready = Self(dir);
        ready.settle(&program);
        ready
    }

    fn settle(&self, program: &Path) {
        for _ in 0..100 {
            match std::process::Command::new(program).arg("ready").output() {
                Err(error) if error.kind() == std::io::ErrorKind::ExecutableFileBusy => {
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
                _ => return,
            }
        }
    }

    fn ipc(&self) -> QuickshellIpc {
        QuickshellIpc {
            pid: 4242,
            search_path: Some(OsString::from(&self.0)),
        }
    }

    fn calls(&self, name: &str) -> String {
        std::fs::read_to_string(self.0.join(format!("{name}.args"))).unwrap_or_default()
    }
}

impl Drop for Bin {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn deadline() -> Deadline {
    Deadline::after(10_000).expect("deadline")
}

#[test]
fn a_call_is_addressed_to_the_detected_process_and_answers_with_its_reply() {
    let bin = Bin::with("qs", SCRIPT);

    let reply = bin
        .ipc()
        .call("notifications", "open", deadline())
        .expect("called");

    assert_eq!(reply, "NOTIFICATION_MODAL_OPEN_SUCCESS");
    assert_eq!(
        bin.calls("qs").trim(),
        "ipc --pid 4242 call notifications open"
    );
}

#[test]
fn an_error_reply_is_a_refusal_that_delivered_nothing() {
    let bin = Bin::with("qs", SCRIPT);

    let error = bin
        .ipc()
        .call("dock", "hide", deadline())
        .expect_err("refused");

    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    assert_eq!(
        error.details.expect("details")["answer"],
        serde_json::json!("ERROR: Function not found")
    );
}

#[test]
fn a_failing_command_is_a_refusal_naming_the_call() {
    let bin = Bin::with("qs", SCRIPT);

    let error = bin
        .ipc()
        .call("broken", "x", deadline())
        .expect_err("refused");

    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(
        error.details.expect("details")["target"],
        serde_json::json!("broken")
    );
}

#[test]
fn the_process_listing_says_which_functions_a_handler_offers() {
    let bin = Bin::with("qs", SCRIPT);
    let ipc = bin.ipc();

    assert!(
        ipc.exposes("notifications", "open", deadline())
            .expect("listed")
    );
    assert!(ipc.exposes("dock", "reveal", deadline()).expect("listed"));
    assert!(!ipc.exposes("dock", "open", deadline()).expect("listed"));
    assert!(!ipc.exposes("tray", "open", deadline()).expect("listed"));
}

#[test]
fn quickshell_is_used_when_there_is_no_qs_command() {
    let bin = Bin::with("quickshell", SCRIPT);

    let reply = bin
        .ipc()
        .call("notifications", "open", deadline())
        .expect("called");

    assert_eq!(reply, "NOTIFICATION_MODAL_OPEN_SUCCESS");
    assert!(!bin.calls("quickshell").is_empty());
}

#[test]
fn a_missing_command_is_a_platform_limit_that_delivered_nothing() {
    let ipc = QuickshellIpc {
        pid: 4242,
        search_path: Some(OsString::from(Path::new("/nonexistent-dir-for-ad-tests"))),
    };

    let error = ipc
        .call("notifications", "open", deadline())
        .expect_err("missing");

    assert_eq!(error.code, ErrorCode::PlatformNotSupported);
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
}

#[test]
fn handler_functions_are_read_only_from_the_named_handler() {
    let listing = "target a\n  function one(): void\ntarget b\n  function two(x: string): string\n";

    assert_eq!(
        handler_functions(listing, "b").collect::<Vec<_>>(),
        vec!["two"]
    );
    assert_eq!(
        handler_functions(listing, "a").collect::<Vec<_>>(),
        vec!["one"]
    );
}
