//! Largest-triangle-three-buckets downsampling (Steinarsson, 2013).
//!
//! Keeps the shape of a long series in a few hundred points, so a chart of
//! thousands draws one short `<path>`. Tooltips still read the full data.

/// The indices of the points to keep, in order: about `threshold` of them,
/// always including the first and last. Below the threshold, or when it is
/// under 3, every index is kept.
///
/// `points` are `(x, y)`; x must be sorted ascending.
///
/// ```
/// use g3_charts::geom::lttb;
///
/// let points: Vec<(f64, f64)> = (0..1000).map(|i| (i as f64, (i as f64 / 50.0).sin())).collect();
/// let kept = lttb(&points, 100);
/// assert_eq!(kept.len(), 100);
/// assert_eq!((kept[0], kept[99]), (0, 999));
/// ```
pub fn lttb(points: &[(f64, f64)], threshold: usize) -> Vec<usize> {
    let n = points.len();
    if threshold >= n || threshold < 3 {
        return (0..n).collect();
    }
    let mut kept = Vec::with_capacity(threshold);
    kept.push(0);
    // The first and last points are fixed; the rest fall into equal buckets.
    let every = (n - 2) as f64 / (threshold - 2) as f64;
    let mut a = 0;
    for bucket in 0..threshold - 2 {
        let start = (bucket as f64 * every) as usize + 1;
        let end = (((bucket + 1) as f64 * every) as usize + 1).min(n - 1);
        // The third corner is the average of the next bucket.
        let next_start = end;
        let next_end = (((bucket + 2) as f64 * every) as usize + 1).min(n);
        let (mut avg_x, mut avg_y) = (0.0, 0.0);
        let span = (next_end - next_start).max(1);
        for &(x, y) in &points[next_start..next_end.max(next_start + 1).min(n)] {
            avg_x += x;
            avg_y += y;
        }
        avg_x /= span as f64;
        avg_y /= span as f64;
        let (ax, ay) = points[a];
        let mut best = start;
        let mut best_area = -1.0;
        for (i, &(x, y)) in points.iter().enumerate().take(end).skip(start) {
            let area = ((ax - avg_x) * (y - ay) - (ax - x) * (avg_y - ay)).abs();
            // A NaN area never wins, so a missing value is skipped where it can be.
            if area > best_area {
                best_area = area;
                best = i;
            }
        }
        kept.push(best);
        a = best;
    }
    kept.push(n - 1);
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    fn series(n: usize) -> Vec<(f64, f64)> {
        (0..n)
            .map(|i| (i as f64, ((i * 7919) % 101) as f64))
            .collect()
    }

    #[test]
    fn keeps_everything_below_the_threshold() {
        assert_eq!(lttb(&series(10), 20), (0..10).collect::<Vec<_>>());
        assert_eq!(lttb(&series(10), 2), (0..10).collect::<Vec<_>>());
        assert!(lttb(&[], 5).is_empty());
    }

    #[test]
    fn keeps_the_count_ends_and_order() {
        for (n, threshold) in [(1_000, 100), (10_000, 600), (101, 100), (50, 3), (7, 5)] {
            let kept = lttb(&series(n), threshold);
            assert_eq!(kept.len(), threshold, "{n} -> {threshold}");
            assert_eq!(kept[0], 0);
            assert_eq!(*kept.last().unwrap(), n - 1);
            assert!(kept.windows(2).all(|w| w[0] < w[1]), "{n} -> {threshold}");
        }
    }

    #[test]
    fn keeps_a_lone_spike() {
        let mut points: Vec<(f64, f64)> = (0..5_000).map(|i| (i as f64, 1.0)).collect();
        points[2_345].1 = 500.0;
        assert!(lttb(&points, 50).contains(&2_345));
    }
}
