//! Nearest point by binary search, for crosshairs over sorted x values.

/// The index of the value in `sorted` nearest to `x`, or `None` when it is
/// empty. Ties go to the earlier point. `O(log n)`.
///
/// ```
/// use g3_charts::geom::nearest_index;
///
/// assert_eq!(nearest_index(&[0, 10, 20], 14.0), Some(1));
/// assert_eq!(nearest_index(&[0, 10, 20], 16.0), Some(2));
/// ```
pub fn nearest_index<T: Copy + PartialOrd + Into<f64>>(sorted: &[T], x: f64) -> Option<usize> {
    nearest_by(sorted.len(), x, |i| sorted[i].into())
}

/// [`nearest_index`] over a sorted key read from each of `len` items, so a
/// slice of records can be searched without copying their keys out.
pub fn nearest_by(len: usize, x: f64, key: impl Fn(usize) -> f64) -> Option<usize> {
    if len == 0 {
        return None;
    }
    let mut lo = 0;
    let mut hi = len;
    // The first index whose key is >= x (partition_point by hand, over a
    // key function rather than a slice).
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if key(mid) < x {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    if lo == 0 {
        return Some(0);
    }
    if lo == len {
        return Some(len - 1);
    }
    let before = x - key(lo - 1);
    let after = key(lo) - x;
    Some(if after < before { lo } else { lo - 1 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edges_and_ties() {
        let xs = [0.0, 10.0, 20.0, 30.0];
        assert_eq!(nearest_index(&xs, -100.0), Some(0));
        assert_eq!(nearest_index(&xs, 100.0), Some(3));
        assert_eq!(nearest_index(&xs, 5.0), Some(0));
        assert_eq!(nearest_index(&xs, 25.0), Some(2));
        assert_eq!(nearest_index(&xs, 20.0), Some(2));
        assert_eq!(nearest_index::<f64>(&[], 1.0), None);
        assert_eq!(nearest_index(&[7.0], 1e9), Some(0));
    }

    #[test]
    fn duplicate_keys_pick_the_first() {
        let xs = [1.0, 2.0, 2.0, 2.0, 3.0];
        assert_eq!(nearest_index(&xs, 2.0), Some(1));
    }

    #[test]
    fn matches_a_linear_scan() {
        let xs: Vec<f64> = (0..500).map(|i| (i * i) as f64 / 7.0).collect();
        for step in 0..2_000 {
            let x = step as f64 * 18.0 - 100.0;
            let scan = xs
                .iter()
                .enumerate()
                .min_by(|a, b| (a.1 - x).abs().total_cmp(&(b.1 - x).abs()))
                .map(|(i, _)| i);
            assert_eq!(nearest_index(&xs, x), scan, "x = {x}");
        }
    }

    #[test]
    fn searches_by_key() {
        let points = [(0_i64, 'a'), (100, 'b'), (250, 'c')];
        let found = nearest_by(points.len(), 180.0, |i| points[i].0 as f64);
        assert_eq!(found, Some(2));
    }
}
