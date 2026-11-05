use agent_desktop_core::Rect;

/// The smallest rectangle holding every monitor of the compositor's
/// layout. Absolute virtual pointer motion is a fraction of this box, so a
/// point is sent relative to its origin with the box size as the extent.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LayoutBox {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl LayoutBox {
    pub(crate) fn around(monitors: &[Rect]) -> Option<Self> {
        let left = monitors.iter().map(|rect| rect.x).reduce(f64::min)?;
        let top = monitors.iter().map(|rect| rect.y).reduce(f64::min)?;
        let right = monitors
            .iter()
            .map(|rect| rect.x + rect.width)
            .reduce(f64::max)?;
        let bottom = monitors
            .iter()
            .map(|rect| rect.y + rect.height)
            .reduce(f64::max)?;
        let width = (right - left).round();
        let height = (bottom - top).round();
        if !(width >= 1.0
            && height >= 1.0
            && width <= f64::from(i16::MAX)
            && height <= f64::from(i16::MAX))
        {
            return None;
        }
        Some(Self {
            x: left,
            y: top,
            width: width as u32,
            height: height as u32,
        })
    }

    /// A layout point relative to the box origin.
    pub(crate) fn local(&self, x: i32, y: i32) -> Option<(i32, i32)> {
        let x = (f64::from(x) - self.x).round();
        let y = (f64::from(y) - self.y).round();
        (x >= 0.0 && y >= 0.0 && x < f64::from(self.width) && y < f64::from(self.height))
            .then_some((x as i32, y as i32))
    }
}

#[cfg(test)]
#[path = "layout_box_tests.rs"]
mod tests;
