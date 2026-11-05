use agent_desktop_core::Rect;

/// One layer-shell surface as Hyprland's `layers` request lists it: a bar,
/// dock, notification or other shell surface. `level` is the layer, from 0
/// for the background to 3 for the overlay; levels 2 and 3 draw above
/// windows.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct HyprLayer {
    pub(crate) monitor: String,
    pub(crate) level: u8,
    pub(crate) namespace: String,
    pub(crate) pid: u32,
    pub(crate) rect: Rect,
}

impl HyprLayer {
    pub(crate) const TOP: u8 = 2;
    pub(crate) const OVERLAY: u8 = 3;

    /// The one surface of process `pid` drawn at exactly `width` by `height`:
    /// where a shell window that has no client of its own is placed. Two
    /// such surfaces, a bar on each of two outputs for instance, leave the
    /// window unplaced instead of guessing between them.
    fn sole_of(layers: &[HyprLayer], pid: u32, width: i32, height: i32) -> Option<&HyprLayer> {
        let mut found = layers.iter().filter(|layer| {
            layer.pid == pid
                && layer.rect.width == f64::from(width)
                && layer.rect.height == f64::from(height)
        });
        match (found.next(), found.next()) {
            (Some(layer), None) => Some(layer),
            _ => None,
        }
    }

    /// The surface a client-less frame is drawn at: the one surface of
    /// `pid` at `size` ([`Self::sole_of`]) when exactly one top-level frame
    /// of the process has that size too. `frames` counts those frames and is
    /// read only once a surface matches; an unreadable count places nothing.
    /// The listing and the snapshot and live placement share this rule, so
    /// they never disagree about which frames a surface belongs to.
    pub(crate) fn placing(
        layers: &[HyprLayer],
        pid: u32,
        size: (i32, i32),
        frames: impl FnOnce() -> Option<usize>,
    ) -> Option<&HyprLayer> {
        let layer = Self::sole_of(layers, pid, size.0, size.1)?;
        (frames()? == 1).then_some(layer)
    }

    /// How many of the process's top-level frames have `size`, given each
    /// frame's size; `None` when any frame's size is unknown, because that
    /// frame might have it too.
    pub(crate) fn frames_of_size(
        sizes: impl IntoIterator<Item = Option<(i32, i32)>>,
        size: (i32, i32),
    ) -> Option<usize> {
        let mut count = 0;
        for seen in sizes {
            count += usize::from(seen? == size);
        }
        Some(count)
    }
}
