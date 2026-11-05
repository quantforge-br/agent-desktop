use agent_desktop_core::{AdapterError, Deadline};
use zbus::Connection;

use crate::system::process::toolkit_id::ToolkitId;
use crate::system::{SharedBus, within};
use crate::tree::properties::node_read::call;
use crate::tree::properties::toolkit_trust::screen_is_toplevel_relative;
use crate::tree::{AtspiElement, COMPONENT_INTERFACE, Extents};

pub(crate) const WINDOW_COORDINATES: u32 = 1;
const SCREEN_COORDINATES: u32 = 0;

/// One `GetExtents` in window coordinates. Unavailable or malformed geometry
/// is unknown; a lost connection ends the read.
pub(crate) async fn read_window_extents_async(
    connection: &Connection,
    element: &AtspiElement,
) -> Result<Option<Extents>, AdapterError> {
    read_extents_async(connection, element, WINDOW_COORDINATES).await
}

pub(crate) async fn read_extents_async(
    connection: &Connection,
    element: &AtspiElement,
    coordinates: u32,
) -> Result<Option<Extents>, AdapterError> {
    match crate::tree::properties::interface_support::supports(
        connection,
        element,
        COMPONENT_INTERFACE,
    )
    .await
    {
        Ok(true) => {}
        Err(error) if crate::system::is_connection_failure(&error) => {
            return Err(crate::system::classify(&error));
        }
        _ => return Ok(None),
    }
    let reply = match call(
        connection,
        element,
        COMPONENT_INTERFACE,
        "GetExtents",
        &(coordinates,),
    )
    .await
    {
        Ok(reply) => reply,
        Err(error) if crate::system::is_connection_failure(&error) => {
            return Err(crate::system::classify(&error));
        }
        Err(_) => return Ok(None),
    };
    let Ok((x, y, width, height)) = reply.body().deserialize::<(i32, i32, i32, i32)>() else {
        return Ok(None);
    };
    Ok(Some(Extents {
        x,
        y,
        width,
        height,
    }))
}

/// An element's rectangle in window coordinates, or `None` when it cannot be
/// read inside the deadline.
pub(crate) fn read_window_extents(
    bus: &SharedBus,
    element: &AtspiElement,
    deadline: Deadline,
) -> Option<Extents> {
    let connection = bus.get(deadline).ok()?;
    within(
        deadline,
        read_window_extents_async(connection.inner(), element),
    )
    .ok()
    .flatten()
}

/// The rectangle that places `element` inside its toplevel. That is the
/// window rectangle, except for a GTK 3 client drawing natively on Hyprland:
/// its window rectangles are relative to the popup surface that holds the
/// node, and its screen rectangles are relative to the toplevel, popups
/// included.
pub(crate) async fn read_placing_extents_async(
    connection: &Connection,
    element: &AtspiElement,
    toolkit: &ToolkitId,
) -> Result<Option<Extents>, AdapterError> {
    let coordinates = if screen_is_toplevel_relative(toolkit) {
        SCREEN_COORDINATES
    } else {
        WINDOW_COORDINATES
    };
    read_extents_async(connection, element, coordinates).await
}

/// [`read_placing_extents_async`] under the deadline; `None` when it cannot
/// be read in time.
pub(crate) fn read_placing_extents(
    bus: &SharedBus,
    element: &AtspiElement,
    toolkit: &ToolkitId,
    deadline: Deadline,
) -> Option<Extents> {
    let connection = bus.get(deadline).ok()?;
    within(
        deadline,
        read_placing_extents_async(connection.inner(), element, toolkit),
    )
    .ok()
    .flatten()
}
