//! The ways the fake dux-shell can depart from the shell it models.

/// Ways the fake shell departs from the shell it models.
#[derive(Default, Clone, Copy)]
pub(super) struct Quirks {
    pub(super) inert_dismiss: bool,
    pub(super) dismiss_takes_neighbour: bool,
    pub(super) silent_service: bool,
    pub(super) never_shows: bool,
    pub(super) unreadable_after_press: bool,
    pub(super) expand_limit: Option<usize>,
    pub(super) keeps_open_after_action: bool,
}
