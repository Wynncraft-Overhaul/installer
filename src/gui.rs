#![allow(non_snake_case)]
use dioxus::prelude::*;
use isahc::ReadResponseExt;

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
    modpack_source: String,
    modpack_branch: String,
}

fn Version(cx: Scope<VersionProps>) -> Element {
    let modpack_source = (cx.props.modpack_source).to_owned();
    let modpack_branch = (cx.props.modpack_branch).to_owned();
    let profile = use_future(cx, (), |_| async move {
        super::init(&modpack_source, &modpack_branch).await
    })
    .value();

    // 'use_future's will always be 'None' on components first render
    if profile.is_none() {
        return cx.render(rsx! {
            div {
                class: "container",
                "Loading..."
            }
        });
    };

    // states can be turned into an Rc using .current() and can be made into an owned value by using .as_ref().to_owned()
    let installer_profile = use_state(cx, || profile.unwrap().to_owned());
    cx.render(rsx! {div {
            class: "container",
            h1 {
                "{cx.props.modpack_branch}"
            }
            div {
                class: "feature-list",
                for feat in &installer_profile.manifest.features {
                    label {
                        if feat.default {
                            rsx!(input {
                                r#type: "checkbox",
                                checked: "true"
                            })
                        } else {
                            rsx!(input {
                                r#type: "checkbox"
                            })
                        }
                        "{feat.name}"
                    }
                }
            }
            div {
                class: "description",
                // Yes this will allow modpacks to include any valid html including script tags
                dangerous_inner_html: "{installer_profile.manifest.description}"
            }
            InstallButton {on_click: move |_| {
                let installer_profile = installer_profile.clone();
                use_future(cx, (), |_| async move {
                    if !installer_profile.installed {
                        super::install(
                            installer_profile.current().as_ref().to_owned()
                        ).await;
                    } else if installer_profile.update_available {
                        super::update(
                            installer_profile.current().as_ref().to_owned()
                        ).await;
                    }
                    println!("Done");
                });
            }}
        }
    })
}

pub fn App(cx: Scope) -> Element {
    let branches: Vec<super::GithubBranch> = serde_json::from_str(
        super::build_http_client()
            .get(super::GH_API.to_owned() + "Commander07/modpack-test/branches")
            .expect("Failed to retrive branches!")
            .text()
            .unwrap()
            .as_str(),
    )
    .expect("Failed to parse branches!");
    cx.render(rsx! {
        style { include_str!("style.css") }
        Header {}
        div {
            class: "fake-body",
            for i in 0..branches.len() {
                Version {
                    modpack_source: String::from("Commander07/modpack-test/"),
                    modpack_branch: branches[i].name.clone(),
                }
            }
        }
    })
}
