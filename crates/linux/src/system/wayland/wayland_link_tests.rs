use std::os::unix::net::UnixStream;
use std::time::{Duration, Instant};

use agent_desktop_core::{Deadline, ErrorCode};
use wayland_client::Connection;

use super::WaylandLink;

#[test]
fn a_compositor_that_never_answers_the_registry_is_a_timeout_within_the_deadline() {
    let (near, _far) = UnixStream::pair().unwrap();
    let connection = Connection::from_socket(near).unwrap();
    let started = Instant::now();
    let error = WaylandLink::over(connection, Deadline::after(150).unwrap())
        .err()
        .unwrap();
    assert_eq!(error.code, ErrorCode::Timeout);
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn a_compositor_that_hangs_up_is_a_lost_connection_not_a_timeout() {
    let (near, far) = UnixStream::pair().unwrap();
    drop(far);
    let connection = Connection::from_socket(near).unwrap();
    let error = WaylandLink::over(connection, Deadline::after(2_000).unwrap())
        .err()
        .unwrap();
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(error.details.unwrap()["cause"], "wayland_connection");
}
