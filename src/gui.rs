#![allow(non_snake_case)]
use dioxus::prelude::*;
use isahc::ReadResponseExt;

fn Header(cx: Scope) -> Element {
    // TODO(figure out how to make this from modpack_source)
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

fn Spinner(cx: Scope) -> Element {
    cx.render(rsx! {
        div {
            class: "lds-ring",
            div {}
            div {}
            div {}
            div {}
        }
    })
}

#[derive(Props, PartialEq)]
struct VersionProps {
    modpack_source: String,
    modpack_branch: String,
    launcher: super::Launcher,
}

fn Version(cx: Scope<VersionProps>) -> Element {
    let modpack_source = (cx.props.modpack_source).to_owned();
    let modpack_branch = (cx.props.modpack_branch).to_owned();
    let launcher = (cx.props.launcher).to_owned();
    let profile = use_future(cx, (), |_| async move {
        super::init(&modpack_source, &modpack_branch, launcher).await
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
    // TODO(Clean this up)
    let installer_profile = use_state(cx, || profile.unwrap().to_owned());
    let installing = use_state(cx, || false);
    let on_submit = move |event: FormEvent| {
        cx.spawn({
            let mut installer_profile = profile.unwrap().to_owned();
            installing.set(true);
            let installing = installing.clone();
            async move {
                for k in event.data.values.keys() {
                    if event.data.values[k] == "true" {
                        installer_profile.enabled_features.push(k.to_owned())
                    }
                }

                if !installer_profile.installed {
                    super::install(installer_profile).await;
                } else if installer_profile.update_available {
                    super::update(installer_profile).await;
                }
                installing.set(false);
            }
        });
    };
    // TODO(Split these renders into multiple components)
    if **installing {
        cx.render(rsx! {
            div {
                class: "version-container",
                div {
                    class: "subtitle-container",
                    h1 {
                        "{installer_profile.manifest.subtitle}"
                    }
                }
                div {
                    class: "version-inner-container",
                    div {
                        class: "container",
                        style: "justify-items: center;",
                        Spinner {}
                        p {
                            "Installing..."
                        }
                    }
                }
            }
        })
    } else {
        cx.render(rsx! {
            div {
                class: "version-container",
                form {
                    onsubmit: on_submit,
                    div {
                        class: "subtitle-container",
                        h1 {
                            "{installer_profile.manifest.subtitle}"
                        }
                    }
                    div {
                        class: "version-inner-container",
                        div {
                            class: "container",
                            div {
                                class: "feature-list",
                                for feat in &installer_profile.manifest.features {
                                    label {
                                        if feat.default {
                                            rsx!(input {
                                                name: "{feat.id}",
                                                r#type: "checkbox",
                                                checked: "true"
                                            })
                                        } else {
                                            rsx!(input {
                                                name: "{feat.id}",
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
                        }
                        input {
                            r#type: "submit",
                            value: "Install",
                            class: "install-button"
                        }
                    }
                }
            }
        })
    }
}

pub fn App(cx: Scope) -> Element {
    let modpack_source = "Commander07/modpack-test/";
    let branches: Vec<super::GithubBranch> = serde_json::from_str(
        super::build_http_client()
            .get(super::GH_API.to_owned() + modpack_source + "branches")
            .expect("Failed to retrive branches!")
            .text()
            .unwrap()
            .as_str(),
    )
    .expect("Failed to parse branches!");
    // TODO(Launcher selection screen)
    cx.render(rsx! {
        style { include_str!("style.css") }
        Header {}
        div {
            class: "fake-body",
            for i in 0..branches.len() {
                Version {
                    modpack_source: modpack_source.to_string(),
                    modpack_branch: branches[i].name.clone(),
                    launcher: super::Launcher::Vanilla(super::get_minecraft_folder())
                }
            }
        }
    })
}
