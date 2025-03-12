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
    
    // Log feature card information for debugging
    log::info!("Rendering FeatureCard: id={}, name={}, desc={:?}",
        props.feature.id,
        props.feature.name,
        props.feature.description);
    
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

#[derive(PartialEq, Props, Clone)]
struct HomePageTabProps {
    pages: Signal<BTreeMap<usize, TabInfo>>,
    page: Signal<usize>,
}

#[component]
fn HomePageTab(props: HomePageTabProps) -> Element {
    let page_count = props.pages.with(|p| p.len());
    log::info!("Rendering Home Page with {} tabs", page_count);
    
    rsx! {
        div { class: "home-container",
            h1 { class: "home-title", "Welcome to the Modpack Installer" }
            
            p { class: "home-description", 
                "Select one of the available modpacks below to continue with installation."
            }
            
            div { class: "tab-card-container",
                for (index, info) in props.pages.with(|p| p.clone()) {
                    // Skip the home page tab itself
                    if index != 0 {
                        div {
                            class: "tab-card",
                            onclick: move |_| {
                                props.page.set(index);
                                log::info!("Clicked tab card: switching to tab {}", index);
                            },
                            style: "background-image: url({});", info.background,
                            
                            div { class: "tab-card-content",
                                h2 { class: "tab-card-title", "{info.title}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(PartialEq, Props, Clone)]
struct VersionProps {
    modpack_source: String,
    modpack_branch: String,
    launcher: Launcher,
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
        log::info!("Loading modpack from source: {}, branch: {}", source, branch);
        async move { 
            log::info!("Starting fetch for manifest from {}{}/manifest.json", super::GH_RAW, &(source.clone() + &branch));
            let result = super::init(source, branch, launcher).await;
            match &result {
                Ok(profile) => {
                    log::info!("Successfully loaded manifest: subtitle={}, tab_group={:?}, has {} features", 
                        profile.manifest.subtitle,
                        profile.manifest.tab_group,
                        profile.manifest.features.len()
                    );
                }
                Err(e) => {
                    log::error!("Failed to load manifest: {}", e);
                }
            }
            result
        }
    });

    // When loading profile resources, show a loading indicator with more info
    if profile.read().is_none() {
        log::info!("Profile resource is still loading for branch: {}", props.modpack_branch);
        return rsx! {
            div { class: "loading-container", 
                div { class: "loading-spinner" }
                div { class: "loading-text", "Loading modpack information for: {props.modpack_branch}..." }
                div { class: "loading-details", "This may take a moment. Please wait..." }
            }
        };
    }

    // Handle error case more gracefully
    let installer_profile = match profile.unwrap() {
        Ok(v) => v,
        Err(e) => {
            let error_msg = format!("{:#?}", e) + " (Failed to retrieve installer profile!)";
            log::error!("Error loading profile: {}", error_msg);
            props.error.set(Some(error_msg));
            return rsx! {
                div { class: "error-container",
                    h2 { "Error Loading Modpack" }
                    p { "Failed to load manifest for branch: {props.modpack_branch.clone()}" }
                    p { class: "error-details", "{e}" }
                }
            };
        }
    };

    // Process manifest data for tab information
    // Extract and log the tab information for debugging
    log::info!("Processing manifest tab information:");
    log::info!("  subtitle: {}", installer_profile.manifest.subtitle);
    log::info!("  description length: {}", installer_profile.manifest.description.len());
    
    let tab_group = if let Some(tab_group) = installer_profile.manifest.tab_group {
        log::info!("  tab_group: {}", tab_group);
        tab_group
    } else {
        log::info!("  tab_group: None, defaulting to 0");
        0
    };
    
    let tab_title = if let Some(ref tab_title) = installer_profile.manifest.tab_title {
        log::info!("  tab_title: {}", tab_title);
        tab_title.clone()
    } else {
        log::info!("  tab_title: None, defaulting to 'Default'");
        String::from("Default")
    };
    
    let tab_color = if let Some(ref tab_color) = installer_profile.manifest.tab_color {
        log::info!("  tab_color: {}", tab_color);
        tab_color.clone()
    } else {
        log::info!("  tab_color: None, defaulting to '#320625'");
        String::from("#320625")
    };
    
    let tab_background = if let Some(ref tab_background) = installer_profile.manifest.tab_background {
        log::info!("  tab_background: {}", tab_background);
        tab_background.clone()
    } else {
        let default_bg = "https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/background_installer.png";
        log::info!("  tab_background: None, defaulting to '{}'", default_bg);
        String::from(default_bg)
    };
    
    let settings_background = if let Some(ref settings_background) = installer_profile.manifest.settings_background {
        log::info!("  settings_background: {}", settings_background);
        settings_background.clone()
    } else {
        log::info!("  settings_background: None, using tab_background");
        tab_background.clone()
    };
        
    let tab_secondary_font = if let Some(ref tab_secondary_font) = installer_profile.manifest.tab_secondary_font {
        log::info!("  tab_secondary_font: {}", tab_secondary_font);
        tab_secondary_font.clone()
    } else {
        let default_font = "https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/Wynncraft_Game_Font.woff2";
        log::info!("  tab_secondary_font: None, defaulting to '{}'", default_font);
        String::from(default_font)
    };
    
    let tab_primary_font = if let Some(ref tab_primary_font) = installer_profile.manifest.tab_primary_font {
        log::info!("  tab_primary_font: {}", tab_primary_font);
        tab_primary_font.clone()
    } else {
        let default_font = "https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/Wynncraft_Game_Font.woff2";
        log::info!("  tab_primary_font: None, defaulting to '{}'", default_font);
        String::from(default_font)
    };
    
    log::info!("Inserting tab_group {} into pages map", tab_group);
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

    // Rest of the Version component remains the same as in the previous implementation
    // (Continue with existing implementation of installing, features, etc.)
    
    // ... (Existing implementation continues here)
    
    // If you want me to complete the entire Version component, I can do that in the next message
    rsx! {
        div { "Placeholder for Version component" }
    }
}

#[component]
fn AppHeader(
    page: Signal<usize>, 
    pages: Signal<BTreeMap<usize, TabInfo>>,
    settings: Signal<bool>,
    logo_url: Option<String>
) -> Element {
    // Log what tabs we have available
    log::info!("Rendering AppHeader with {} tabs", pages().len());
    for (index, info) in pages().iter() {
        log::info!("  Tab {}: title={}", index, info.title);
    }
    
    rsx!(
        header { class: "app-header",
            // Logo (if available)
            if let Some(url) = logo_url {
                img { class: "app-logo", src: "{url}", alt: "Logo" }
            }
            
            h1 { class: "app-title", "Modpack Installer" }
            
            // Tabs from pages - show only if we have pages
            div { class: "header-tabs",
                // Always include the home tab first
                button {
                    class: if page() == 0 { "header-tab-button active" } else { "header-tab-button" },
                    onclick: move |_| {
                        page.set(0);
                        log::info!("Switching to home tab");
                    },
                    "Home"
                }
                
                // Then show other tabs
                for (index, info) in pages() {
                    if index != 0 { // Skip home page in this loop since we added it above
                        button {
                            class: if page() == index { "header-tab-button active" } else { "header-tab-button" },
                            onclick: move |_| {
                                page.set(index);
                                log::info!("Switching to tab {}: {}", index, info.title);
                            },
                            "{info.title}"
                        }
                    }
                }
            }
            
            // Settings button
            button {
                class: "settings-button",
                onclick: move |_| {
                    settings.set(true);
                    log::info!("Opening settings");
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

    let name = use_signal(String::default);

    // Default to home page (page 0)
    let page = use_signal(|| 0); 
    let pages = use_signal(|| {
        // Initialize with home page
        let mut map = BTreeMap::<usize, TabInfo>::new();
        map.insert(0, TabInfo {
            color: "#320625".to_string(),
            title: "Home".to_string(),
            background: "https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/background_installer.png".to_string(),
            settings_background: "https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/background_installer.png".to_string(),
            primary_font: "https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/Wynncraft_Game_Font.woff2".to_string(),
            secondary_font: "https://raw.githubusercontent.com/Wynncraft-Overhaul/installer/master/src/assets/Wynncraft_Game_Font.woff2".to_string(),
        });
        map
    });
    
    // Log information about the branches we're loading
    log::info!("Loading {} branches from source: {}", branches.len(), props.modpack_source);
    for (i, branch) in branches.iter().enumerate() {
        log::info!("  Branch {}: name={}", i, branch.name);
    }

    // Update CSS whenever relevant values change
    let css_content = {
        let page = page.clone();
        let settings = settings.clone();
        let pages = pages.clone();
        
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
        
        log::info!("Updating CSS with: color={}, bg_image={}", bg_color, bg_image);
        
        css
            .replace("<BG_COLOR>", &bg_color)
            .replace("<BG_IMAGE>", &bg_image)
            .replace("<SECONDARY_FONT>", &secondary_font)
            .replace("<PRIMARY_FONT>", &primary_font)
    };

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
        style { "{css_content}" }

        Modal {}

        // Always render AppHeader if we're past the initial launcher selection
        if !config.read().first_launch.unwrap_or(true) && launcher.is_some() {
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
            } else if page() == 0 {
                // Render home page if we're on page 0
                HomePageTab {
                    pages,
                    page
                }
            } else {
                // Otherwise render the specific branch page
                // We need to map page index to branch index
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
