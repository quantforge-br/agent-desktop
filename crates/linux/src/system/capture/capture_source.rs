/// What an image copy capture copies: one output by its connector name, or
/// one toplevel by its foreign toplevel identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CaptureSource {
    Output(String),
    Toplevel(String),
}
