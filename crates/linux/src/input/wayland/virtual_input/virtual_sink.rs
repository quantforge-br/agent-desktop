use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline};
use wayland_client::protocol::wl_pointer::Axis;

use super::layout_box::LayoutBox;
use super::virtual_devices::VirtualDevices;
use super::virtual_key_table::VirtualKeyTable;
use crate::input::keyboard::keymap::Keymap;
use crate::input::keyboard::keymap_edit::KeymapEdit;
use crate::input::x11::pointer_read::PointerRead;
use crate::input::x11::sink::X11Sink;
use crate::input::x11::sink_error::SinkError;
use crate::system::hyprland::HyprInstance;

const RELEASE_ALLOWANCE: Duration = Duration::from_secs(2);
const BTN_LEFT: u32 = 272;
const BTN_RIGHT: u32 = 273;
const BTN_MIDDLE: u32 = 274;
const BTN_SIDE: u32 = 275;
const BTN_EXTRA: u32 = 276;

fn sink_error(error: AdapterError) -> SinkError {
    SinkError::Request(error.platform_detail.clone().unwrap_or(error.message))
}

/// The point one pixel beside `target` to pass through first when the cursor
/// already rests on `target`. Hyprland ignores motion to the cursor's own
/// position, so a surface that appeared or reappeared under a still cursor
/// never gets pointer focus and a press there reaches nothing.
fn nudge_before(target: (u32, u32), here: Option<(u32, u32)>, width: u32) -> Option<(u32, u32)> {
    if here != Some(target) {
        return None;
    }
    let x = if target.0 + 1 < width {
        target.0 + 1
    } else {
        target.0.checked_sub(1)?
    };
    Some((x, target.1))
}

/// Runs planned key, button and motion steps through the compositor's
/// virtual keyboard and virtual pointer, in the X11 button and keycode
/// numbering the planners use. Motion is relative to the layout box. The
/// pointer position is read back from Hyprland; held buttons and keys are
/// not observable there, so they are never reported as held.
pub(crate) struct VirtualSink {
    devices: VirtualDevices,
    layout: LayoutBox,
    instance: HyprInstance,
    deadline: Deadline,
    at: Option<(u32, u32)>,
}

impl VirtualSink {
    pub(crate) fn open(
        instance: HyprInstance,
        layout: LayoutBox,
        deadline: Deadline,
    ) -> Result<Self, AdapterError> {
        let devices = VirtualDevices::open(VirtualKeyTable::default(), true, deadline)?;
        let deadline =
            Deadline::from_duration(deadline.remaining() + RELEASE_ALLOWANCE).unwrap_or(deadline);
        Ok(Self {
            devices,
            layout,
            instance,
            deadline,
            at: None,
        })
    }

    fn resting_point(&mut self) -> Option<(u32, u32)> {
        if self.at.is_none() {
            let point = self.instance.cursor(self.deadline).ok()?;
            let x = u32::try_from((point.x - self.layout.x).round() as i64).ok()?;
            let y = u32::try_from((point.y - self.layout.y).round() as i64).ok()?;
            self.at = Some((x, y));
        }
        self.at
    }
}

impl X11Sink for VirtualSink {
    fn key(&mut self, code: u8, down: bool) -> Result<(), SinkError> {
        self.devices.key(code, down);
        Ok(())
    }

    fn button(&mut self, code: u8, down: bool) -> Result<(), SinkError> {
        let pressed = match code {
            1 => BTN_LEFT,
            2 => BTN_MIDDLE,
            3 => BTN_RIGHT,
            8 => BTN_SIDE,
            9 => BTN_EXTRA,
            4..=7 => {
                if !down {
                    return Ok(());
                }
                let (axis, positive) = match code {
                    4 => (Axis::VerticalScroll, false),
                    5 => (Axis::VerticalScroll, true),
                    6 => (Axis::HorizontalScroll, false),
                    _ => (Axis::HorizontalScroll, true),
                };
                return self.devices.wheel(axis, positive).map_err(sink_error);
            }
            other => return Err(SinkError::Request(format!("no pointer button {other}"))),
        };
        self.devices.button(pressed, down).map_err(sink_error)
    }

    fn move_to(&mut self, x: i16, y: i16) -> Result<(), SinkError> {
        let (Ok(x), Ok(y)) = (u32::try_from(x), u32::try_from(y)) else {
            return Err(SinkError::Request(
                "a pointer position left the layout".to_owned(),
            ));
        };
        let extent = (self.layout.width, self.layout.height);
        let here = self.resting_point();
        if let Some((nudge_x, nudge_y)) = nudge_before((x, y), here, extent.0) {
            self.devices
                .move_to(nudge_x, nudge_y, extent)
                .map_err(sink_error)?;
        }
        self.devices.move_to(x, y, extent).map_err(sink_error)?;
        self.at = Some((x, y));
        Ok(())
    }

    fn round_trip(&mut self) -> Result<(), SinkError> {
        self.devices.sync(self.deadline).map_err(sink_error)
    }

    fn pointer(&mut self) -> Result<PointerRead, SinkError> {
        let point = self.instance.cursor(self.deadline).map_err(sink_error)?;
        Ok(PointerRead {
            x: (point.x - self.layout.x).round() as i32,
            y: (point.y - self.layout.y).round() as i32,
            mask: 0,
        })
    }

    fn keys_down(&mut self) -> Result<[u8; 32], SinkError> {
        Ok([0; 32])
    }

    fn keymap(&mut self) -> Result<Keymap, SinkError> {
        Ok(self.devices.table().keymap())
    }

    fn ensure_text_state(&mut self) -> Result<(), AdapterError> {
        Ok(())
    }

    fn edit_keymap(&mut self, edit: &KeymapEdit) -> Result<(), SinkError> {
        let keysym = edit.keysyms.first().copied().unwrap_or(0);
        self.devices.remap(edit.keycode, keysym).map_err(sink_error)
    }

    fn root_size(&mut self) -> Result<(u16, u16), SinkError> {
        Ok((
            u16::try_from(self.layout.width).unwrap_or(u16::MAX),
            u16::try_from(self.layout.height).unwrap_or(u16::MAX),
        ))
    }
}

#[cfg(test)]
#[path = "virtual_sink_tests.rs"]
mod tests;
