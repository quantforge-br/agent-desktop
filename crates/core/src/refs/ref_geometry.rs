use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RefGeometry {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bounds: Option<crate::Rect>,
    pub bounds_hash: Option<u64>,
    /// The frame `bounds` is measured in. Absent means a screen rectangle,
    /// which older refmaps stored without a tag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bounds_space: Option<crate::BoundsSpace>,
}

impl RefGeometry {
    /// The stored hash when it measures a screen rectangle, the frame live
    /// element bounds are read in. A window-space hash identifies the element
    /// but says nothing about whether its live bounds have moved.
    pub fn screen_bounds_hash(&self) -> Option<u64> {
        match self.bounds_space.unwrap_or_default() {
            crate::BoundsSpace::Screen => self.bounds_hash,
            crate::BoundsSpace::Window => None,
        }
    }
}

#[cfg(test)]
#[path = "ref_geometry_tests.rs"]
mod tests;
