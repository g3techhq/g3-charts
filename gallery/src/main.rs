//! The g3-charts gallery: every chart, server-rendered then hydrated, in
//! light or dark (`?theme=dark`) and Material or iOS (`?mode=ios`).

use dioxus::prelude::*;
use g3_charts::prelude::*;
use g3_ui::{AppWrapper, Card, ComponentMode, Content, ContentWidth, Header, Theme};

mod data;

const GALLERY_CSS: Asset = asset!(
    "/assets/gallery.css",
    AssetOptions::css().with_static_head(true)
);

#[derive(Clone, Debug, PartialEq, Routable)]
enum Route {
    #[route("/?:theme&:mode")]
    Gallery { theme: String, mode: String },
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        Router::<Route> {}
    }
}

#[component]
fn Gallery(theme: String, mode: String) -> Element {
    let dark = theme == "dark";
    let ios = mode == "ios";
    let app_theme = if dark {
        Theme::default_dark()
    } else {
        Theme::default_light()
    };
    let app_mode = if ios {
        ComponentMode::Ios
    } else {
        ComponentMode::Md
    };
    let usd = ValueFormat::Currency {
        symbol: "$",
        decimals: 0,
    };
    let pct = ValueFormat::Percent { decimals: 1 };
    let link = |theme: &str, mode: &str| Route::Gallery {
        theme: theme.into(),
        mode: mode.into(),
    };
    let theme_name = if dark { "dark" } else { "light" };
    let mode_name = if ios { "ios" } else { "md" };

    rsx! {
        document::Link { rel: "stylesheet", href: GALLERY_CSS }
        ChartStyles {}
        AppWrapper { theme: app_theme, mode: app_mode,
            Header { title: "g3-charts" }
            Content { width: ContentWidth::Wide,
                nav { class: "gallery-switches", aria_label: "Gallery appearance",
                    Link { to: link("light", mode_name), aria_current: (!dark).then_some("page"), "Light" }
                    Link { to: link("dark", mode_name), aria_current: dark.then_some("page"), "Dark" }
                    Link { to: link(theme_name, "md"), aria_current: (!ios).then_some("page"), "Material" }
                    Link { to: link(theme_name, "ios"), aria_current: ios.then_some("page"), "iOS" }
                }
                div { class: "gallery-grid",
                    Card { title: "Donut",
                        Donut {
                            slices: data::classes(),
                            title: "Holdings by asset class",
                            description: "US stocks are just over half of the portfolio.",
                            format: usd,
                            center_label: "Invested",
                        }
                    }
                    Card { title: "Pie",
                        Donut {
                            slices: data::classes(),
                            title: "Holdings by asset class, as a pie",
                            hole: 0.0,
                            format: usd,
                        }
                    }
                    Card { title: "Sunburst: target vs actual",
                        Sunburst {
                            root: data::target(),
                            actual: Some(data::actual()),
                            title: "Target allocation against holdings",
                            description: "Inner rings are asset classes, outer rings funds. Thin bands show what is held.",
                            format: pct,
                            center_label: "Target",
                        }
                    }
                    Card { title: "Bullet: weight vs target",
                        TargetBars {
                            rows: data::weights(),
                            title: "Fund weights against targets",
                            format: pct,
                            on_select: move |_| {},
                        }
                    }
                    Card { title: "Grouped",
                        TargetBars {
                            rows: data::weights(),
                            title: "Fund weights against targets, grouped",
                            variant: TargetBarVariant::Grouped,
                            format: pct,
                        }
                    }
                    Card { title: "Drift",
                        TargetBars {
                            rows: data::weights(),
                            title: "Drift from target",
                            variant: TargetBarVariant::Drift,
                            format: pct,
                        }
                    }
                    div { class: "gallery-wide",
                        Card { title: "Portfolio value, five years daily",
                            LineChart {
                                series: data::history(1_826),
                                title: "Portfolio value and contributions",
                                description: "Daily value since January 2021, with the total paid in.",
                                y_format: usd,
                            }
                        }
                    }
                    div { class: "gallery-wide",
                        Card { title: "Stress: 10,000 points",
                            LineChart {
                                series: data::stress(10_000),
                                title: "One week of minute prices",
                                y_format: ValueFormat::Number { decimals: 2 },
                                y_domain: YDomain::Auto,
                            }
                        }
                    }
                    Card { title: "Sparklines",
                        for (seed, fund, drift) in [(3_u64, "VTI", 0.002), (5, "VEA", -0.001), (9, "BND", 0.0003)] {
                            div { key: "{fund}", class: "gallery-holding",
                                span { "{fund}" }
                                Sparkline {
                                    values: data::closes(seed, drift),
                                    label: "{fund}, last 30 days",
                                    area: true,
                                    format: ValueFormat::Number { decimals: 2 },
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
