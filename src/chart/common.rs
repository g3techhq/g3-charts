//! What every chart shares: ids, the figure frame, the SVG title, the hidden
//! data table, and the stylesheet link.

use dioxus::core::provide_root_context;
use dioxus::prelude::*;
use std::cell::Cell;
use std::rc::Rc;

#[derive(Clone)]
struct ChartIds(Rc<Cell<u64>>);

/// An id prefix unique in the document and the same on the server and the
/// hydrating client.
///
/// The same pattern as g3-ui's element ids: a counter held by the document
/// counts the same way in both renders, where a process-wide one would carry
/// over from whatever the server rendered before.
pub(crate) fn use_chart_id(explicit: Option<String>) -> String {
    let generated = use_hook(|| {
        let ids = try_consume_context::<ChartIds>()
            .unwrap_or_else(|| provide_root_context(ChartIds(Rc::new(Cell::new(1)))));
        let number = ids.0.get();
        ids.0.set(number + 1);
        format!("g3c-{number}")
    });
    explicit
        .filter(|id| !id.trim().is_empty())
        .unwrap_or(generated)
}

/// A signal that follows a plain prop, so a memo that reads it reruns when
/// the prop changes.
///
/// Like g3-ui's `use_synced_signal`, this writes during render, which is
/// otherwise not done: it writes only when the value changed, compares with
/// `peek` so the owner never subscribes to it, and does what `#[component]`
/// does for a `ReadSignal` prop.
pub(crate) fn use_synced<T: PartialEq + Clone + 'static>(value: T) -> Signal<T> {
    let mut signal = use_signal(|| value.clone());
    if *signal.peek() != value {
        signal.set(value);
    }
    signal
}

/// Escape text for HTML: content and attribute values alike.
pub(crate) fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// The SVG's `<title>` and `<desc>`, which `aria-labelledby` on the `svg`
/// points at (`{id}-title {id}-desc`).
///
/// The Dioxus fork has no SVG `title` element (its name is taken by HTML's),
/// so the title is written as markup inside a `g`. Inside an SVG parent,
/// `innerHTML` parses it into the SVG namespace on the client too.
#[component]
pub(crate) fn SvgLabel(id: String, title: String, description: Option<String>) -> Element {
    let markup = format!(
        "<title id=\"{}-title\">{}</title>",
        escape(&id),
        escape(&title)
    );
    rsx! {
        g { dangerous_inner_html: markup }
        if let Some(description) = description {
            desc { id: "{id}-desc", "{description}" }
        }
    }
}

/// The `aria-labelledby` value for an SVG labelled by [`SvgLabel`].
pub(crate) fn labelled_by(id: &str, has_description: bool) -> String {
    if has_description {
        format!("{id}-title {id}-desc")
    } else {
        format!("{id}-title")
    }
}

/// A visually hidden table of every value, for screen readers and anyone
/// reading the page without the picture.
#[component]
pub(crate) fn DataTable(caption: String, headers: Vec<String>, rows: Vec<Vec<String>>) -> Element {
    rsx! {
        table { class: "g3-chart-table",
            caption { "{caption}" }
            thead {
                tr {
                    for (i, header) in headers.iter().enumerate() {
                        th { key: "{i}", scope: "col", "{header}" }
                    }
                }
            }
            tbody {
                for (r, row) in rows.iter().enumerate() {
                    tr { key: "{r}",
                        for (c, cell) in row.iter().enumerate() {
                            if c == 0 {
                                th { key: "{c}", scope: "row", "{cell}" }
                            } else {
                                td { key: "{c}", "{cell}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The classes and attributes of a chart's outer `figure`.
pub(crate) fn frame_class(kind: &str, class: Option<&str>) -> String {
    g3_ui::merge_classes(format!("g3-chart g3-chart-{kind}"), class)
}

/// Links the chart stylesheet. Put it once beside g3-ui's `AppWrapper`, in
/// the app's shell (desktop and mobile need it; a web page already has it in
/// its head).
///
/// Rendered on every build, web included, like g3-ui's own link: each head
/// element takes a slot in the fullstack hydration data, so the server and
/// the client must render the same ones.
#[component]
pub fn ChartStyles() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: crate::CHARTS_CSS }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_markup() {
        assert_eq!(escape("<b>&\"'"), "&lt;b&gt;&amp;&quot;&#39;");
        assert_eq!(labelled_by("g3c-1", true), "g3c-1-title g3c-1-desc");
    }
}
