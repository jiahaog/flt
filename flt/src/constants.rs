pub(crate) const FPS: usize = 60;
pub(crate) const PIXEL_RATIO: f64 = 0.5;
// Number of pixel for each scroll event as the terminal doesn't tell us how
// many lines the mouse has scrolled by.
pub(crate) const SCROLL_DELTA: f64 = 10.0;

/// How much to scale the pixel ratio when zooming.
pub(crate) const ZOOM_FACTOR: f64 = 1.1;
