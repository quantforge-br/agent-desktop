use super::*;

struct SurfaceOpensLateAdapter {
    observations: AtomicUsize,
    opens_after: usize,
}

fn surface_absent() -> AdapterError {
    AdapterError::new(ErrorCode::ElementNotFound, "no open menu")
        .with_details(json!({ "kind": crate::SURFACE_ABSENT, "surface": "menu" }))
}

impl ObservationOps for SurfaceOpensLateAdapter {
    fn observe_tree(
        &self,
        root: crate::locator::live_locator::ObservationRoot<'_>,
        _request: &crate::locator::live_locator::ObservationRequest,
    ) -> Result<crate::locator::live_locator::ObservedTree, AdapterError> {
        if self.observations.fetch_add(1, Ordering::SeqCst) < self.opens_after {
            return Err(surface_absent());
        }
        crate::adapter::observed_tree(&root, window_node(vec![button_node("Preferences")]))
    }

    fn list_windows(
        &self,
        filter: &WindowFilter,
        deadline: crate::Deadline,
    ) -> Result<Vec<WindowInfo>, AdapterError> {
        StaticTreeAdapter {
            tree: window_node(Vec::new()),
        }
        .list_windows(filter, deadline)
    }

    fn get_tree(
        &self,
        _win: &WindowInfo,
        _opts: &crate::adapter::TreeOptions,
        _deadline: crate::Deadline,
    ) -> Result<AccessibilityNode, AdapterError> {
        if self.observations.fetch_add(1, Ordering::SeqCst) < self.opens_after {
            return Err(surface_absent());
        }
        Ok(window_node(vec![button_node("Preferences")]))
    }
}

impl ActionOps for SurfaceOpensLateAdapter {}
impl InputOps for SurfaceOpensLateAdapter {}
impl SystemOps for SurfaceOpensLateAdapter {}

#[test]
fn a_wait_for_an_element_of_a_surface_not_open_yet_polls_until_it_opens() {
    let _guard = HomeGuard::new();
    let adapter = SurfaceOpensLateAdapter {
        observations: AtomicUsize::new(0),
        opens_after: 2,
    };
    let value = execute(
        WaitSelectorInput {
            timeout_ms: 5_000,
            ..base_input("button:Preferences", false)
        },
        &adapter,
        &CommandContext::default(),
    )
    .expect("the element appears once its surface opens");
    assert_eq!(value["matched_selector"], "button:Preferences");
    assert!(adapter.observations.load(Ordering::SeqCst) > 2);
}

#[test]
fn a_wait_for_an_element_to_go_is_over_when_its_surface_is_closed() {
    let _guard = HomeGuard::new();
    let value = execute(
        WaitSelectorInput {
            timeout_ms: 30_000,
            ..base_input("button:Preferences", true)
        },
        &SurfaceOpensLateAdapter {
            observations: AtomicUsize::new(0),
            opens_after: usize::MAX,
        },
        &CommandContext::default(),
    )
    .expect("a closed surface has no element left");
    assert_eq!(value["target_absent"], true);
}

#[test]
fn an_element_not_found_without_the_surface_kind_still_ends_the_wait() {
    let _guard = HomeGuard::new();
    let error = execute(
        WaitSelectorInput {
            timeout_ms: 30_000,
            ..base_input("button:Preferences", false)
        },
        &CodeErrorAdapter {
            code: ErrorCode::ElementNotFound,
        },
        &CommandContext::default(),
    )
    .expect_err("an unmarked ELEMENT_NOT_FOUND is terminal");
    assert_eq!(error.code(), "ELEMENT_NOT_FOUND");
}
