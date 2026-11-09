//! Made-up but realistic portfolio data, the same on every build so the
//! server and the hydrating client agree.

use g3_charts::geom::{DAY, days_from_civil};
use g3_charts::prelude::*;

/// A small deterministic generator: the gallery must not differ between the
/// server render and the client.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Roughly normal, mean 0, deviation 1.
    fn normal(&mut self) -> f64 {
        (0..6).map(|_| self.next()).sum::<f64>() - 3.0
    }
}

pub fn target() -> Node {
    Node::group(
        "root",
        "Portfolio",
        vec![
            Node::group(
                "us",
                "US stocks",
                vec![
                    Node::leaf("vti", "VTI · Total market", 0.40),
                    Node::leaf("vb", "VB · Small cap", 0.10),
                ],
            ),
            Node::group(
                "intl",
                "International",
                vec![
                    Node::leaf("vea", "VEA · Developed", 0.20),
                    Node::leaf("vwo", "VWO · Emerging", 0.05),
                ],
            ),
            Node::group(
                "bonds",
                "Bonds",
                vec![
                    Node::leaf("bnd", "BND · Total bond", 0.15),
                    Node::leaf("vtip", "VTIP · TIPS", 0.05),
                ],
            ),
            Node::leaf("cash", "Cash", 0.05).with_color(ChartColor::Neutral),
        ],
    )
}

pub fn actual() -> Node {
    Node::group(
        "root",
        "Portfolio",
        vec![
            Node::group(
                "us",
                "US stocks",
                vec![
                    Node::leaf("vti", "VTI · Total market", 46_200.0),
                    Node::leaf("vb", "VB · Small cap", 9_400.0),
                ],
            ),
            Node::group(
                "intl",
                "International",
                vec![
                    Node::leaf("vea", "VEA · Developed", 17_900.0),
                    Node::leaf("vwo", "VWO · Emerging", 4_100.0),
                ],
            ),
            Node::group(
                "bonds",
                "Bonds",
                vec![
                    Node::leaf("bnd", "BND · Total bond", 14_300.0),
                    Node::leaf("vtip", "VTIP · TIPS", 4_800.0),
                ],
            ),
            Node::leaf("cash", "Cash", 3_300.0),
        ],
    )
}

/// One row per fund: target weight against actual weight.
pub fn weights() -> Vec<TargetRow> {
    let target = target();
    let actual = actual();
    let total = actual.total();
    target
        .children
        .iter()
        .flat_map(|group| {
            let leaves: Vec<&Node> = if group.children.is_empty() {
                vec![group]
            } else {
                group.children.iter().collect()
            };
            let actual_group = actual.child(&group.id);
            leaves.into_iter().map(move |leaf| {
                let held = if leaf.id == group.id {
                    actual_group.map_or(0.0, Node::total)
                } else {
                    actual_group
                        .and_then(|g| g.child(&leaf.id))
                        .map_or(0.0, Node::total)
                };
                TargetRow::new(
                    leaf.id.clone(),
                    leaf.label.clone(),
                    leaf.value,
                    held / total,
                )
            })
        })
        .collect()
}

/// Asset classes, for the donut.
pub fn classes() -> Vec<Slice> {
    actual()
        .children
        .iter()
        .map(|group| {
            let slice = Slice::new(group.id.clone(), group.label.clone(), group.total());
            if group.id == "cash" {
                slice.with_color(ChartColor::Neutral)
            } else {
                slice
            }
        })
        .collect()
}

/// `days` of daily values from 2021-01-04: the portfolio and what was paid in.
pub fn history(days: usize) -> Vec<Series> {
    let start = days_from_civil(2021, 1, 4) * DAY;
    let mut random = Lcg(7);
    let mut value = 40_000.0;
    let mut paid = 40_000.0;
    let mut portfolio = Vec::with_capacity(days);
    let mut contributions = Vec::with_capacity(days);
    for day in 0..days {
        let t = start + day as i64 * DAY;
        // A deposit on the first of each month.
        if day % 30 == 0 && day > 0 {
            value += 1_000.0;
            paid += 1_000.0;
        }
        value *= 1.0 + 0.0003 + 0.009 * random.normal();
        portfolio.push(Point::new(t, value));
        contributions.push(Point::new(t, paid));
    }
    vec![
        Series::new("value", "Portfolio", portfolio).with_area(),
        Series::new("paid", "Contributions", contributions).with_color(ChartColor::Neutral),
    ]
}

/// One noisy series of `n` points a minute apart, for the stress test.
pub fn stress(n: usize) -> Vec<Series> {
    let start = days_from_civil(2026, 11, 2) * DAY + 14 * 3_600_000;
    let mut random = Lcg(42);
    let mut value = 100.0;
    let points = (0..n)
        .map(|i| {
            value += random.normal() * 0.4;
            Point::new(start + i as i64 * 60_000, value)
        })
        .collect();
    vec![Series::new("ticks", "Price", points)]
}

/// The last 30 closes of a fund.
pub fn closes(seed: u64, drift: f64) -> Vec<f64> {
    let mut random = Lcg(seed);
    let mut value = 100.0;
    (0..30)
        .map(|_| {
            value *= 1.0 + drift + 0.012 * random.normal();
            value
        })
        .collect()
}
