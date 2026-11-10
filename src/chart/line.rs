//! Time-series line and area charts, with a crosshair.

use super::common::{DataTable, SvgLabel, frame_class, labelled_by, use_chart_id, use_synced};
pub use super::line_layout::YDomain;
use super::line_layout::{LineLayout, LineOptions, PLOT, line_layout, nearest_point};
use super::pointer::{fraction, optional_listener};
use crate::geom::{DAY, ValueFormat, civil_from_days, date_label, fmt};
use crate::model::Series;
use dioxus::prelude::*;
use g3_ui::{ComponentMode, use_component_mode};

/// Rows past this make the hidden table a monthly summary.
const TABLE_ROWS: usize = 400;

/// Values over time: one line per series, optionally filled to the bottom,
/// on a calendar-aligned time axis.
///
/// Long series are downsampled (largest-triangle-three-buckets) to
/// `max_points` for drawing, one `<path>` each; the crosshair, the tooltip and
/// the table read the full data.
///
/// The crosshair follows a mouse over the plot, or a finger dragged across
/// it (vertical drags still scroll the page). A tap leaves it where it
/// landed. The plot is a slider for the keyboard and screen readers: arrows
/// step through points, Page Up and Page Down jump a tenth, Home and End go
/// to the ends, and each step is announced as its date and values.
#[component]
pub fn LineChart(
    /// The series, drawn in order (the first on the bottom). The crosshair
    /// steps through the first series' points.
    series: ReadSignal<Vec<Series>>,
    /// The chart's accessible name, and the hidden table's caption.
    title: String,
    /// A longer description for screen readers.
    description: Option<String>,
    /// How values are written. Defaults to whole numbers; axis labels keep
    /// only the decimals their step needs.
    y_format: Option<ValueFormat>,
    /// How the value axis is fitted. Defaults to [`YDomain::Auto`].
    y_domain: Option<YDomain>,
    /// Minutes east of UTC that dates are shown in. Defaults to `0`.
    utc_offset_minutes: Option<i32>,
    /// The most points drawn per series. Defaults to `600`, enough for a
    /// plot 1440px wide.
    max_points: Option<usize>,
    /// The plot's CSS height, such as `"16rem"`. Defaults to a ratio that
    /// follows the chart's width.
    height: Option<String>,
    /// The element id prefix. Generated when not set.
    id: Option<String>,
    /// Extra classes for the figure.
    class: Option<String>,
    /// Platform look. Defaults to the ambient mode.
    mode: Option<ComponentMode>,
) -> Element {
    let mode = use_component_mode(mode);
    let id = use_chart_id(id);
    let format = y_format.unwrap_or_default();
    let offset = utc_offset_minutes.unwrap_or(0);
    let options = LineOptions {
        y_domain: y_domain.unwrap_or_default(),
        y_format: format,
        offset_minutes: offset,
        max_points: max_points.unwrap_or(600),
    };
    let options = use_synced(options);
    let geometry = use_memo(move || line_layout(&series.read(), &options.read()));
    let layout = geometry.read();
    let has_description = description.is_some();
    let (headers, rows) = table(&layout, format, offset);
    let style = height.map(|h| format!("height:{h}"));

    rsx! {
        figure {
            class: frame_class("line", class.as_deref()),
            "data-g3-mode": mode.as_str(),
            if layout.series.len() >= 2 {
                ul { class: "g3-chart-legend g3-chart-legend-inline", aria_hidden: "true",
                    for s in layout.series.iter() {
                        li { key: "{s.id}", class: "g3-chart-legend-item",
                            span { class: "g3-chart-swatch g3-chart-swatch-line", style: "background:{s.color}" }
                            span { class: "g3-chart-legend-label", "{s.label}" }
                        }
                    }
                }
            }
            div { class: "g3-chart-cartesian",
                div { class: "g3-chart-yaxis", aria_hidden: "true",
                    for tick in layout.y_ticks.iter() {
                        span {
                            key: "{tick.label}",
                            class: "g3-chart-tick",
                            "data-sparse": tick.sparse.then_some("true"),
                            style: "top:{tick.pct:.3}%",
                            "{tick.label}"
                        }
                    }
                }
                div { class: "g3-chart-plot", style,
                    svg {
                        class: "g3-chart-svg",
                        view_box: "0 0 {PLOT} {PLOT}",
                        preserve_aspect_ratio: "none",
                        role: "img",
                        "aria-labelledby": labelled_by(&id, has_description),
                        SvgLabel { id: id.clone(), title: title.clone(), description: description.clone() }
                        for tick in layout.y_ticks.iter() {
                            line {
                                key: "grid-{tick.label}",
                                class: "g3-chart-grid",
                                "data-sparse": tick.sparse.then_some("true"),
                                x1: "0",
                                x2: "{PLOT}",
                                y1: "{fmt(tick.pct * PLOT / 100.0)}",
                                y2: "{fmt(tick.pct * PLOT / 100.0)}",
                            }
                        }
                        for s in layout.series.iter() {
                            if let Some(area) = s.area.as_ref() {
                                path { key: "area-{s.id}", class: "g3-chart-area", d: "{area}", style: "fill:{s.color}" }
                            }
                        }
                        for s in layout.series.iter() {
                            path { key: "line-{s.id}", class: "g3-chart-line", d: "{s.line}", style: "stroke:{s.color}" }
                        }
                    }
                    Scrubber { geometry, format, offset, title: title.clone() }
                }
                div { class: "g3-chart-xaxis", aria_hidden: "true",
                    for tick in layout.x_ticks.iter() {
                        span {
                            key: "{tick.pct}",
                            class: "g3-chart-tick",
                            "data-sparse": tick.sparse.then_some("true"),
                            "data-edge": edge(tick.pct),
                            style: "left:{tick.pct:.3}%",
                            "{tick.label}"
                        }
                    }
                }
            }
            DataTable { caption: title.clone(), headers, rows }
        }
    }
}

/// Labels near an end align inward so they stay inside the chart.
fn edge(pct: f64) -> Option<&'static str> {
    if pct < 6.0 {
        Some("start")
    } else if pct > 94.0 {
        Some("end")
    } else {
        None
    }
}

/// The crosshair, its tooltip and the slider that drives them. Its own
/// component, so a pointer move re-renders these few nodes and never the
/// paths or axes.
#[component]
fn Scrubber(
    geometry: Memo<LineLayout>,
    format: ValueFormat,
    offset: i32,
    title: String,
) -> Element {
    let mut hover = use_signal(|| None::<usize>);
    let mut scrubbing = use_signal(|| false);
    let mut width = use_signal(|| 0.0_f64);

    let mut show = move |index: Option<usize>| {
        if *hover.peek() != index {
            hover.set(index);
        }
    };
    let mut at_offset = move |offset_x: f64| {
        let Some(f) = fraction(offset_x, *width.peek()) else {
            return;
        };
        let layout = geometry.peek();
        let t = layout.x.invert(f * PLOT);
        let index = layout
            .series
            .first()
            .and_then(|s| nearest_point(&s.points, t));
        drop(layout);
        show(index);
    };
    let moves = optional_listener(
        "onpointermove",
        scrubbing().then_some(move |event: Event<PointerData>| {
            at_offset(event.data().element_coordinates().x);
        }),
    );

    let layout = geometry.read();
    let count = layout.series.first().map_or(0, |s| s.points.len());
    let index = hover().filter(|&i| i < count);
    let primary = index.and_then(|i| layout.series.first().map(|s| s.points[i]));
    let readings: Vec<(&'static str, String, Option<f64>, f64)> = primary
        .map(|p| {
            layout
                .series
                .iter()
                .map(|s| {
                    let near = nearest_point(&s.points, p.t as f64).map(|i| s.points[i]);
                    let value = near.map(|n| n.v).filter(|v| v.is_finite());
                    let top = value.map_or(f64::NAN, |v| 100.0 - layout.y.percent(v));
                    (s.color, s.label.clone(), value, top)
                })
                .collect()
        })
        .unwrap_or_default();
    let x_pct = primary.map(|p| layout.x.map(p.t as f64) / PLOT * 100.0);
    let value_text = primary.map(|p| {
        let values: Vec<String> = readings
            .iter()
            .map(|(_, label, v, _)| {
                format!("{label} {}", v.map_or("–".into(), |v| format.format(v)))
            })
            .collect();
        format!("{}: {}", date_label(p.t, offset), values.join(", "))
    });
    let step = (count / 10).max(1);

    rsx! {
        div {
            class: "g3-chart-hit",
            role: "slider",
            tabindex: "0",
            aria_label: "Inspect {title}",
            aria_valuemin: "0",
            aria_valuemax: "{count.saturating_sub(1)}",
            aria_valuenow: "{index.unwrap_or(0)}",
            aria_valuetext: value_text.clone().unwrap_or_else(|| "No point selected".into()),
            onresize: move |event| {
                if let Ok(size) = event.data().get_content_box_size() {
                    width.set(size.width);
                }
            },
            onpointerenter: move |event| {
                if event.data().pointer_type() == "mouse" {
                    scrubbing.set(true);
                }
            },
            onpointerdown: move |event| {
                scrubbing.set(true);
                at_offset(event.data().element_coordinates().x);
            },
            onpointerup: move |event| {
                if event.data().pointer_type() != "mouse" {
                    scrubbing.set(false);
                }
            },
            onpointercancel: move |_| scrubbing.set(false),
            onpointerleave: move |event| {
                scrubbing.set(false);
                if event.data().pointer_type() == "mouse" {
                    show(None);
                }
            },
            onkeydown: move |event| {
                if count == 0 {
                    return;
                }
                let at = hover.peek().unwrap_or(0) as i64;
                let last = count as i64 - 1;
                let next = match event.key() {
                    Key::Named(NamedKey::ArrowRight | NamedKey::ArrowUp) => at + 1,
                    Key::Named(NamedKey::ArrowLeft | NamedKey::ArrowDown) => at - 1,
                    Key::Named(NamedKey::PageUp) => at + step as i64,
                    Key::Named(NamedKey::PageDown) => at - step as i64,
                    Key::Named(NamedKey::Home) => 0,
                    Key::Named(NamedKey::End) => last,
                    Key::Named(NamedKey::Escape) => {
                        show(None);
                        return;
                    }
                    _ => return,
                };
                event.prevent_default();
                show(Some(next.clamp(0, last) as usize));
            },
            ..vec![moves],
        }
        if let (Some(x), Some(p)) = (x_pct, primary) {
            div { class: "g3-chart-crosshair", aria_hidden: "true", style: "left:{x:.3}%",
                for (color, label, _, top) in readings.iter() {
                    if top.is_finite() {
                        span { key: "{label}", class: "g3-chart-dot", style: "top:{top:.3}%;background:{color}" }
                    }
                }
            }
            // Beside the crosshair, not inside it, so its CSS can measure the
            // plot and keep a narrow chart's tooltip within it.
            div {
                class: "g3-chart-tooltip",
                aria_hidden: "true",
                "data-side": if x > 60.0 { "left" } else { "right" },
                style: "--g3-chart-x:{x:.3}",
                span { class: "g3-chart-tooltip-title", "{date_label(p.t, offset)}" }
                for (color, label, value, _) in readings.iter() {
                    span { key: "{label}", class: "g3-chart-tooltip-row",
                        span { class: "g3-chart-swatch", style: "background:{color}" }
                        span { class: "g3-chart-tooltip-label", "{label}" }
                        span { class: "g3-chart-tooltip-value",
                            {value.map_or("–".into(), |v| format.format(v))}
                        }
                    }
                }
            }
        }
    }
}

/// The hidden table: every point, or the last point of each month once the
/// first series has more than [`TABLE_ROWS`].
fn table(layout: &LineLayout, format: ValueFormat, offset: i32) -> (Vec<String>, Vec<Vec<String>>) {
    let mut headers = vec!["Date".to_string()];
    headers.extend(layout.series.iter().map(|s| s.label.clone()));
    let Some(primary) = layout.series.first() else {
        return (headers, Vec::new());
    };
    let points = &primary.points;
    let mut picked: Vec<usize> = if points.len() > TABLE_ROWS {
        let month = |t: i64| {
            let date = civil_from_days((t + i64::from(offset) * 60_000).div_euclid(DAY));
            (date.year, date.month)
        };
        (0..points.len())
            .filter(|&i| i + 1 == points.len() || month(points[i].t) != month(points[i + 1].t))
            .collect()
    } else {
        (0..points.len()).collect()
    };
    if picked.len() > TABLE_ROWS {
        let stride = picked.len().div_ceil(TABLE_ROWS);
        let last = *picked.last().unwrap_or(&0);
        picked = picked.into_iter().step_by(stride).collect();
        if picked.last() != Some(&last) {
            picked.push(last);
        }
    }
    let rows = picked
        .into_iter()
        .map(|i| {
            let t = points[i].t;
            let mut row = vec![date_label(t, offset)];
            row.extend(layout.series.iter().map(|s| {
                nearest_point(&s.points, t as f64)
                    .map(|j| s.points[j].v)
                    .map_or("–".into(), |v| format.format(v))
            }));
            row
        })
        .collect();
    (headers, rows)
}
