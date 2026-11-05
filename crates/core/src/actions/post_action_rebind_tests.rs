use super::*;
use crate::{ActionOps, Deadline, InputOps, ObservationOps, SystemOps};
use std::sync::atomic::{AtomicBool, Ordering};

/// A toolbar that re-renders its toggle when the toggle is pressed: the
/// original element is destroyed and a new one, `Replacement`, shows the
/// state the replacement's `checked` holds.
struct RerenderingToolbar {
    dispatched: AtomicBool,
    replacement_checked: bool,
}

struct Replacement;

fn switch(checked: bool) -> LiveElement {
    LiveElement {
        identity: crate::adapter::live_identity("Toggle Primary Side Bar"),
        state: crate::ElementState {
            role: "switch".into(),
            states: if checked {
                vec!["checked".into()]
            } else {
                vec![]
            },
            value: None,
            enabled: Some(true),
            hidden: Some(false),
            offscreen: Some(false),
        },
        states_complete: true,
        bounds: None,
        available_actions: vec![],
    }
}

impl ObservationOps for RerenderingToolbar {
    fn get_live_element(
        &self,
        handle: &NativeHandle,
        _: Deadline,
    ) -> Result<LiveElement, AdapterError> {
        if handle.downcast_ref::<Replacement>().is_some() {
            return Ok(switch(self.replacement_checked));
        }
        if self.dispatched.load(Ordering::SeqCst) {
            return Err(AdapterError::stale_ref_because(
                "Stored ref's element no longer exists",
            ));
        }
        Ok(switch(true))
    }
}

impl ActionOps for RerenderingToolbar {
    fn execute_action(
        &self,
        _: &NativeHandle,
        request: ActionRequest,
        _: &InteractionLease,
    ) -> Result<ActionResult, AdapterError> {
        self.dispatched.store(true, Ordering::SeqCst);
        Ok(ActionResult::delivered_unverified(request.action.name()))
    }
}

impl InputOps for RerenderingToolbar {}
impl SystemOps for RerenderingToolbar {}

fn toggle(adapter: &RerenderingToolbar) -> Result<ActionResult, AdapterError> {
    execute_verified_action_rebinding(
        adapter,
        &NativeHandle::null(),
        ActionRequest::headless(Action::Toggle),
        &InteractionLease::guarded(Deadline::after(5_000).unwrap(), ()).unwrap(),
        &|_| Some(NativeHandle::new(Replacement)),
    )
}

#[test]
fn a_replacement_showing_the_change_verifies_a_toggle_that_destroyed_its_element() {
    let adapter = RerenderingToolbar {
        dispatched: AtomicBool::new(false),
        replacement_checked: false,
    };
    let result = toggle(&adapter).unwrap();
    assert_eq!(
        result.disposition(),
        DeliverySemantics::delivered_verified()
    );
    assert_eq!(
        result.details.unwrap()["verification_target"],
        "replacement"
    );
}

#[test]
fn a_replacement_still_in_the_old_state_leaves_the_verification_failure() {
    let adapter = RerenderingToolbar {
        dispatched: AtomicBool::new(false),
        replacement_checked: true,
    };
    let error = toggle(&adapter).unwrap_err();
    assert_eq!(error.code, ErrorCode::StaleRef);
    assert_eq!(error.disposition, DeliverySemantics::delivered_unverified());
}
