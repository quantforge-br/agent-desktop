use agent_desktop_core::Rect;

use super::{extents_of, layer_place};
use crate::system::hyprland::hypr_layer::HyprLayer;
use crate::system::x11::FrameFacts;
use crate::tree::Extents;

#[test]
fn layout_rectangles_round_to_whole_pixels_and_overflow_is_no_place() {
    let rect = Rect {
        x: -20.4,
        y: 1440.6,
        width: 500.0,
        height: 616.5,
    };
    assert_eq!(
        extents_of(&rect),
        Some(Extents {
            x: -20,
            y: 1441,
            width: 500,
            height: 617,
        })
    );
    assert_eq!(extents_of(&Rect { x: 1e12, ..rect }), None);
}

fn layer(pid: u32, y: f64, width: f64, height: f64) -> HyprLayer {
    HyprLayer {
        monitor: "DP-1".into(),
        level: HyprLayer::TOP,
        namespace: "dms:dock".into(),
        pid,
        rect: Rect {
            x: 0.0,
            y,
            width,
            height,
        },
    }
}

fn shell_frame(pid: u32, width: i32, height: i32) -> FrameFacts {
    FrameFacts {
        pid,
        title: String::new(),
        extents: Some(Extents {
            x: 0,
            y: 0,
            width,
            height,
        }),
        global_position: false,
        size_identity: false,
    }
}

#[test]
fn a_frame_with_no_client_sits_at_the_corner_of_its_layer() {
    let layers = [layer(7, 726.0, 1280.0, 74.0), layer(7, 0.0, 1280.0, 44.0)];
    assert_eq!(
        layer_place(&layers, &shell_frame(7, 1280, 74), |_| Some(1)),
        Some(Extents {
            x: 0,
            y: 726,
            width: 1280,
            height: 74,
        })
    );
}

#[test]
fn a_frame_two_layers_could_be_or_another_process_owns_is_not_placed() {
    let twins = [layer(7, 726.0, 1280.0, 74.0), layer(7, 0.0, 1280.0, 74.0)];
    assert_eq!(
        layer_place(&twins, &shell_frame(7, 1280, 74), |_| Some(1)),
        None
    );
    assert_eq!(
        layer_place(
            &[layer(8, 726.0, 1280.0, 74.0)],
            &shell_frame(7, 1280, 74),
            |_| Some(1)
        ),
        None
    );
    let mut unsized_frame = shell_frame(7, 1280, 74);
    unsized_frame.extents = None;
    assert_eq!(layer_place(&twins[..1], &unsized_frame, |_| Some(1)), None);
}

#[test]
fn two_frames_of_one_size_share_a_layer_so_neither_is_placed() {
    let layers = [layer(7, 726.0, 1280.0, 74.0)];
    let frame = shell_frame(7, 1280, 74);
    assert_eq!(layer_place(&layers, &frame, |_| Some(2)), None);
}

#[test]
fn a_frame_count_that_could_not_be_read_places_nothing() {
    let layers = [layer(7, 726.0, 1280.0, 74.0)];
    let frame = shell_frame(7, 1280, 74);
    assert_eq!(layer_place(&layers, &frame, |_| None), None);
}

#[test]
fn the_count_is_asked_for_the_frames_size_and_only_once_a_layer_matches() {
    let layers = [layer(7, 726.0, 1280.0, 74.0)];
    let asked = std::cell::Cell::new(None);
    let placed = layer_place(&layers, &shell_frame(7, 1280, 74), |size| {
        asked.set(Some(size));
        Some(1)
    });
    assert!(placed.is_some());
    assert_eq!(asked.get(), Some((1280, 74)));
    let asked = std::cell::Cell::new(false);
    let _ = layer_place(&layers, &shell_frame(8, 1280, 74), |_| {
        asked.set(true);
        Some(1)
    });
    assert!(!asked.get());
}
