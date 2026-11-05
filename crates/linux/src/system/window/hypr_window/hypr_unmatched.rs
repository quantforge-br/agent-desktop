use agent_desktop_core::{AdapterError, DeliverySemantics, ErrorCode};
use serde_json::json;

/// Why a listed window has no single Hyprland client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Unmatched {
    Missing,
    Several,
}

impl Unmatched {
    /// The refusal for an operation on a window without a single client.
    pub(crate) fn into_error(self) -> AdapterError {
        match self {
            Self::Missing => AdapterError::new(
                ErrorCode::WindowNotFound,
                "Hyprland lists no mapped window of this process for this window",
            )
            .with_suggestion("Run 'list-windows' to find the window again")
            .with_details(
                json!({ "cause": "hyprland_client_missing", "physical_delivery_started": false }),
            ),
            Self::Several => AdapterError::ambiguous_target(
                "More than one Hyprland window of this process matches this window, so none is changed",
            )
            .with_suggestion("Give the windows different titles, then retry")
            .with_details(
                json!({ "cause": "hyprland_client_ambiguous", "physical_delivery_started": false }),
            ),
        }
        .with_disposition(DeliverySemantics::not_delivered())
    }
}
