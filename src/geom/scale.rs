//! Linear scales: data values to drawing coordinates and back.

/// Maps a data domain linearly onto a drawing range.
///
/// The range may run backwards (SVG's y axis grows downward, so a y scale
/// is usually `(1000.0, 0.0)`).
///
/// ```
/// use g3_charts::geom::LinearScale;
///
/// let y = LinearScale::new((0.0, 50.0), (1000.0, 0.0));
/// assert_eq!(y.map(25.0), 500.0);
/// assert_eq!(y.invert(250.0), 37.5);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinearScale {
    /// The data interval, `(start, end)`.
    pub domain: (f64, f64),
    /// The drawing interval the domain maps onto, `(start, end)`.
    pub range: (f64, f64),
}

impl LinearScale {
    /// A scale from `domain` onto `range`.
    pub fn new(domain: (f64, f64), range: (f64, f64)) -> Self {
        Self { domain, range }
    }

    /// The drawing coordinate of a data value. A zero-width domain maps
    /// everything to the middle of the range rather than dividing by zero.
    pub fn map(&self, value: f64) -> f64 {
        let (d0, d1) = self.domain;
        let (r0, r1) = self.range;
        let span = d1 - d0;
        if span == 0.0 || !span.is_finite() {
            return (r0 + r1) / 2.0;
        }
        r0 + (value - d0) / span * (r1 - r0)
    }

    /// [`map`](Self::map), clamped to the range.
    pub fn map_clamped(&self, value: f64) -> f64 {
        let (r0, r1) = self.range;
        self.map(value).clamp(r0.min(r1), r0.max(r1))
    }

    /// The data value at a drawing coordinate.
    pub fn invert(&self, coordinate: f64) -> f64 {
        let (d0, d1) = self.domain;
        let (r0, r1) = self.range;
        let span = r1 - r0;
        if span == 0.0 || !span.is_finite() {
            return (d0 + d1) / 2.0;
        }
        d0 + (coordinate - r0) / span * (d1 - d0)
    }

    /// A data value as a percentage of the range, `0.0..=100.0` when the
    /// range is `(0, 100)`. Used for HTML labels positioned over a plot.
    pub fn percent(&self, value: f64) -> f64 {
        LinearScale::new(self.domain, (0.0, 100.0)).map(value)
    }
}

/// Evenly spaced bands, for one row or column per category.
///
/// ```
/// use g3_charts::geom::BandScale;
///
/// let rows = BandScale::new(4, (0.0, 100.0), 0.5);
/// assert_eq!(rows.step(), 25.0);
/// assert_eq!(rows.bandwidth(), 12.5);
/// assert_eq!(rows.start(1), 31.25);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BandScale {
    /// How many bands.
    pub count: usize,
    /// The interval the bands share.
    pub range: (f64, f64),
    /// The fraction of each step left empty between bands, `0.0..1.0`.
    pub padding: f64,
}

impl BandScale {
    /// `count` bands over `range`, with `padding` of each step left empty.
    pub fn new(count: usize, range: (f64, f64), padding: f64) -> Self {
        Self {
            count,
            range,
            padding: padding.clamp(0.0, 0.95),
        }
    }

    /// The distance from one band's start to the next.
    pub fn step(&self) -> f64 {
        if self.count == 0 {
            return 0.0;
        }
        (self.range.1 - self.range.0) / self.count as f64
    }

    /// The width of one band.
    pub fn bandwidth(&self) -> f64 {
        self.step() * (1.0 - self.padding)
    }

    /// Where band `index` starts.
    pub fn start(&self, index: usize) -> f64 {
        self.range.0 + self.step() * (index as f64 + self.padding / 2.0)
    }

    /// The middle of band `index`.
    pub fn center(&self, index: usize) -> f64 {
        self.start(index) + self.bandwidth() / 2.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_and_invert_round_trip() {
        let scales = [
            LinearScale::new((0.0, 1.0), (0.0, 1000.0)),
            LinearScale::new((-50.0, 50.0), (1000.0, 0.0)),
            LinearScale::new((1e9, 2e9), (12.0, 388.0)),
            LinearScale::new((0.001, 0.002), (0.0, 1.0)),
        ];
        for scale in scales {
            for t in [0.0, 0.1, 0.5, 0.77, 1.0, 1.5, -0.5] {
                let value = scale.domain.0 + t * (scale.domain.1 - scale.domain.0);
                let back = scale.invert(scale.map(value));
                let tolerance = 1e-9 * value.abs().max(1.0);
                assert!(
                    (back - value).abs() < tolerance,
                    "{scale:?} {value} -> {back}"
                );
            }
        }
    }

    #[test]
    fn ends_map_to_range_ends() {
        let y = LinearScale::new((10.0, 20.0), (1000.0, 0.0));
        assert_eq!(y.map(10.0), 1000.0);
        assert_eq!(y.map(20.0), 0.0);
        assert_eq!(y.map_clamped(30.0), 0.0);
        assert_eq!(y.map_clamped(-30.0), 1000.0);
        assert_eq!(y.percent(15.0), 50.0);
    }

    #[test]
    fn zero_width_domains_do_not_divide_by_zero() {
        let flat = LinearScale::new((5.0, 5.0), (0.0, 100.0));
        assert_eq!(flat.map(5.0), 50.0);
        assert!(flat.map(99.0).is_finite());
        let point = LinearScale::new((0.0, 1.0), (7.0, 7.0));
        assert_eq!(point.invert(7.0), 0.5);
    }

    #[test]
    fn bands_fill_the_range() {
        let bands = BandScale::new(5, (0.0, 500.0), 0.0);
        assert_eq!(bands.start(0), 0.0);
        assert_eq!(bands.start(4) + bands.bandwidth(), 500.0);
        assert_eq!(bands.center(2), 250.0);
        assert_eq!(BandScale::new(0, (0.0, 1.0), 0.1).step(), 0.0);
    }
}
