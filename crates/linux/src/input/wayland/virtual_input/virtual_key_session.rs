use agent_desktop_core::{AdapterError, Deadline};

use super::virtual_devices::VirtualDevices;
use super::virtual_key_table::VirtualKeyTable;
use crate::input::wayland::key_session::KeySession;

/// A keysym delivery through `zwp_virtual_keyboard_v1`. The keymap is made
/// from the keysyms of the delivery, so every keysym is reachable whatever
/// the session's own layout is. Each key waits for the compositor to
/// process it before the next.
pub(crate) struct VirtualKeySession {
    table: Option<VirtualKeyTable>,
    devices: Option<VirtualDevices>,
}

impl VirtualKeySession {
    pub(crate) fn new(keysyms: impl IntoIterator<Item = u32>) -> Self {
        Self {
            table: Some(VirtualKeyTable::with(keysyms)),
            devices: None,
        }
    }

    fn devices(&mut self) -> Result<&mut VirtualDevices, AdapterError> {
        self.devices.as_mut().ok_or_else(|| {
            AdapterError::internal("The virtual keyboard was used before it was created")
        })
    }
}

impl KeySession for VirtualKeySession {
    fn create(&mut self, deadline: Deadline) -> Result<(), AdapterError> {
        let table = self.table.take().unwrap_or_default();
        self.devices = Some(VirtualDevices::open(table, false, deadline)?);
        Ok(())
    }

    fn start(&mut self, _deadline: Deadline) -> Result<(), AdapterError> {
        Ok(())
    }

    fn keysym(&mut self, keysym: u32, down: bool, deadline: Deadline) -> Result<(), AdapterError> {
        let devices = self.devices()?;
        let code = devices.code_for(keysym)?;
        devices.key(code, down);
        devices.sync(deadline)
    }

    fn stop(&mut self, deadline: Deadline) -> Result<(), AdapterError> {
        let Some(mut devices) = self.devices.take() else {
            return Ok(());
        };
        let outcome = devices.sync(deadline);
        drop(devices);
        outcome
    }
}
