//! Line chart geometry: domains, ticks, downsampled paths, and lookups for
//! the crosshair. Pure, no Dioxus.

use crate::geom::{
    LinearScale, PathBuilder, TimeInterval, ValueFormat, linear_ticks, lttb, nearest_by,
    nice_domain, nice_step, time_interval, time_label, time_ticks,
};
use crate::model::{ChartColor, Point, Series};

/// The plot's drawing units: a 1000 × 1000 viewBox stretched to the plot's
/// box, with strokes that do not scale.
pub(crate) const PLOT: f64 = 1000.0;

/// How a [`LineChart`](crate::LineChart)'s value axis is fitted.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum YDomain {
    /// Fit the data, widened to nice ticks.
    #[default]
    Auto,
    /// Fit the data and zero.
    IncludeZero,
    /// Exactly this `(min, max)`.
    Fixed(f64, f64),
}

/// One axis tick: where it sits (percent from the left, or from the top for
/// the value axis), its label, and whether narrow charts keep it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Tick {
    pub pct: f64,
    pub label: String,
    pub sparse: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DrawnSeries {
    pub id: String,
    pub label: String,
    pub color: &'static str,
    pub line: String,
    pub area: Option<String>,
    /// The full, sorted data, for tooltips and the table.
    pub points: Vec<Point>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LineLayout {
    pub series: Vec<DrawnSeries>,
    pub x: LinearScale,
    pub y: LinearScale,
    pub x_ticks: Vec<Tick>,
    pub y_ticks: Vec<Tick>,
    pub y_format: ValueFormat,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LineOptions {
    pub y_domain: YDomain,
    pub y_format: ValueFormat,
    pub offset_minutes: i32,
    pub max_points: usize,
}

pub(crate) fn line_layout(input: &[Series], options: &LineOptions) -> LineLayout {
    let mut sorted: Vec<(usize, &Series, Vec<Point>)> = input
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let mut points = s.points.clone();
            if !points.windows(2).all(|w| w[0].t <= w[1].t) {
                points.sort_by_key(|p| p.t);
            }
            (i, s, points)
        })
        .collect();
    let finite = || {
        sorted
            .iter()
            .flat_map(|(_, _, p)| p.iter())
            .filter(|p| p.v.is_finite())
    };
    let t_min = finite().map(|p| p.t).min().unwrap_or(0);
    let t_max = finite().map(|p| p.t).max().unwrap_or(1).max(t_min + 1);
    let v_min = finite().map(|p| p.v).fold(f64::INFINITY, f64::min);
    let v_max = finite().map(|p| p.v).fold(f64::NEG_INFINITY, f64::max);
    let (v_min, v_max) = if v_min.is_finite() {
        (v_min, v_max)
    } else {
        (0.0, 1.0)
    };
    let (y0, y1) = match options.y_domain {
        YDomain::Auto => nice_domain(v_min, v_max, 5),
        YDomain::IncludeZero => nice_domain(v_min.min(0.0), v_max.max(0.0), 5),
        YDomain::Fixed(a, b) => (a.min(b), a.max(b)),
    };
    let x = LinearScale::new((t_min as f64, t_max as f64), (0.0, PLOT));
    let y = LinearScale::new((y0, y1), (PLOT, 0.0));

    let series = sorted
        .drain(..)
        .map(|(i, s, points)| {
            let xy: Vec<(f64, f64)> = points.iter().map(|p| (p.t as f64, p.v)).collect();
            let kept = lttb(&xy, options.max_points.max(3));
            let mut line = PathBuilder::with_capacity(kept.len());
            for &k in &kept {
                line.line_to(x.map(xy[k].0), y.map(xy[k].1));
            }
            let area = s.area.then(|| {
                let mut area = PathBuilder::with_capacity(kept.len() + 3);
                let finite: Vec<usize> = kept
                    .iter()
                    .copied()
                    .filter(|&k| xy[k].1.is_finite())
                    .collect();
                if let (Some(&first), Some(&last)) = (finite.first(), finite.last()) {
                    area.move_to(x.map(xy[first].0), PLOT);
                    for &k in &finite {
                        area.line_to(x.map(xy[k].0), y.map(xy[k].1));
                    }
                    area.line_to(x.map(xy[last].0), PLOT);
                    area.close();
                }
                area.finish()
            });
            DrawnSeries {
                id: s.id.clone(),
                label: s.label.clone(),
                color: s.color.unwrap_or(ChartColor::nth(i)).css(),
                line: line.finish(),
                area,
                points,
            }
        })
        .collect();

    let y_format = |count: usize| options.y_format.for_step(nice_step(y1 - y0, count));
    let sparse_y = linear_ticks(y0, y1, 3);
    let dense_format = y_format(5);
    let y_ticks = linear_ticks(y0, y1, 5)
        .into_iter()
        .map(|v| Tick {
            // Percent from the top: the value axis runs upward.
            pct: 100.0 - y.percent(v),
            label: dense_format.format(v),
            sparse: sparse_y.iter().any(|s| (s - v).abs() <= (y1 - y0) * 1e-9),
        })
        .collect();

    let span = t_max - t_min;
    let dense = time_interval(span, 6);
    let sparse = time_interval(span, 3);
    let sparse_ticks = time_ticks(t_min, t_max, sparse, options.offset_minutes);
    let x_ticks = x_tick_set(
        t_min,
        t_max,
        dense,
        &sparse_ticks,
        options.offset_minutes,
        &x,
    );

    LineLayout {
        series,
        x,
        y,
        x_ticks,
        y_ticks,
        y_format: y_format(5),
    }
}

fn x_tick_set(
    t_min: i64,
    t_max: i64,
    dense: TimeInterval,
    sparse: &[i64],
    offset: i32,
    x: &LinearScale,
) -> Vec<Tick> {
    time_ticks(t_min, t_max, dense, offset)
        .into_iter()
        .enumerate()
        .map(|(i, t)| Tick {
            pct: x.map(t as f64) / PLOT * 100.0,
            label: time_label(t, dense, offset, i == 0),
            sparse: sparse.contains(&t),
        })
        .collect()
}

/// The point in `series` nearest to time `t`.
pub(crate) fn nearest_point(points: &[Point], t: f64) -> Option<usize> {
    nearest_by(points.len(), t, |i| points[i].t as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::DAY;

    fn options() -> LineOptions {
        LineOptions {
            y_domain: YDomain::Auto,
            y_format: ValueFormat::Number { decimals: 0 },
            offset_minutes: 0,
            max_points: 600,
        }
    }

    fn daily(n: usize) -> Series {
        let start = crate::geom::days_from_civil(2021, 1, 4) * DAY;
        Series::new(
            "value",
            "Value",
            (0..n)
                .map(|i| {
                    Point::new(
                        start + i as i64 * DAY,
                        1_000.0 + (i as f64 / 9.0).sin() * 200.0 + i as f64,
                    )
                })
                .collect(),
        )
    }

    #[test]
    fn long_series_draw_one_short_path() {
        let layout = line_layout(&[daily(10_000).with_area()], &options());
        let series = &layout.series[0];
        assert_eq!(series.points.len(), 10_000);
        assert_eq!(series.line.matches('L').count(), 599);
        assert!(series.area.as_ref().unwrap().ends_with('Z'));
        assert!(!series.line.contains("NaN"));
    }

    #[test]
    fn domains_and_ticks_cover_the_data() {
        let layout = line_layout(&[daily(365)], &options());
        let (lo, hi) = layout.y.domain;
        assert!(lo <= 800.0 && hi >= 1_560.0, "{lo}..{hi}");
        assert!(
            layout
                .y_ticks
                .iter()
                .all(|t| (0.0..=100.0).contains(&t.pct))
        );
        assert!(layout.y_ticks.iter().any(|t| t.sparse));
        assert!(layout.x_ticks.len() >= 4);
        assert!(layout.x_ticks.iter().any(|t| t.sparse));
        assert!(
            layout
                .x_ticks
                .iter()
                .all(|t| (0.0..=100.0).contains(&t.pct))
        );
        // A top tick sits at the top.
        let top = layout
            .y_ticks
            .iter()
            .map(|t| t.pct)
            .fold(f64::INFINITY, f64::min);
        assert!(top.abs() < 1e-9);
    }

    #[test]
    fn unsorted_empty_and_zero_options() {
        let mut series = daily(30);
        series.points.reverse();
        let layout = line_layout(&[series], &options());
        assert!(layout.series[0].points.windows(2).all(|w| w[0].t <= w[1].t));
        let empty = line_layout(&[Series::new("e", "E", vec![])], &options());
        assert_eq!(empty.series[0].line, "");
        let zero = line_layout(
            &[daily(30)],
            &LineOptions {
                y_domain: YDomain::IncludeZero,
                ..options()
            },
        );
        assert_eq!(zero.y.domain.0, 0.0);
    }

    #[test]
    fn nearest_point_snaps_by_time() {
        let series = daily(10);
        let t = series.points[3].t as f64 + 0.4 * DAY as f64;
        assert_eq!(nearest_point(&series.points, t), Some(3));
        assert_eq!(nearest_point(&[], t), None);
    }
}
