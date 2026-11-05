use agent_desktop_core::{DeliverySemantics, ErrorCode};

use super::gate;
use crate::input::backend::Backend;

#[test]
fn a_wayland_session_without_the_pointer_service_refuses_before_anything_moves() {
    let error = gate(Ok(Backend::Wayland), || false).expect_err("refused");

    assert_eq!(error.code, ErrorCode::ActionNotSupported);
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    assert_eq!(
        error
            .details
            .as_ref()
            .and_then(|details| details.get("physical_delivery_started")),
        Some(&serde_json::json!(false))
    );
}

#[test]
fn a_wayland_session_with_the_pointer_service_may_click() {
    assert!(gate(Ok(Backend::Wayland), || true).is_ok());
}

#[test]
fn the_service_is_not_probed_outside_wayland() {
    let probed = std::cell::Cell::new(false);
    let result = gate(
        Err(agent_desktop_core::AdapterError::internal("no display")),
        || {
            probed.set(true);
            false
        },
    );

    assert!(result.is_ok());
    assert!(!probed.get());
}
