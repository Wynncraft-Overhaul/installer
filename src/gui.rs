#![allow(non_snake_case)]
use dioxus::prelude::*;

#[derive(Props)]
struct InstallButtonProps<'a> {
    on_click: EventHandler<'a, MouseEvent>,
}

fn InstallButton<'a>(cx: Scope<'a, InstallButtonProps<'a>>) -> Element<'a> {
    cx.render(rsx! {
        button {
            class: "install-button",
            onclick: move |evt| {
                cx.props.on_click.call(evt);
            },
            "Install"
        }
    })
}

fn Header(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            class: "header",
            div {
                class: "header-inner",
                h1 {
                    "Majestic Overhaul"
                }
            }
        }
    })
}

#[derive(Props, PartialEq)]
struct VersionProps {
    name: &'static str,
    features: Vec<&'static str>,
    always: Vec<&'static str>,
}

fn Version(cx: Scope<VersionProps>) -> Element {
    cx.render(rsx! {div {
            class: "container",
            h1 {
                cx.props.name
            }
            div {
                class: "feature-list",
                for feat in &cx.props.features {
                    label {
                        input {
                            r#type: "checkbox",
                            checked: "true"
                        }
                        "{feat}"
                    }
                }
            }
            p {
                "Always included:"
            }
            ul {
                for al in &cx.props.always {
                    li {
                        "{al}"
                    }
                }
            }
            InstallButton {on_click: move |event: Event<MouseData>| {
                println!("Install!!!!!!");
                event.stop_propagation();
            }}
        }
    })
}

pub fn App(cx: Scope) -> Element {
    cx.render(rsx! {
        style { include_str!("style.css") }
        Header {}
        div {
            class: "fake-body",
            Version {
                name: "Immersive Version",
                features: vec!["Shaders", "Texturepacks", "Optional Mods", "Bobby"],
                always: vec!["Performance", "Immersiveness"]
            }
            Version {
                name: "Performance Version",
                features: vec!["Shaders", "Texturepacks", "Wynntils", "Bobby"],
                always: vec!["Performance"]
            }
        }
    })
}
