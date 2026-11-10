//! The data the charts draw.

/// A series or segment color. Only theme tokens: a chart can never be handed
/// a color the theme does not define.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChartColor {
    /// Categorical slot `1..=8`, `--g3-color-chart-N` (with a validated
    /// default when the theme does not set it). Slots past 8 wrap,
    /// but a ninth series should fold into "Other" instead.
    Series(u8),
    /// `--g3-color-accent`.
    Accent,
    /// A recessive grey, mixed from the text and card colors. For "Other",
    /// cash, or anything that should not compete.
    Neutral,
}

impl ChartColor {
    /// The CSS color value. It resolves anywhere on a page that loads the
    /// stylesheet, not only inside a chart, so an app's own legend can use it.
    ///
    /// ```
    /// use g3_charts::ChartColor;
    ///
    /// assert!(ChartColor::Series(2).css().starts_with("var(--g3-color-chart-2,"));
    /// ```
    pub fn css(self) -> &'static str {
        match self {
            ChartColor::Series(n) => SERIES[(usize::from(n.max(1)) - 1) % SERIES.len()],
            ChartColor::Accent => "var(--g3-color-accent)",
            ChartColor::Neutral => {
                "color-mix(in oklab, var(--g3-color-text-tertiary) 55%, var(--g3-color-card))"
            }
        }
    }

    /// The default color for the item at `index` (from zero) among its
    /// siblings: slot `index + 1`.
    pub fn nth(index: usize) -> ChartColor {
        ChartColor::Series((index % SERIES.len()) as u8 + 1)
    }
}

/// A token set anywhere above the chart (by the theme or the app) wins;
/// otherwise the stylesheet's validated default for the light or dark scheme.
const SERIES: [&str; 8] = [
    "var(--g3-color-chart-1, var(--g3-chart-default-1))",
    "var(--g3-color-chart-2, var(--g3-chart-default-2))",
    "var(--g3-color-chart-3, var(--g3-chart-default-3))",
    "var(--g3-color-chart-4, var(--g3-chart-default-4))",
    "var(--g3-color-chart-5, var(--g3-chart-default-5))",
    "var(--g3-color-chart-6, var(--g3-chart-default-6))",
    "var(--g3-color-chart-7, var(--g3-chart-default-7))",
    "var(--g3-color-chart-8, var(--g3-chart-default-8))",
];

/// One donut slice.
#[derive(Clone, Debug, PartialEq)]
pub struct Slice {
    /// Stable identity: keys the markup and comes back in
    /// [`SliceSelect`](crate::SliceSelect).
    pub id: String,
    /// What the slice is called, in the legend, tooltip and table.
    pub label: String,
    /// Its value. Negative and non-finite values draw nothing.
    pub value: f64,
    /// Defaults to the slot for its position.
    pub color: Option<ChartColor>,
}

impl Slice {
    /// A slice with the default color.
    pub fn new(id: impl Into<String>, label: impl Into<String>, value: f64) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            value,
            color: None,
        }
    }

    /// This slice with a color.
    pub fn with_color(mut self, color: ChartColor) -> Self {
        self.color = Some(color);
        self
    }
}

/// A node of a nested allocation, for [`Sunburst`](crate::Sunburst).
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    /// Stable identity among its siblings. A sunburst's `actual` tree is
    /// matched to its target tree by these.
    pub id: String,
    /// What the node is called.
    pub label: String,
    /// Its own value. A node with children is drawn at the sum of theirs,
    /// and this is used only when it has none.
    pub value: f64,
    /// Defaults to the slot for its position on the first ring; deeper
    /// rings take their parent's hue.
    pub color: Option<ChartColor>,
    /// Its parts, drawn on the next ring out.
    pub children: Vec<Node>,
}

impl Node {
    /// A leaf.
    pub fn leaf(id: impl Into<String>, label: impl Into<String>, value: f64) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            value,
            color: None,
            children: Vec::new(),
        }
    }

    /// A node made of `children`.
    pub fn group(id: impl Into<String>, label: impl Into<String>, children: Vec<Node>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            value: 0.0,
            color: None,
            children,
        }
    }

    /// This node with a color.
    pub fn with_color(mut self, color: ChartColor) -> Self {
        self.color = Some(color);
        self
    }

    /// The value it is drawn at: its own for a leaf, the sum of its
    /// children's otherwise. Negative and non-finite values count as zero.
    pub fn total(&self) -> f64 {
        if self.children.is_empty() {
            if self.value.is_finite() {
                self.value.max(0.0)
            } else {
                0.0
            }
        } else {
            self.children.iter().map(Node::total).sum()
        }
    }

    /// The child with this id.
    pub fn child(&self, id: &str) -> Option<&Node> {
        self.children.iter().find(|c| c.id == id)
    }
}

/// One row of [`TargetBars`](crate::TargetBars).
#[derive(Clone, Debug, PartialEq)]
pub struct TargetRow {
    /// Stable identity.
    pub id: String,
    /// What the row is called.
    pub label: String,
    /// The goal, such as a target weight of `0.40`.
    pub target: f64,
    /// Where it is now, such as `0.42`.
    pub actual: f64,
    /// The actual bar's color. Defaults to the slot for its position.
    pub color: Option<ChartColor>,
}

impl TargetRow {
    /// A row with the default color.
    pub fn new(id: impl Into<String>, label: impl Into<String>, target: f64, actual: f64) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            target,
            actual,
            color: None,
        }
    }
}

/// One sample of a time series.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    /// When: Unix milliseconds, UTC.
    pub t: i64,
    /// The value. A non-finite value leaves a gap in the line.
    pub v: f64,
}

impl Point {
    /// A point.
    pub fn new(t: i64, v: f64) -> Self {
        Self { t, v }
    }
}

/// A line on a [`LineChart`](crate::LineChart).
#[derive(Clone, Debug, PartialEq)]
pub struct Series {
    /// Stable identity.
    pub id: String,
    /// What the series is called, in the legend and tooltip.
    pub label: String,
    /// The samples, sorted by time. Unsorted input is sorted (at a cost).
    pub points: Vec<Point>,
    /// Defaults to the slot for its position.
    pub color: Option<ChartColor>,
    /// Fill the area between the line and the bottom of the plot.
    pub area: bool,
}

impl Series {
    /// A line with the default color and no area.
    pub fn new(id: impl Into<String>, label: impl Into<String>, points: Vec<Point>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            points,
            color: None,
            area: false,
        }
    }

    /// This series with its area filled.
    pub fn with_area(mut self) -> Self {
        self.area = true;
        self
    }

    /// This series with a color.
    pub fn with_color(mut self, color: ChartColor) -> Self {
        self.color = Some(color);
        self
    }
}
