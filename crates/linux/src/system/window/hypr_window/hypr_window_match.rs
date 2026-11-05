use agent_desktop_core::Rect;

use super::hypr_unmatched::Unmatched;
use super::hypr_window_key::WindowKey;
use crate::system::hyprland::hypr_client::HyprClient;

const BOUNDS_TOLERANCE: f64 = 2.0;

fn same_place(frame: &Rect, bounds: &Rect) -> bool {
    (frame.x - bounds.x).abs() <= BOUNDS_TOLERANCE
        && (frame.y - bounds.y).abs() <= BOUNDS_TOLERANCE
        && (frame.width - bounds.width).abs() <= BOUNDS_TOLERANCE
        && (frame.height - bounds.height).abs() <= BOUNDS_TOLERANCE
}

/// The one mapped client of the window's process with the window's title.
/// A process with a single mapped client answers with it even when the
/// titles differ, but only for a window its listing shows alone and, when
/// the window's bounds are known, only at the client's place: nothing else
/// can show that window. Several clients sharing the title are told apart
/// by the listed bounds only.
pub(crate) fn match_client<'a>(
    clients: &'a [HyprClient],
    key: WindowKey<'_>,
) -> Result<&'a HyprClient, Unmatched> {
    let owned: Vec<&HyprClient> = clients
        .iter()
        .filter(|client| client.state.mapped && client.owner.pid == key.pid)
        .collect();
    let titled: Vec<&HyprClient> = owned
        .iter()
        .copied()
        .filter(|client| client.owner.title == key.title)
        .collect();
    let lone = match owned.as_slice() {
        [only] => {
            key.alone
                && key
                    .bounds
                    .is_none_or(|bounds| same_place(&only.frame, bounds))
        }
        _ => false,
    };
    let candidates = match (titled.len(), lone) {
        (0, true) => owned,
        (0, false) => return Err(Unmatched::Missing),
        _ => titled,
    };
    if let [only] = candidates.as_slice() {
        return Ok(only);
    }
    let placed: Vec<&HyprClient> = key
        .bounds
        .map(|bounds| {
            candidates
                .iter()
                .copied()
                .filter(|client| same_place(&client.frame, bounds))
                .collect()
        })
        .unwrap_or_default();
    match placed.as_slice() {
        [only] => Ok(only),
        _ => Err(Unmatched::Several),
    }
}

#[cfg(test)]
#[path = "hypr_window_match_tests.rs"]
mod tests;
