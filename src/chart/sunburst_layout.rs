//! Where each node of a sunburst goes: pure geometry, no Dioxus.

use crate::geom::{Dash, angles, dash};
use crate::model::{ChartColor, Node};

/// One drawn ring segment.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Arc {
    /// Position in depth-first order, which keyboard movement follows.
    pub index: usize,
    /// Ids from the first ring down to this node.
    pub path: Vec<String>,
    /// `path` joined, unique within the chart.
    pub key: String,
    /// `1` for the first ring.
    pub depth: usize,
    pub label: String,
    /// The node's own breadcrumb: `US stocks › VTI`.
    pub crumb: String,
    pub value: f64,
    /// Share of the whole target.
    pub share: f64,
    /// Share of the whole actual tree, when one was given.
    pub actual_share: Option<f64>,
    pub color: String,
    pub radius: f64,
    pub width: f64,
    pub dash: Dash,
    /// The dash on the thin actual band, when there is one.
    pub actual: Option<(f64, Dash)>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Layout {
    pub total: f64,
    pub actual_total: Option<f64>,
    pub arcs: Vec<Arc>,
}

/// Ring sizes: the hole, then per level a target ring, and with `actual` a
/// thin band outside it.
pub(crate) const OUTER: f64 = 94.0;
const RING_GAP: f64 = 1.5;
const BAND: f64 = 4.5;
const GAP: f64 = 1.2;

pub(crate) fn layout(root: &Node, actual: Option<&Node>, depth: usize, hole: f64) -> Layout {
    let depth = depth.clamp(1, 3);
    let total = root.total();
    let actual_total = actual.map(Node::total);
    let inner = OUTER * hole.clamp(0.1, 0.8);
    let level = (OUTER - inner) / depth as f64;
    let band = if actual.is_some() {
        BAND + RING_GAP
    } else {
        0.0
    };
    let width = (level - RING_GAP - band).max(2.0);
    let mut arcs = Vec::new();
    let mut walk = Walk {
        total,
        actual_total,
        depth,
        inner,
        level,
        width,
        arcs: &mut arcs,
    };
    walk.children(
        root,
        actual,
        1,
        (0.0, 360.0),
        actual.map(|_| (0.0, 360.0)),
        &[],
        None,
        "",
    );
    Layout {
        total,
        actual_total,
        arcs,
    }
}

struct Walk<'a> {
    total: f64,
    actual_total: Option<f64>,
    depth: usize,
    inner: f64,
    level: f64,
    width: f64,
    arcs: &'a mut Vec<Arc>,
}

impl Walk<'_> {
    #[allow(
        clippy::too_many_arguments,
        reason = "one recursive step's whole context; a struct per call adds nothing"
    )]
    fn children(
        &mut self,
        node: &Node,
        actual: Option<&Node>,
        depth: usize,
        span: (f64, f64),
        actual_span: Option<(f64, f64)>,
        path: &[String],
        parent_color: Option<&str>,
        parent_crumb: &str,
    ) {
        if depth > self.depth || node.children.is_empty() {
            return;
        }
        let values: Vec<f64> = node.children.iter().map(Node::total).collect();
        let sweep = span.1 - span.0;
        let target_angles = angles(&values, span.0, sweep);
        let actual_angles = actual_span.map(|(start, end)| {
            let matched: Vec<f64> = node
                .children
                .iter()
                .map(|child| {
                    actual
                        .and_then(|a| a.child(&child.id))
                        .map_or(0.0, Node::total)
                })
                .collect();
            // Actual parts the target doesn't name share the leftover arc,
            // so the band still adds up to its parent's actual share.
            let parent = actual.map_or(0.0, Node::total);
            let named: f64 = matched.iter().sum();
            let mut with_rest = matched;
            with_rest.push((parent - named).max(0.0));
            let mut spans = angles(&with_rest, start, end - start);
            spans.pop();
            spans
        });
        let radius = self.inner + (depth - 1) as f64 * self.level + self.width / 2.0;
        let band_radius = radius + self.width / 2.0 + RING_GAP + BAND / 2.0;
        for (i, (child, (start, end))) in node.children.iter().zip(target_angles).enumerate() {
            let color = match (child.color, parent_color) {
                (Some(color), _) => color.css().to_string(),
                (None, Some(parent)) => {
                    let mix = 100usize.saturating_sub(14 * (i + 1)).max(40);
                    format!("color-mix(in oklab, {parent} {mix}%, var(--g3-color-card))")
                }
                (None, None) => ChartColor::nth(i).css().to_string(),
            };
            let mut child_path = path.to_vec();
            child_path.push(child.id.clone());
            let crumb = if parent_crumb.is_empty() {
                child.label.clone()
            } else {
                format!("{parent_crumb} › {}", child.label)
            };
            let child_actual = actual.and_then(|a| a.child(&child.id));
            let child_actual_span = actual_angles.as_ref().map(|spans| spans[i]);
            let actual_band =
                child_actual_span.map(|(s, e)| (band_radius, dash(band_radius, s, e, GAP)));
            let index = self.arcs.len();
            self.arcs.push(Arc {
                index,
                key: child_path.join("/"),
                path: child_path.clone(),
                depth,
                label: child.label.clone(),
                crumb: crumb.clone(),
                value: child.total(),
                share: if self.total > 0.0 {
                    (end - start) / 360.0
                } else {
                    0.0
                },
                actual_share: match (self.actual_total, child_actual_span) {
                    (Some(t), Some((s, e))) if t > 0.0 => Some((e - s) / 360.0),
                    (Some(_), _) => Some(0.0),
                    _ => None,
                },
                color: color.clone(),
                radius,
                width: self.width,
                dash: dash(radius, start, end, GAP),
                actual: actual_band,
            });
            self.children(
                child,
                child_actual,
                depth + 1,
                (start, end),
                child_actual_span,
                &child_path,
                Some(&color),
                &crumb,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target() -> Node {
        Node::group(
            "root",
            "All",
            vec![
                Node::group(
                    "us",
                    "US stocks",
                    vec![Node::leaf("vti", "VTI", 50.0), Node::leaf("vb", "VB", 10.0)],
                ),
                Node::group(
                    "intl",
                    "International",
                    vec![Node::leaf("vxus", "VXUS", 25.0)],
                ),
                Node::leaf("bnd", "Bonds", 15.0),
            ],
        )
    }

    #[test]
    fn shares_nest_inside_their_parents() {
        let layout = layout(&target(), None, 2, 0.34);
        assert_eq!(layout.total, 100.0);
        let keys: Vec<&str> = layout.arcs.iter().map(|a| a.key.as_str()).collect();
        assert_eq!(
            keys,
            vec!["us", "us/vti", "us/vb", "intl", "intl/vxus", "bnd"]
        );
        let share = |key: &str| layout.arcs.iter().find(|a| a.key == key).unwrap().share;
        assert!((share("us") - 0.6).abs() < 1e-9);
        assert!((share("us/vti") - 0.5).abs() < 1e-9);
        assert!((share("bnd") - 0.15).abs() < 1e-9);
        assert_eq!(layout.arcs[1].crumb, "US stocks › VTI");
        assert!(
            layout.arcs[1]
                .color
                .starts_with("color-mix(in oklab, var(--g3-color-chart-1,")
        );
        assert!(layout.arcs[1].radius > layout.arcs[0].radius);
        assert!(layout.arcs.iter().all(|a| a.actual.is_none()));
    }

    #[test]
    fn depth_limits_the_rings() {
        let layout = layout(&target(), None, 1, 0.34);
        assert_eq!(layout.arcs.len(), 3);
    }

    #[test]
    fn actual_bands_match_by_id_and_keep_the_rest() {
        let actual = Node::group(
            "root",
            "All",
            vec![
                Node::group(
                    "us",
                    "US stocks",
                    vec![
                        Node::leaf("vti", "VTI", 55.0),
                        Node::leaf("old", "Old fund", 5.0),
                    ],
                ),
                Node::leaf("bnd", "Bonds", 40.0),
            ],
        );
        let layout = layout(&target(), Some(&actual), 2, 0.34);
        let find = |key: &str| layout.arcs.iter().find(|a| a.key == key).unwrap();
        assert_eq!(layout.actual_total, Some(100.0));
        assert!((find("us").actual_share.unwrap() - 0.6).abs() < 1e-9);
        assert!((find("us/vti").actual_share.unwrap() - 0.55).abs() < 1e-9);
        assert_eq!(find("us/vb").actual_share, Some(0.0));
        assert_eq!(find("intl").actual_share, Some(0.0));
        assert!((find("bnd").actual_share.unwrap() - 0.4).abs() < 1e-9);
        let (band, _) = find("us").actual.clone().unwrap();
        assert!(band > find("us").radius + find("us").width / 2.0);
        assert!(band < find("us/vti").radius - find("us/vti").width / 2.0);
    }

    #[test]
    fn an_empty_tree_draws_nothing() {
        let layout = layout(&Node::leaf("root", "All", 0.0), None, 2, 0.34);
        assert!(layout.arcs.is_empty());
    }
}
