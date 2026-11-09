//! Nested allocations: pies within pies.

use super::common::{DataTable, SvgLabel, frame_class, labelled_by, use_chart_id, use_synced};
use super::keys::{KeyMove, key_move};
use super::sunburst_layout::{Layout, OUTER, layout};
use crate::geom::ValueFormat;
use crate::model::Node;
use dioxus::prelude::*;
use g3_ui::{ComponentMode, use_component_mode};

/// What a [`Sunburst`] reports when a segment is selected.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeSelect {
    /// Ids from the first ring down to the selected node.
    pub path: Vec<String>,
}

const CENTER: f64 = 100.0;

/// A nested allocation as rings: the root's children on the first ring,
/// their children on the next, and so on (up to three rings).
///
/// Pass `actual` to compare holdings with targets. Its nodes are matched to
/// `root`'s by id, and a thin band outside each ring shows the actual
/// shares, so drift shows as the band's boundaries sliding past the
/// target's. The hole names the active segment with both shares.
///
/// To drill down, pass the subtree as `root` (and `actual`) when
/// `on_select` reports a path; the chart keeps no drill state.
///
/// Keyboard: one tab stop; arrows walk the segments depth first, Enter
/// selects. Every node is in the hidden table with both shares and drift.
#[component]
pub fn Sunburst(
    /// The target tree. Its own value is ignored; its children are the first
    /// ring.
    root: ReadSignal<Node>,
    /// The actual tree, matched to `root` by id. No band when `None`.
    #[props(default)]
    actual: ReadSignal<Option<Node>>,
    /// The chart's accessible name, and the hidden table's caption.
    title: String,
    /// A longer description for screen readers.
    description: Option<String>,
    /// How many rings to draw, `1..=3`. Defaults to `2`.
    depth: Option<usize>,
    /// The hole, as a fraction of the radius. Defaults to `0.34`.
    hole: Option<f64>,
    /// How values are written. Defaults to whole numbers.
    format: Option<ValueFormat>,
    /// The caption under the total in the hole.
    center_label: Option<String>,
    /// The selected node's path. The chart keeps its own when none is passed.
    selected: Option<Signal<Option<Vec<String>>>>,
    /// Called when a segment is selected.
    on_select: Option<EventHandler<NodeSelect>>,
    /// The element id prefix. Generated when not set.
    id: Option<String>,
    /// Extra classes for the figure.
    class: Option<String>,
    /// Platform look. Defaults to the ambient mode.
    mode: Option<ComponentMode>,
    /// Replaces the default centre content while no segment is active.
    center: Option<Element>,
) -> Element {
    let mode = use_component_mode(mode);
    let id = use_chart_id(id);
    let own_selection = use_signal(|| None::<Vec<String>>);
    let mut selected = selected.unwrap_or(own_selection);
    let mut active = use_signal(|| None::<usize>);
    let depth = use_synced(depth.unwrap_or(2));
    let hole = hole.unwrap_or(0.34);
    let hole_signal = use_synced(hole);
    let geometry =
        use_memo(move || layout(&root.read(), actual.read().as_ref(), depth(), hole_signal()));
    let format = format.unwrap_or_default();

    let mut select = move |index: usize| {
        let path = geometry.peek().arcs.get(index).map(|a| a.path.clone());
        if let Some(path) = path {
            active.set(Some(index));
            selected.set(Some(path.clone()));
            if let Some(on_select) = on_select {
                on_select.call(NodeSelect { path });
            }
        }
    };

    let layout = geometry.read();
    let selected_index = selected
        .read()
        .as_ref()
        .and_then(|path| layout.arcs.iter().position(|a| &a.path == path));
    let focus_index = active().or(selected_index);
    let drawn: Vec<usize> = layout
        .arcs
        .iter()
        .filter(|a| a.share > 0.0)
        .map(|a| a.index)
        .collect();
    let share = ValueFormat::Percent { decimals: 1 };
    let compare = layout.actual_total.is_some();
    let mut headers = vec![
        "Name".to_string(),
        "Value".to_string(),
        "Target share".to_string(),
    ];
    if compare {
        headers.extend(["Actual share".to_string(), "Drift".to_string()]);
    }
    let rows: Vec<Vec<String>> = layout
        .arcs
        .iter()
        .map(|a| {
            let mut row = vec![
                a.crumb.clone(),
                format.format(a.value),
                share.format(a.share),
            ];
            if let Some(actual) = a.actual_share {
                row.push(share.format(actual));
                row.push(share.format_signed(actual - a.share));
            }
            row
        })
        .collect();
    let has_description = description.is_some();
    let label_for = |a: &super::sunburst_layout::Arc| match a.actual_share {
        Some(actual) => format!(
            "{}: target {}, actual {}",
            a.crumb,
            share.format(a.share),
            share.format(actual)
        ),
        None => format!(
            "{}: {}, {}",
            a.crumb,
            format.format(a.value),
            share.format(a.share)
        ),
    };
    let first_ring = OUTER * hole.clamp(0.1, 0.8);

    rsx! {
        figure {
            class: frame_class("sunburst", class.as_deref()),
            "data-g3-mode": mode.as_str(),
            div { class: "g3-chart-body",
                div { class: "g3-chart-square",
                    svg {
                        class: "g3-chart-svg",
                        view_box: "0 0 200 200",
                        role: "group",
                        tabindex: "0",
                        "aria-labelledby": labelled_by(&id, has_description),
                        "aria-activedescendant": focus_index.map(|i| format!("{id}-seg-{i}")),
                        onkeydown: move |event| {
                            let at = focus_index.and_then(|i| drawn.iter().position(|d| *d == i));
                            match key_move(&event.key(), at, drawn.len()) {
                                KeyMove::To(position) => {
                                    event.prevent_default();
                                    active.set(Some(drawn[position]));
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
                        g { transform: "rotate(-90 {CENTER} {CENTER})",
                            for arc in layout.arcs.iter().filter(|a| a.share > 0.0) {
                                circle {
                                    key: "{arc.key}",
                                    id: "{id}-seg-{arc.index}",
                                    class: "g3-chart-seg",
                                    role: "img",
                                    "aria-label": label_for(arc),
                                    "data-depth": "{arc.depth}",
                                    "data-active": (focus_index == Some(arc.index)).then_some("true"),
                                    "data-selected": (selected_index == Some(arc.index)).then_some("true"),
                                    cx: "{CENTER}",
                                    cy: "{CENTER}",
                                    r: "{arc.radius}",
                                    style: "stroke:{arc.color};stroke-width:{arc.width};stroke-dasharray:{arc.dash.array};stroke-dashoffset:{arc.dash.offset}",
                                    onpointerenter: {
                                        let index = arc.index;
                                        move |_| active.set(Some(index))
                                    },
                                    onpointerleave: move |_| active.set(None),
                                    onclick: {
                                        let index = arc.index;
                                        move |_| select(index)
                                    },
                                }
                            }
                            for arc in layout.arcs.iter().filter(|a| a.actual_share.unwrap_or(0.0) > 0.0) {
                                if let Some((radius, band)) = arc.actual.as_ref() {
                                    circle {
                                        key: "actual-{arc.key}",
                                        class: "g3-chart-band",
                                        "aria-hidden": "true",
                                        cx: "{CENTER}",
                                        cy: "{CENTER}",
                                        r: "{radius}",
                                        style: "stroke:{arc.color};stroke-dasharray:{band.array};stroke-dashoffset:{band.offset}",
                                    }
                                }
                            }
                        }
                    }
                    if first_ring > 18.0 {
                        SunburstCenter {
                            geometry,
                            active,
                            selected_index,
                            format,
                            center_label: center_label.clone(),
                            custom: center.clone(),
                        }
                    }
                }
                if compare {
                    p { class: "g3-chart-key", aria_hidden: "true",
                        span { class: "g3-chart-key-target" }
                        "Target"
                        span { class: "g3-chart-key-actual" }
                        "Actual"
                    }
                }
            }
            DataTable { caption: title.clone(), headers, rows }
        }
    }
}

#[component]
fn SunburstCenter(
    geometry: Memo<Layout>,
    active: Signal<Option<usize>>,
    selected_index: Option<usize>,
    format: ValueFormat,
    center_label: Option<String>,
    custom: Option<Element>,
) -> Element {
    let layout = geometry.read();
    let shown = active().or(selected_index).and_then(|i| layout.arcs.get(i));
    let share = ValueFormat::Percent { decimals: 1 };
    rsx! {
        div { class: "g3-chart-center", aria_hidden: "true",
            if let Some(arc) = shown {
                span { class: "g3-chart-center-label", "{arc.label}" }
                if let Some(actual) = arc.actual_share {
                    span { class: "g3-chart-center-value", "{share.format(actual)}" }
                    span { class: "g3-chart-center-caption", "target {share.format(arc.share)}" }
                } else {
                    span { class: "g3-chart-center-value", "{share.format(arc.share)}" }
                    span { class: "g3-chart-center-caption", "{format.format(arc.value)}" }
                }
            } else if let Some(custom) = custom {
                {custom}
            } else {
                span { class: "g3-chart-center-value", "{format.format(layout.total)}" }
                if let Some(label) = center_label {
                    span { class: "g3-chart-center-caption", "{label}" }
                }
            }
        }
    }
}
