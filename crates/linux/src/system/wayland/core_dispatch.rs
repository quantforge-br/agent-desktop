//! Event handling for the core globals and the input devices this client
//! binds. Interfaces whose events carry nothing this adapter reads are
//! ignored.

use wayland_client::protocol::{
    wl_buffer::WlBuffer, wl_callback, wl_callback::WlCallback, wl_output, wl_output::WlOutput,
    wl_registry, wl_registry::WlRegistry, wl_seat, wl_seat::WlSeat, wl_shm::WlShm,
    wl_shm_pool::WlShmPool,
};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, WEnum, delegate_noop};
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::{
    zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1,
    zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
};
use wayland_protocols_wlr::virtual_pointer::v1::client::{
    zwlr_virtual_pointer_manager_v1::ZwlrVirtualPointerManagerV1,
    zwlr_virtual_pointer_v1::ZwlrVirtualPointerV1,
};

use super::wayland_global::WaylandGlobal;
use super::wayland_state::WaylandState;

impl Dispatch<WlRegistry, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _: &WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => state.globals.push(WaylandGlobal {
                name,
                interface,
                version,
            }),
            wl_registry::Event::GlobalRemove { name } => {
                state.globals.retain(|global| global.name != name);
            }
            _ => {}
        }
    }
}

impl Dispatch<WlCallback, u32> for WaylandState {
    fn event(
        state: &mut Self,
        _: &WlCallback,
        event: wl_callback::Event,
        token: &u32,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_callback::Event::Done { .. } = event {
            state.synced = *token;
        }
    }
}

impl Dispatch<WlOutput, ()> for WaylandState {
    fn event(
        state: &mut Self,
        proxy: &WlOutput,
        event: wl_output::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_output::Event::Name { name } = event
            && let Some(output) = state
                .outputs
                .iter_mut()
                .find(|output| output.proxy.id() == proxy.id())
        {
            output.name = Some(name);
        }
    }
}

impl Dispatch<WlSeat, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _: &WlSeat,
        event: wl_seat::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities {
            capabilities: WEnum::Value(capabilities),
        } = event
        {
            state.seat_pointer = Some(capabilities.contains(wl_seat::Capability::Pointer));
        }
    }
}

delegate_noop!(WaylandState: ignore WlShm);
delegate_noop!(WaylandState: WlShmPool);
delegate_noop!(WaylandState: ignore WlBuffer);
delegate_noop!(WaylandState: ZwpVirtualKeyboardManagerV1);
delegate_noop!(WaylandState: ZwpVirtualKeyboardV1);
delegate_noop!(WaylandState: ZwlrVirtualPointerManagerV1);
delegate_noop!(WaylandState: ZwlrVirtualPointerV1);
