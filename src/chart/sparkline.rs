//! A word-sized line: no axes, no interaction.

use crate::geom::{LinearScale, PathBuilder, ValueFormat, lttb};
use crate::model::ChartColor;
use dioxus::prelude::*;

const WIDTH: f64 = 100.0;
const HEIGHT: f64 = 30.0;
/// Enough for a sparkline at any size it is drawn.
const MAX_POINTS: usize = 120;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Spark {
    pub line: String,
    pub area: String,
    pub summary: Option<(f64, f64, f64, f64)>,
}

pub(crate) fn spark(values: &[f64]) -> Spark {
    let finite: Vec<f64> = values.iter().copied().filter(|v| v.is_finite()).collect();
    let (Some(&first), Some(&last)) = (finite.first(), finite.last()) else {
        return Spark {
            line: String::new(),
            area: String::new(),
            summary: None,
        };
    };
    let low = finite.iter().copied().fold(f64::INFINITY, f64::min);
    let high = finite.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let x = LinearScale::new((0.0, (values.len().max(2) - 1) as f64), (0.0, WIDTH));
    // A 2-unit margin keeps the 2px stroke inside the box at the extremes.
    let y = LinearScale::new((low, high), (HEIGHT - 2.0, 2.0));
    let xy: Vec<(f64, f64)> = values
        .iter()
        .enumerate()
        .map(|(i, &v)| (i as f64, v))
        .collect();
    let kept = lttb(&xy, MAX_POINTS);
    let mut line = PathBuilder::with_capacity(kept.len());
    let mut area = PathBuilder::with_capacity(kept.len() + 3);
    let mut opened = false;
    for &k in &kept {
        let (i, v) = xy[k];
        line.line_to(x.map(i), y.map(v));
        if v.is_finite() {
            if !opened {
                area.move_to(x.map(i), HEIGHT);
                opened = true;
            }
            area.line_to(x.map(i), y.map(v));
        }
    }
    if let Some(&k) = kept.iter().rev().find(|&&k| xy[k].1.is_finite()) {
        area.line_to(x.map(xy[k].0), HEIGHT);
        area.close();
    }
    Spark {
        line: line.finish(),
        area: area.finish(),
        summary: Some((first, last, low, high)),
    }
}

/// A small trend line to sit beside a number, such as a holding's last 30
/// days. An image to assistive technology, named by `label` plus its first,
/// last, lowest and highest values.
#[component]
pub fn Sparkline(
    /// The values, evenly spaced. A non-finite value leaves a gap.
    values: ReadSignal<Vec<f64>>,
    /// What it shows, such as `"VTI, last 30 days"`.
    label: String,
    /// Fill under the line.
    area: Option<bool>,
    /// Defaults to the accent.
    color: Option<ChartColor>,
    /// How the summary values are written. Defaults to whole numbers.
    format: Option<ValueFormat>,
    /// Extra classes for the SVG.
    class: Option<String>,
) -> Element {
    let geometry = use_memo(move || spark(&values.read()));
    let spark = geometry.read();
    let color = color.unwrap_or(ChartColor::Accent).css();
    let format = format.unwrap_or_default();
    let name = match spark.summary {
        Some((first, last, low, high)) => format!(
            "{label}: from {} to {}, low {}, high {}",
            format.format(first),
            format.format(last),
            format.format(low),
            format.format(high)
        ),
        None => format!("{label}: no data"),
    };
    rsx! {
        svg {
            class: g3_ui::merge_classes("g3-chart-sparkline", class.as_deref()),
            view_box: "0 0 {WIDTH} {HEIGHT}",
            preserve_aspect_ratio: "none",
            role: "img",
            "aria-label": name,
            if area.unwrap_or(false) {
                path { class: "g3-chart-area", d: "{spark.area}", style: "fill:{color}" }
            }
            path { class: "g3-chart-line", d: "{spark.line}", style: "stroke:{color}" }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarises_and_fits_the_box() {
        let s = spark(&[3.0, 1.0, f64::NAN, 4.0, 1.5]);
        assert_eq!(s.summary, Some((3.0, 1.5, 1.0, 4.0)));
        assert!(s.line.starts_with("M0,"));
        assert!(s.area.ends_with('Z'));
        assert!(!s.line.contains("NaN"));
    }

    #[test]
    fn empty_and_flat_series_are_safe() {
        assert_eq!(spark(&[]).summary, None);
        assert_eq!(spark(&[f64::NAN]).line, "");
        let flat = spark(&[5.0, 5.0, 5.0]);
        assert!(flat.line.contains("15"), "{}", flat.line);
        assert_eq!(spark(&[2.0]).line, "M0,15");
    }

    #[test]
    fn long_series_are_downsampled() {
        let values: Vec<f64> = (0..5_000).map(|i| (i as f64).sqrt()).collect();
        assert_eq!(spark(&values).line.matches('L').count(), MAX_POINTS - 1);
    }
}
