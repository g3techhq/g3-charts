//! "Nice" axis ticks: steps of 1, 2 or 5 times a power of ten.

/// The nice step that splits `span` into about `count` intervals.
///
/// Returns `0.0` for an empty or non-finite span, or a zero count.
///
/// ```
/// use g3_charts::geom::nice_step;
///
/// assert_eq!(nice_step(100.0, 5), 20.0);
/// assert_eq!(nice_step(1.0, 4), 0.2);
/// ```
pub fn nice_step(span: f64, count: usize) -> f64 {
    if !span.is_finite() || span <= 0.0 || count == 0 {
        return 0.0;
    }
    let raw = span / count as f64;
    let power = 10f64.powf(raw.log10().floor());
    let fraction = raw / power;
    // Thresholds are the geometric means between 1, 2, 5 and 10, so the
    // chosen step is the one closest to the raw step on a log scale.
    let nice = if fraction < 1.414 {
        1.0
    } else if fraction < 3.162 {
        2.0
    } else if fraction < 7.071 {
        5.0
    } else {
        10.0
    };
    nice * power
}

/// Widen `(min, max)` outward to whole multiples of the nice step for
/// `count` ticks. A zero-width domain is widened around its value, so a flat
/// series still gets an axis.
///
/// ```
/// use g3_charts::geom::nice_domain;
///
/// assert_eq!(nice_domain(3.0, 97.0, 5), (0.0, 100.0));
/// ```
pub fn nice_domain(min: f64, max: f64, count: usize) -> (f64, f64) {
    let (min, max) = widen_flat(min, max);
    let step = nice_step(max - min, count);
    if step == 0.0 {
        return (min, max);
    }
    (
        snap((min / step).floor() * step, step),
        snap((max / step).ceil() * step, step),
    )
}

/// Tick values covering `(min, max)` at the nice step for about `count`
/// ticks, inside the domain (both ends included when they fall on a step).
///
/// ```
/// use g3_charts::geom::linear_ticks;
///
/// assert_eq!(linear_ticks(0.0, 1.0, 5), vec![0.0, 0.2, 0.4, 0.6, 0.8, 1.0]);
/// ```
pub fn linear_ticks(min: f64, max: f64, count: usize) -> Vec<f64> {
    let (min, max) = widen_flat(min, max);
    let step = nice_step(max - min, count);
    if step == 0.0 {
        return Vec::new();
    }
    // Work in whole step counts, so 0.1 + 0.2 never shows as 0.30000000000000004.
    let first = (min / step - 1e-9).ceil() as i64;
    let last = (max / step + 1e-9).floor() as i64;
    (first..=last)
        .map(|i| snap(i as f64 * step, step))
        .collect()
}

/// The number of decimals a tick label needs at this step.
///
/// ```
/// use g3_charts::geom::step_decimals;
///
/// assert_eq!(step_decimals(0.25), 2);
/// assert_eq!(step_decimals(50.0), 0);
/// ```
pub fn step_decimals(step: f64) -> usize {
    if !step.is_finite() || step <= 0.0 || step >= 1.0 {
        return 0;
    }
    let mut decimals = 0;
    let mut scaled = step;
    while decimals < 12 && (scaled - scaled.round()).abs() > 1e-9 * scaled.max(1.0) {
        scaled *= 10.0;
        decimals += 1;
    }
    decimals
}

fn widen_flat(min: f64, max: f64) -> (f64, f64) {
    let (min, max) = if min <= max { (min, max) } else { (max, min) };
    if max - min > f64::EPSILON * max.abs().max(1.0) {
        return (min, max);
    }
    let pad = if min == 0.0 { 1.0 } else { min.abs() * 0.1 };
    (min - pad, max + pad)
}

/// Round away float noise below a thousandth of a step.
fn snap(value: f64, step: f64) -> f64 {
    let decimals = step_decimals(step) as i32 + 3;
    let factor = 10f64.powi(decimals);
    let snapped = (value * factor).round() / factor;
    if snapped == 0.0 { 0.0 } else { snapped }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_are_one_two_or_five_times_a_power_of_ten() {
        for exponent in -6..=12 {
            for span in [1.0, 1.7, 3.3, 4.9, 7.5, 9.99] {
                let span = span * 10f64.powi(exponent);
                for count in 2..=10 {
                    let step = nice_step(span, count);
                    let power = 10f64.powf(step.log10().floor());
                    let mantissa = (step / power * 1000.0).round() / 1000.0;
                    assert!(
                        [1.0, 2.0, 5.0, 10.0].contains(&mantissa),
                        "span {span}, count {count}: step {step}"
                    );
                    let ticks = span / step;
                    assert!(
                        ticks >= count as f64 / 2.5 && ticks <= count as f64 * 2.5,
                        "span {span}, count {count}: {ticks} intervals"
                    );
                }
            }
        }
    }

    #[test]
    fn degenerate_inputs_give_no_step() {
        assert_eq!(nice_step(0.0, 5), 0.0);
        assert_eq!(nice_step(-1.0, 5), 0.0);
        assert_eq!(nice_step(f64::NAN, 5), 0.0);
        assert_eq!(nice_step(f64::INFINITY, 5), 0.0);
        assert_eq!(nice_step(10.0, 0), 0.0);
        assert!(linear_ticks(f64::NAN, 1.0, 5).is_empty());
    }

    #[test]
    fn domains_widen_to_steps_and_contain_the_data() {
        assert_eq!(nice_domain(3.0, 97.0, 5), (0.0, 100.0));
        assert_eq!(nice_domain(-12.0, 37.0, 5), (-20.0, 40.0));
        assert_eq!(nice_domain(0.013, 0.087, 4), (0.0, 0.1));
        let (lo, hi) = nice_domain(123_456.0, 987_654.0, 6);
        assert!(lo <= 123_456.0 && hi >= 987_654.0);
        // Reversed input is accepted.
        assert_eq!(nice_domain(97.0, 3.0, 5), (0.0, 100.0));
    }

    #[test]
    fn a_flat_series_still_gets_an_axis() {
        let (lo, hi) = nice_domain(50.0, 50.0, 5);
        assert!(lo < 50.0 && hi > 50.0);
        let (lo, hi) = nice_domain(0.0, 0.0, 5);
        assert!(lo < 0.0 && hi > 0.0);
        assert!(linear_ticks(50.0, 50.0, 5).len() >= 2);
    }

    #[test]
    fn ticks_are_exact_and_inside_the_domain() {
        assert_eq!(linear_ticks(0.0, 0.3, 3), vec![0.0, 0.1, 0.2, 0.3]);
        assert_eq!(linear_ticks(-20.0, 40.0, 3), vec![-20.0, 0.0, 20.0, 40.0]);
        assert_eq!(linear_ticks(1.0, 9.0, 4), vec![2.0, 4.0, 6.0, 8.0]);
        let ticks = linear_ticks(-0.000_003, 0.000_007, 5);
        assert!(ticks.contains(&0.0));
        assert!(ticks.iter().all(|t| (-0.000_003..=0.000_007).contains(t)));
    }

    #[test]
    fn decimals_follow_the_step() {
        assert_eq!(step_decimals(0.1), 1);
        assert_eq!(step_decimals(0.05), 2);
        assert_eq!(step_decimals(0.000_002), 6);
        assert_eq!(step_decimals(2.0), 0);
    }
}
