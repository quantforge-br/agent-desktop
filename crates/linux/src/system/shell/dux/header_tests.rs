use super::tab_header_start;
use crate::tree::vocabulary::role_numbers::{FILLER, LIST_ITEM, PUSH_BUTTON, RADIO_BUTTON};

#[test]
fn two_tabs_followed_by_header_buttons_start_the_header() {
    let roles = [
        PUSH_BUTTON,
        RADIO_BUTTON,
        RADIO_BUTTON,
        PUSH_BUTTON,
        PUSH_BUTTON,
        PUSH_BUTTON,
        FILLER,
    ];

    assert_eq!(tab_header_start(&roles), Some(1));
}

#[test]
fn a_single_tab_is_a_header_too() {
    let roles = [RADIO_BUTTON, PUSH_BUTTON, PUSH_BUTTON, PUSH_BUTTON];

    assert_eq!(tab_header_start(&roles), Some(0));
}

#[test]
fn the_four_filters_of_the_history_are_no_header() {
    let roles = [
        RADIO_BUTTON,
        RADIO_BUTTON,
        RADIO_BUTTON,
        RADIO_BUTTON,
        PUSH_BUTTON,
        PUSH_BUTTON,
        PUSH_BUTTON,
    ];

    assert_eq!(tab_header_start(&roles), None);
}

#[test]
fn tabs_followed_by_lists_are_no_header() {
    let roles = [RADIO_BUTTON, RADIO_BUTTON, FILLER, LIST_ITEM];

    assert_eq!(tab_header_start(&roles), None);
}

#[test]
fn too_few_buttons_after_the_tabs_are_no_header() {
    let roles = [RADIO_BUTTON, RADIO_BUTTON, PUSH_BUTTON, PUSH_BUTTON];

    assert_eq!(tab_header_start(&roles), None);
}
