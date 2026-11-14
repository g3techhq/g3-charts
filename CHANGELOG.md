# Changelog

All notable changes to g3-charts are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- `Donut`, with a centre label, a pie mode (`hole: Some(0.0)`), a legend, and
  slice selection.
- `Sunburst` for nested allocations, with an optional `actual` tree drawn as
  thin bands beside each target ring.
- `TargetBars` in bullet, grouped and drift variants.
- `LineChart` with a calendar-aligned time axis, area fills, LTTB
  downsampling, and a crosshair tooltip driven by pointer, touch or keyboard.
  In a card narrower than about 16rem the plot keeps its minimum height and
  fits the width, rather than overflowing the card. Below 30rem the tooltip
  slides along the top of the plot instead of sitting beside the line, so it
  stays inside the chart.
- `Sparkline`.
- `geom`: linear, band and time scales, nice ticks, arc paths and dashes,
  LTTB, nearest-point search, path building, and value formatting.
- `ChartColor::css()` resolves anywhere on the page, not only inside a chart:
  the default palette is declared on `:root`, so an app's own legend swatches
  match the chart.
- `ChartStyles`, the stylesheet link for desktop and mobile shells.
- Series tokens `--g3-color-chart-1` … `-8` (and `-positive`/`-negative` for
  drift), with a colour-blind-safe default palette for light and dark.

### Fixed

- Clicking a `Donut`, `Sunburst` or `LineChart` no longer draws the browser's
  focus outline around it; the accent ring still shows for keyboard focus.
