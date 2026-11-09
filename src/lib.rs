#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

use manganis::{Asset, AssetOptions, asset};

/// The chart stylesheet, inside `@layer g3`.
///
/// On the web it is linked into the document head at build time. Desktop and
/// mobile bundles only collect assets something links at runtime, so render
/// [`ChartStyles`] once in the app shell as well.
///
/// Not minified: `dx` minifies with lightningcss, which rewrites
/// `light-dark()` into a polyfill that only works when the same stylesheet
/// sets `color-scheme`. g3 themes set it inline, so the default palette's
/// dark steps would never apply.
pub static CHARTS_CSS: Asset = asset!(
    "/assets/g3-charts.css",
    AssetOptions::css()
        .with_static_head(true)
        .with_minify(false)
);

mod chart;
pub mod geom;
mod model;
pub mod prelude;

pub use chart::bars::{RowSelect, TargetBarVariant, TargetBars};
pub use chart::common::ChartStyles;
pub use chart::donut::{Donut, SliceSelect};
pub use chart::line::{LineChart, YDomain};
pub use chart::sparkline::Sparkline;
pub use chart::sunburst::{NodeSelect, Sunburst};
pub use geom::ValueFormat;
pub use model::{ChartColor, Node, Point, Series, Slice, TargetRow};

#[cfg(test)]
mod tests;
