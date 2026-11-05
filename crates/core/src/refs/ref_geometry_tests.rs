use super::*;

fn geometry(bounds_space: Option<crate::BoundsSpace>) -> RefGeometry {
    RefGeometry {
        bounds: None,
        bounds_hash: Some(42),
        bounds_space,
    }
}

#[test]
fn only_a_screen_space_hash_is_a_baseline_for_live_bounds() {
    assert_eq!(geometry(None).screen_bounds_hash(), Some(42));
    assert_eq!(
        geometry(Some(crate::BoundsSpace::Screen)).screen_bounds_hash(),
        Some(42)
    );
    assert_eq!(
        geometry(Some(crate::BoundsSpace::Window)).screen_bounds_hash(),
        None
    );
}
