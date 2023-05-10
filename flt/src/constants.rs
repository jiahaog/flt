// TODO(jiahaog): Not sure if this works, setting it to 1 still looks fast.
pub(crate) const FPS: usize = 60;

pub(crate) const DEFAULT_PIXEL_RATIO: f64 = 0.3;

/// Multiplier applied to the pixel ratio when zooming / scaling.
pub(crate) const ZOOM_FACTOR: f64 = 1.1;

/// Number of pixel for each scroll event as the terminal doesn't tell us how
/// many lines the mouse has scrolled by.
pub(crate) const SCROLL_DELTA: f64 = 10.0;
