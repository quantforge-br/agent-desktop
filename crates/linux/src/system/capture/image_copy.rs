use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode, ImageBuffer};
use serde_json::json;
use wayland_client::protocol::wl_output::WlOutput;
use wayland_client::protocol::wl_shm::WlShm;
use wayland_protocols::ext::foreign_toplevel_list::v1::client::ext_foreign_toplevel_list_v1::ExtForeignToplevelListV1;
use wayland_protocols::ext::image_capture_source::v1::client::{
    ext_foreign_toplevel_image_capture_source_manager_v1::ExtForeignToplevelImageCaptureSourceManagerV1,
    ext_image_capture_source_v1::ExtImageCaptureSourceV1,
    ext_output_image_capture_source_manager_v1::ExtOutputImageCaptureSourceManagerV1,
};
use wayland_protocols::ext::image_copy_capture::v1::client::{
    ext_image_copy_capture_manager_v1::{ExtImageCopyCaptureManagerV1, Options},
    ext_image_copy_capture_session_v1::ExtImageCopyCaptureSessionV1,
};

use super::capture_source::CaptureSource;
use super::png_encode::encode_png;
use super::shm_frame::{ShmFrame, choose_format};
use crate::system::wayland::WaylandLink;
use crate::system::wayland::frame_status::FrameStatus;
use crate::system::wayland::wayland_output::WaylandOutput;

const OUTPUT_VERSION: u32 = 4;

fn missing(global: &str) -> AdapterError {
    AdapterError::new(
        ErrorCode::PlatformNotSupported,
        "The Wayland compositor offers no image copy capture for this target",
    )
    .with_details(json!({ "cause": "image_copy_capture_missing", "global": global }))
    .with_disposition(DeliverySemantics::not_delivered())
}

fn gone(what: &str, name: &str) -> AdapterError {
    AdapterError::new(
        ErrorCode::WindowNotFound,
        format!("The compositor no longer lists the {what} to capture"),
    )
    .with_details(json!({ "target": name }))
    .with_suggestion("List the windows or displays again and retry with one that exists now")
}

fn failed(reason: impl Into<String>) -> AdapterError {
    AdapterError::new(ErrorCode::ActionFailed, reason)
        .with_disposition(DeliverySemantics::not_delivered())
}

fn output_named(
    link: &mut WaylandLink,
    name: &str,
    deadline: Deadline,
) -> Result<WlOutput, AdapterError> {
    let globals: Vec<_> = link
        .state
        .globals
        .iter()
        .filter(|global| global.interface == "wl_output")
        .cloned()
        .collect();
    for global in &globals {
        let proxy: WlOutput = link.bind_global(global, OUTPUT_VERSION, ());
        link.state.outputs.push(WaylandOutput { proxy, name: None });
    }
    link.sync(deadline)?;
    link.state
        .outputs
        .iter()
        .find(|output| output.name.as_deref() == Some(name))
        .map(|output| output.proxy.clone())
        .ok_or_else(|| gone("display", name))
}

fn source_of(
    link: &mut WaylandLink,
    source: &CaptureSource,
    deadline: Deadline,
) -> Result<ExtImageCaptureSourceV1, AdapterError> {
    let handle = link.handle();
    match source {
        CaptureSource::Output(name) => {
            let sources: ExtOutputImageCaptureSourceManagerV1 = link
                .bind(1, ())
                .ok_or_else(|| missing("ext_output_image_capture_source_manager_v1"))?;
            let output = output_named(link, name, deadline)?;
            Ok(sources.create_source(&output, &handle, ()))
        }
        CaptureSource::Toplevel(identifier) => {
            let sources: ExtForeignToplevelImageCaptureSourceManagerV1 = link
                .bind(1, ())
                .ok_or_else(|| missing("ext_foreign_toplevel_image_capture_source_manager_v1"))?;
            let _list: ExtForeignToplevelListV1 = link
                .bind(1, ())
                .ok_or_else(|| missing("ext_foreign_toplevel_list_v1"))?;
            link.sync(deadline)?;
            link.sync(deadline)?;
            let toplevel = link
                .state
                .toplevels
                .iter()
                .find(|toplevel| toplevel.identifier == *identifier && !toplevel.closed)
                .map(|toplevel| toplevel.handle.clone())
                .ok_or_else(|| gone("window", identifier))?;
            Ok(sources.create_source(&toplevel, &handle, ()))
        }
    }
}

fn copy_frame(
    link: &mut WaylandLink,
    session: &ExtImageCopyCaptureSessionV1,
    shm: &WlShm,
    deadline: Deadline,
) -> Result<ShmFrame, AdapterError> {
    link.dispatch_until(deadline, |state| {
        state.capture.constraints_done || state.capture.stopped
    })?;
    if link.state.capture.stopped {
        return Err(failed("The compositor stopped the capture before it began"));
    }
    let size = link
        .state
        .capture
        .size
        .ok_or_else(|| failed("The compositor gave the capture no buffer size"))?;
    let format = choose_format(&link.state.capture.shm_formats).ok_or_else(|| {
        failed("The compositor offers no 8-bit shared-memory format for the capture")
            .with_details(json!({ "offered_formats": link.state.capture.shm_formats }))
    })?;
    let handle = link.handle();
    let buffer = ShmFrame::allocate(shm, &handle, size, format)?;
    let frame = session.create_frame(&handle, ());
    frame.attach_buffer(&buffer.buffer);
    let (width, height) = (
        i32::try_from(size.0).unwrap_or(i32::MAX),
        i32::try_from(size.1).unwrap_or(i32::MAX),
    );
    frame.damage_buffer(0, 0, width, height);
    frame.capture();
    let waited = link.dispatch_until(deadline, |state| {
        state.capture.frame != FrameStatus::Pending || state.capture.stopped
    });
    frame.destroy();
    waited?;
    match link.state.capture.frame {
        FrameStatus::Ready => Ok(buffer),
        FrameStatus::Failed(reason) => Err(failed("The compositor could not copy the frame")
            .with_details(json!({ "failure_reason": reason }))),
        FrameStatus::Pending => Err(failed("The compositor stopped the capture mid-frame")),
    }
}

/// Copies one frame of `source` through `ext_image_copy_capture_v1` into
/// shared memory and encodes it as PNG. `scale` is the pixels per logical
/// unit of the frame. The cursor is not painted.
pub(crate) fn capture(
    source: &CaptureSource,
    scale: f64,
    deadline: Deadline,
) -> Result<ImageBuffer, AdapterError> {
    let mut link = WaylandLink::connect(deadline)?;
    let shm: WlShm = link.bind(1, ()).ok_or_else(|| missing("wl_shm"))?;
    let copier: ExtImageCopyCaptureManagerV1 = link
        .bind(1, ())
        .ok_or_else(|| missing("ext_image_copy_capture_manager_v1"))?;
    let image_source = source_of(&mut link, source, deadline)?;
    let session = copier.create_session(&image_source, Options::empty(), &link.handle(), ());
    let copied = copy_frame(&mut link, &session, &shm, deadline);
    session.destroy();
    image_source.destroy();
    let _ = link.flush();
    let frame = copied?;
    encode_png(&frame.rgb()?, frame.width, frame.height, scale)
}
