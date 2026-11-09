//! Target-vs-actual bars: bullet, grouped and drift.
//!
//! The rows are HTML, not SVG: labels and values wrap like text, the bars
//! keep their rounded ends at any width, and the visible text is itself what
//! a screen reader reads.

use super::common::{DataTable, frame_class, use_chart_id, use_synced};
use crate::geom::{ValueFormat, nice_domain};
use crate::model::{ChartColor, TargetRow};
use dioxus::prelude::*;
use g3_ui::{ComponentMode, use_component_mode};

/// How [`TargetBars`] draws each row.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TargetBarVariant {
    /// The actual bar with a tick at the target: compact, one line per row.
    #[default]
    Bullet,
    /// A target bar and an actual bar, one above the other.
    Grouped,
    /// Actual minus target, either side of zero: how far each row has
    /// drifted.
    Drift,
}

/// What [`TargetBars`] reports when a row is selected.
#[derive(Clone, Debug, PartialEq)]
pub struct RowSelect {
    /// The row's id.
    pub id: String,
    /// Its position in `rows`.
    pub index: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BarRow {
    pub id: String,
    pub label: String,
    pub target: f64,
    pub actual: f64,
    pub color: &'static str,
    /// Bullet and grouped: percentages of the track. Drift: `start` and
    /// `width` of the drift bar.
    pub target_pct: f64,
    pub actual_pct: f64,
    pub drift_start: f64,
    pub drift_width: f64,
}

/// The rows' positions on a shared scale. Bullet and grouped run from zero
/// to a nice maximum; drift is symmetric around zero, which sits mid-track.
pub(crate) fn bar_rows(
    rows: &[TargetRow],
    variant: TargetBarVariant,
    domain: Option<(f64, f64)>,
) -> (Vec<BarRow>, (f64, f64)) {
    let clean = |v: f64| if v.is_finite() { v } else { 0.0 };
    let domain = domain.unwrap_or_else(|| match variant {
        TargetBarVariant::Drift => {
            let widest = rows
                .iter()
                .map(|r| (clean(r.actual) - clean(r.target)).abs())
                .fold(0.0, f64::max);
            let (_, hi) = nice_domain(0.0, widest.max(f64::MIN_POSITIVE), 2);
            (-hi, hi)
        }
        _ => {
            let top = rows
                .iter()
                .map(|r| clean(r.actual).max(clean(r.target)))
                .fold(0.0, f64::max);
            (0.0, nice_domain(0.0, top.max(f64::MIN_POSITIVE), 4).1)
        }
    });
    let (lo, hi) = domain;
    let span = (hi - lo).max(f64::MIN_POSITIVE);
    let pct = |v: f64| ((clean(v) - lo) / span * 100.0).clamp(0.0, 100.0);
    let zero = pct(0.0);
    let bars = rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let drift = pct(clean(row.actual) - clean(row.target));
            BarRow {
                id: row.id.clone(),
                label: row.label.clone(),
                target: clean(row.target),
                actual: clean(row.actual),
                color: row.color.unwrap_or(ChartColor::nth(i)).css(),
                target_pct: pct(row.target),
                actual_pct: pct(row.actual),
                drift_start: zero.min(drift),
                drift_width: (drift - zero).abs(),
            }
        })
        .collect();
    (bars, domain)
}

/// Each row's actual value against its target, such as each fund's weight
/// against its target weight.
///
/// Rows are buttons when `on_select` is set, so they are focusable and
/// keyboard operable with no extra wiring; otherwise plain text. Narrow
/// containers stack the label above the bar.
#[component]
pub fn TargetBars(
    /// The rows, in order.
    rows: ReadSignal<Vec<TargetRow>>,
    /// The chart's accessible name, and the hidden table's caption.
    title: String,
    /// How each row is drawn. Defaults to [`TargetBarVariant::Bullet`].
    variant: Option<TargetBarVariant>,
    /// The scale's `(min, max)`. Defaults to zero to a nice maximum, or a
    /// symmetric range for drift.
    domain: Option<(f64, f64)>,
    /// How values are written. Defaults to whole numbers.
    format: Option<ValueFormat>,
    /// Called when a row is selected. Rows are buttons when this is set.
    on_select: Option<EventHandler<RowSelect>>,
    /// The element id prefix. Generated when not set.
    id: Option<String>,
    /// Extra classes for the figure.
    class: Option<String>,
    /// Platform look. Defaults to the ambient mode.
    mode: Option<ComponentMode>,
) -> Element {
    let mode = use_component_mode(mode);
    let id = use_chart_id(id);
    let variant = variant.unwrap_or_default();
    let settings = use_synced((variant, domain));
    let geometry = use_memo(move || {
        let (variant, domain) = settings();
        bar_rows(&rows.read(), variant, domain)
    });
    let format = format.unwrap_or_default();
    let (bars, _) = &*geometry.read();
    let variant_name = match variant {
        TargetBarVariant::Bullet => "bullet",
        TargetBarVariant::Grouped => "grouped",
        TargetBarVariant::Drift => "drift",
    };
    let table_rows: Vec<Vec<String>> = bars
        .iter()
        .map(|b| {
            vec![
                b.label.clone(),
                format.format(b.target),
                format.format(b.actual),
                format.format_signed(b.actual - b.target),
            ]
        })
        .collect();
    let value_text = |b: &BarRow| match variant {
        TargetBarVariant::Drift => format.format_signed(b.actual - b.target),
        _ => format.format(b.actual),
    };
    let detail_text = |b: &BarRow| match variant {
        TargetBarVariant::Drift => {
            format!("{} of {}", format.format(b.actual), format.format(b.target))
        }
        _ => format!("target {}", format.format(b.target)),
    };

    rsx! {
        figure {
            class: frame_class("bars", class.as_deref()),
            "data-g3-mode": mode.as_str(),
            "data-variant": variant_name,
            "aria-labelledby": "{id}-caption",
            figcaption { id: "{id}-caption", class: "g3-chart-sr", "{title}" }
            ul { class: "g3-chart-bar-list", aria_hidden: on_select.is_none().then_some("true"),
                for (index, bar) in bars.iter().enumerate() {
                    li { key: "{bar.id}", class: "g3-chart-bar-item",
                        if let Some(on_select) = on_select {
                            button {
                                class: "g3-chart-bar-row",
                                r#type: "button",
                                onclick: {
                                    let id = bar.id.clone();
                                    move |_| on_select.call(RowSelect { id: id.clone(), index })
                                },
                                BarContent { bar: bar.clone(), variant, value: value_text(bar), detail: detail_text(bar) }
                            }
                        } else {
                            div { class: "g3-chart-bar-row",
                                BarContent { bar: bar.clone(), variant, value: value_text(bar), detail: detail_text(bar) }
                            }
                        }
                    }
                }
            }
            DataTable {
                caption: title.clone(),
                headers: vec!["Name".to_string(), "Target".to_string(), "Actual".to_string(), "Drift".to_string()],
                rows: table_rows,
            }
        }
    }
}

#[component]
fn BarContent(bar: BarRow, variant: TargetBarVariant, value: String, detail: String) -> Element {
    rsx! {
        span { class: "g3-chart-bar-label", "{bar.label}" }
        span { class: "g3-chart-bar-track", aria_hidden: "true",
            match variant {
                TargetBarVariant::Bullet => rsx! {
                    span { class: "g3-chart-bar-fill", style: "width:{bar.actual_pct:.2}%;background:{bar.color}" }
                    span { class: "g3-chart-bar-target", style: "left:{bar.target_pct:.2}%" }
                },
                TargetBarVariant::Grouped => rsx! {
                    span { class: "g3-chart-bar-fill g3-chart-bar-goal", style: "width:{bar.target_pct:.2}%" }
                    span { class: "g3-chart-bar-fill", style: "width:{bar.actual_pct:.2}%;background:{bar.color}" }
                },
                TargetBarVariant::Drift => rsx! {
                    span { class: "g3-chart-bar-zero" }
                    span {
                        class: "g3-chart-bar-fill",
                        "data-sign": if bar.actual >= bar.target { "positive" } else { "negative" },
                        style: "left:{bar.drift_start:.2}%;width:{bar.drift_width:.2}%",
                    }
                },
            }
        }
        span { class: "g3-chart-bar-value", "{value}" }
        span { class: "g3-chart-bar-detail", "{detail}" }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<TargetRow> {
        vec![
            TargetRow::new("vti", "VTI", 0.40, 0.42),
            TargetRow::new("vxus", "VXUS", 0.30, 0.27),
            TargetRow::new("bnd", "BND", 0.30, 0.31),
        ]
    }

    #[test]
    fn bullet_scale_runs_from_zero_to_a_nice_top() {
        let (bars, domain) = bar_rows(&rows(), TargetBarVariant::Bullet, None);
        assert_eq!(domain, (0.0, 0.5));
        assert!((bars[0].actual_pct - 84.0).abs() < 1e-9);
        assert!((bars[0].target_pct - 80.0).abs() < 1e-9);
    }

    #[test]
    fn drift_is_symmetric_around_the_middle() {
        let (bars, (lo, hi)) = bar_rows(&rows(), TargetBarVariant::Drift, None);
        assert_eq!(lo, -hi);
        // VXUS is under target: its bar ends at the middle.
        assert!((bars[1].drift_start + bars[1].drift_width - 50.0).abs() < 1e-9);
        // VTI is over: it starts there.
        assert!((bars[0].drift_start - 50.0).abs() < 1e-9);
    }

    #[test]
    fn bad_values_and_empty_rows_are_safe() {
        let (bars, _) = bar_rows(
            &[TargetRow::new("x", "X", f64::NAN, 0.0)],
            TargetBarVariant::Bullet,
            None,
        );
        assert_eq!(bars[0].actual_pct, 0.0);
        assert!(bar_rows(&[], TargetBarVariant::Drift, None).0.is_empty());
        let (bars, _) = bar_rows(&rows(), TargetBarVariant::Bullet, Some((0.0, 0.2)));
        assert_eq!(bars[0].actual_pct, 100.0);
    }
}
