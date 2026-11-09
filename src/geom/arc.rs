//! Arc maths for donuts and sunbursts.
//!
//! Angles are degrees, clockwise from twelve o'clock, as a reader names
//! them. SVG's y axis points down, so clockwise on screen is a positive
//! angle there too.

use super::path::fmt;

/// The point at `radius` and `degrees` from the centre.
///
/// ```
/// use g3_charts::geom::polar;
///
/// let (x, y) = polar((100.0, 100.0), 50.0, 90.0);
/// assert!((x - 150.0).abs() < 1e-9 && (y - 100.0).abs() < 1e-9);
/// ```
pub fn polar(center: (f64, f64), radius: f64, degrees: f64) -> (f64, f64) {
    let radians = (degrees - 90.0).to_radians();
    (
        center.0 + radius * radians.cos(),
        center.1 + radius * radians.sin(),
    )
}

/// The angle of a point seen from the centre, in `0.0..360.0`.
pub fn angle_at(center: (f64, f64), point: (f64, f64)) -> f64 {
    let degrees = (point.1 - center.1).atan2(point.0 - center.0).to_degrees() + 90.0;
    degrees.rem_euclid(360.0)
}

/// The middle of a ring segment, where a label or tooltip anchors.
pub fn centroid(center: (f64, f64), inner: f64, outer: f64, start: f64, end: f64) -> (f64, f64) {
    polar(center, (inner + outer) / 2.0, (start + end) / 2.0)
}

/// An SVG path for the ring segment between two radii and two angles. An
/// `inner` radius of zero draws a pie wedge. A sweep of 360° or more draws a
/// whole ring as two half arcs, because one arc between equal points draws
/// nothing.
///
/// ```
/// use g3_charts::geom::sector_path;
///
/// let d = sector_path((0.0, 0.0), 0.0, 10.0, 0.0, 90.0);
/// assert_eq!(d, "M0,0L0,-10A10,10,0,0,1,10,0Z");
/// ```
pub fn sector_path(center: (f64, f64), inner: f64, outer: f64, start: f64, end: f64) -> String {
    let (start, end) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };
    let sweep = end - start;
    if !sweep.is_finite() || sweep <= 0.0 || outer <= 0.0 {
        return String::new();
    }
    let inner = inner.clamp(0.0, outer);
    if sweep >= 360.0 {
        return ring_path(center, inner, outer);
    }
    let large = u8::from(sweep > 180.0);
    let (o0, o1) = (polar(center, outer, start), polar(center, outer, end));
    let r = fmt(outer);
    let outer_arc = format!(
        "M{},{}A{r},{r},0,{large},1,{},{}",
        fmt(o0.0),
        fmt(o0.1),
        fmt(o1.0),
        fmt(o1.1)
    );
    if inner == 0.0 {
        return format!("M{},{}L{}", fmt(center.0), fmt(center.1), &outer_arc[1..]) + "Z";
    }
    let (i1, i0) = (polar(center, inner, end), polar(center, inner, start));
    let ri = fmt(inner);
    format!(
        "{outer_arc}L{},{}A{ri},{ri},0,{large},0,{},{}Z",
        fmt(i1.0),
        fmt(i1.1),
        fmt(i0.0),
        fmt(i0.1)
    )
}

fn ring_path(center: (f64, f64), inner: f64, outer: f64) -> String {
    let circle = |r: f64, sweep: u8| {
        let (top, bottom) = (polar(center, r, 0.0), polar(center, r, 180.0));
        let r = fmt(r);
        format!(
            "M{},{}A{r},{r},0,1,{sweep},{},{}A{r},{r},0,1,{sweep},{},{}Z",
            fmt(top.0),
            fmt(top.1),
            fmt(bottom.0),
            fmt(bottom.1),
            fmt(top.0),
            fmt(top.1)
        )
    };
    if inner == 0.0 {
        circle(outer, 1)
    } else {
        // The inner circle runs the other way, so the even-odd hole is empty
        // under either fill rule.
        circle(outer, 1) + &circle(inner, 0)
    }
}

/// How a ring segment is drawn as a dash along a stroked circle: what
/// `stroke-dasharray` and `stroke-dashoffset` get. Dashes animate with a CSS
/// transition in every web view, where a path's `d` does not on iOS.
#[derive(Clone, Debug, PartialEq)]
pub struct Dash {
    /// The dash length and the gap after it.
    pub array: String,
    /// Moves the dash's start to the segment's start angle.
    pub offset: String,
}

/// The dash for a segment from `start` to `end` degrees on a circle of
/// `radius`, with `gap` length units left empty at its end (the surface gap
/// between neighbours). A segment shorter than the gap keeps a sliver, so a
/// tiny value stays visible and focusable.
///
/// The circle it applies to starts at twelve o'clock (rotate it -90°).
pub fn dash(radius: f64, start: f64, end: f64, gap: f64) -> Dash {
    let circumference = std::f64::consts::TAU * radius.max(0.0);
    let (start, end) = if start <= end {
        (start, end)
    } else {
        (end, start)
    };
    let sweep = (end - start).clamp(0.0, 360.0);
    let length = circumference * sweep / 360.0;
    let drawn = if sweep >= 360.0 {
        circumference
    } else {
        (length - gap).max(length.min(0.75))
    };
    Dash {
        array: format!("{} {}", fmt(drawn), fmt(circumference)),
        offset: fmt(-circumference * start / 360.0),
    }
}

/// Split `values` into consecutive `(start, end)` angles over `sweep`
/// degrees from `start`. Negative and non-finite values count as zero; an
/// all-zero input gives zero-width segments.
pub fn angles(values: &[f64], start: f64, sweep: f64) -> Vec<(f64, f64)> {
    let clean = |v: f64| if v.is_finite() && v > 0.0 { v } else { 0.0 };
    let total: f64 = values.iter().map(|&v| clean(v)).sum();
    let mut at = start;
    values
        .iter()
        .map(|&v| {
            let share = if total > 0.0 { clean(v) / total } else { 0.0 };
            let from = at;
            at += share * sweep;
            (from, at)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: (f64, f64), b: (f64, f64)) -> bool {
        (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9
    }

    #[test]
    fn polar_and_angle_agree_clockwise_from_twelve() {
        let c = (190.0, 190.0);
        assert!(close(polar(c, 10.0, 0.0), (190.0, 180.0)));
        assert!(close(polar(c, 10.0, 180.0), (190.0, 200.0)));
        assert!(close(polar(c, 10.0, 270.0), (180.0, 190.0)));
        for degrees in [0.0, 1.0, 45.0, 90.0, 179.0, 181.0, 300.0, 359.5] {
            let back = angle_at(c, polar(c, 42.0, degrees));
            assert!((back - degrees).abs() < 1e-9, "{degrees} -> {back}");
        }
    }

    #[test]
    fn sectors_use_the_large_arc_flag_past_half_a_turn() {
        let small = sector_path((0.0, 0.0), 5.0, 10.0, 0.0, 179.0);
        let large = sector_path((0.0, 0.0), 5.0, 10.0, 0.0, 181.0);
        assert!(small.contains("A10,10,0,0,1,"));
        assert!(large.contains("A10,10,0,1,1,"));
        assert!(large.contains("A5,5,0,1,0,"));
    }

    #[test]
    fn full_and_empty_sweeps_are_handled() {
        let ring = sector_path((50.0, 50.0), 30.0, 40.0, 0.0, 360.0);
        assert_eq!(ring.matches('M').count(), 2, "{ring}");
        let disc = sector_path((50.0, 50.0), 0.0, 40.0, 10.0, 400.0);
        assert_eq!(disc.matches('A').count(), 2);
        assert_eq!(sector_path((0.0, 0.0), 1.0, 2.0, 30.0, 30.0), "");
        assert_eq!(sector_path((0.0, 0.0), 1.0, 2.0, 0.0, f64::NAN), "");
        for d in [ring, disc, sector_path((0.0, 0.0), 1.0, 2.0, 0.0, 359.99)] {
            assert!(!d.contains("NaN") && !d.contains("inf"), "{d}");
        }
    }

    #[test]
    fn centroids_sit_mid_ring() {
        let c = centroid((0.0, 0.0), 10.0, 20.0, 0.0, 180.0);
        assert!(close(c, (15.0, 0.0)));
    }

    #[test]
    fn dashes_start_at_the_segment_and_keep_a_sliver() {
        let r = 100.0 / std::f64::consts::TAU; // circumference 100
        let quarter = dash(r, 90.0, 180.0, 0.0);
        assert_eq!(quarter.array, "25 100");
        assert_eq!(quarter.offset, "-25");
        assert_eq!(dash(r, 0.0, 360.0, 2.0).array, "100 100");
        assert_eq!(dash(r, 0.0, 90.0, 2.0).array, "23 100");
        assert_eq!(dash(r, 0.0, 0.36, 2.0).array, "0.1 100");
    }

    #[test]
    fn angles_split_the_sweep_in_order() {
        let a = angles(&[1.0, 3.0, 0.0, f64::NAN, -2.0], 0.0, 360.0);
        assert_eq!(a[0], (0.0, 90.0));
        assert_eq!(a[1], (90.0, 360.0));
        assert_eq!(a[2], (360.0, 360.0));
        assert_eq!(a[4], (360.0, 360.0));
        assert!(angles(&[0.0, 0.0], 0.0, 360.0).iter().all(|(s, e)| s == e));
    }
}
