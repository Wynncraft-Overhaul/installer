use std::{collections::BTreeMap, path::PathBuf};

use base64::{engine, Engine};
use dioxus::prelude::*;
use modal::{Modal, ModalContext};

use crate::{get_app_data, get_installed_packs, get_launcher, uninstall, Launcher, PackName};

mod modal;

#[derive(Clone)]
struct TabInfo {
    color: String,
    title: String,
    background: String,
    settings_background: String,
    primary_font: String,
    secondary_font: String,
}

#[component]
fn ProgressView(value: i64, max: i64, status: String, title: String) -> Element {
    rsx!(
        div { class: "version-container",
            div { class: "subtitle-container",
                h1 { "{title}" }
            }
            div { class: "container", style: "justify-items: center;",
                progress { max, value: "{value}" }
                p { class: "progress-status", "{status}" }
            }
        }
    )
}

#[derive(PartialEq, Props, Clone)]
struct CreditsProps {
    manifest: super::Manifest,
    enabled: Vec<String>,
    credits: Signal<bool>,
}

#[component]
fn Credits(mut props: CreditsProps) -> Element {
    rsx! {
        div { class: "version-container",
            div { class: "subtitle-container",
                h1 { "{props.manifest.subtitle}" }
            }
            div { class: "container",
                div { class: "info-container",
                    div { class: "button-container",
                        button {
                            class: "credits-button",
                            onclick: move |evt| {
                                props.credits.set(false);
                                evt.stop_propagation();
                            },
                            "X"
                        }
                    }
                    div { class: "credits",
                        div { class: "credits-inner",
                            ul {
                                for r#mod in props.manifest.mods {
                                    if props.enabled.contains(&r#mod.id) {
                                        li {
                                            "{r#mod.name} by "
                                            for author in &r#mod.authors {
                                                a { href: "{author.link}",
                                                    if r#mod.authors.last().unwrap() == author {
                                                        {author.name.to_string()}
                                                    } else {
                                                        {author.name.to_string() + ", "}
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                for shaderpack in props.manifest.shaderpacks {
                                    if props.enabled.contains(&shaderpack.id) {
                                        li {
                                            "{shaderpack.name} by "
                                            for author in &shaderpack.authors {
                                                a { href: "{author.link}",
                                                    if shaderpack.authors.last().unwrap() == author {
                                                        {author.name.to_string()}
                                                    } else {
                                                        {author.name.to_string() + ", "}
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                for resourcepack in props.manifest.resourcepacks {
                                    if props.enabled.contains(&resourcepack.id) {
                                        li {
                                            "{resourcepack.name} by "
                                            for author in &resourcepack.authors {
                                                a { href: "{author.link}",
                                                    if resourcepack.authors.last().unwrap() == author {
                                                        {author.name.to_string()}
                                                    } else {
                                                        {author.name.to_string() + ", "}
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                for include in props.manifest.include {
                                    if props.enabled.contains(&include.id) && include.authors.is_some()
                                        && include.name.is_some()
                                    {
                                        li {
                                            "{include.name.as_ref().unwrap()} by "
                                            for author in &include.authors.as_ref().unwrap() {
                                                a { href: "{author.link}",
                                                    if include.authors.as_ref().unwrap().last().unwrap() == author {
                                                        {author.name.to_string()}
                                                    } else {
                                                        {author.name.to_string() + ", "}
                                                    }
 
                                                }
 
                                            }
 
                                        }
 
                                    }
 
                                }
 
                            }
 
                        }
 
                    }
 
                }
 
            }
 
        }
 
    }
 
}
#[component]
fn PackUninstallButton(launcher: Launcher, pack: PackName) -> Element {
    // TODO: Handle uninstall error
    let mut hidden = use_signal(|| false);
    rsx!(
        li { hidden,
            button {
                class: "pack-uninstall",
                onclick: move |_| {
                    uninstall(&launcher, &pack.uuid).unwrap();
                    *hidden.write() = true;
                },
                "{pack.name}"
            }
        }
    )
}

#[derive(PartialEq, Props, Clone)]
struct SettingsProps {
    config: Signal<super::Config>,
    settings: Signal<bool>,
    config_path: PathBuf,
    error: Signal<Option<String>>,
    b64_id: String,
}

#[component]
fn Settings(mut props: SettingsProps) -> Element {
    let mut vanilla = None;
    let mut multimc = None;
    let mut prism = None;
    let mut custom = None;
    let launcher = get_launcher(&props.config.read().launcher).unwrap();
    let packs = match get_installed_packs(&launcher) {
        Ok(v) => v,
        Err(err) => {
            *props.error.write() = Some(err.to_string());
            return None;
        }
    };
    match &props.config.read().launcher[..] {
        "vanilla" => vanilla = Some("true"),
        "multimc-MultiMC" => multimc = Some("true"),
        "multimc-PrismLauncher" => prism = Some("true"),
        _ => {}
    }
    if props.config.read().launcher.starts_with("custom") {
        custom = Some("true")
    }

    rsx! {
        div { class: "container", style: "width: 24vw;",
            form {
                id: "settings",
                onsubmit: move |event| {
                    props
                        .config
                        .write()
                        .launcher = event.data.values()["launcher-select"].as_value();
                    if let Err(e) = std::fs::write(
                        &props.config_path,
                        serde_json::to_vec(&*props.config.read()).unwrap(),
                    ) {
                        props.error.set(Some(format!("{:#?}", e) + " (Failed to write config!)"));
                    }
                    props.settings.set(false);
                },
                div { class: "label",
                    span { "Launcher:" }
                    select {
                        name: "launcher-select",
                        id: "launcher-select",
                        form: "settings",
                        class: "credits-button",
                        if super::get_minecraft_folder().is_dir() {
                            option { value: "vanilla", selected: vanilla, "Vanilla" }
                        }
                        if super::get_multimc_folder("MultiMC").is_ok() {
                            option { value: "multimc-MultiMC", selected: multimc, "MultiMC" }
                        }
                        if super::get_multimc_folder("PrismLauncher").is_ok() {
                            option {
                                value: "multimc-PrismLauncher",
                                selected: prism,
                                "Prism Launcher"
                            }
                        }
                        if custom.is_some() {
                            option {
                                value: "{props.config.read().launcher}",
                                selected: custom,
                                "Custom MultiMC"
                            }
                        }
                    }
                }
                CustomMultiMCButton {
                    config: props.config,
                    config_path: props.config_path.clone(),
                    error: props.error,
                    b64_id: props.b64_id.clone()
                }
                input {
                    r#type: "submit",
                    value: "Save",
                    class: "install-button",
                    id: "save"
                }
                button {
                    class: "uninstall-button",
                    r#type: "button",
                    disabled: packs.is_empty(),
                    onclick: move |evt| {
                        let mut modal = use_context::<ModalContext>();
                        modal
                            .open(
                                "Select modpack to uninstall.",
                                rsx! {
                                    ul {
                                        for pack in packs.clone() {
                                            PackUninstallButton { launcher: launcher.clone(), pack }
                                        }
                                    }
                                },
                                false,
                                Some(|_| {}),
                            );
                        evt.stop_propagation();
                    },
                    "Uninstall"
                }
            }
        }
    }
}

#[derive(PartialEq, Props, Clone)]
struct LauncherProps {
    config: Signal<super::Config>,
    config_path: PathBuf,
    error: Signal<Option<String>>,
    b64_id: String,
}

#[component]
fn Launcher(mut props: LauncherProps) -> Element {
    let mut vanilla = None;
    let mut multimc = None;
    let mut prism = None;
    match &props.config.read().launcher[..] {
        "vanilla" => vanilla = Some("true"),
        "multimc-MultiMC" => multimc = Some("true"),
        "multimc-PrismLauncher" => prism = Some("true"),
        _ => {}
    }
    let has_supported_launcher = super::get_minecraft_folder().is_dir()
        || super::get_multimc_folder("MultiMC").is_ok()
        || super::get_multimc_folder("PrismLauncher").is_ok();
    if !has_supported_launcher {
        rsx!(NoLauncherFound {
            config: props.config,
            config_path: props.config_path,
            error: props.error,
            b64_id: props.b64_id.clone()
        })
    } else {
        rsx! {
            div { class: "container", style: "width: 24vw;",
                form {
                    id: "settings",
                    onsubmit: move |event| {
                        props
                            .config
                            .write()
                            .launcher = event.data.values()["launcher-select"].as_value();
                        props.config.write().first_launch = Some(false);
                        if let Err(e) = std::fs::write(
                            &props.config_path,
                            serde_json::to_vec(&*props.config.read()).unwrap(),
                        ) {
                            props.error.set(Some(format!("{:#?}", e) + " (Failed to write config!)"));
                        }
                    },
                    div { class: "label",
                        span { "Launcher:" }
                        select {
                            name: "launcher-select",
                            id: "launcher-select",
                            form: "settings",
                            class: "credits-button",
                            if super::get_minecraft_folder().is_dir() {
                                option { value: "vanilla", selected: vanilla, "Vanilla" }
                            }
                            if super::get_multimc_folder("MultiMC").is_ok() {
                                option {
                                    value: "multimc-MultiMC",
                                    selected: multimc,
                                    "MultiMC"
                                }
                            }
                            if super::get_multimc_folder("PrismLauncher").is_ok() {
                                option {
                                    value: "multimc-PrismLauncher",
                                    selected: prism,
                                    "Prism Launcher"
                                }
                            }
                        }
                    }
                    CustomMultiMCButton {
                        config: props.config,
                        config_path: props.config_path.clone(),
                        error: props.error,
                        b64_id: props.b64_id.clone()
                    }
                    input {
                        r#type: "submit",
                        value: "Continue",
                        class: "install-button",
                        id: "save"
                    }
                }
            }
        }
    }
}

#[component]
fn CustomMultiMCButton(mut props: LauncherProps) -> Element {
    let custom_multimc = move |_evt| {
        let directory_dialog = rfd::FileDialog::new()
            .set_title("Pick root directory of desired MultiMC based launcher.")
            .set_directory(get_app_data());
        let directory = directory_dialog.pick_folder();
        match directory {
            Some(path) => {
                if !path.join("instances").is_dir() {
                    return;
                }
                let path = path.to_str();
                if path.is_none() {
                    props
                        .error
                        .set(Some(String::from("Could not get path to directory!")));
                }
                props.config.write().launcher = format!("custom-{}", path.unwrap());
                props.config.write().first_launch = Some(false);
                if let Err(e) = std::fs::write(
                    &props.config_path,
                    serde_json::to_vec(&*props.config.read()).unwrap(),
                ) {
                    props
                        .error
                        .set(Some(format!("{:#?}", e) + " (Failed to write config!)"));
                }
            }
            None => {}
        }
    };
    rsx!(
        button {
            class: "install-button custom-multimc-button",
            onclick: custom_multimc,
            r#type: "button",
            "Use custom MultiMC directory"
        }
    )
}

#[component]
fn NoLauncherFound(props: LauncherProps) -> Element {
    rsx! {
        div { class: "container", style: "width: 48vw;",
            h1 { "No supported launcher found!" }
            p {
                "Only Prism Launcher, MultiMC and the vanilla launcher are supported by default, other MultiMC launchers can be added using the button below."
                br {}
                br {}
                "If you have any of these installed then please make sure you are on the latest version of the installer, if you are, open a thread in #📂modpack-issues on the discord. Please make sure your thread contains the following information: Launcher your having issues with, directory of the launcher and your OS."
            }
            CustomMultiMCButton {
                config: props.config,
                config_path: props.config_path,
                error: props.error,
                b64_id: props.b64_id.clone()
            }
        }
    }
}

fn feature_change(
    local_features: Signal<Option<Vec<String>>>,
    mut modify: Signal<bool>,
    evt: FormEvent,
    feat: &super::Feature,
    mut modify_count: Signal<i32>,
    mut enabled_features: Signal<Vec<String>>,
) {
    let enabled = match &*evt.data.value() {
        "true" => true,
        "false" => false,
        _ => panic!("Invalid bool from feature"),
    };
    if enabled {
        enabled_features.with_mut(|x| {
            if !x.contains(&feat.id) {
                x.push(feat.id.clone());
            }
        })
    } else {
        enabled_features.with_mut(|x| {
            if x.contains(&feat.id) {
                x.retain(|x| x != &feat.id);
            }
        })
    }
    if local_features.read().is_some() {
        let modify_res = local_features.unwrap().contains(&feat.id) != enabled;
        if modify_count.with(|x| *x <= 1) {
            modify.set(local_features.unwrap().contains(&feat.id) != enabled);
        }
        if modify_res {
            modify_count.with_mut(|x| *x += 1);
        } else {
            modify_count.with_mut(|x| *x -= 1);
        }
    }
}

#[derive(PartialEq, Props, Clone)]
struct VersionProps {
    modpack_source: String,
    modpack_branch: String,
    launcher: super::Launcher,
    error: Signal<Option<String>>,
    name: Signal<String>,
    page: Signal<usize>,
    pages: Signal<BTreeMap<usize, TabInfo>>,
}

#[component]
fn Version(mut props: VersionProps) -> Element {
    let profile = use_resource(move || {
        let source = props.modpack_source.clone();
        let branch = props.modpack_branch.clone();
        let launcher = props.launcher.clone();
        async move { super::init(source, branch, launcher).await }
    });

    // 'use_future's will always be 'None' on components first render
    if profile.read().is_none() {
        return rsx! {
            div { class: "container", "Loading..." }
        };
    };

    let installer_profile = match profile.unwrap() {
        Ok(v) => v,
        Err(e) => {
            props.error.set(Some(
                format!("{:#?}", e) + " (Failed to retrieve installer profile!)",
            ));
            return None;
        }
    };

    let tab_group = if let Some(tab_group) = installer_profile.manifest.tab_group {
        tab_group
    } else {
        0
    };
    let tab_title = if let Some(ref tab_title) = installer_profile.manifest.tab_title {
        tab_title.clone()
    } else {
        String::from("Default")
    };
    let tab_color = if let Some(ref tab_color) = installer_profile.manifest.tab_color {
        tab_color.clone()
    } else {
        String::from("#320625")
    };
    let tab_background = if let Some(ref tab_background) = installer_profile.manifest.tab_background
    {
        tab_background.clone()
    } else {
        String::from("https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/background_installer.png")
    };
    let settings_background =
        if let Some(ref settings_background) = installer_profile.manifest.settings_background {
            settings_background.clone()
        } else {
            tab_background.clone()
        };
    let tab_secondary_font = if let Some(ref tab_secondary_font) =
        installer_profile.manifest.tab_secondary_font
    {
        tab_secondary_font.clone()
    } else {
        String::from("https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/Wynncraft_Game_Font.woff2")
    };
    let tab_primary_font = if let Some(ref tab_primary_font) =
        installer_profile.manifest.tab_primary_font
    {
        tab_primary_font.clone()
    } else {
        String::from("https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/Wynncraft_Game_Font.woff2")
    };
    props.pages.with_mut(|x| {
        x.insert(
            tab_group,
            TabInfo {
                color: tab_color,
                title: tab_title,
                background: tab_background,
                settings_background,
                primary_font: tab_primary_font,
                secondary_font: tab_secondary_font,
            },
        )
    });

    let mut installing = use_signal(|| false);
    let mut progress_status = use_signal(|| "");
    let mut install_progress = use_signal(|| 0);
    let mut modify = use_signal(|| false);
    let mut modify_count = use_signal(|| 0);
    let enabled_features = use_signal(|| {
        if installer_profile.installed {
            installer_profile
                .local_manifest
                .as_ref()
                .unwrap()
                .enabled_features
                .clone()
        } else {
            installer_profile.enabled_features.clone()
        }
    });
    let mut install_item_amount = use_signal(|| 0);
    let mut credits = use_signal(|| false);
    let mut installed = use_signal(|| installer_profile.installed);
    let mut update_available = use_signal(|| installer_profile.update_available);
    let mut local_features = use_signal(|| {
        if let Some(manifest) = installer_profile.local_manifest.clone() {
            Some(manifest.enabled_features)
        } else {
            None
        }
    });
    let movable_profile = installer_profile.clone();
    let on_submit = move |_| {
        // TODO: Don't do naive item amount calculation
        *install_item_amount.write() = movable_profile.manifest.mods.len()
            + movable_profile.manifest.resourcepacks.len()
            + movable_profile.manifest.shaderpacks.len()
            + movable_profile.manifest.include.len();
        let movable_profile = movable_profile.clone();
        let movable_profile2 = movable_profile.clone();
        async move {
            let install = move |canceled| {
                let mut installer_profile = movable_profile.clone();
                spawn(async move {
                    if canceled {
                        return;
                    }
                    installing.set(true);
                    installer_profile.enabled_features = enabled_features.read().clone();
                    installer_profile.manifest.enabled_features = enabled_features.read().clone();
                    local_features.set(Some(enabled_features.read().clone()));

                    if !*installed.read() {
                        progress_status.set("Installing");
                        match super::install(&installer_profile, move || {
                            install_progress.with_mut(|x| *x += 1);
                        })
                        .await
                        {
                            Ok(_) => {
                                let _ = isahc::post(
                                    "https://tracking.commander07.workers.dev/track",
                                    format!(
                                        "{{
                                        \"projectId\": \"55db8403a4f24f3aa5afd33fd1962888\",
                                        \"dataSourceId\": \"{}\",
                                        \"userAction\": \"install\",
                                        \"additionalData\": {{
                                            \"features\": {:?},
                                            \"version\": \"{}\",
                                            \"launcher\": \"{}\"
                                        }}
                                    }}",
                                        installer_profile.manifest.uuid,
                                        installer_profile.manifest.enabled_features,
                                        installer_profile.manifest.modpack_version,
                                        installer_profile.launcher.unwrap(),
                                    ),
                                );
                            }
                            Err(e) => {
                                props.error.set(Some(
                                    format!("{:#?}", e) + " (Failed to install modpack!)",
                                ));
                                installing.set(false);
                                return;
                            }
                        }
                        installed.set(true);
                    } else if *update_available.read() {
                        progress_status.set("Updating");
                        match super::update(&installer_profile, move || {
                            install_progress.with_mut(|x| *x += 1);
                        })
                        .await
                        {
                            Ok(_) => {
                                let _ = isahc::post(
                                    "https://tracking.commander07.workers.dev/track",
                                    format!(
                                        "{{
                                    \"projectId\": \"55db8403a4f24f3aa5afd33fd1962888\",
                                    \"dataSourceId\": \"{}\",
                                    \"userAction\": \"update\",
                                    \"additionalData\": {{
                                        \"old_version\": \"{}\",
                                        \"new_version\": \"{}\"
                                    }}
                                }}",
                                        installer_profile.manifest.uuid,
                                        installer_profile.local_manifest.unwrap().modpack_version,
                                        installer_profile.manifest.modpack_version
                                    ),
                                );
                            }
                            Err(e) => {
                                props.error.set(Some(
                                    format!("{:#?}", e) + " (Failed to update modpack!)",
                                ));
                                installing.set(false);
                                return;
                            }
                        }
                        update_available.set(false);
                    } else if *modify.read() {
                        progress_status.set("Modifying");
                        match super::update(&installer_profile, move || {
                            *install_progress.write() += 1
                        })
                        .await
                        {
                            Ok(_) => {
                                let _ = isahc::post(
                                    "https://tracking.commander07.workers.dev/track",
                                    format!(
                                        "{{
                                    \"projectId\": \"55db8403a4f24f3aa5afd33fd1962888\",
                                    \"dataSourceId\": \"{}\",
                                    \"userAction\": \"modify\",
                                    \"additionalData\": {{
                                        \"features\": {:?}
                                    }}
                                }}",
                                        installer_profile.manifest.uuid,
                                        installer_profile.manifest.enabled_features
                                    ),
                                );
                            }
                            Err(e) => {
                                props.error.set(Some(
                                    format!("{:#?}", e) + " (Failed to modify modpack!)",
                                ));
                                installing.set(false);
                                return;
                            }
                        }
                        modify.with_mut(|x| *x = false);
                        modify_count.with_mut(|x| *x = 0);
                        update_available.set(false);
                    }
                    installing.set(false);
                });
            };

            if let Some(contents) = movable_profile2.manifest.popup_contents {
                use_context::<ModalContext>().open(
                    movable_profile2.manifest.popup_title.unwrap_or_default(),
                    rsx!(div {
                        dangerous_inner_html: "{contents}"
                    }),
                    true,
                    Some(install),
                )
            } else {
                install(false);
            }
        }
    };

    let install_disable = if *installed.read() && !*update_available.read() && !*modify.read() {
        Some("true")
    } else {
        None
    };

    if *props.name.read() == String::default() {
        props.name.set(installer_profile.manifest.name.clone())
    }
    if (props.page)() != tab_group {
        return None;
    }
rsx! {
rsx! {
    // Add style to the rsx block
    style { "
        .tooltip {{
            position: relative;
            display: inline-flex;
            align-items: center;
            flex-wrap: wrap;
            cursor: pointer;
            gap: 5px;
            width: 100%;
        }}

        .tooltip .tooltiptext {{
            visibility: hidden;
            opacity: 0;
            position: absolute;  /* Tooltip positioned above the element */
            background-color: rgba(0, 0, 0, 0.9);
            color: white;
            padding: 6px 10px;
            border-radius: 5px;
            z-index: 99999;
            white-space: normal;
            max-width: 250px;
            text-align: left;
            word-wrap: break-word;

            /* Position the tooltip above the element */
            bottom: 100%;
            left: 50%;
            transform: translateX(-50%);  /* Center tooltip horizontally */
            margin-bottom: 10px;  /* Adjust for space above */
        }}

        .tooltip:hover .tooltiptext {{
            visibility: visible;
            opacity: 1;
        }}

        /* Optional feature list and checkbox styles */
        .feature-list input[type="checkbox"] {{
            flex-shrink: 0;
            width: 18px;
            height: 18px;
        }}

        .feature-list label {{
            display: flex;
            align-items: center;
            flex-wrap: wrap;
            gap: 5px;
            width: 100%;
            line-height: 1.2;
        }}

        .feature-list label div {{
            flex: 1;
            word-wrap: break-word;
            overflow-wrap: break-word;
        }}

        .feature-list {{
            max-height: 200px;
            overflow-y: auto;
            overflow-x: hidden;
            border: 1px solid #ccc;
            padding: 5px;
            position: relative;
        }}
    " }

    // Render the form with optional feature text and checkboxes
    if *installing.read() {
        ProgressView {
            value: install_progress(),
            max: install_item_amount() as i64,
            title: installer_profile.manifest.subtitle,
            status: progress_status.to_string()
        }
    } else if *credits.read() {
        Credits {
            manifest: installer_profile.manifest,
            enabled: installer_profile.enabled_features,
            credits
        }
    } else {
        div { class: "version-container",
            form { onsubmit: on_submit,
                div { class: "subtitle-container",
                    h1 { "{installer_profile.manifest.subtitle}" }
                }
                div { class: "container",
                    div { class: "info-container",
                        div { class: "button-container",
                            button {
                                class: "credits-button",
                                onclick: move |evt| {
                                    credits.set(true);
                                    evt.stop_propagation();
                                },
                                "i"
                            }
                        }
                        div { style: "width: 21vw",
                            div {
                                class: "description",
                                dangerous_inner_html: "{installer_profile.manifest.description}"
                            }
                            p { style: "font-size: 1.2em;margin-bottom: .5em;",
                                "Optional features:"
                            }
                            div { class: "feature-list",
                                for feat in installer_profile.manifest.features {
                                    if !feat.hidden {
                                        label { class: "tooltip",
                                            style: "display: flex; align-items: center; flex-wrap: wrap; cursor: pointer;",
                                            input {
                                                checked: if installer_profile.installed {
                                                    if enabled_features.with(|x| x.contains(&feat.id)) { Some("true") } else { None }
                                                } else {
                                                    if feat.default { Some("true") } else { None }
                                                },
                                                name: "{feat.id}",
                                                onchange: move |evt| {
                                                    feature_change(
                                                        local_features,
                                                        modify,
                                                        evt,
                                                        &feat,
                                                        modify_count,
                                                        enabled_features,
                                                    )
                                                },
                                                r#type: "checkbox"
                                            }

                                            div { "{feat.name}" }

                                            match feat.description {
                                                Some(ref desc) => rsx!(span {
                                                    class: "tooltiptext",
                                                    "{desc}",
                                                }),
                                                None => rsx!("")
                                                }
 

 
                                            }
 

 
                                        }
 

 
                                    }
 

 
                                }
 

 
                            }
 

 
                        }
                        input {
                            r#type: "submit",
                            value: if !installer_profile.installed {
                                "Install"
                            } else {
                                if !*modify.read() { "Update" } else { "Modify" }
                            },
                            class: "install-button",
                            disabled: install_disable
                        }
 

 
                    }
 

 
                }
 

 
            }
 

 
        }
 

 
    }
 

 
}
#[component]
fn Pagination(mut page: Signal<usize>, mut pages: Signal<BTreeMap<usize, TabInfo>>) -> Element {
    rsx!(
        div { class: "pagination",
            for (index , info) in pages() {
                button {
                    class: "toolbar-button",
                    disabled: index == page(),
                    onclick: move |evt| {
                        *page.write() = index;
                        evt.stop_propagation();
                    },
                    "{info.title}"
                }
            }
        }
    )
}

#[derive(Clone)]
pub(crate) struct AppProps {
    pub branches: Vec<super::GithubBranch>,
    pub modpack_source: String,
    pub config: super::Config,
    pub config_path: PathBuf,
}

pub(crate) fn app() -> Element {
    let props = use_context::<AppProps>();
    let css = include_str!("assets/style.css");
    let branches = props.branches;
    let config = use_signal(|| props.config);
    let mut settings = use_signal(|| false);
    let mut err: Signal<Option<String>> = use_signal(|| None);

    let name = use_signal(String::default);

    let page = use_signal(|| 0);
    let pages = use_signal(|| BTreeMap::<usize, TabInfo>::new());
    let css = css
        .replace(
            "<BG_COLOR>",
            match pages().get(&page()) {
                Some(x) => &x.color,
                None => "#320625",
            },
        )
        .replace(
            "<BG_IMAGE>",
            match pages().get(&page()) {
                Some(x) => {
                    if settings() {
                        &x.settings_background
                    } else {
                        &x.background
                    }
                },
                None => "https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/background_installer.png",
            },
        ).replace("<SECONDARY_FONT>", match pages().get(&page()) {
            Some(x) => &x.secondary_font,
            None => "https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/Wynncraft_Game_Font.woff2",
        }).replace("<PRIMARY_FONT>", match pages().get(&page()) {
            Some(x) => &x.primary_font,
            None => "https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/Wynncraft_Game_Font.woff2",
        });

    let cfg = config.with(|cfg| cfg.clone());
    let launcher = match super::get_launcher(&cfg.launcher) {
        Ok(val) => Some(val),
        Err(_) => None,
    };

    let mut modal_context = use_context_provider(|| ModalContext::default());
    if let Some(e) = err() {
        modal_context.open("Error", rsx! {
            p {
                "The installer encountered an error if the problem does not resolve itself please open a thread in #📂modpack-issues on the discord."
            }
            textarea { class: "error-area", readonly: true, "{e}" }
        }, false, Some(move |_| err.set(None)));
    }

    rsx! {
        style { "{css}" }

        Modal {}

        if *settings.read() {
            div { class: "toolbar" }
            div { class: "fake-body",
                Settings {
                    config,
                    settings,
                    config_path: props.config_path,
                    error: err,
                    b64_id: engine::general_purpose::URL_SAFE_NO_PAD.encode(props.modpack_source)
                }
            }
        } else if config.read().first_launch.unwrap_or(true) || launcher.is_none() {
            div { class: "fake-body",
                Launcher {
                    config,
                    config_path: props.config_path,
                    error: err,
                    b64_id: engine::general_purpose::URL_SAFE_NO_PAD.encode(props.modpack_source)
                }
            }
        } else {
            div { class: "toolbar",
                Pagination { page, pages }
                button {
                    class: "toolbar-button",
                    style: "padding: 0;margin-right: 0;",
                    onclick: move |evt| {
                        settings.set(true);
                        evt.stop_propagation();
                    },
                    img { src: "https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/cog_icon.png" }
                }
            }
            div { class: "fake-body",
                for i in 0..branches.len() {
                    Version {
                        modpack_source: props.modpack_source.clone(),
                        modpack_branch: branches[i].name.clone(),
                        launcher: launcher.as_ref().unwrap().clone(),
                        error: err,
                        name,
                        page,
                        pages
                    }
                }
            }
        }
    }
}
