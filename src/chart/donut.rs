//! A donut or pie, with a centre label.

use super::common::{DataTable, SvgLabel, frame_class, labelled_by, use_chart_id, use_synced};
use super::keys::{KeyMove, key_move};
use crate::geom::{Dash, ValueFormat, angles, dash};
use crate::model::{ChartColor, Slice};
use dioxus::prelude::*;
use g3_ui::{ComponentMode, use_component_mode};

/// What a [`Donut`] reports when a slice is tapped, clicked or chosen with
/// Enter.
#[derive(Clone, Debug, PartialEq)]
pub struct SliceSelect {
    /// The slice's id.
    pub id: String,
    /// Its position in `slices`.
    pub index: usize,
}

const SIZE: f64 = 200.0;
const CENTER: f64 = SIZE / 2.0;
/// Leaves room inside the viewBox for the active segment's outline.
const OUTER: f64 = 94.0;
/// The surface gap between neighbouring segments, in viewBox units (about
/// 2px at the usual size).
const GAP: f64 = 1.6;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Segment {
    pub index: usize,
    pub id: String,
    pub label: String,
    pub value: f64,
    pub share: f64,
    pub color: &'static str,
    pub dash: Dash,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Ring {
    pub total: f64,
    /// Every slice, drawn or not, in data order.
    pub segments: Vec<Segment>,
}

pub(crate) fn ring(slices: &[Slice], radius: f64) -> Ring {
    let values: Vec<f64> = slices.iter().map(|s| s.value).collect();
    let total: f64 = values.iter().filter(|v| v.is_finite() && **v > 0.0).sum();
    let segments = slices
        .iter()
        .zip(angles(&values, 0.0, 360.0))
        .enumerate()
        .map(|(index, (slice, (start, end)))| Segment {
            index,
            id: slice.id.clone(),
            label: slice.label.clone(),
            value: slice.value,
            share: if total > 0.0 {
                (end - start) / 360.0
            } else {
                0.0
            },
            color: slice.color.unwrap_or(ChartColor::nth(index)).css(),
            dash: dash(radius, start, end, GAP),
        })
        .collect();
    Ring { total, segments }
}

/// A donut (or, with `hole: Some(0.0)`, a pie) of parts of a whole, drawn
/// clockwise from twelve o'clock in data order, with a label in the hole.
///
/// Hovering, focusing or tapping a slice shows its label, value and share in
/// the hole. The chart is one tab stop: arrow keys move between slices,
/// Home and End jump to the ends, and Enter or Space selects. Every value,
/// including empty slices, is also in a visually hidden table.
///
/// ```
/// use dioxus::prelude::*;
/// use g3_charts::prelude::*;
///
/// fn Allocation() -> Element {
///     let slices = vec![
///         Slice::new("vti", "US stocks", 60_000.0),
///         Slice::new("vxus", "International", 25_000.0),
///         Slice::new("bnd", "Bonds", 15_000.0),
///     ];
///     rsx! {
///         Donut {
///             slices,
///             title: "Portfolio by asset class",
///             format: ValueFormat::Currency { symbol: "$", decimals: 0 },
///             center_label: "Invested",
///         }
///     }
/// }
/// ```
#[component]
pub fn Donut(
    /// The slices, in drawing order.
    slices: ReadSignal<Vec<Slice>>,
    /// The chart's accessible name, and the hidden table's caption.
    title: String,
    /// A longer description for screen readers: what the chart shows and
    /// any conclusion it supports.
    description: Option<String>,
    /// The hole, as a fraction of the radius. Defaults to `0.62`; `0.0`
    /// draws a pie, which has no centre label.
    hole: Option<f64>,
    /// How values are written. Defaults to whole numbers.
    format: Option<ValueFormat>,
    /// The caption under the total in the hole, such as `"Invested"`.
    center_label: Option<String>,
    /// Show the legend. Defaults to `true` when there are two slices or more.
    legend: Option<bool>,
    /// The selected slice's id. The chart keeps its own when none is passed.
    selected: Option<Signal<Option<String>>>,
    /// Called when a slice is selected.
    on_select: Option<EventHandler<SliceSelect>>,
    /// The element id prefix. Generated when not set.
    id: Option<String>,
    /// Extra classes for the figure.
    class: Option<String>,
    /// Platform look. Defaults to the ambient mode.
    mode: Option<ComponentMode>,
    /// Replaces the default centre content (the total and `center_label`)
    /// while no slice is active.
    center: Option<Element>,
) -> Element {
    let mode = use_component_mode(mode);
    let id = use_chart_id(id);
    let own_selection = use_signal(|| None::<String>);
    let mut selected = selected.unwrap_or(own_selection);
    let mut active = use_signal(|| None::<usize>);
    let hole = hole.unwrap_or(0.62).clamp(0.0, 0.9);
    let inner = OUTER * hole;
    let radius = (OUTER + inner) / 2.0;
    let width = OUTER - inner;
    let radius_signal = use_synced(radius);
    let geometry = use_memo(move || ring(&slices.read(), radius_signal()));
    let format = format.unwrap_or_default();

    let mut select = move |index: usize| {
        let picked = geometry.peek().segments.get(index).map(|s| s.id.clone());
        if let Some(picked) = picked {
            active.set(Some(index));
            selected.set(Some(picked.clone()));
            if let Some(on_select) = on_select {
                on_select.call(SliceSelect { id: picked, index });
            }
        }
    };

    let ring = geometry.read();
    let drawn: Vec<&Segment> = ring.segments.iter().filter(|s| s.share > 0.0).collect();
    let selected_index = selected
        .read()
        .as_ref()
        .and_then(|id| ring.segments.iter().position(|s| &s.id == id));
    let focus_index = active().or(selected_index);
    let show_legend = legend.unwrap_or(ring.segments.len() >= 2);
    let share_format = ValueFormat::Percent { decimals: 1 };
    let rows: Vec<Vec<String>> = ring
        .segments
        .iter()
        .map(|s| {
            vec![
                s.label.clone(),
                format.format(s.value),
                share_format.format(s.share),
            ]
        })
        .collect();
    let has_description = description.is_some();
    let drawn_indices: Vec<usize> = drawn.iter().map(|s| s.index).collect();

    rsx! {
        figure {
            class: frame_class("donut", class.as_deref()),
            "data-g3-mode": mode.as_str(),
            div { class: "g3-chart-body",
                div { class: "g3-chart-square",
                    svg {
                        class: "g3-chart-svg",
                        view_box: "0 0 {SIZE} {SIZE}",
                        role: "group",
                        tabindex: "0",
                        "aria-labelledby": labelled_by(&id, has_description),
                        "aria-activedescendant": focus_index.map(|i| format!("{id}-seg-{i}")),
                        onkeydown: move |event| {
                            let count = drawn_indices.len();
                            let at = focus_index.and_then(|i| drawn_indices.iter().position(|d| *d == i));
                            match key_move(&event.key(), at, count) {
                                KeyMove::To(position) => {
                                    event.prevent_default();
                                    active.set(Some(drawn_indices[position]));
                                }
                                KeyMove::Activate => {
                                    if let Some(index) = focus_index {
                                        event.prevent_default();
                                        select(index);
                                    }
                                }
                                KeyMove::Leave => active.set(None),
                                KeyMove::None => {}
                            }
                        },
                        onblur: move |_| active.set(None),
                        SvgLabel { id: id.clone(), title: title.clone(), description: description.clone() }
                        circle {
                            class: "g3-chart-track",
                            cx: "{CENTER}",
                            cy: "{CENTER}",
                            r: "{radius}",
                            style: "stroke-width:{width}",
                        }
                        g { transform: "rotate(-90 {CENTER} {CENTER})",
                            for segment in drawn.iter() {
                                circle {
                                    key: "{segment.id}",
                                    id: "{id}-seg-{segment.index}",
                                    class: "g3-chart-seg",
                                    role: "img",
                                    "aria-label": "{segment.label}: {format.format(segment.value)}, {share_format.format(segment.share)}",
                                    "data-active": (focus_index == Some(segment.index)).then_some("true"),
                                    "data-selected": (selected_index == Some(segment.index)).then_some("true"),
                                    cx: "{CENTER}",
                                    cy: "{CENTER}",
                                    r: "{radius}",
                                    style: "stroke:{segment.color};stroke-width:{width};stroke-dasharray:{segment.dash.array};stroke-dashoffset:{segment.dash.offset}",
                                    onpointerenter: {
                                        let index = segment.index;
                                        move |_| active.set(Some(index))
                                    },
                                    onpointerleave: move |_| active.set(None),
                                    onclick: {
                                        let index = segment.index;
                                        move |_| select(index)
                                    },
                                }
                            }
                        }
                    }
                    if hole > 0.0 {
                        DonutCenter {
                            geometry,
                            active,
                            selected_index,
                            format,
                            center_label: center_label.clone(),
                            custom: center.clone(),
                        }
                    }
                }
                if show_legend {
                    ul { class: "g3-chart-legend", aria_hidden: "true",
                        for segment in ring.segments.iter() {
                            li {
                                key: "{segment.id}",
                                class: "g3-chart-legend-item",
                                "data-active": (focus_index == Some(segment.index)).then_some("true"),
                                span { class: "g3-chart-swatch", style: "background:{segment.color}" }
                                span { class: "g3-chart-legend-label", "{segment.label}" }
                                span { class: "g3-chart-legend-value", "{share_format.format(segment.share)}" }
                            }
                        }
                    }
                }
            }
            DataTable {
                caption: title.clone(),
                headers: vec!["Name".to_string(), "Value".to_string(), "Share".to_string()],
                rows,
            }
        }
    }
}

/// The hole's text. Its own component, so hovering a slice re-renders only
/// this, not the segments.
#[component]
fn DonutCenter(
    geometry: Memo<Ring>,
    active: Signal<Option<usize>>,
    selected_index: Option<usize>,
    format: ValueFormat,
    center_label: Option<String>,
    custom: Option<Element>,
) -> Element {
    let ring = geometry.read();
    let shown = active()
        .or(selected_index)
        .and_then(|i| ring.segments.get(i));
    let share = ValueFormat::Percent { decimals: 1 };
    rsx! {
        div { class: "g3-chart-center", aria_hidden: "true",
            if let Some(segment) = shown {
                span { class: "g3-chart-center-label", "{segment.label}" }
                span { class: "g3-chart-center-value", "{format.format(segment.value)}" }
                span { class: "g3-chart-center-caption", "{share.format(segment.share)}" }
            } else if let Some(custom) = custom {
                {custom}
            } else {
                span { class: "g3-chart-center-value", "{format.format(ring.total)}" }
                if let Some(label) = center_label {
                    span { class: "g3-chart-center-caption", "{label}" }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shares_follow_values_and_skip_bad_ones() {
        let slices = vec![
            Slice::new("a", "A", 30.0),
            Slice::new("b", "B", 0.0),
            Slice::new("c", "C", f64::NAN),
            Slice::new("d", "D", 70.0),
        ];
        let ring = ring(&slices, 50.0);
        assert_eq!(ring.total, 100.0);
        let shares: Vec<f64> = ring
            .segments
            .iter()
            .map(|s| (s.share * 1e6).round() / 1e6)
            .collect();
        assert_eq!(shares, vec![0.3, 0.0, 0.0, 0.7]);
        assert!(
            ring.segments[3]
                .color
                .starts_with("var(--g3-color-chart-4,")
        );
    }

    #[test]
    fn an_empty_total_draws_nothing() {
        let ring = ring(&[Slice::new("a", "A", 0.0)], 50.0);
        assert_eq!(ring.total, 0.0);
        assert!(ring.segments.iter().all(|s| s.share == 0.0));
    }
}
