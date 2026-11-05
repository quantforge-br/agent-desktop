//! Event handling for the foreign toplevel list and the image copy capture
//! objects a screenshot creates.

use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle, WEnum, delegate_noop, event_created_child,
};
use wayland_protocols::ext::foreign_toplevel_list::v1::client::{
    ext_foreign_toplevel_handle_v1::{self, ExtForeignToplevelHandleV1},
    ext_foreign_toplevel_list_v1::{self, ExtForeignToplevelListV1},
};
use wayland_protocols::ext::image_capture_source::v1::client::{
    ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1,
    ext_image_capture_source_v1::ExtImageCaptureSourceV1,
    ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1,
};
use wayland_protocols::ext::image_copy_capture::v1::client::{
    ext_image_copy_capture_frame_v1::{self, ExtImageCopyCaptureFrameV1},
    ext_image_copy_capture_manager_v1::ExtImageCopyCaptureManagerV1,
    ext_image_copy_capture_session_v1::{self, ExtImageCopyCaptureSessionV1},
};

use super::foreign_toplevel::ForeignToplevel;
use super::frame_status::FrameStatus;
use super::wayland_state::WaylandState;

impl Dispatch<ExtForeignToplevelListV1, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _: &ExtForeignToplevelListV1,
        event: ext_foreign_toplevel_list_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ext_foreign_toplevel_list_v1::Event::Toplevel { toplevel } => {
                state.toplevels.push(ForeignToplevel {
                    handle: toplevel,
                    identifier: String::new(),
                    title: String::new(),
                    app_id: String::new(),
                    closed: false,
                });
            }
            ext_foreign_toplevel_list_v1::Event::Finished => state.toplevels_listed = true,
            _ => {}
        }
    }

    event_created_child!(WaylandState, ExtForeignToplevelListV1, [
        ext_foreign_toplevel_list_v1::EVT_TOPLEVEL_OPCODE => (ExtForeignToplevelHandleV1, ()),
    ]);
}

impl Dispatch<ExtForeignToplevelHandleV1, ()> for WaylandState {
    fn event(
        state: &mut Self,
        proxy: &ExtForeignToplevelHandleV1,
        event: ext_foreign_toplevel_handle_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let Some(toplevel) = state
            .toplevels
            .iter_mut()
            .find(|toplevel| toplevel.handle.id() == proxy.id())
        else {
            return;
        };
        match event {
            ext_foreign_toplevel_handle_v1::Event::Identifier { identifier } => {
                toplevel.identifier = identifier;
            }
            ext_foreign_toplevel_handle_v1::Event::Title { title } => toplevel.title = title,
            ext_foreign_toplevel_handle_v1::Event::AppId { app_id } => toplevel.app_id = app_id,
            ext_foreign_toplevel_handle_v1::Event::Closed => toplevel.closed = true,
            _ => {}
        }
    }
}

impl Dispatch<ExtImageCopyCaptureSessionV1, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _: &ExtImageCopyCaptureSessionV1,
        event: ext_image_copy_capture_session_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let capture = &mut state.capture;
        match event {
            ext_image_copy_capture_session_v1::Event::BufferSize { width, height } => {
                capture.size = Some((width, height));
            }
            ext_image_copy_capture_session_v1::Event::ShmFormat {
                format: WEnum::Value(format),
            } => {
                capture.shm_formats.push(format.into());
            }
            ext_image_copy_capture_session_v1::Event::Done => capture.constraints_done = true,
            ext_image_copy_capture_session_v1::Event::Stopped => capture.stopped = true,
            _ => {}
        }
    }
}

impl Dispatch<ExtImageCopyCaptureFrameV1, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _: &ExtImageCopyCaptureFrameV1,
        event: ext_image_copy_capture_frame_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let capture = &mut state.capture;
        match event {
            ext_image_copy_capture_frame_v1::Event::Transform {
                transform: WEnum::Value(transform),
            } => {
                capture.transform = transform.into();
            }
            ext_image_copy_capture_frame_v1::Event::Ready => capture.frame = FrameStatus::Ready,
            ext_image_copy_capture_frame_v1::Event::Failed { reason } => {
                capture.frame = FrameStatus::Failed(match reason {
                    WEnum::Value(reason) => reason.into(),
                    WEnum::Unknown(raw) => raw,
                });
            }
            _ => {}
        }
    }
}

delegate_noop!(WaylandState: ExtImageCopyCaptureManagerV1);
delegate_noop!(WaylandState: ExtOutputImageCaptureSourceManagerV1);
delegate_noop!(WaylandState: ExtForeignToplevelImageCaptureSourceManagerV1);
delegate_noop!(WaylandState: ExtImageCaptureSourceV1);
