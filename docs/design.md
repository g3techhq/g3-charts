# g3-charts design (v1, approved 2026-11-08)

> **Changes during the build.** Where this doc and the code differ, the code
> wins. Each change, and why:
>
> - **Keyboard and focus.** Moving focus from Rust needs either JS or an
>   `onmounted` call per segment, and both are costly on mobile. So:
>   - Ring charts are one tab stop using `aria-activedescendant`. Segments
>     are `role="img"` and still reachable by screen-reader swiping.
>   - The line chart's plot is a `role="slider"` with `aria-valuetext`,
>     which screen readers announce natively, so it needs no live region.
> - **Pointer moves (§6).** The Rust `pointermove` listener is attached only
>   while a finger is down or a mouse is over the plot (`chart/pointer.rs`),
>   so the rest of the app never waits on it. There is no time-based
>   throttle, because the wasm and native clocks would need a dependency.
>   Updates are de-duplicated by index instead.
> - **Hover on ring charts.** It uses Rust `pointerenter`/`pointerleave`
>   (direct listeners, not delegated, and only on enter and leave), plus CSS
>   `:has()` to fade the other marks. Pure CSS would need one rule per slice
>   index.
> - **`TargetBars` rows are HTML, not SVG.** Rounded bar ends would distort
>   in a stretched SVG, and the visible text is then readable as it stands.
> - **Centre content.** It is a `center: Option<Element>` prop, not
>   `children`.
> - **Default palette.** Fallbacks are `--g3-chart-default-N` custom
>   properties, so `--g3-color-chart-N` set anywhere above the chart wins.
> - **Gallery port.** The gallery serves on port 8095, because Docker often
>   holds 8090.

Dioxus chart components drawn as SVG from Rust, with no JavaScript, for the
g3 stack. The first consumer is the investing app: a nested allocation pie
(target vs actual), drift bars and a portfolio value line.

## 1. Where it lives

**Its own repository, `g3techhq/g3-charts`**, at `Projects/g3-charts`, laid
out like g3-route-transitions and g3-cache: the same `justfile`,
`lefthook.yml`, `deny.toml`, `_typos.toml`, `rustfmt.toml`,
`rust-toolchain.toml` (1.95.0, wasm32), `AGENTS.md`/`CLAUDE.md`, and
`CHANGELOG.md`.

Why not inside g3-ui:

- Every g3 library is its own repo. g3-ui is one crate, not a workspace (its
  playground is a separate root).
- Apps that never chart don't build it.
- The dependency runs one way (charts → g3-ui), and the two release on
  their own schedules.
- g3-ui has uncommitted work in progress right now.

Pinning works the same way as in g3-ui:

- `dioxus` and `manganis` at `=0.8.0-alpha.3`.
- `g3-ui = "0.7"`.
- A `[patch.crates-io]` block naming the Gateway3B fork, plus
  `g3-ui = { git = "https://github.com/g3techhq/g3-ui", branch = "main" }`.
  The gallery crate carries the same block.

Local builds pick up the sibling checkouts through `Projects/.cargo/config.toml`,
which needs one new line, `g3-charts = { path = ".../g3-charts" }`, once an app
depends on it. Nothing is published while the fork is in use.

## 2. Dependencies (approval needed)

| Crate | Where | Why |
| --- | --- | --- |
| `dioxus =0.8.0-alpha.3` | lib | components (default features off where possible: macro, html, hooks, signals, document) |
| `manganis =0.8.0-alpha.3` | lib | the stylesheet asset, as g3-ui does |
| `g3-ui 0.7` | lib | `ComponentMode`/`use_component_mode`, `merge_classes`, `Theme` tokens |
| `dioxus-ssr`, `dioxus-html =0.8.0-alpha.3` | dev | markup and ARIA tests, as in g3-ui |
| `dioxus` (`fullstack`, `router`), `g3-ui`, `g3-charts` | `gallery/` only | a server-rendered, hydrating demo app |

There is no date crate: time ticks use exact UTC calendar maths over `i64`
Unix milliseconds (Howard Hinnant's `civil_from_days`, about 30 lines). There
are no number-format or geometry crates either. `js-sys` (already in g3-ui's
tree) is added only if the pointer throttle needs a clock on wasm (see §6).

## 3. Module layout

```
src/lib.rs            exports, crate docs, CHARTS_CSS asset
src/prelude.rs        components + data types
src/geom/             pure maths, no Dioxus, unit tested
  scale.rs            LinearScale (map / invert / clamp), BandScale (bar rows)
  ticks.rs            nice_step (1-2-5 × 10^k), nice_domain, linear_ticks
  time.rs             TimeScale, UTC civil dates, time ticks (sec→year) + labels
  arc.rs              polar points, annular sector path, centroid, angle_at,
                      full-circle and tiny-slice edge cases
  lttb.rs             largest-triangle-three-buckets → indices into the input
  nearest.rs          nearest index by binary search (partition_point)
  path.rs             compact, NaN-safe SVG `d` builder (fixed precision)
  format.rs           ValueFormat: Number, Percent, Currency, Compact, Custom(fn)
src/model.rs          Slice, Node, TargetRow, Series, Point, ChartColor
src/chart/
  frame.rs            ChartFrame: <figure>, ids, title/desc, stylesheet link,
                      container, hidden data table
  a11y.rs             DataTable, roving tabindex, visually-hidden helpers
  pointer.rs          throttled pointer → index, Crosshair/Tooltip components
  donut.rs  sunburst.rs  bars.rs  line.rs  sparkline.rs
src/tests/            SSR markup + ARIA tests (dioxus-ssr), stylesheet-class test
assets/g3-charts.css  one stylesheet, inside `@layer g3`
gallery/              dx fullstack app (own root), every chart, light/dark, md/ios
```

`geom` is public, so apps can reuse the scales and ticks.

## 4. Component API

The conventions are g3-ui's:

- Every prop is typed and documented. Optional props are `Option<T>`, with
  the default stated.
- Every component takes `class` and `mode`.
- Each component's own event is `on_` + verb, and carries a named struct.

Data props are `ReadSignal<T>`, so a parent can pass a memo of a
thousands-point series without cloning it on every render. A plain value
still works. This is the one departure from g3-ui's plain-prop rule, made
because chart data is large.

```rust
// model.rs
pub struct Slice  { pub id: String, pub label: String, pub value: f64, pub color: Option<ChartColor> }
pub struct Node   { pub id: String, pub label: String, pub value: f64, pub color: Option<ChartColor>,
                    pub children: Vec<Node> }
pub struct TargetRow { pub id: String, pub label: String, pub target: f64, pub actual: f64,
                       pub color: Option<ChartColor> }
pub struct Point  { pub t: i64 /* Unix ms, UTC */, pub v: f64 }
pub struct Series { pub id: String, pub label: String, pub points: Vec<Point>,
                    pub color: Option<ChartColor>, pub area: bool }
/// Only tokens: no colour string can reach the markup.
pub enum ChartColor { Series(u8 /* 1..=8 */), Accent, Neutral }
```

| Component | Main props | Notes |
| --- | --- | --- |
| `Donut` | `slices`, `title`, `description`, `hole` (default 0.62; 0 = pie), `format`, `center_label`, `children` (replaces the default centre: total + label), `selected: Option<Signal<Option<String>>>`, `on_select: SliceSelect` | Clockwise from 12 o'clock, in data order. Zero slices are skipped but stay in the table. |
| `Sunburst` | `root: ReadSignal<Node>`, `actual: Option<ReadSignal<Node>>`, `depth` (default 2, max 3), `title`, `description`, `format`, `children`, `selected`, `on_select: NodeSelect { path }` | Rings are depth levels. With `actual`, nodes are matched by `id`, and a thin outer band on each segment shows the actual share against the target arc. To drill down, the app passes a subtree; the component holds no drill state. |
| `TargetBars` | `rows`, `variant: TargetBarVariant { Bullet (default), Grouped, Drift }`, `domain`, `format`, `title`, `description`, `on_select` | Horizontal rows: an HTML grid (label and value wrap like text) with an SVG track per row. Bullet: a target tick over the actual bar. Grouped: two bars. Drift: diverging from zero, actual − target. |
| `LineChart` | `series`, `title`, `description`, `y_format`, `y_domain: YDomain { Auto, IncludeZero, Fixed(a, b) }`, `utc_offset_minutes`, `max_points` (default 600), `height` | Time x axis, one `<path>` per series (plus one area path if `area`). Crosshair and tooltip. |
| `Sparkline` | `values: ReadSignal<Vec<f64>>`, `label` (accessible name), `area`, `color`, `class` | `role="img"`, no axes, no interaction. Its `aria-label` adds first / last / min / max. |

## 5. Rendering and SSR

- **One tree on every build.** Calls are gated, never markup, so the server
  HTML hydrates as-is. Ids come from an SSR-stable counter (a copy of g3-ui's
  `use_element_id` pattern, `g3c-` prefix; g3-ui's is `pub(crate)`).
- **Sizes are unknown at SSR**, so cartesian charts don't measure first.
  - The plot is an SVG with a fixed viewBox (0..1000 × 0..1000) and
    `preserveAspectRatio="none"`. Strokes use
    `vector-effect: non-scaling-stroke`, so lines stay 2px at any size.
  - Text (axis labels, tooltip) is HTML positioned in `%` over the SVG, so
    it is never stretched.
  - Point markers and the crosshair dot are also HTML, so they stay round.
  - Donut and sunburst use a square viewBox with `meet`.
- **Responsive through container queries.** Each chart is its own container
  (`container: g3-chart / inline-size`), so a chart in a narrow card adapts
  whatever the viewport. Ticks are emitted at two densities (about 3 and
  about 7), and `@container g3-chart (width < 30rem)` hides the dense set.
  Other rules: aspect ratio, legend below vs beside, and hiding direct
  labels.
- **Downsampling:** LTTB runs in a `use_memo` keyed on the data, not the
  width, so the server and client draw the same path. 600 points is enough
  for a 1440px plot. Tooltips read the full data, not the downsampled path.
- **Arc segments:**
  - Donut and sunburst segments are stroked circles positioned with
    `stroke-dasharray`/`stroke-dashoffset`. Those animate in every WebView,
    including iOS, where a CSS `d` transition does not.
  - `geom::arc` still provides the sector paths, centroids for labels, and
    the angle for hit testing.
  - A 2px surface gap separates segments.

## 6. Interaction (mobile-first)

- **Categorical charts (donut, sunburst, bars): almost no Rust round-trips.**
  - Hover and focus are pure CSS. `:hover`, `:focus-visible` and
    `[aria-selected=true]` on a segment reveal that segment's centre or
    tooltip node through `:has()`.
  - A tap calls Rust once (`onclick` → `selected`), because iOS doesn't
    focus or hover SVG on tap.
- **Line chart crosshair:**
  - A transparent HTML overlay takes the pointer events, so offsets are
    always relative to the overlay. Its width comes from `onresize` and is
    stored in a signal that only the handler `peek`s.
  - Touch uses `touch-action: pan-y`: dragging sideways scrubs and vertical
    scrolling still works. A mouse gets hover.
  - The handler throttles to about one update per frame (16ms) and finds
    the nearest x with `partition_point`. It writes
    `hover: Signal<Option<usize>>` only when the index changes.
  - Only a small `Crosshair` child component reads `hover`, so a move
    re-renders a line, a few dots and the tooltip text, never the paths or
    axes.
  - Keyboard: the plot is focusable. Arrows step through points, and
    Home/End jump to the ends. A polite live region announces keyboard
    steps only.
- **Limitation:** with no JS, every `pointermove` still crosses into Rust. On
  Android's WebView that crossing is the costly part, and the steps above
  only reduce the work done after it. If a real device stutters, the fix is
  an optional, roughly 30-line script that sends one move per animation
  frame (as g3-ui's gesture scripts do). It is not in v1 unless you want it.

## 7. Colour and theme

- Components write only `var(--g3-color-*)` or `color-mix()` over them.
  `ChartColor` makes any other colour impossible to pass.
- **Series palette:** `--g3-color-chart-1` … `-8`, plus
  `--g3-color-chart-positive` / `-negative` for drift.
  - The hues are assigned in a fixed order and follow the entity, never its
    rank.
  - Status colours (`success`/`danger`) are never used as series colours.
  - g3-ui's `Theme` has no chart tokens yet. For v1 the stylesheet reads
    `var(--g3-color-chart-N, light-dark(<light>, <dark>))`, where
    `light-dark()` follows the `color-scheme` the g3 theme sets. Fallback
    literals appear in that one place only, and an app can override any
    slot.
  - The default steps start from a validated 8-hue categorical set and are
    checked with a colour-vision-deficiency validator against g3-ui's light
    and dark card colours before shipping.
  - Later (g3-ui 0.8): move the palette into `Theme` and delete the
    fallbacks.
- **Sunburst children** take their parent's hue, stepped with
  `color-mix(in oklab, <parent> N%, var(--g3-color-card))`.
- Text uses only the text tokens, never a series colour. Grid and axes use
  `--g3-color-border` and `text-tertiary`.
- **Mode:**
  - `ios`: rounded bar ends, hairline grid, an SF-style tooltip.
  - `md`: square ends, an elevated tooltip.
  - Both read `data-g3-mode`, as g3-ui does.

## 8. Accessibility

- Every chart is a `<figure>`:
  - The visible title is optional. The SVG is labelled with
    `aria-labelledby` → `<title>` + `<desc>`.
  - The fork leaves out SVG `<title>` (its name clashes with HTML
    `title`). v1 emits it through a `g` whose `dangerous_inner_html` holds
    the escaped title; `desc` exists. Alternatively, we add an SVG `title`
    to the fork (see §11).
- **`role`:**
  - Charts with no focusable parts (sparkline) use `role="img"`.
  - Charts with focusable segments use `role="group"` on the SVG, with
    each segment `role="img"`, an `aria-label` ("VTI: 42.0%, target
    40.0%"), and a roving `tabindex`. A whole-chart `role="img"` would hide
    the segments from screen readers.
- A **visually hidden `<table>`** holds every value, including zero slices
  and the full un-downsampled series. Long series are summarised by period
  (monthly for more than 400 points).
- At least 2 series get a legend, and at most 4 also get direct labels,
  so identity never rests on colour alone.

## 9. Animation

CSS transitions only:

- Donut and sunburst animate `stroke-dasharray`/`stroke-dashoffset`.
- Bars animate `transform: scaleX`.
- A line draws in once through `pathLength="1"` and a dash offset.
- The crosshair animates `transform` over 80ms.

All of this sits under `@media (prefers-reduced-motion: no-preference)`, so
reduced motion gets none of it. Nothing animates on the server-rendered
first paint unless it was already planned to.

## 10. Testing, commands, gallery

- **Unit tests** (`geom`):
  - Scale map/invert round trips.
  - `nice_step` and tick counts over 1e-6..1e12 and negative domains.
  - Time ticks across month, year and leap-year boundaries.
  - Arc paths for a full circle, a single slice, 0 and 359.99°, and no
    `NaN`.
  - LTTB keeps the first and last points, the count is respected, and it is
    the identity below the threshold.
  - Nearest-index at the edges and on ties.
- **Markup tests** (dioxus-ssr): roles, labels, the table, ids, no `NaN`,
  and every `g3-` class defined in the stylesheet.
- **`justfile`**, matching the g3 libraries: `check` (host, wasm32,
  aarch64-linux-android, plus the gallery's web/server/mobile features; iOS
  skipped off macOS), `test` (nextest + doc tests), `lint` / `lint-strict`,
  `format` / `format-check`, `spell`, `security`, `pre-push`, `ci`, `gallery`
  (`dx serve`).
- **Gallery:** one page per chart with realistic portfolio data:
  - A three-level allocation, target vs actual.
  - Five years of daily values, plus a 10k-point stress series.
  - Light/dark and md/ios toggles.

  Checked in the browser at 390×844 and 1440×900 in light and dark, with
  the keyboard, and with the console checked for hydration errors.

## 11. Open questions (my recommendation first)

1. **Placement:** its own repo `g3techhq/g3-charts`, or a crate in g3-ui?
   → own repo.
2. **Palette:** token names `--g3-color-chart-*` with fallbacks in
   g3-charts.css now, moving into g3-ui `Theme` later? Or change g3-ui now?
   → fallbacks now.
3. **Dependencies** in §2: OK? (No new crates beyond the Dioxus/g3 set,
   `js-sys` on wasm only if needed.)
4. **Pointer moves** cross into Rust (no JS) with throttling and index
   de-duplication. Accept for v1, and add the optional frame-coalescing
   script only if a real Android device stutters? → yes.
5. **SVG `<title>`:** inner-HTML workaround in v1, or add `title` to the
   Dioxus fork? → workaround now, fork fix later.
6. **Drift variant** in `TargetBars`: you didn't ask for it, but the
   investing app needs it. Include? → include, it is small.
7. **Shared config:** add the `g3-charts` line to `Projects/.cargo/config.toml`
   (outside any repo) when the first app depends on it? → yes, at that point.
