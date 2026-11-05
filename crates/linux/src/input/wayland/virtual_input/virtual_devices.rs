use std::io::Write;
use std::os::fd::AsFd;
use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode};
use rustix::fs::{MemfdFlags, memfd_create};
use serde_json::json;
use wayland_client::protocol::wl_pointer::{Axis, AxisSource, ButtonState};
use wayland_client::protocol::wl_seat::WlSeat;
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::{
    zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1,
    zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
};
use wayland_protocols_wlr::virtual_pointer::v1::client::{
    zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1,
    zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1,
};

use super::virtual_key_table::VirtualKeyTable;
use crate::system::wayland::WaylandLink;

const XKB_V1: u32 = 1;
const EVDEV_OFFSET: u8 = 8;
const KEY_RELEASED: u32 = 0;
const KEY_PRESSED: u32 = 1;
const WHEEL_STEP: f64 = 15.0;
/// How long clients get to bind a new `wl_pointer` after this device gave
/// the seat its first pointer. Hyprland enters a pointer bound after the
/// pointer reached the client's surface at an unknown position, so a press
/// before the next motion lands nowhere in the client.
const SEAT_POINTER_SETTLE: Duration = Duration::from_millis(100);

fn missing(global: &str) -> AdapterError {
    AdapterError::new(
        ErrorCode::PlatformNotSupported,
        "The Wayland compositor offers no virtual input device for this delivery",
    )
    .with_suggestion("Use the semantic action instead of physical input")
    .with_details(json!({
        "cause": "virtual_input_missing",
        "global": global,
        "physical_delivery_started": false,
    }))
    .with_disposition(DeliverySemantics::not_delivered())
}

fn keymap_failure(error: impl std::fmt::Display) -> AdapterError {
    AdapterError::internal("The virtual keyboard keymap could not be prepared")
        .with_platform_detail(error.to_string())
        .with_details(json!({ "physical_delivery_started": false }))
        .with_disposition(DeliverySemantics::not_delivered())
}

/// A virtual keyboard and, when asked for, a virtual pointer on the
/// compositor's seat. Dropping it destroys both, and the compositor
/// releases whatever they still hold.
pub(crate) struct VirtualDevices {
    link: WaylandLink,
    keyboard: ZwpVirtualKeyboardV1,
    pointer: Option<ZwlrVirtualPointerV1>,
    table: VirtualKeyTable,
    modifiers: u32,
}

impl VirtualDevices {
    pub(crate) fn open(
        table: VirtualKeyTable,
        with_pointer: bool,
        deadline: Deadline,
    ) -> Result<Self, AdapterError> {
        let mut link = WaylandLink::connect(deadline)?;
        let seat: WlSeat = link.bind(7, ()).ok_or_else(|| missing("wl_seat"))?;
        link.sync(deadline)?;
        let seat_had_pointer = link.state.seat_pointer == Some(true);
        let keyboards: ZwpVirtualKeyboardManagerV1 = link
            .bind(1, ())
            .ok_or_else(|| missing("zwp_virtual_keyboard_manager_v1"))?;
        let handle = link.handle();
        let keyboard = keyboards.create_virtual_keyboard(&seat, &handle, ());
        let pointer = if with_pointer {
            let pointers: ZwlrVirtualPointerManagerV1 = link
                .bind(2, ())
                .ok_or_else(|| missing("zwlr_virtual_pointer_manager_v1"))?;
            Some(pointers.create_virtual_pointer(Some(&seat), &handle, ()))
        } else {
            None
        };
        let mut devices = Self {
            link,
            keyboard,
            pointer,
            table,
            modifiers: 0,
        };
        devices.upload_keymap()?;
        devices.sync(deadline)?;
        if with_pointer && !seat_had_pointer {
            std::thread::sleep(SEAT_POINTER_SETTLE.min(deadline.remaining()));
            devices.sync(deadline)?;
        }
        Ok(devices)
    }

    /// The event time in milliseconds of the monotonic clock, the time base
    /// Hyprland stamps its own input with. Qt drops a press or motion whose
    /// time runs behind the events it already saw.
    fn now(&self) -> u32 {
        let time = rustix::time::clock_gettime(rustix::time::ClockId::Monotonic);
        let millis = time.tv_sec.unsigned_abs() * 1_000 + time.tv_nsec.unsigned_abs() / 1_000_000;
        (millis & u64::from(u32::MAX)) as u32
    }

    fn upload_keymap(&mut self) -> Result<(), AdapterError> {
        let mut text = self.table.xkb_text().into_bytes();
        text.push(0);
        let fd =
            memfd_create("agent-desktop-keymap", MemfdFlags::CLOEXEC).map_err(keymap_failure)?;
        let mut file = std::fs::File::from(fd);
        file.write_all(&text).map_err(keymap_failure)?;
        let size = u32::try_from(text.len()).map_err(keymap_failure)?;
        self.keyboard.keymap(XKB_V1, file.as_fd(), size);
        Ok(())
    }

    pub(crate) fn table(&self) -> &VirtualKeyTable {
        &self.table
    }

    /// Gives `keysym` a key, uploading the grown keymap when it changed.
    pub(crate) fn code_for(&mut self, keysym: u32) -> Result<u8, AdapterError> {
        match self.table.insert(keysym) {
            Some((code, false)) => Ok(code),
            Some((code, true)) => {
                self.upload_keymap()?;
                Ok(code)
            }
            None => {
                let held = self.modifiers;
                let mut fresh = VirtualKeyTable::default();
                let (code, _) = fresh.insert(keysym).ok_or_else(|| keymap_failure("full"))?;
                if held != 0 {
                    return Err(keymap_failure(
                        "the keymap is full while a modifier is held",
                    ));
                }
                self.table = fresh;
                self.upload_keymap()?;
                Ok(code)
            }
        }
    }

    /// Puts `keysym` on `code` and uploads the changed keymap.
    pub(crate) fn remap(&mut self, code: u8, keysym: u32) -> Result<(), AdapterError> {
        if !self.table.assign(code, keysym) {
            return Err(keymap_failure("keycode below the first virtual keycode"));
        }
        self.upload_keymap()
    }

    /// Presses or releases the key `code`. A modifier key also updates the
    /// modifier state the compositor gives the focused client.
    pub(crate) fn key(&mut self, code: u8, down: bool) {
        let state = if down { KEY_PRESSED } else { KEY_RELEASED };
        let time = self.now();
        self.keyboard
            .key(time, u32::from(code.saturating_sub(EVDEV_OFFSET)), state);
        if let Some(mask) = self.table.modifier_mask(code) {
            if down {
                self.modifiers |= mask;
            } else {
                self.modifiers &= !mask;
            }
            self.keyboard.modifiers(self.modifiers, 0, 0, 0);
        }
    }

    fn pointer(&self) -> Result<&ZwlrVirtualPointerV1, AdapterError> {
        self.pointer
            .as_ref()
            .ok_or_else(|| missing("zwlr_virtual_pointer_v1"))
    }

    /// Moves the pointer to `(x, y)` within a layout box `extent` wide and
    /// high whose origin the compositor maps to its own layout origin.
    pub(crate) fn move_to(&self, x: u32, y: u32, extent: (u32, u32)) -> Result<(), AdapterError> {
        let pointer = self.pointer()?;
        pointer.motion_absolute(self.now(), x, y, extent.0, extent.1);
        pointer.frame();
        Ok(())
    }

    pub(crate) fn button(&self, button: u32, down: bool) -> Result<(), AdapterError> {
        let pointer = self.pointer()?;
        let state = if down {
            ButtonState::Pressed
        } else {
            ButtonState::Released
        };
        pointer.button(self.now(), button, state);
        pointer.frame();
        Ok(())
    }

    /// Turns the wheel one detent on `axis`, toward positive values when
    /// `positive`.
    pub(crate) fn wheel(&self, axis: Axis, positive: bool) -> Result<(), AdapterError> {
        let pointer = self.pointer()?;
        let steps = if positive { 1 } else { -1 };
        let time = self.now();
        pointer.axis_source(AxisSource::Wheel);
        pointer.axis_discrete(time, axis, WHEEL_STEP * f64::from(steps), steps);
        pointer.frame();
        Ok(())
    }

    /// Waits until the compositor has processed every request sent so far.
    pub(crate) fn sync(&mut self, deadline: Deadline) -> Result<(), AdapterError> {
        self.link.sync(deadline)
    }
}

impl Drop for VirtualDevices {
    fn drop(&mut self) {
        if let Some(pointer) = self.pointer.take() {
            pointer.destroy();
        }
        self.keyboard.destroy();
        let _ = self.link.flush();
    }
}
