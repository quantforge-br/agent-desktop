use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode};
use zbus::Connection;
use zbus::zvariant::OwnedValue;

use crate::system::{ACCESSIBLE_INTERFACE, PROPERTIES_INTERFACE, SharedBus, within};
use crate::tree::AtspiElement;
use crate::tree::vocabulary::state_bits::{CHECKED, StateBits};

const ACTION_INTERFACE: &str = "org.a11y.atspi.Action";
const PRESS: &str = "Press";

/// What a landmark rule needs about a node beyond its light reading.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Details {
    pub(crate) description: String,
    pub(crate) checked: bool,
}

/// Reads and presses nodes of the shell's accessibility application; the bus
/// implements it and a test substitutes a described tree.
pub(crate) trait Reader {
    fn details(&self, element: &AtspiElement) -> Result<Details, AdapterError>;
    fn press(&self, element: &AtspiElement) -> Result<(), AdapterError>;
}

/// The reader that talks to the accessibility bus.
pub(crate) struct LiveReader {
    connection: Connection,
    deadline: Deadline,
}

impl LiveReader {
    pub(crate) fn new(bus: &SharedBus, deadline: Deadline) -> Result<Self, AdapterError> {
        Ok(Self {
            connection: bus.get(deadline)?.inner().clone(),
            deadline,
        })
    }
}

async fn description_of(connection: &Connection, element: &AtspiElement) -> String {
    connection
        .call_method(
            Some(element.bus_name()),
            element.path(),
            Some(PROPERTIES_INTERFACE),
            "Get",
            &(ACCESSIBLE_INTERFACE, "Description"),
        )
        .await
        .ok()
        .and_then(|reply| reply.body().deserialize::<OwnedValue>().ok())
        .and_then(|value| value.downcast_ref::<&str>().ok().map(str::to_owned))
        .unwrap_or_default()
}

async fn checked_of(connection: &Connection, element: &AtspiElement) -> bool {
    let reply = connection
        .call_method(
            Some(element.bus_name()),
            element.path(),
            Some(ACCESSIBLE_INTERFACE),
            "GetState",
            &(),
        )
        .await
        .ok();
    reply
        .and_then(|reply| reply.body().deserialize::<Vec<u32>>().ok())
        .is_some_and(|words| StateBits::new(&words).has(CHECKED))
}

async fn action_names(
    connection: &Connection,
    element: &AtspiElement,
) -> Result<Vec<String>, zbus::Error> {
    let reply = connection
        .call_method(
            Some(element.bus_name()),
            element.path(),
            Some(ACTION_INTERFACE),
            "GetActions",
            &(),
        )
        .await?;
    let actions: Vec<(String, String, String)> = reply.body().deserialize()?;
    Ok(actions.into_iter().map(|(name, _, _)| name).collect())
}

fn unpressable(element: &AtspiElement, why: &str) -> AdapterError {
    AdapterError::new(
        ErrorCode::ActionNotSupported,
        "The shell control cannot be pressed through its accessibility actions",
    )
    .with_suggestion("Run list-notifications again; the shell's layout may have changed")
    .with_platform_detail(format!("{}{}: {why}", element.bus_name(), element.path()))
    .with_disposition(DeliverySemantics::not_delivered())
}

async fn press_one(connection: &Connection, element: &AtspiElement) -> Result<(), AdapterError> {
    let names = action_names(connection, element)
        .await
        .map_err(|error| unpressable(element, &error.to_string()))?;
    let index = names
        .iter()
        .position(|name| name.eq_ignore_ascii_case(PRESS))
        .and_then(|position| i32::try_from(position).ok())
        .ok_or_else(|| unpressable(element, "no Press action"))?;
    let reply = connection
        .call_method(
            Some(element.bus_name()),
            element.path(),
            Some(ACTION_INTERFACE),
            "DoAction",
            &(index,),
        )
        .await
        .map_err(|error| unpressable(element, &error.to_string()))?;
    let accepted: bool = reply
        .body()
        .deserialize()
        .map_err(|error| unpressable(element, &error.to_string()))?;
    if accepted {
        Ok(())
    } else {
        Err(unpressable(element, "DoAction answered false"))
    }
}

impl Reader for LiveReader {
    fn details(&self, element: &AtspiElement) -> Result<Details, AdapterError> {
        within(self.deadline, async {
            let (description, checked) = futures_util::future::join(
                description_of(&self.connection, element),
                checked_of(&self.connection, element),
            )
            .await;
            Ok(Details {
                description,
                checked,
            })
        })
    }

    fn press(&self, element: &AtspiElement) -> Result<(), AdapterError> {
        within(self.deadline, press_one(&self.connection, element))
    }
}
