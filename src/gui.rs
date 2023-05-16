#![allow(non_snake_case)]
use std::path::PathBuf;

use base64::{engine, Engine};
use dioxus::prelude::*;
use regex::Regex;

use crate::{get_launcher, InstallerProfile};

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
struct CreditsProps {
    manifest: super::Manifest,
}

fn Credits(cx: Scope<CreditsProps>) -> Element {
    cx.render(rsx! {
        ul {
            for r#mod in &cx.props.manifest.mods {
                li {
                    "{r#mod.name} by "
                    for author in &r#mod.authors {
                        a {
                            href: "{author.link}",
                            if r#mod.authors.last().unwrap() == author {
                                author.name.to_string()
                            } else {
                                author.name.to_string() + ", "
                            }
                        }
                    }
                }
            }
            for shaderpack in &cx.props.manifest.shaderpacks {
                li {
                    "{shaderpack.name} by "
                    for author in &shaderpack.authors {
                        a {
                            href: "{author.link}",
                            if shaderpack.authors.last().unwrap() == author {
                                author.name.to_string()
                            } else {
                                author.name.to_string() + ", "
                            }
                        }
                    }
                }
            }
            for resourcepack in &cx.props.manifest.resourcepacks {
                li {
                    "{resourcepack.name} by "
                    for author in &resourcepack.authors {
                        a {
                            href: "{author.link}",
                            if resourcepack.authors.last().unwrap() == author {
                                author.name.to_string()
                            } else {
                                author.name.to_string() + ", "
                            }
                        }
                    }
                }
            }
        }
    })
}

#[derive(Props, PartialEq)]
struct SettingsProps<'a> {
    config: &'a UseRef<super::Config>,
    settings: &'a UseState<bool>,
    config_path: &'a PathBuf,
    error: &'a UseRef<Option<String>>,
    b64_id: String,
}

fn Settings<'a>(cx: Scope<'a, SettingsProps<'a>>) -> Element {
    let mut vanilla = None;
    let mut multimc = None;
    let mut prism = None;
    let launcher = cx.props.config.with(|cfg| cfg.launcher.clone());
    match &*launcher {
        "vanilla" => vanilla = Some("true"),
        "multimc-MultiMC" => multimc = Some("true"),
        "multimc-PrismLauncher" => prism = Some("true"),
        _ => {}
    }
    cx.render(rsx! {
        div {
            class: "version-inner-container",
            style: "width: 21.25vw;",
            div {
                class: "container",
                style: "width: 20vw;",
                form {
                    id: "settings",
                    onsubmit: move |event| {
                        cx.props.config.with_mut(|cfg| cfg.launcher = event.data.values["launcher-select"].clone());
                        let res = std::fs::write(cx.props.config_path, serde_json::to_vec(&*cx.props.config.read()).unwrap());
                        match res {
                            Ok(_) => {},
                            Err(e) => {
                                cx.props.error.set(Some(format!("{:#?}", e) + " (Failed to write config!)"));
                            },
                        }
                        cx.props.settings.set(false);
                    },
                    div {
                        class: "label",
                        span {
                            "Launcher:"
                        }
                        select {
                            name: "launcher-select",
                            id: "launcher-select",
                            form: "settings",
                            class: "credits-button",
                            if super::get_minecraft_folder().is_dir() {
                                rsx!(option {
                                    value: "vanilla",
                                    selected: vanilla,
                                    "Vanilla"
                                })
                            }
                            if super::get_multimc_folder("MultiMC").is_ok() {
                                rsx!(option {
                                    value: "multimc-MultiMC",
                                    selected: multimc,
                                    "MultiMC"
                                })
                            }
                            if super::get_multimc_folder("PrismLauncher").is_ok() {
                                rsx!(option {
                                    value: "multimc-PrismLauncher",
                                    selected: prism,
                                    "Prism Launcher"
                                })
                            }
                        }
                    }
                    input {
                        r#type: "submit",
                        value: "Save",
                        class: "install-button",
                        id: "save"
                    }
                    button {
                        class: "uninstall-button",
                        onclick: move |_evt| {
                            super::uninstall(&get_launcher(&launcher).unwrap(), &cx.props.b64_id);
                        },
                        "Uninstall"
                    }
                }
            }
        }
    })
}

fn feature_change(
    installer_profile: &UseRef<InstallerProfile>,
    modify: &UseState<bool>,
    evt: FormEvent,
    feat: &super::Feature,
    modify_count: &UseRef<i32>,
) {
    let enabled = match &*evt.data.value {
        "true" => true,
        "false" => false,
        _ => panic!("Invalid bool from feature"),
    };
    if installer_profile.with(|profile| profile.installed) {
        let modify_res = installer_profile.with(|profile| {
            profile
                .local_manifest
                .as_ref()
                .unwrap()
                .enabled_features
                .contains(&feat.id)
                != enabled
        });
        if modify_count.with(|x| *x <= 1) {
            modify.set(
                installer_profile.with(|profile| {
                    profile
                        .local_manifest
                        .as_ref()
                        .unwrap()
                        .enabled_features
                        .contains(&feat.id)
                }) != enabled,
            );
        }
        if modify_res {
            modify_count.with_mut(|x| *x += 1);
        } else {
            modify_count.with_mut(|x| *x -= 1);
        }
    }
}

#[derive(Props)]
struct VersionProps<'a> {
    modpack_source: &'a String,
    modpack_branch: String,
    launcher: super::Launcher,
    error: &'a UseRef<Option<String>>,
}

fn Version<'a>(cx: Scope<'a, VersionProps<'a>>) -> Element<'a> {
    let modpack_source = (cx.props.modpack_source).to_owned();
    let modpack_branch = (cx.props.modpack_branch).to_owned();
    let launcher = (cx.props.launcher).to_owned();
    // TODO(Remove weird clonage)
    let error = cx.props.error.clone();
    let err = error.clone();
    let profile = use_future(cx, (), |_| async move {
        super::init(modpack_source, modpack_branch, launcher).await
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
    match profile.unwrap() {
        Ok(_) => (),
        Err(e) => {
            err.set(Some(
                format!("{:#?}", e) + " (Failed to retrieve installer profile!)",
            ));
            return None;
        }
    }
    // states can be turned into an Rc using .current() and can be made into an owned value by using .as_ref().to_owned()
    // TODO(Clean this up)
    let installer_profile = use_ref(cx, || profile.unwrap().to_owned().unwrap());
    let installing = use_state(cx, || false);
    let modify = use_state(cx, || false);
    let modify_count = use_ref(cx, || 0);
    let credits = use_state(cx, || false);
    let on_submit = move |event: FormEvent| {
        cx.spawn({
            installing.set(true);
            let installing = installing.clone();
            let installer_profile = installer_profile.clone();
            let modify = modify.clone();
            let modify_count = modify_count.clone();
            let error = error.clone();
            async move {
                for k in event.data.values.keys() {
                    if event.data.values[k] == "true" {
                        installer_profile
                            .with_mut(|profile| profile.enabled_features.push(k.to_owned()));
                        installer_profile.with_mut(|profile| {
                            profile.manifest.enabled_features.push(k.to_owned())
                        });
                    } else {
                        installer_profile
                            .with_mut(|profile| profile.enabled_features.retain(|x| x != k));
                        installer_profile.with_mut(|profile| {
                            profile.manifest.enabled_features.retain(|x| x != k)
                        });
                    }
                }

                if !installer_profile.with(|profile| profile.installed) {
                    match super::install(installer_profile.with(|profile| profile.clone())).await {
                        Ok(_) => {}
                        Err(e) => {
                            error.set(Some(format!("{:#?}", e) + " (Failed to install modpack!)"));
                        }
                    }
                } else if installer_profile.with(|profile| profile.update_available) {
                    match super::update(installer_profile.with(|profile| profile.clone())).await {
                        Ok(_) => {}
                        Err(e) => {
                            error.set(Some(format!("{:#?}", e) + " (Failed to update modpack!)"));
                        }
                    }
                } else if *modify {
                    match super::update(installer_profile.with(|profile| profile.clone())).await {
                        Ok(_) => {}
                        Err(e) => {
                            error.set(Some(format!("{:#?}", e) + " (Failed to modify modpack!)"));
                        }
                    }
                    modify.with_mut(|x| *x = false);
                    modify_count.with_mut(|x| *x = 0);
                }
                installer_profile.with_mut(|profile| {
                    profile.local_manifest = Some(profile.manifest.clone());
                    profile.installed = true;
                    profile.update_available = false;
                });
                installing.set(false);
            }
        });
    };
    let re = Regex::new("on\\w*=\"").unwrap();
    let description = installer_profile
        .with(|profile| profile.manifest.clone())
        .description
        .replace("<script>", "<noscript>")
        .replace("<script/>", "<noscript/>");
    let description = re.replace_all(description.as_str(), "harmless=\"");
    let install_disable = if installer_profile.with(|profile| profile.installed)
        && installer_profile.with(|profile| !profile.update_available)
        && !modify
    {
        Some("true")
    } else {
        None
    };
    // TODO(Split these renders into multiple components)
    if **installing {
        cx.render(rsx! {
            div {
                class: "version-container",
                div {
                    class: "subtitle-container",
                    h1 {
                        "{installer_profile.with(|profile| profile.manifest.subtitle.clone())}"
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
    } else if **credits {
        cx.render(rsx! {
            div {
                class: "version-container",
                div {
                    class: "subtitle-container",
                    h1 {
                        "{installer_profile.with(|profile| profile.manifest.subtitle.clone())}"
                    }
                }
                div {
                    class: "version-inner-container",
                    div {
                        class: "container",
                        div {
                            class: "info-container",
                            div {
                                class: "button-container",
                                button {
                                    class: "credits-button",
                                    onclick: move |_| {
                                        credits.set(false);
                                    },
                                    "X"
                                }
                            }
                            div {
                                class: "credits",
                                Credits {
                                    manifest: installer_profile.with(|profile| profile.manifest.clone())
                                }
                            }
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
                            "{installer_profile.with(|profile| profile.manifest.subtitle.clone())}"
                        }
                    }
                    div {
                        class: "version-inner-container",
                        div {
                            class: "container",
                            div {
                                class: "info-container",
                                div {
                                    class: "button-container",
                                    button {
                                        class: "credits-button",
                                        onclick: move |_| {
                                            credits.set(true);
                                        },
                                        "i"
                                    }
                                }
                                div {
                                    class: "feature-list",
                                    for feat in installer_profile.with(|profile| profile.manifest.features.clone()) {
                                        label {
                                            input {
                                                checked: if installer_profile.with(|profile| profile.installed) {
                                                    if installer_profile.with(|profile| profile.local_manifest.as_ref().unwrap().enabled_features.contains(&feat.id)) {
                                                        Some("true")
                                                    } else {
                                                        None
                                                    }
                                                } else {
                                                    if feat.default {
                                                        Some("true")
                                                    } else {
                                                        None
                                                    }
                                                },
                                                name: "{feat.id}",
                                                onchange: move |evt| {
                                                    feature_change(installer_profile, modify, evt, &feat, modify_count);
                                                },
                                                r#type: "checkbox",    
                                            }
                                            "{feat.name}"
                                        }
                                    }
                                }
                            }
                            div {
                                class: "description",
                                dangerous_inner_html: "{description}"
                            }
                        }
                        input {
                            r#type: "submit",
                            value: if !installer_profile.with(|profile| profile.installed) {"Install"} else {if **modify {"Modify"} else {"Update"}},
                            class: "install-button",
                            disabled: install_disable
                        }
                    }
                }
            }
        })
    }
}

#[derive(Props, PartialEq)]
struct ErrorProps {
    error: String,
}

fn Error(cx: Scope<ErrorProps>) -> Element {
    cx.render(rsx! {
        "{cx.props.error}"
    })
}

#[derive(Props, PartialEq)]
pub(crate) struct AppProps<'a> {
    pub branches: Vec<super::GithubBranch>,
    pub modpack_source: String,
    pub config: super::Config,
    pub config_path: PathBuf,
    pub style_css: &'a str,
}

pub(crate) fn App<'a>(cx: Scope<'a, AppProps>) -> Element<'a> {
    let modpack_source = &cx.props.modpack_source;
    let branches = &cx.props.branches;
    let config: &UseRef<super::Config> = use_ref(cx, || cx.props.config.clone());
    let settings: &UseState<bool> = use_state(cx, || false);
    let cog = String::from("data:image/png;base64,") + include_str!("assets/cog_icon.png.base64");
    let err: &UseRef<Option<String>> = use_ref(cx, || None);
    if err.with(|e| e.is_some()) {
        return cx.render(rsx!(Error {
            error: err.with(|e| e.clone().unwrap())
        }));
    }
    let cfg = config.with(|cfg| cfg.clone());
    let launcher = match super::get_launcher(&cfg.launcher) {
        Ok(val) => Some(val),
        Err(e) => {
            err.set(Some(format!("{:#?}", e) + " (Invalid launcher!)"));
            None
        }
    };
    if err.with(|e| e.is_some()) {
        return cx.render(rsx!(Error {
            error: err.with(|e| e.clone().unwrap())
        }));
    }
    let launcher = launcher.unwrap();
    cx.render(rsx! {
        style { cx.props.style_css }
        if **settings {
            rsx!{
                Header {}
                div {
                    class: "fake-body",
                    Settings {
                        config: config,
                        settings: settings
                        config_path: &cx.props.config_path,
                        error: err,
                        b64_id: engine::general_purpose::URL_SAFE_NO_PAD.encode(modpack_source)
                    }
                }
            }
        } else {
            rsx!{
                button {
                    class: "settings-button",
                    onclick: move |_| {
                        settings.set(true);
                    },
                    img {
                        src: "{cog}",
                    }
                }
                Header {}
                div {
                    class: "fake-body",
                    for i in 0..branches.len() {
                        Version {
                            modpack_source: modpack_source,
                            modpack_branch: branches[i].name.clone(),
                            launcher: launcher.clone(),
                            error: err
                        }
                    }
                }
            }
        }
    })
}
