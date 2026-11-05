use std::cell::Cell;

use super::{Backend, DuxShell, classify};
use crate::system::shell::dux::proof::Proof;

#[test]
fn gnome_shell_owning_its_bus_name_wins_even_over_a_quickshell_notification_owner() {
    let asked = Cell::new(false);

    let backend = classify(Some(10), Some(50), |_| {
        asked.set(true);
        Some(Proof::DmsParent)
    });

    assert_eq!(backend, Backend::Gnome);
    assert!(!asked.get());
}

#[test]
fn a_recognised_notification_owner_is_the_dux_shell_with_its_proof() {
    let backend = classify(None, Some(50), |pid| {
        (pid == 50).then_some(Proof::ConfigDirectory)
    });

    assert_eq!(
        backend,
        Backend::Dux(DuxShell {
            pid: 50,
            proof: Proof::ConfigDirectory
        })
    );
}

#[test]
fn an_unrecognised_notification_owner_leaves_the_session_to_the_gnome_rules() {
    assert_eq!(classify(None, Some(50), |_| None), Backend::Unrecognised);
}

#[test]
fn a_session_with_no_notification_owner_is_unrecognised() {
    let asked = Cell::new(false);

    let backend = classify(None, None, |_| {
        asked.set(true);
        Some(Proof::DmsParent)
    });

    assert_eq!(backend, Backend::Unrecognised);
    assert!(!asked.get());
}
