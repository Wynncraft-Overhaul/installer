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
        div { class: "progress-container",
            div { class: "progress-header",
                h1 { "{title}" }
            }
            div { class: "progress-content",
                progress { class: "progress-bar", max, value: "{value}" }
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
        div { class: "credits-container",
            div { class: "credits-header",
                h1 { "{props.manifest.subtitle}" }
                button {
                    class: "close-button",
                    onclick: move |evt| {
                        props.credits.set(false);
                        evt.stop_propagation();
                    },
                    "Close"
                }
            }
            div { class: "credits-content",
                div { class: "credits-list",
                    ul {
                        for r#mod in props.manifest.mods {
                            if props.enabled.contains(&r#mod.id) {
                                li { class: "credit-item",
                                    div { class: "credit-name", "{r#mod.name}" }
                                    div { class: "credit-authors",
                                        "by "
                                        for author in &r#mod.authors {
                                            a { href: "{author.link}", class: "credit-author",
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
                        }
                        // Similar blocks for shaderpacks, resourcepacks, include - keeping structure same
                        for shaderpack in props.manifest.shaderpacks {
                            if props.enabled.contains(&shaderpack.id) {
                                li { class: "credit-item",
                                    div { class: "credit-name", "{shaderpack.name}" }
                                    div { class: "credit-authors",
                                        "by "
                                        for author in &shaderpack.authors {
                                            a { href: "{author.link}", class: "credit-author",
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
                        }
                        for resourcepack in props.manifest.resourcepacks {
                            if props.enabled.contains(&resourcepack.id) {
                                li { class: "credit-item",
                                    div { class: "credit-name", "{resourcepack.name}" }
                                    div { class: "credit-authors",
                                        "by "
                                        for author in &resourcepack.authors {
                                            a { href: "{author.link}", class: "credit-author",
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
                        }
                        for include in props.manifest.include {
                            if props.enabled.contains(&include.id) && include.authors.is_some() 
                               && include.name.is_some() {
                                li { class: "credit-item",
                                    div { class: "credit-name", "{include.name.as_ref().unwrap()}" }
                                    div { class: "credit-authors",
                                        "by "
                                        for author in &include.authors.as_ref().unwrap() {
                                            a { href: "{author.link}", class: "credit-author",
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

#[component]
fn PackUninstallButton(launcher: Launcher, pack: PackName) -> Element {
    let mut hidden = use_signal(|| false);
    rsx!(
        li { hidden,
            button {
                class: "uninstall-list-item",
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
        div { class: "settings-container",
            h1 { class: "settings-title", "Settings" }
            form {
                id: "settings",
                class: "settings-form",
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
                
                div { class: "setting-group",
                    label { class: "setting-label", "Launcher:" }
                    select {
                        name: "launcher-select",
                        id: "launcher-select",
                        form: "settings",
                        class: "setting-select",
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
                
                div { class: "settings-buttons",
                    input {
                        r#type: "submit",
                        value: "Save",
                        class: "primary-button",
                        id: "save"
                    }
                    
                    button {
                        class: "secondary-button",
                        r#type: "button",
                        disabled: packs.is_empty(),
                        onclick: move |evt| {
                            let mut modal = use_context::<ModalContext>();
                            modal
                                .open(
                                    "Select modpack to uninstall",
                                    rsx! {
                                        div { class: "uninstall-list-container",
                                            ul { class: "uninstall-list",
                                                for pack in packs.clone() {
                                                    PackUninstallButton { launcher: launcher.clone(), pack }
                                                }
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
            div { class: "launcher-container",
                h1 { class: "launcher-title", "Select Launcher" }
                form {
                    id: "launcher-form",
                    class: "launcher-form",
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
                    
                    div { class: "setting-group",
                        label { class: "setting-label", "Launcher:" }
                        select {
                            name: "launcher-select",
                            id: "launcher-select",
                            form: "launcher-form",
                            class: "setting-select",
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
                        class: "primary-button",
                        id: "continue-button"
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
            class: "secondary-button custom-multimc-button",
            onclick: custom_multimc,
            r#type: "button",
            "Use custom MultiMC directory"
        }
    )
}

#[component]
fn NoLauncherFound(props: LauncherProps) -> Element {
    rsx! {
        div { class: "no-launcher-container",
            h1 { class: "no-launcher-title", "No supported launcher found!" }
            div { class: "no-launcher-message",
                p {
                    "Only Prism Launcher, MultiMC and the vanilla launcher are supported by default, other MultiMC launchers can be added using the button below."
                }
                p {
                    "If you have any of these installed then please make sure you are on the latest version of the installer, if you are, open a thread in #📂modpack-issues on the discord. Please make sure your thread contains the following information: Launcher your having issues with, directory of the launcher and your OS."
                }
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

// Feature Card component to display features in card format
#[derive(PartialEq, Props, Clone)]
struct FeatureCardProps {
    feature: super::Feature,
    enabled: bool,
    on_toggle: EventHandler<Event<FormData>>,
}

#[component]
fn FeatureCard(props: FeatureCardProps) -> Element {
    let enabled = props.enabled;
    let feature_id = props.feature.id.clone();
    
    rsx! {
        div { 
            class: if enabled { "feature-card feature-enabled" } else { "feature-card feature-disabled" },
            h3 { class: "feature-card-title", "{props.feature.name}" }
            
            if let Some(description) = &props.feature.description {
                div { class: "feature-card-description", "{description}" }
            }
            
            label {
                class: if enabled { "feature-toggle-button enabled" } else { "feature-toggle-button disabled" },
                input {
                    r#type: "checkbox",
                    name: "{feature_id}",
                    checked: if enabled { Some("true") } else { None },
                    onchange: move |evt| props.on_toggle.call(evt),
                    style: "display: none;"
                }
                if enabled { "Enabled" } else { "Disabled" }
            }
        }
    }
}

fn feature_change(
    local_features: Signal<Option<Vec<String>>>,
    mut modify: Signal<bool>,
    evt: Event<FormData>,
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

    // When loading profile resources, show a loading indicator
    if profile.read().is_none() {
        return rsx! {
            div { class: "loading-container", 
                div { class: "loading-spinner" }
                div { class: "loading-text", "Loading modpack information..." }
            }
        };
    }

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
    
    let tab_background = if let Some(ref tab_background) = installer_profile.manifest.tab_background {
        tab_background.clone()
    } else {
        String::from("https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/background_installer.png")
    };
    
    let settings_background = if let Some(ref settings_background) = installer_profile.manifest.settings_background {
        settings_background.clone()
    } else {
        tab_background.clone()
    };
        
    let tab_secondary_font = if let Some(ref tab_secondary_font) = installer_profile.manifest.tab_secondary_font {
        tab_secondary_font.clone()
    } else {
        String::from("https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/Wynncraft_Game_Font.woff2")
    };
    
    let tab_primary_font = if let Some(ref tab_primary_font) = installer_profile.manifest.tab_primary_font {
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
                        dangerous_inner_html: "{contents}",
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
    
    // Now add the stylesheet in a separate style tag to avoid issues
    rsx! {
        style { "
            
            .feature-card {{
                background-color: rgba(255, 255, 255, 0.9);
                border-radius: 12px;
                padding: 16px;
                box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
                transition: all 0.3s ease;
                display: flex;
                flex-direction: column;
                height: 100%;
            }}
            
            .feature-disabled {{
                opacity: 0.7;
            }}
            
            .feature-card-title {{
                margin-top: 0;
                margin-bottom: 8px;
                font-size: 18px;
                font-weight: 600;
                color: #333;
            }}
            
            .feature-card-description {{
                flex-grow: 1;
                margin-bottom: 16px;
                font-size: 14px;
                color: #555;
                line-height: 1.4;
            }}
            
            .feature-toggle-button {{
                border: none;
                border-radius: 20px;
                padding: 8px 16px;
                font-weight: bold;
                cursor: pointer;
                transition: background-color 0.3s ease;
                width: 100%;
                display: block;
                text-align: center;
            }}
            
            .feature-toggle-button.enabled {{
                background-color: #4caf50;
                color: white;
            }}
            
            .feature-toggle-button.disabled {{
                background-color: #f44336;
                color: white;
            }}
            
            /* Feature Cards Grid */
            .feature-cards-container {{
                display: grid;
                grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
                gap: 16px;
                width: 100%;
                padding: 16px;
                max-height: 500px;
                overflow-y: auto;
                margin-bottom: 24px;
            }}
            
            /* Main Layout */
            .app-header {{
                display: flex;
                align-items: center;
                padding: 16px 24px;
                background-color: rgba(51, 51, 51, 0.9);
                border-bottom: 1px solid rgba(255, 255, 255, 0.1);
                box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
            }}
            
            .app-logo {{
                height: 40px;
                margin-right: 16px;
            }}
            
            .app-title {{
                color: white;
                margin: 0;
                flex-grow: 1;
                font-size: 20px;
            }}
            
            .header-tabs {{
                display: flex;
                gap: 8px;
                margin-right: 16px;
            }}
            
            .header-tab-button {{
                background-color: transparent;
                color: white;
                border: none;
                padding: 8px 16px;
                border-radius: 20px;
                cursor: pointer;
                transition: background-color 0.3s ease;
            }}
            
            .header-tab-button:hover {{
                background-color: rgba(255, 255, 255, 0.1);
            }}
            
            .header-tab-button.active {{
                background-color: #4a90e2;
            }}
            
            .settings-button {{
                background-color: transparent;
                color: white;
                border: none;
                padding: 8px 16px;
                border-radius: 20px;
                cursor: pointer;
                font-weight: 500;
            }}
            
            .settings-button:hover {{
                background-color: rgba(255, 255, 255, 0.1);
            }}
            
            /* Container styles */
            .main-container {{
                max-width: 1200px;
                margin: 0 auto;
                padding: 24px;
            }}
            
            .version-container {{
                background-color: rgba(255, 255, 255, 0.8);
                border-radius: 12px;
                padding: 24px;
                box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
            }}
            
            .content-header {{
                margin-bottom: 24px;
                border-bottom: 1px solid #eee;
                padding-bottom: 16px;
            }}
            
            .content-header h1 {{
                margin-top: 0;
                color: #333;
            }}
            
            .content-description {{
                margin-bottom: 24px;
                line-height: 1.6;
                color: #444;
            }}
            
            .credits-link {{
                display: inline-block;
                color: #4a90e2;
                margin-bottom: 16px;
                font-weight: 500;
                cursor: pointer;
                text-decoration: underline;
            }}
            
            .install-button-container {{
                display: flex;
                justify-content: center;
                margin-top: 24px;
            }}
            
            .main-install-button {{
                background-color: #4a90e2;
                color: white;
                border: none;
                border-radius: 24px;
                padding: 12px 48px;
                font-size: 18px;
                font-weight: bold;
                cursor: pointer;
                transition: background-color 0.3s ease, transform 0.2s ease;
                box-shadow: 0 4px 8px rgba(0, 0, 0, 0.2);
            }}
            
            .main-install-button:hover {{
                background-color: #3a7bc8;
                transform: translateY(-2px);
            }}
            
            .main-install-button:disabled {{
                background-color: #9e9e9e;
                cursor: not-allowed;
                transform: none;
                box-shadow: none;
            }}
            
            /* Progress View */
            .progress-container {{
                background-color: rgba(255, 255, 255, 0.8);
                border-radius: 12px;
                padding: 24px;
                box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
                text-align: center;
            }}
            
            .progress-header h1 {{
                margin-top: 0;
                color: #333;
            }}
            
            .progress-bar {{
                width: 100%;
                height: 24px;
                border-radius: 12px;
                margin-bottom: 16px;
            }}
            
            .progress-status {{
                font-size: 18px;
                color: #555;
            }}
            
            /* Settings */
            .settings-container, 
            .launcher-container,
            .no-launcher-container {{
                background-color: rgba(255, 255, 255, 0.8);
                border-radius: 12px;
                padding: 24px;
                box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
                max-width: 600px;
                margin: 0 auto;
            }}
            
            .settings-title,
            .launcher-title,
            .no-launcher-title {{
                margin-top: 0;
                margin-bottom: 24px;
                color: #333;
                text-align: center;
            }}
            
            .settings-form,
            .launcher-form {{
                display: flex;
                flex-direction: column;
                gap: 16px;
            }}
            
            .setting-group {{
                display: flex;
                flex-direction: column;
                gap: 8px;
            }}
            
            .setting-label {{
                font-weight: 500;
                color: #555;
            }}
            
            .setting-select {{
                padding: 10px;
                border-radius: 8px;
                border: 1px solid #ccc;
                background-color: white;
            }}
            
            .settings-buttons {{
                display: flex;
                gap: 16px;
                margin-top: 16px;
            }}
            
            .primary-button,
            .secondary-button {{
                padding: 10px 20px;
                border-radius: 8px;
                border: none;
                font-weight: 500;
                cursor: pointer;
                transition: background-color 0.3s ease;
            }}
            
            .primary-button {{
                background-color: #4a90e2;
                color: white;
                flex: 1;
            }}
            
            .secondary-button {{
                background-color: #f0f0f0;
                color: #333;
                flex: 1;
            }}
            
            .custom-multimc-button {{
                margin-bottom: 8px;
            }}
            
            /* Credits */
            .credits-container {{
                background-color: rgba(255, 255, 255, 0.8);
                border-radius: 12px;
                padding: 24px;
                box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
            }}
            
            .credits-header {{
                display: flex;
                justify-content: space-between;
                align-items: center;
                margin-bottom: 24px;
                border-bottom: 1px solid #eee;
                padding-bottom: 16px;
            }}
            
            .credits-header h1 {{
                margin: 0;
                color: #333;
            }}
            
            .close-button {{
                background-color: #f0f0f0;
                border: none;
                border-radius: 8px;
                padding: 8px 16px;
                cursor: pointer;
            }}
            
            .credits-list {{
                max-height: 500px;
                overflow-y: auto;
            }}
            
            .credit-item {{
                margin-bottom: 16px;
                padding-bottom: 16px;
                border-bottom: 1px solid #eee;
                list-style-type: none;
            }}
            
            .credit-name {{
                font-weight: 600;
                margin-bottom: 4px;
            }}
            
            .credit-author {{
                color: #4a90e2;
                text-decoration: none;
            }}
            
            .credit-author:hover {{
                text-decoration: underline;
            }}
            
            /* Uninstall list */
            .uninstall-list-container {{
                max-height: 400px;
                overflow-y: auto;
            }}
            
            .uninstall-list {{
                list-style-type: none;
                padding: 0;
                margin: 0;
            }}
            
            .uninstall-list-item {{
                padding: 10px 16px;
                margin-bottom: 8px;
                background-color: #f0f0f0;
                border: none;
                border-radius: 8px;
                width: 100%;
                text-align: left;
                cursor: pointer;
            }}
            
            .uninstall-list-item:hover {{
                background-color: #e0e0e0;
            }}
            
            /* Loading */
            .loading-container {{
                display: flex;
                flex-direction: column;
                justify-content: center;
                align-items: center;
                height: 300px;
                background-color: rgba(255, 255, 255, 0.8);
                border-radius: 12px;
                box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
                padding: 24px;
                margin: 0 auto;
                max-width: 600px;
            }}
            
            .loading-text {{
                font-size: 18px;
                color: #333;
                margin-top: 20px;
            }}
            
            .loading-spinner {{
                border: 4px solid rgba(0, 0, 0, 0.1);
                border-left-color: #4a90e2;
                border-radius: 50%;
                width: 40px;
                height: 40px;
                animation: spin 1s linear infinite;
            }}
            
            @keyframes spin {{
                0% {{ transform: rotate(0deg); }}
                100% {{ transform: rotate(360deg); }}
            }}
        " }

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
                    // Header section with title and subtitle
                    div { class: "content-header",
                        h1 { "{installer_profile.manifest.subtitle}" }
                    }
                    
                    // Description section
                    div { class: "content-description",
                        dangerous_inner_html: "{installer_profile.manifest.description}",
                        
                        // Credits link
                        div {
                            a {
                                class: "credits-link",
                                onclick: move |evt| {
                                    credits.set(true);
                                    evt.stop_propagation();
                                },
                                "View Credits"
                            }
                        }
                    }
                    
                    // Features heading
                    h2 { "Optional Features" }
                    
                    // Feature cards in a responsive grid
                    div { class: "feature-cards-container",
                        for feat in installer_profile.manifest.features {
                            if !feat.hidden {
                                FeatureCard {
                                    feature: feat.clone(),
                                    enabled: if installer_profile.installed {
                                        enabled_features.with(|x| x.contains(&feat.id))
                                    } else {
                                        feat.default
                                    },
                                    on_toggle: move |evt| {
                                        feature_change(
                                            local_features,
                                            modify,
                                            evt,
                                            &feat,
                                            modify_count,
                                            enabled_features,
                                        )
                                    }
                                }
                            }
                        }
                    }
                    
                    // Install/Update/Modify button at the bottom
                    div { class: "install-button-container",
                        button {
                            r#type: "submit",
                            class: "main-install-button",
                            disabled: install_disable,
                            if !installer_profile.installed {
                                "Install"
                            } else {
                                if !*modify.read() { "Update" } else { "Modify" }
                            }
                        }
                    }
                }
            }
        }
    }
}

// New header component with tabs
#[component]
fn AppHeader(
    page: Signal<usize>, 
    pages: Signal<BTreeMap<usize, TabInfo>>,
    settings: Signal<bool>,
    logo_url: Option<String>
) -> Element {
    rsx!(
        header { class: "app-header",
            // Logo (if available)
            if let Some(url) = logo_url {
                img { class: "app-logo", src: "{url}", alt: "Logo" }
            }
            
            h1 { class: "app-title", "Modpack Installer" }
            
            // Tabs from pages
            div { class: "header-tabs",
                for (index, info) in pages() {
                    button {
                        class: if page() == index { "header-tab-button active" } else { "header-tab-button" },
                        onclick: move |_| {
                            page.set(index);
                        },
                        "{info.title}"
                    }
                }
            }
            
            // Settings button
            button {
                class: "settings-button",
                onclick: move |_| {
                    settings.set(true);
                },
                "Settings"
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
    let settings = use_signal(|| false);
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

    // Determine which logo to use - could be made configurable via manifest
    let logo_url = Some("https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/logo.png".to_string());

    rsx! {
        style { "{css}" }

        Modal {}

        // Restructured layout with header always visible
        if !config.read().first_launch.unwrap_or(true) && launcher.is_some() && !settings() {
            AppHeader {
                page,
                pages,
                settings,
                logo_url
            }
        }

        div { class: "main-container",
            if settings() {
                Settings {
                    config,
                    settings,
                    config_path: props.config_path,
                    error: err,
                    b64_id: engine::general_purpose::URL_SAFE_NO_PAD.encode(props.modpack_source)
                }
            } else if config.read().first_launch.unwrap_or(true) || launcher.is_none() {
                Launcher {
                    config,
                    config_path: props.config_path,
                    error: err,
                    b64_id: engine::general_purpose::URL_SAFE_NO_PAD.encode(props.modpack_source)
                }
            } else {
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
