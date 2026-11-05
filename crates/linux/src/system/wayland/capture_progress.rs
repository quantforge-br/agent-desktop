use super::frame_status::FrameStatus;

/// What one image copy capture session has announced so far: the buffer
/// constraints that end with `done`, whether the session stopped, and the
/// state of the frame in flight.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CaptureProgress {
    pub(crate) size: Option<(u32, u32)>,
    pub(crate) shm_formats: Vec<u32>,
    pub(crate) constraints_done: bool,
    pub(crate) stopped: bool,
    pub(crate) frame: FrameStatus,
    pub(crate) transform: u32,
}
