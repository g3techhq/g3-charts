# g3-charts

Dioxus charts for the g3 stack, drawn as SVG from Rust with no JavaScript.
They render on the server, hydrate on the web, and run in the Android and iOS
web views.

| Component | For |
| --- | --- |
| `Donut` | parts of a whole, with a label in the hole (`hole: Some(0.0)` for a pie) |
| `Sunburst` | nested allocations, optionally against what is actually held |
| `TargetBars` | each row's actual against its target: bullet, grouped or drift |
| `LineChart` | values over time, with a crosshair; thousands of points are fine |
| `Sparkline` | a word-sized trend beside a number |

```rust
use dioxus::prelude::*;
use g3_charts::prelude::*;

#[component]
fn Holdings(slices: ReadSignal<Vec<Slice>>) -> Element {
    rsx! {
        Donut {
            slices,
            title: "Holdings by asset class",
            format: ValueFormat::Currency { symbol: "$", decimals: 0 },
            center_label: "Invested",
        }
    }
}
```

Render `ChartStyles {}` once beside g3-ui's `AppWrapper` in the app shell. On
the web the stylesheet is already in the page head, but desktop and mobile
bundles need the runtime link.

## What every chart does

- **Theme colors only.** Series use `--g3-color-chart-1` … `-8`, falling back
  to a palette checked for colour-blind safety in light and dark. Set those
  tokens on your theme or app to restyle the charts. Everything else is
  `--g3-color-*` and `color-mix()` over them, and `ChartColor` cannot carry
  any other color.
- **Same markup on server and client.** The cartesian plot is a stretched
  1000 × 1000 viewBox with non-scaling strokes. Labels are HTML positioned
  in percent, so nothing has to be measured before the first paint.
- **Container queries.** Each chart is a `g3-chart` container, so it adapts
  to the card it sits in, not the viewport.
- **Accessible:**
  - An SVG `<title>`/`<desc>` names the chart.
  - Marks are reachable by screen readers and the keyboard. Ring charts are
    one tab stop moved with the arrow keys; the line chart is a slider.
  - A visually hidden table holds every value.
- **Motion:** CSS transitions, only under
  `prefers-reduced-motion: no-preference`.
- **Cheap on mobile.** A Rust pointer-move listener exists only while a
  finger is down on a line chart or a mouse is over it. Only the crosshair
  re-renders, and the nearest point is found by binary search.

`g3_charts::geom` holds the maths, with no Dioxus in it, and is public:
linear, band and time scales, nice ticks, arcs, LTTB downsampling,
nearest-point search and value formatting.

## Development

```bash
just check         # host, wasm32 and Android; gallery web, server and mobile
just test          # nextest + doc tests
just lint-strict   # clippy with warnings as errors
just gallery       # every chart at http://127.0.0.1:8095 (?theme=dark&mode=ios)
```

See [AGENTS.md](AGENTS.md) for conventions and [docs/design.md](docs/design.md)
for the design.

## License

MIT or Apache-2.0, at your option.
