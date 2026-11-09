//! Markup tests, rendered with dioxus-ssr: roles, labels, ids, the hidden
//! table, and that every `g3-chart` class the markup uses is in the
//! stylesheet.

use crate::geom::{DAY, ValueFormat, days_from_civil};
use crate::prelude::*;
use dioxus::prelude::*;
use std::collections::BTreeSet;

const STYLESHEET: &str = include_str!("../assets/g3-charts.css");

fn stylesheet_classes() -> BTreeSet<String> {
    let mut classes = BTreeSet::new();
    for (i, _) in STYLESHEET.match_indices(".g3-") {
        let rest = &STYLESHEET[i + 1..];
        let end = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
            .unwrap_or(rest.len());
        classes.insert(rest[..end].to_string());
    }
    classes
}

fn render(app: fn() -> Element) -> String {
    let mut dom = VirtualDom::new(app);
    dom.rebuild_in_place();
    let html = dioxus_ssr::render(&dom);
    let defined = stylesheet_classes();
    let mut missing = BTreeSet::new();
    for chunk in html.split("class=\"").skip(1) {
        let value = chunk.split('"').next().unwrap_or_default();
        for class in value.split_whitespace() {
            if class.starts_with("g3-chart") && !defined.contains(class) {
                missing.insert(class.to_string());
            }
        }
    }
    assert!(
        missing.is_empty(),
        "classes missing from g3-charts.css: {missing:?}"
    );
    assert!(!html.contains("NaN") && !html.contains("inf"), "{html}");
    html
}

fn count(html: &str, needle: &str) -> usize {
    html.matches(needle).count()
}

fn allocation() -> Node {
    Node::group(
        "root",
        "Portfolio",
        vec![
            Node::group(
                "us",
                "US stocks",
                vec![Node::leaf("vti", "VTI", 45.0), Node::leaf("vb", "VB", 10.0)],
            ),
            Node::group(
                "intl",
                "International",
                vec![Node::leaf("vxus", "VXUS", 30.0)],
            ),
            Node::leaf("bnd", "Bonds", 15.0),
        ],
    )
}

#[test]
fn donut_is_labelled_and_tabled() {
    let html = render(|| {
        rsx! {
            Donut {
                slices: vec![
                    Slice::new("a", "Stocks <US>", 60.0),
                    Slice::new("b", "Bonds", 40.0),
                    Slice::new("c", "Cash", 0.0),
                ],
                title: "Allocation",
                description: "Most is in stocks.",
                format: ValueFormat::Currency { symbol: "$", decimals: 0 },
                center_label: "Invested",
            }
        }
    });
    assert!(html.contains(r#"role="group""#));
    assert!(html.contains(r#"tabindex="0""#));
    assert!(html.contains(r#"aria-labelledby="g3c-1-title g3c-1-desc""#));
    // The title is escaped inside the SVG title.
    assert!(
        html.contains("<title id=\"g3c-1-title\">Allocation</title>"),
        "{html}"
    );
    assert!(html.contains("<desc id=\"g3c-1-desc\">Most is in stocks.</desc>"));
    // Two drawn segments; the empty one stays in the table.
    assert_eq!(count(&html, r#"class="g3-chart-seg""#), 2);
    assert!(
        html.contains(r#"aria-label="Stocks &#60;US&#62;: $60, 60.0%""#),
        "{html}"
    );
    assert!(html.contains("<td>$0</td>"));
    assert!(html.contains(r#"<th scope="row">Cash</th>"#));
    // The hole shows the total.
    assert!(html.contains("$100"));
    assert!(html.contains("Invested"));
    assert!(!html.contains("aria-activedescendant"));
}

#[test]
fn each_chart_gets_its_own_ids() {
    let html = render(|| {
        rsx! {
            Donut { slices: vec![Slice::new("a", "A", 1.0)], title: "One" }
            Donut { slices: vec![Slice::new("a", "A", 1.0)], title: "Two" }
        }
    });
    assert!(html.contains("g3c-1-title") && html.contains("g3c-2-title"));
    assert!(html.contains("g3c-2-seg-0"));
}

#[test]
fn a_selected_slice_is_the_active_descendant() {
    #[component]
    fn Selected() -> Element {
        let selected = use_signal(|| Some("b".to_string()));
        rsx! {
            Donut {
                slices: vec![Slice::new("a", "A", 1.0), Slice::new("b", "B", 3.0)],
                title: "Picked",
                selected,
            }
        }
    }
    let html = render(Selected);
    assert!(
        html.contains(r#"aria-activedescendant="g3c-1-seg-1""#),
        "{html}"
    );
    assert_eq!(count(&html, "data-selected=\"true\""), 1);
    // The hole names the selected slice.
    assert!(html.contains("75.0%"));
}

#[test]
fn sunburst_compares_target_and_actual() {
    let html = render(|| {
        let mut actual = allocation();
        actual.children[2] = Node::leaf("bnd", "Bonds", 25.0);
        rsx! {
            Sunburst {
                root: allocation(),
                actual: Some(actual),
                title: "Target vs actual",
                format: ValueFormat::Number { decimals: 0 },
            }
        }
    });
    assert_eq!(count(&html, r#"class="g3-chart-seg""#), 6);
    assert!(count(&html, r#"class="g3-chart-band""#) >= 5);
    assert!(
        html.contains("US stocks › VTI: target 45.0%, actual 40.9%"),
        "{html}"
    );
    assert!(html.contains("<th scope=\"col\">Drift</th>"));
    assert!(html.contains("+7.7%"));
    assert!(html.contains("g3-chart-key"));
}

#[test]
fn sunburst_without_actual_has_no_bands() {
    let html = render(|| rsx! { Sunburst { root: allocation(), title: "Target", depth: 1 } });
    assert_eq!(count(&html, r#"class="g3-chart-seg""#), 3);
    assert_eq!(count(&html, "g3-chart-band"), 0);
    assert!(!html.contains("Actual share"));
}

#[test]
fn target_bars_are_text_or_buttons() {
    fn rows() -> Vec<TargetRow> {
        vec![
            TargetRow::new("vti", "VTI", 0.40, 0.42),
            TargetRow::new("bnd", "BND", 0.30, 0.27),
        ]
    }
    let plain = render(|| {
        rsx! {
            TargetBars { rows: rows(), title: "Drift", variant: TargetBarVariant::Drift, format: ValueFormat::Percent { decimals: 1 } }
        }
    });
    assert!(plain.contains(r#"aria-hidden="true""#));
    assert!(plain.contains("+2.0%") && plain.contains("-3.0%"));
    assert!(plain.contains(r#"data-sign="negative""#));
    assert!(!plain.contains("<button"));
    assert!(plain.contains("<caption>Drift</caption>"));

    let interactive = render(|| {
        rsx! {
            TargetBars { rows: rows(), title: "Weights", on_select: move |_| {}, format: ValueFormat::Percent { decimals: 1 } }
        }
    });
    assert_eq!(count(&interactive, "<button"), 2);
    assert!(interactive.contains("target 40.0%"));
    assert!(!interactive.contains(r#"class="g3-chart-bar-list" aria-hidden"#));
}

#[test]
fn line_chart_draws_one_path_per_series_and_a_slider() {
    let html = render(|| {
        let start = days_from_civil(2021, 1, 4) * DAY;
        let points = |scale: f64| {
            (0..3_000)
                .map(|i| Point::new(start + i * DAY, scale * (1_000.0 + i as f64)))
                .collect::<Vec<_>>()
        };
        rsx! {
            LineChart {
                series: vec![
                    Series::new("total", "Portfolio", points(1.0)).with_area(),
                    Series::new("cost", "Contributions", points(0.8)),
                ],
                title: "Portfolio value",
                y_format: ValueFormat::Currency { symbol: "$", decimals: 0 },
            }
        }
    });
    assert_eq!(count(&html, r#"class="g3-chart-line""#), 2);
    assert_eq!(count(&html, r#"class="g3-chart-area""#), 1);
    assert!(html.contains(r#"role="slider""#));
    assert!(html.contains(r#"aria-valuemax="2999""#));
    assert!(html.contains(r#"preserveAspectRatio="none""#), "{html}");
    assert!(html.contains("data-sparse=\"true\""));
    // Monthly summary rows, not 3,000.
    let rows = count(&html, "<tr>") - 1;
    assert!((90..=110).contains(&rows), "{rows} rows");
    // Nothing is hovered on the server.
    assert!(!html.contains("g3-chart-crosshair"));
    assert!(html.contains("g3-chart-legend"));
}

#[test]
fn sparkline_is_a_named_image() {
    let html = render(|| {
        rsx! {
            Sparkline { values: vec![10.0, 12.0, 9.0, 14.0], label: "VTI, 30 days", area: true }
        }
    });
    assert!(html.contains(r#"role="img""#));
    assert!(
        html.contains(r#"aria-label="VTI, 30 days: from 10 to 14, low 9, high 14""#),
        "{html}"
    );
    assert_eq!(count(&html, "<path"), 2);
}

#[test]
fn empty_charts_render_safely() {
    render(|| {
        rsx! {
            Donut { slices: vec![], title: "Empty" }
            Sunburst { root: Node::leaf("r", "R", 0.0), title: "Empty" }
            TargetBars { rows: vec![], title: "Empty" }
            LineChart { series: vec![], title: "Empty" }
            Sparkline { values: vec![], label: "Empty" }
        }
    });
}
