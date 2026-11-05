/// Where one `ext_image_copy_capture_frame_v1` stands. `Failed` keeps the
/// protocol's failure reason.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum FrameStatus {
    #[default]
    Pending,
    Ready,
    Failed(u32),
}
