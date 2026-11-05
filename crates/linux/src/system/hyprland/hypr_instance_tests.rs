use std::io::{Read, Write};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

use agent_desktop_core::{Deadline, ErrorCode};

use super::HyprInstance;

fn scratch(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ad-hypr-{label}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn instance_dir(runtime: &Path, signature: &str, served: &str) -> PathBuf {
    let directory = runtime.join("hypr").join(signature);
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("hyprland.lock"), format!("4242\n{served}\n")).unwrap();
    directory
}

#[test]
fn an_instance_serving_another_wayland_socket_is_not_this_session() {
    let runtime = scratch("served");
    let directory = instance_dir(&runtime, "abc_1", "wayland-1");
    let _listener = UnixListener::bind(directory.join(".socket.sock")).unwrap();
    let root = runtime.to_str().unwrap();
    assert!(HyprInstance::find(root, "abc_1", "wayland-1").is_some());
    assert!(HyprInstance::find(root, "abc_1", &format!("{root}/wayland-1")).is_some());
    assert!(HyprInstance::find(root, "abc_1", "wayland-2").is_none());
}

#[test]
fn a_signature_that_could_leave_the_runtime_directory_is_refused() {
    let runtime = scratch("signature");
    let root = runtime.to_str().unwrap();
    instance_dir(&runtime, "abc_1", "wayland-1");
    assert!(HyprInstance::find(root, "../abc_1", "wayland-1").is_none());
    assert!(HyprInstance::find(root, "", "wayland-1").is_none());
}

#[test]
fn an_instance_without_its_request_socket_is_absent() {
    let runtime = scratch("socketless");
    instance_dir(&runtime, "abc_1", "wayland-1");
    assert!(HyprInstance::find(runtime.to_str().unwrap(), "abc_1", "wayland-1").is_none());
}

#[test]
fn a_request_returns_the_whole_reply_after_the_command_is_read() {
    let socket = scratch("reply").join("request.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut command = [0_u8; 9];
        stream.read_exact(&mut command).unwrap();
        stream.write_all(b"[{\"id\": 1}]").unwrap();
        String::from_utf8(command.to_vec()).unwrap()
    });
    let reply = HyprInstance::at(socket)
        .request("j/clients", Deadline::after(2_000).unwrap())
        .unwrap();
    assert_eq!(reply, "[{\"id\": 1}]");
    assert_eq!(server.join().unwrap(), "j/clients");
}

#[test]
fn a_compositor_that_never_answers_is_a_timeout() {
    let socket = scratch("silent").join("silent.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        thread::sleep(Duration::from_millis(600));
        drop(stream);
    });
    let error = HyprInstance::at(socket)
        .request("j/monitors", Deadline::after(150).unwrap())
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::Timeout);
    assert_eq!(error.details.unwrap()["hyprland_ipc"], "silent");
    server.join().unwrap();
}

#[test]
fn a_missing_socket_is_unreachable_not_a_timeout() {
    let error = HyprInstance::at(scratch("gone").join("gone.sock"))
        .request("j/monitors", Deadline::after(500).unwrap())
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::PlatformNotSupported);
    assert_eq!(error.details.unwrap()["hyprland_ipc"], "unreachable");
}
