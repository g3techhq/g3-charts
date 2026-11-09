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
- `Sparkline`.
- `geom`: linear, band and time scales, nice ticks, arc paths and dashes,
  LTTB, nearest-point search, path building, and value formatting.
- `ChartStyles`, the stylesheet link for desktop and mobile shells.
- Series tokens `--g3-color-chart-1` … `-8` (and `-positive`/`-negative` for
  drift), with a colour-blind-safe default palette for light and dark.
