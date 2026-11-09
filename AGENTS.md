# AGENTS.md

Instructions for coding agents working in this repository (Claude Code, Codex,
Cursor, Copilot, and anything else that reads `AGENTS.md`). Useful for people
too. Follow these over your defaults.

## What this is

**g3-charts** is the g3 stack's chart library. It provides Dioxus components
drawn as SVG from Rust, with no JavaScript, for web and for the Android and iOS
web views. Its conventions are g3-ui's (`../g3-ui/AGENTS.md`); this file adds
only what is specific to charts. The design and its reasons are in
`docs/design.md`.

| Piece | Version | Reference |
| --- | --- | --- |
| Dioxus | 0.8.0-alpha.3, from the [Gateway3B/dioxus](https://github.com/Gateway3B/dioxus) fork | the fork's source, not the 0.7 guide |
| g3-ui | 0.7 | `ComponentMode`, `merge_classes`, the theme tokens |
| Rust | 1.88+, edition 2024 | `rust-toolchain.toml` |

**Your training data is probably wrong about this Dioxus.** SVG elements have
no `aria_*` builder methods (write `"aria-label": ..`), there is no SVG `title`
element (see `SvgLabel`), and listeners cannot be optional in `rsx!` (see
`chart/pointer.rs`). Prefer the patterns in `src/` over what you remember.

## Map

```
src/geom/            Pure maths, no Dioxus: scales, ticks, time, arcs, LTTB,
                     nearest point, paths, formats. Unit tested in place.
src/model.rs         Slice, Node, TargetRow, Series, Point, ChartColor
src/chart/common.rs  Ids, SvgLabel, DataTable, ChartStyles, use_synced
src/chart/pointer.rs Listeners that exist only while needed
src/chart/*.rs       One file per chart; `*_layout.rs` holds its pure geometry
src/tests.rs         Markup and ARIA tests (dioxus-ssr), stylesheet classes
assets/g3-charts.css The one stylesheet, inside `@layer g3`
gallery/             Fullstack demo app (own root): `just gallery`
```

## Commands

```bash
just check        # library: host, wasm32, Android; gallery: web, server, mobile
just test         # nextest + doc tests
just lint-strict  # clippy, warnings as errors, host + wasm32 + Android + gallery
just format       # rustfmt (crate and gallery)
just pre-push     # all of the above plus typos
just gallery      # dx serve on 127.0.0.1:8095
```

## Definition of done

1. `just pre-push` passes.
2. A user-visible change has a line under `## [Unreleased]` in `CHANGELOG.md`.
3. A new or changed chart has geometry tests next to its layout, markup tests in
   `src/tests.rs`, and a card in the gallery.
4. A visual change was looked at in the gallery, in light and dark, at
   390×844 and 1440×900, with the browser console checked for hydration
   errors.

If you could not do one of these, say which and why.

## Rules

- **Maths goes in `geom` or a `*_layout.rs`**, as plain functions with unit
  tests. A component computes nothing that a test cannot reach without
  rendering.
- **Colors are tokens.**
  - Series colors come from `ChartColor`.
  - Everything else is `var(--g3-color-*)` or `color-mix()` over it.
  - The only literals are the default palette in the stylesheet. If you
    change it, run the palette validator against g3-ui's light card
    (`#f7f8fa`) and dark card (`#1f2937`).
  - Status colors (success, warning, danger) are never series colors.
- **No pointer-move listener at rest.** On native renderers, one Rust
  `onpointermove` makes every pointer move in the app wait on Rust. Add one
  only while a gesture is in progress, through `optional_listener`, on the
  element that took the pointer.
- **Fast state gets its own component.** Hover and crosshair state live in
  a child (`DonutCenter`, `Scrubber`), so a move never re-renders paths or
  axes.
- **Plain props a memo reads go through `use_synced`**, because `use_memo`
  keeps its first closure. Data props are `ReadSignal<T>`.
- **One tree on every build.** The server renders what the client hydrates:
  gate calls, never markup. Nothing measured at runtime changes markup the
  server rendered.
- **Accessibility:**
  - Every chart is named by its `title` and has the hidden `DataTable`.
  - Interactive SVG is `role="group"` with `aria-activedescendant`, not a
    `role="img"` that would hide its marks.
  - Decorative HTML (legend, axes, centre text) is `aria-hidden`.
- **Pin `dioxus*` and `manganis` exactly**, from the fork through
  `[patch.crates-io]` in both `Cargo.toml` and `gallery/Cargo.toml`.
- **Ask before adding a dependency.** Nothing is published while g3-ui
  depends on the fork.
- Commits follow Conventional Commits.

## Readability

Follow [docs/code-style.md](docs/code-style.md) for Rust imports, section
spacing and RSX layout.
