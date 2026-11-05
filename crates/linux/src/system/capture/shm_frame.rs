use std::os::fd::AsFd;
use std::os::unix::fs::FileExt;

use agent_desktop_core::{AdapterError, ErrorCode};
use rustix::fs::{MemfdFlags, memfd_create};
use wayland_client::QueueHandle;
use wayland_client::protocol::wl_buffer::WlBuffer;
use wayland_client::protocol::wl_shm::{Format, WlShm};

use super::pixels::MAX_PIXELS;
use crate::system::wayland::wayland_state::WaylandState;

const ARGB8888: u32 = 0;
const XRGB8888: u32 = 1;
const ABGR8888: u32 = 0x3432_4241;
const XBGR8888: u32 = 0x3432_4258;
const PREFERRED: [u32; 4] = [XRGB8888, ARGB8888, XBGR8888, ABGR8888];
const BYTES_PER_PIXEL: u32 = 4;

fn failure(what: &str, error: impl std::fmt::Display) -> AdapterError {
    AdapterError::new(ErrorCode::ActionFailed, format!("{what}: {error}"))
}

/// The first 8-bit-per-channel format of the compositor's offer that this
/// reader converts, or `None` when it offers only others.
pub(crate) fn choose_format(offered: &[u32]) -> Option<u32> {
    PREFERRED
        .into_iter()
        .find(|format| offered.contains(format))
}

/// Packed 32-bit pixels as RGB rows. `format` is a `wl_shm` format code;
/// the `*RGB8888` formats are little-endian words, so blue comes first in
/// memory, while the `*BGR8888` formats put red first.
pub(crate) fn to_rgb(pixels: &[u8], format: u32, width: u32, height: u32) -> Option<Vec<u8>> {
    let count = usize::try_from(u64::from(width) * u64::from(height)).ok()?;
    if pixels.len() < count.checked_mul(4)? {
        return None;
    }
    let red_first = match format {
        XRGB8888 | ARGB8888 => false,
        XBGR8888 | ABGR8888 => true,
        _ => return None,
    };
    let mut rgb = Vec::with_capacity(count * 3);
    for pixel in pixels[..count * 4].chunks_exact(4) {
        if red_first {
            rgb.extend_from_slice(&pixel[..3]);
        } else {
            rgb.extend_from_slice(&[pixel[2], pixel[1], pixel[0]]);
        }
    }
    Some(rgb)
}

/// One shared-memory buffer the compositor copies a frame into.
pub(crate) struct ShmFrame {
    file: std::fs::File,
    pub(crate) buffer: WlBuffer,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) format: u32,
}

impl ShmFrame {
    pub(crate) fn allocate(
        shm: &WlShm,
        handle: &QueueHandle<WaylandState>,
        size: (u32, u32),
        format: u32,
    ) -> Result<Self, AdapterError> {
        let (width, height) = size;
        if u64::from(width) * u64::from(height) > MAX_PIXELS || width == 0 || height == 0 {
            return Err(AdapterError::new(
                ErrorCode::ActionFailed,
                format!("The capture is {width}x{height}, outside the readable pixel range"),
            ));
        }
        let stride = width * BYTES_PER_PIXEL;
        let length = u64::from(stride) * u64::from(height);
        let fd = memfd_create("agent-desktop-capture", MemfdFlags::CLOEXEC)
            .map_err(|error| failure("The capture buffer could not be created", error))?;
        let file = std::fs::File::from(fd);
        file.set_len(length)
            .map_err(|error| failure("The capture buffer could not be sized", error))?;
        let pool_size = i32::try_from(length)
            .map_err(|error| failure("The capture buffer is too large", error))?;
        let wire_format = Format::try_from(format)
            .map_err(|()| failure("The capture format is unknown", format))?;
        let pool = shm.create_pool(file.as_fd(), pool_size, handle, ());
        let buffer = pool.create_buffer(
            0,
            i32::try_from(width).unwrap_or(i32::MAX),
            i32::try_from(height).unwrap_or(i32::MAX),
            i32::try_from(stride).unwrap_or(i32::MAX),
            wire_format,
            handle,
            (),
        );
        pool.destroy();
        Ok(Self {
            file,
            buffer,
            width,
            height,
            format,
        })
    }

    /// The copied frame as RGB rows.
    pub(crate) fn rgb(&self) -> Result<Vec<u8>, AdapterError> {
        let length = usize::try_from(u64::from(self.width) * u64::from(self.height) * 4)
            .map_err(|error| failure("The capture is too large", error))?;
        let mut pixels = vec![0_u8; length];
        self.file
            .read_exact_at(&mut pixels, 0)
            .map_err(|error| failure("The captured frame could not be read", error))?;
        to_rgb(&pixels, self.format, self.width, self.height)
            .ok_or_else(|| failure("The captured frame is unusable", "short or unknown format"))
    }
}

impl Drop for ShmFrame {
    fn drop(&mut self) {
        self.buffer.destroy();
    }
}

#[cfg(test)]
#[path = "shm_frame_tests.rs"]
mod tests;
