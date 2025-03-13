use std::{collections::BTreeMap, path::PathBuf};

use base64::{engine, Engine};
use dioxus::prelude::*;
use futures::StreamExt;
use log::{error, debug};
use modal::{Modal, ModalContext};

use crate::{get_app_data, get_installed_packs, get_launcher, uninstall, InstallerProfile, Launcher, PackName};

mod modal;

#[derive(Debug, Clone)]
struct TabInfo {
    color: String,
    title: String, 
    background: String,
    settings_background: String,
    primary_font: String,
    secondary_font: String,
    modpacks: Vec<InstallerProfile>,
}

// Home Page component to display all available modpacks as a grid
#[component]
fn HomePage(
    pages: Signal<BTreeMap<usize, TabInfo>>,
    page: Signal<usize>
) -> Element {
    debug!("Rendering HomePage with {} tabs", pages().len());
    
    rsx! {
        div { class: "home-container",
            h1 { class: "home-title", "Available Modpacks" }
            
            div { class: "home-grid",
                for (index, info) in pages() {
                    div { 
                        class: "home-pack-card",
                        style: "background-image: url('{info.background}'); background-color: {info.color};",
                        onclick: move |_| {
                            page.set(index);
                            debug!("Navigating to tab {}: {}", index, info.title);
                        },
                        div { class: "home-pack-info",
                            h2 { class: "home-pack-title", "{info.title}" }
                            div { class: "home-pack-button", "View Modpack" }
                        }
                    }
                }
            }
        }
    }
}
// Special value for home page
const HOME_PAGE: usize = usize::MAX;

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
        if let Some(path) = directory {
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
    on_toggle: EventHandler<FormEvent>,
}

#[component]
fn FeatureCard(props: FeatureCardProps) -> Element {
    let enabled = props.enabled;
    let feature_id = props.feature.id.clone();
    
    rsx! {
        div { 
            class: if enabled { "feature-card feature-enabled" } else { "feature-card feature-disabled" },
            h3 { class: "feature-card-title", "{props.feature.name}" }
            
            // Render description if available
            if let Some(description) = &props.feature.description {
                div { class: "feature-card-description", "{description}" }
            }
            
            // Toggle button with hidden checkbox
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

async fn init_branch(source: String, branch: String, launcher: Launcher, mut pages: Signal<BTreeMap<usize, TabInfo>>) -> Result<(), String> {
    debug!("Initializing modpack from source: {}, branch: {}", source, branch);
    let profile = crate::init(source.to_owned(), branch.to_owned(), launcher).await?;

    // Process manifest data for tab information
    // Extract and log the tab information for debugging
    debug!("Processing manifest tab information:");
    debug!("  subtitle: {}", profile.manifest.subtitle);
    debug!("  description length: {}", profile.manifest.description.len());

    let tab_group = if let Some(tab_group) = profile.manifest.tab_group {
        debug!("  tab_group: {}", tab_group);
        tab_group
    } else {
        debug!("  tab_group: None, defaulting to 0");
        0
    };

    if pages.read().get(&tab_group).is_some_and(|x| x.modpacks.contains(&profile)) {
        return Ok(());
    }

    let tab_created = pages.read().contains_key(&tab_group);
    let movable_profile = profile.clone();
    use_effect(move || {
        let profile = &movable_profile;
        if !tab_created {
            let tab_title = if let Some(ref tab_title) = profile.manifest.tab_title {
                debug!("  tab_title: {}", tab_title);
                tab_title.clone()
            } else {
                debug!("  tab_title: None, using subtitle");
                profile.manifest.subtitle.clone()
            };

            let tab_color = if let Some(ref tab_color) = profile.manifest.tab_color {
                debug!("  tab_color: {}", tab_color);
                tab_color.clone()
            } else {
                debug!("  tab_color: None, defaulting to '#320625'");
                String::from("#320625")
            };

            let tab_background = if let Some(ref tab_background) = profile.manifest.tab_background {
                debug!("  tab_background: {}", tab_background);
                tab_background.clone()
            } else {
                let default_bg = "https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/background_installer.png";
                debug!("  tab_background: None, defaulting to '{}'", default_bg);
                String::from(default_bg)
            };

            let settings_background = if let Some(ref settings_background) = profile.manifest.settings_background {
                debug!("  settings_background: {}", settings_background);
                settings_background.clone()
            } else {
                debug!("  settings_background: None, using tab_background");
                tab_background.clone()
            };

            let tab_secondary_font = if let Some(ref tab_secondary_font) = profile.manifest.tab_secondary_font {
                debug!("  tab_secondary_font: {}", tab_secondary_font);
                tab_secondary_font.clone()
            } else {
                let default_font = "https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/Wynncraft_Game_Font.woff2";
                debug!("  tab_secondary_font: None, defaulting to '{}'", default_font);
                String::from(default_font)
            };

            let tab_primary_font = if let Some(ref tab_primary_font) = profile.manifest.tab_primary_font {
                debug!("  tab_primary_font: {}", tab_primary_font);
                tab_primary_font.clone()
            } else {
                let default_font = "https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/Wynncraft_Game_Font.woff2";
                debug!("  tab_primary_font: None, defaulting to '{}'", default_font);
                String::from(default_font)
            };

            let tab_info = TabInfo {
                color: tab_color,
                title: tab_title,
                background: tab_background,
                settings_background,
                primary_font: tab_primary_font,
                secondary_font: tab_secondary_font,
                modpacks: vec![],
            };
            pages.write().insert(tab_group, tab_info.clone());
            debug!("Inserted tab_group {} into pages map", tab_group);
        }
    });

    use_effect(move || {
        let profile = profile.clone();
        pages.write().entry(tab_group).and_modify(move |x| x.modpacks.push(profile));
        debug!("Inserted {} into tab_group {}", branch, tab_group);
    });
    // pages.write().entry(tab_group).and_modify(|x| x.modpacks.push(profile));


    Ok(())
}

#[component]
fn Version(installer_profile: InstallerProfile, error: Signal<Option<String>>) -> Element {
    debug!("Rendering Version component for source: {}, branch: {}",
              installer_profile.modpack_source, installer_profile.modpack_branch);  

    let mut installing = use_signal(|| false);
    let mut progress_status = use_signal(|| "");
    let mut install_progress = use_signal(|| 0);
    let mut modify = use_signal(|| false);
    let mut modify_count = use_signal(|| 0);

    // Fix: Initialize enabled_features properly
    let enabled_features = use_signal(|| {
        let mut features = vec!["default".to_string()];
        
        if installer_profile.installed && installer_profile.local_manifest.is_some() {
            features = installer_profile.local_manifest.as_ref().unwrap().enabled_features.clone();
        } else {
            // Add default features
            for feat in &installer_profile.manifest.features {
                if feat.default {
                    features.push(feat.id.clone());
                }
            }
        }

        debug!("Initial enabled features: {:?}", features);
        features
    });

    let mut install_item_amount = use_signal(|| 0);
    let mut credits = use_signal(|| false);
    let installed = use_signal(|| installer_profile.installed);
    let mut update_available = use_signal(|| installer_profile.update_available);
    
    // Clone local_manifest to prevent ownership issues
    let mut local_features = use_signal(|| {
        if let Some(ref manifest) = installer_profile.local_manifest {
            Some(manifest.enabled_features.clone())
        } else {
            None
        }
    });
    
    let movable_profile = installer_profile.clone();
    let on_submit = move |_| {
        // Calculate total items to process for progress tracking
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
                        match crate::install(&installer_profile, move || {
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
                                error.set(Some(
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
                                error.set(Some(
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
    debug!("Version: reached RSX");
    rsx! {
        "TEST"
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
                    // Header section with title and subtitle (using manifest data)
                    div { class: "content-header",
                        h1 { "{installer_profile.manifest.subtitle}" }
                    }
                    
                    // Description section (using manifest data)
                    div { class: "content-description",
                        // The 'dangerous_inner_html' directive renders HTML content safely
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
                    
                    {
                        debug!("Rendering {} features for manifest",
                                  installer_profile.manifest.features.len());
                        for feat in &installer_profile.manifest.features {
                            debug!("Feature: id={}, name={}, hidden={}, default={}",
                                      feat.id, feat.name, feat.hidden, feat.default);
                        }
                    }
                    
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
    // Log what tabs we have available

    debug!("Rendering AppHeader with {} tabs", pages().len());
    for (index, info) in pages().iter() {
        debug!("  Tab {}: title={}", index, info.title);
    }
  
    rsx!(
        header { class: "app-header",
            // Logo (if available) serves as home button
            if let Some(url) = logo_url {
                img { 
                    class: "app-logo", 
                    src: "{url}", 
                    alt: "Logo",
                    onclick: move |_| {
                        page.set(HOME_PAGE);
                        debug!("Navigating to home page via logo");
                    },
                    style: "cursor: pointer;"
                }
            }
            
            h1 { 
                class: "app-title", 
                onclick: move |_| {
                    page.set(HOME_PAGE);
                    debug!("Navigating to home page via title");
                },
                style: "cursor: pointer;",
                "Modpack Installer" 
            }
            
            // Tabs from pages - show only if we have pages
            div { class: "header-tabs",
                // Home tab
                button {
                    class: if page() == HOME_PAGE { "header-tab-button active" } else { "header-tab-button" },
                    onclick: move |_| {
                        page.set(HOME_PAGE);
                        debug!("Navigating to home page via tab");
                    },
                    "Home"
                }

                if !pages().is_empty() {
                    for (index, info) in pages() {
                        button {
                            class: if page() == index { "header-tab-button active" } else { "header-tab-button" },
                            onclick: move |_| {
                                page.set(index);
                                debug!("Switching to tab {}: {}", index, info.title);
                            },
                            "{info.title}"
                        }
                    }
                } else {
                    // If no tabs, show a message for debugging purposes
                    span { style: "color: #888; font-style: italic;", "Loading tabs..." }

                }
            }
            
            // Settings button
            button {
                class: "settings-button",
                onclick: move |_| {
                    settings.set(true);
                    debug!("Opening settings");
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
    let branches = props.branches.clone();
    let config = use_signal(|| props.config);
    let settings = use_signal(|| false);
    let mut err: Signal<Option<String>> = use_signal(|| None);

    let page = use_signal(|| usize::MAX);
    let pages = use_signal(BTreeMap::<usize, TabInfo>::new);

    debug!("Rendering page: {}, {:?}", page(), pages().get(&page()));

    let cfg = config.with(|cfg| cfg.clone());
    let launcher = match super::get_launcher(&cfg.launcher) {
        Ok(val) => {
            debug!("Successfully loaded launcher: {}", cfg.launcher);
            Some(val)
        },
        Err(e) => {
            error!("Failed to load launcher: {} - {}", cfg.launcher, e);
            None
        },
    };

    let packs: Resource<()> = {
        let source = props.modpack_source.clone();
        let branches = branches.clone();
        let launcher = launcher.clone();
        use_resource(move || {
            let source = source.clone();
            let branches = branches.clone();
            let launcher = launcher.clone();
            debug!("Loading {} branches from source: {}", branches.len(), source);
            async move {
                let source = source.clone();
                let branches = branches.clone();
                let launcher = launcher.clone();
                // FIXME: Somewhat slow
                // FIXME: Some packs missing
                futures::stream::iter(branches.iter().map(|x| init_branch(source.clone(), x.name.clone(), launcher.clone().unwrap(), pages.clone()))).for_each_concurrent(usize::MAX, |fut| async move {fut.await.unwrap();}).await; // FIXME: handle failed branch init
            }
        })
    };

    
    // Update CSS whenever relevant values change
    let css_content = {
        let default_color = "#320625".to_string();
        let default_bg = "https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/background_installer.png".to_string();
        let default_font = "https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/Wynncraft_Game_Font.woff2".to_string();
        
        let bg_color = match pages().get(&page()) {
            Some(x) => x.color.clone(),
            None => default_color,
        };
        
        let bg_image = match pages().get(&page()) {
            Some(x) => {
                if settings() {
                    x.settings_background.clone()
                } else {
                    x.background.clone()
                }
            },
            None => default_bg,
        };
        
        let secondary_font = match pages().get(&page()) {
            Some(x) => x.secondary_font.clone(),
            None => default_font.clone(),
        };
        
        let primary_font = match pages().get(&page()) {
            Some(x) => x.primary_font.clone(),
            None => default_font,
        };
        
        debug!("Updating CSS with: color={}, bg_image={}, secondary_font={}, primary_font={}", bg_color, bg_image, secondary_font, primary_font);
        
        css
            .replace("<BG_COLOR>", &bg_color)
            .replace("<BG_IMAGE>", &bg_image)
            .replace("<SECONDARY_FONT>", &secondary_font)
            .replace("<PRIMARY_FONT>", &primary_font)
    };

    let mut modal_context = use_context_provider(ModalContext::default);
    if let Some(e) = err() {
        modal_context.open("Error", rsx! {
            p {
                "The installer encountered an error if the problem does not resolve itself please open a thread in #📂modpack-issues on the discord."
            }
            textarea { class: "error-area", readonly: true, "{e}" }
        }, false, Some(move |_| err.set(None)));
    }

    // Determine which logo to use
    let logo_url = Some("https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/icon.png".to_string());

    rsx! {
        style { "{css_content}" }

        Modal {}

        // Always render AppHeader if we're past the initial launcher selection or in settings
        if !config.read().first_launch.unwrap_or(true) && launcher.is_some() && !*settings.read() {
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
                    config_path: props.config_path.clone(),
                    error: err,
                    b64_id: engine::general_purpose::URL_SAFE_NO_PAD.encode(props.modpack_source)
                }
            } else if config.read().first_launch.unwrap_or(true) || launcher.is_none() {
                Launcher {
                    config,
                    config_path: props.config_path.clone(),
                    error: err,
                    b64_id: engine::general_purpose::URL_SAFE_NO_PAD.encode(props.modpack_source)
                }
            } else {
                if packs.read().is_none() {
                    div { class: "loading-container",
                        div { class: "loading-spinner" }
                        div { class: "loading-text", "Loading modpack information..." }
                    }
                } else if page() == HOME_PAGE {
                    HomePage {
                        pages,
                        page
                    }
                } else {
                    for installer_profile in pages().get(&page()).cloned().unwrap().modpacks {
                        // "{installer_profile.manifest.subtitle}"
                        Version { // FIXME: does not show
                            installer_profile,
                            error: err.clone(),
                        }
                    }
                }

            }
        }
    }
}
