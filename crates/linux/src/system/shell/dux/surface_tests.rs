use agent_desktop_core::{DeliverySemantics, ErrorCode, SnapshotSurface};

use super::{Surface, of};
use crate::tree::vocabulary::role_numbers::{FILLER, PUSH_BUTTON, RADIO_BUTTON};

#[test]
fn the_surfaces_the_shell_has_map_to_their_own() {
    assert_eq!(
        of(SnapshotSurface::ActionCenter).expect("kind"),
        Surface::NotificationCenter
    );
    assert_eq!(
        of(SnapshotSurface::NotificationCenter).expect("kind"),
        Surface::NotificationCenter
    );
    assert_eq!(
        of(SnapshotSurface::QuickSettings).expect("kind"),
        Surface::ControlCenter
    );
    assert_eq!(
        of(SnapshotSurface::Spotlight).expect("kind"),
        Surface::Launcher
    );
    assert_eq!(of(SnapshotSurface::Taskbar).expect("kind"), Surface::Dock);
    assert_eq!(of(SnapshotSurface::Dock).expect("kind"), Surface::Dock);
    assert_eq!(of(SnapshotSurface::SystemTray).expect("kind"), Surface::Bar);
}

#[test]
fn surfaces_the_shell_cannot_present_are_refused_with_what_it_offers() {
    for requested in [
        SnapshotSurface::StartMenu,
        SnapshotSurface::MenuBarExtras,
        SnapshotSurface::Menubar,
        SnapshotSurface::SystemTrayOverflow,
        SnapshotSurface::Toolbar,
    ] {
        let error = of(requested).expect_err("refused");

        assert_eq!(error.code, ErrorCode::InvalidArgs, "{requested:?}");
        assert_eq!(error.disposition, DeliverySemantics::not_delivered());
        assert!(
            error
                .suggestion
                .as_deref()
                .is_some_and(|text| text.contains("spotlight"))
        );
    }
}

#[test]
fn every_surface_the_shell_names_answers_for_the_snapshot_surfaces_that_map_to_it() {
    for surface in Surface::ALL {
        for named in surface.snapshots() {
            assert_eq!(of(*named).expect("kind"), surface);
        }
    }
}

#[test]
fn only_the_notification_center_and_the_launcher_are_told_apart_by_content() {
    let tabs = [
        RADIO_BUTTON,
        RADIO_BUTTON,
        PUSH_BUTTON,
        PUSH_BUTTON,
        PUSH_BUTTON,
    ];
    let plain = [FILLER, PUSH_BUTTON];

    assert!(Surface::NotificationCenter.recognises(&tabs));
    assert!(!Surface::NotificationCenter.recognises(&plain));
    assert!(!Surface::Launcher.recognises(&tabs));
    assert!(Surface::Launcher.recognises(&plain));
    assert!(Surface::ControlCenter.recognises(&tabs) && Surface::Dock.recognises(&plain));
}
