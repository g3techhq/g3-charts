//! Compact SVG path data.

use std::fmt::Write;

/// A coordinate for SVG markup: at most two decimals, no trailing zeros, no
/// `-0`, and `0` for a value that is not finite, so `NaN` never reaches the
/// page.
///
/// ```
/// use g3_charts::geom::fmt;
///
/// assert_eq!(fmt(12.3456), "12.35");
/// assert_eq!(fmt(-0.001), "0");
/// assert_eq!(fmt(f64::NAN), "0");
/// ```
pub fn fmt(value: f64) -> String {
    if !value.is_finite() {
        return "0".into();
    }
    let rounded = (value * 100.0).round() / 100.0;
    if rounded == 0.0 {
        return "0".into();
    }
    let mut text = format!("{rounded:.2}");
    while text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}

/// Builds the `d` attribute of a polyline or area, one series per path.
///
/// ```
/// use g3_charts::geom::PathBuilder;
///
/// let mut line = PathBuilder::default();
/// line.line_to(0.0, 10.0);
/// line.line_to(5.5, 2.25);
/// assert_eq!(line.finish(), "M0,10L5.5,2.25");
/// ```
#[derive(Clone, Debug, Default)]
pub struct PathBuilder {
    d: String,
    started: bool,
}

impl PathBuilder {
    /// An empty path with room for `points` points.
    pub fn with_capacity(points: usize) -> Self {
        Self {
            d: String::with_capacity(points * 12),
            started: false,
        }
    }

    /// Start a new sub-path at a point.
    pub fn move_to(&mut self, x: f64, y: f64) {
        let _ = write!(self.d, "M{},{}", fmt(x), fmt(y));
        self.started = true;
    }

    /// A straight line to a point. The first point of a path moves there
    /// instead. A point that is not finite breaks the line, so a gap in the
    /// data shows as a gap.
    pub fn line_to(&mut self, x: f64, y: f64) {
        if !x.is_finite() || !y.is_finite() {
            self.started = false;
            return;
        }
        if self.started {
            let _ = write!(self.d, "L{},{}", fmt(x), fmt(y));
        } else {
            self.move_to(x, y);
        }
    }

    /// Close the current sub-path.
    pub fn close(&mut self) {
        if !self.d.is_empty() {
            self.d.push('Z');
        }
        self.started = false;
    }

    /// The finished `d` string.
    pub fn finish(self) -> String {
        self.d
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_are_short_and_safe() {
        assert_eq!(fmt(1.0), "1");
        assert_eq!(fmt(1.5), "1.5");
        assert_eq!(fmt(-12.3456), "-12.35");
        assert_eq!(fmt(1e-12), "0");
        assert_eq!(fmt(f64::INFINITY), "0");
        assert_eq!(fmt(999.999), "1000");
    }

    #[test]
    fn non_finite_points_break_the_line() {
        let mut path = PathBuilder::with_capacity(4);
        path.line_to(0.0, 0.0);
        path.line_to(1.0, f64::NAN);
        path.line_to(2.0, 2.0);
        path.line_to(3.0, 3.0);
        path.close();
        let d = path.finish();
        assert_eq!(d, "M0,0M2,2L3,3Z");
        assert!(!d.contains("NaN"));
    }
}
