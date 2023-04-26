#![cfg_attr(
    all(target_os = "windows", not(debug_assertions),),
    windows_subsystem = "windows"
)]
use async_trait::async_trait;
use base64::{engine, Engine};
use cached::proc_macro::cached;
use cached::SizedCache;
use chrono::{DateTime, Utc};
use clap::Parser;
use dioxus_desktop::tao::window::Icon;
use dioxus_desktop::{Config as DioxusConfig, LogicalSize, WindowBuilder};
use futures::StreamExt;
use image::io::Reader as ImageReader;
use image::{DynamicImage, ImageOutputFormat};
use isahc::config::RedirectPolicy;
use isahc::prelude::Configurable;
use isahc::{AsyncBody, AsyncReadResponseExt, HttpClient, ReadResponseExt, Response};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{
    env, fs,
    io::Cursor,
    path::{Path, PathBuf},
    time::SystemTime,
};

mod gui;

const CURRENT_MANIFEST_VERSION: i32 = 3;
const GH_API: &str = "https://api.github.com/repos/";
const GH_RAW: &str = "https://raw.githubusercontent.com/";
const CONCURRENCY: usize = 14;

fn default_id() -> String {
    String::from("default")
}
macro_rules! add_headers {
    ($items:expr, $($headers:expr),*) => {
        $items.$(header($headers.next().unwrap().0, $headers.next().unwrap().1))*
    };
}

struct CachedResponse {
    resp: Response<AsyncBody>,
    bytes: Vec<u8>,
}

impl Clone for CachedResponse {
    fn clone(&self) -> Self {
        let builder = Response::builder()
            .status(self.resp.status())
            .version(self.resp.version());
        let builder = add_headers!(builder, self.resp.headers().into_iter());

        Self {
            resp: builder.body(AsyncBody::from(self.bytes.clone())).unwrap(),
            bytes: self.bytes.clone(),
        }
    }
}

#[derive(Debug, Clone)]
struct CachedHttpClient {
    http_client: HttpClient,
}

impl CachedHttpClient {
    fn new() -> CachedHttpClient {
        CachedHttpClient {
            http_client: build_http_client(),
        }
    }

    async fn get_async<T: Into<String>>(
        &self,
        url: T,
    ) -> Result<Response<AsyncBody>, isahc::Error> {
        let resp = get_cached(&self.http_client, url.into()).await;
        match resp {
            Ok(val) => Ok(val.resp),
            Err(val) => Err(val),
        }
    }
}

#[cached(
    type = "SizedCache<String, Result<CachedResponse, isahc::Error>>",
    create = "{ SizedCache::with_size(100) }",
    convert = r#"{ format!("{}", url) }"#
)]
async fn get_cached(http_client: &HttpClient, url: String) -> Result<CachedResponse, isahc::Error> {
    let resp = http_client.get_async(url).await;
    match resp {
        Ok(mut val) => {
            let bytes = val.bytes().await.unwrap();
            // CachedResponse needs to be cloned in order to init the AsyncBody otherwise the cache will not return anything on first call
            Ok(CachedResponse { resp: val, bytes }.clone())
        }
        Err(err) => Err(err),
    }
}

fn build_http_client() -> HttpClient {
    HttpClient::builder()
        .redirect_policy(RedirectPolicy::Limit(5))
        .default_headers(&[(
            "User-Agent",
            "wynncraft-overhaul/installer/0.1.0 (commander#4392)",
        )])
        .build()
        .unwrap()
}

#[async_trait]
trait Downloadable {
    async fn download(
        &self,
        modpack_root: &Path,
        loader_type: &str,
        http_client: &CachedHttpClient,
    ) -> PathBuf;
}
trait DownloadableGetters {
    fn get_name(&self) -> &String;
    fn get_location(&self) -> &String;
    fn get_version(&self) -> &String;
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
struct Config {
    launcher: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
struct Author {
    name: String,
    link: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
struct Mod {
    name: String,
    source: String,
    location: String,
    version: String,
    path: Option<PathBuf>,
    #[serde(default = "default_id")]
    id: String,
    authors: Vec<Author>,
}
impl DownloadableGetters for Mod {
    fn get_name(&self) -> &String {
        &self.name
    }
    fn get_location(&self) -> &String {
        &self.location
    }
    fn get_version(&self) -> &String {
        &self.version
    }
}
#[async_trait]
impl Downloadable for Mod {
    async fn download(
        &self,
        modpack_root: &Path,
        loader_type: &str,
        http_client: &CachedHttpClient,
    ) -> PathBuf {
        match self.source.as_str() {
            "modrinth" => {
                download_from_modrinth(self, modpack_root, loader_type, "mod", http_client).await
            }
            "ddl" => download_from_ddl(self, modpack_root, "mod", http_client).await,
            _ => panic!("Unsupported source '{}'!", self.source.as_str()),
        }
    }
}
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
struct Shaderpack {
    name: String,
    source: String,
    location: String,
    version: String,
    path: Option<PathBuf>,
    #[serde(default = "default_id")]
    id: String,
    authors: Vec<Author>,
}
impl DownloadableGetters for Shaderpack {
    fn get_name(&self) -> &String {
        &self.name
    }
    fn get_location(&self) -> &String {
        &self.location
    }
    fn get_version(&self) -> &String {
        &self.version
    }
}
#[async_trait]
impl Downloadable for Shaderpack {
    async fn download(
        &self,
        modpack_root: &Path,
        loader_type: &str,
        http_client: &CachedHttpClient,
    ) -> PathBuf {
        match self.source.as_str() {
            "modrinth" => {
                download_from_modrinth(self, modpack_root, loader_type, "shaderpack", http_client)
                    .await
            }
            "ddl" => download_from_ddl(self, modpack_root, "shaderpack", http_client).await,
            _ => panic!("Unsupported source '{}'!", self.source.as_str()),
        }
    }
}
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
struct Resourcepack {
    name: String,
    source: String,
    location: String,
    version: String,
    path: Option<PathBuf>,
    #[serde(default = "default_id")]
    id: String,
    authors: Vec<Author>,
}
impl DownloadableGetters for Resourcepack {
    fn get_name(&self) -> &String {
        &self.name
    }
    fn get_location(&self) -> &String {
        &self.location
    }
    fn get_version(&self) -> &String {
        &self.version
    }
}
#[async_trait]
impl Downloadable for Resourcepack {
    async fn download(
        &self,
        modpack_root: &Path,
        loader_type: &str,
        http_client: &CachedHttpClient,
    ) -> PathBuf {
        match self.source.as_str() {
            "modrinth" => {
                download_from_modrinth(self, modpack_root, loader_type, "resourcepack", http_client)
                    .await
            }
            "ddl" => download_from_ddl(self, modpack_root, "resourcepack", http_client).await,
            _ => panic!("Unsupported source '{}'!", self.source.as_str()),
        }
    }
}
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
struct Loader {
    r#type: String,
    version: String,
    minecraft_version: String,
}
#[async_trait]
impl Downloadable for Loader {
    async fn download(&self, root: &Path, _: &str, http_client: &CachedHttpClient) -> PathBuf {
        match self.r#type.as_str() {
            "fabric" => download_fabric(self, root, http_client).await,
            _ => panic!("Unsupported loader '{}'!", self.r#type.as_str()),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
struct Feature {
    id: String,
    name: String,
    default: bool,
}
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
struct Include {
    location: String,
    #[serde(default = "default_id")]
    id: String,
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
struct Manifest {
    manifest_version: i32,
    modpack_version: String,
    name: String,
    subtitle: String,
    description: String,
    icon: bool,
    uuid: String,
    loader: Loader,
    mods: Vec<Mod>,
    shaderpacks: Vec<Shaderpack>,
    resourcepacks: Vec<Resourcepack>,
    include: Vec<Include>,
    features: Vec<Feature>,
}
#[allow(non_snake_case)]
#[derive(Debug, Deserialize, Serialize)]
struct LauncherProfile {
    lastUsed: String,
    lastVersionId: String,
    created: String,
    name: String,
    icon: String,
    r#type: String,
    gameDir: Option<String>,
    javaDir: Option<String>,
    javaArgs: Option<String>,
    logConfig: Option<String>,
    logConfigIsXML: Option<bool>,
    resolution: Option<HashMap<String, i32>>,
}
#[allow(non_snake_case)]
#[derive(Debug, Deserialize, Serialize)]
struct LauncherProfilesSettings {
    enableAnalytics: bool,
    enableAdvanced: bool,
    keepLauncherOpen: bool,
    soundOn: bool,
    showMenu: bool,
    enableSnapshots: bool,
    enableHistorical: bool,
    enableReleases: bool,
    profileSorting: String,
    showGameLog: bool,
    crashAssistance: bool,
}
#[derive(Debug, Deserialize, Serialize)]
struct LauncherProfiles {
    settings: LauncherProfilesSettings,
    profiles: HashMap<String, LauncherProfile>,
    version: i32,
}
#[derive(Debug, Deserialize, Serialize)]
struct ModrinthFile {
    url: String,
    filename: String,
}
#[derive(Debug, Deserialize, Serialize)]
struct ModrinthObject {
    version_number: String,
    files: Vec<ModrinthFile>,
    loaders: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct GithubRepo {
    // Theres a lot more fields but we only care about default_branch
    // https://docs.github.com/en/rest/repos/repos?apiVersion=2022-11-28#get-a-repository
    default_branch: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct GithubRelease {
    tag_name: String,
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct GithubBranch {
    name: String,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, Serialize)]
struct MMCComponent {
    #[serde(skip_serializing_if = "Option::is_none")]
    cachedVolatile: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dependencyOnly: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    important: Option<bool>,
    uid: String,
    version: String,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, Serialize)]
struct MMCPack {
    components: Vec<MMCComponent>,
    formatVersion: i32,
}

async fn download_fabric(loader: &Loader, root: &Path, http_client: &CachedHttpClient) -> PathBuf {
    // TODO(download into .minecraft not modpack root)
    let url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json",
        loader.minecraft_version, loader.version
    );
    let loader_name = format!(
        "fabric-loader-{}-{}",
        &loader.version, &loader.minecraft_version
    );
    let fabric_path = root.join(Path::new(&format!("versions/{}", &loader_name)));
    if fabric_path
        .join(Path::new(&format!("{}.json", &loader_name)))
        .exists()
    {
        return PathBuf::new();
    }
    let resp = http_client
        .get_async(url.as_str())
        .await
        .expect("Failed to download fabric loader!")
        .text()
        .await
        .unwrap();
    fs::create_dir_all(&fabric_path).expect("Failed to create fabric directory");
    fs::write(
        fabric_path.join(Path::new(&format!("{}.json", &loader_name))),
        resp,
    )
    .expect("Failed to write fabric json");
    fs::write(
        fabric_path.join(Path::new(&format!("{}.jar", &loader_name))),
        "",
    )
    .expect("Failed to write fabric dummy jar");
    fabric_path
}

async fn download_from_ddl<T: Downloadable + DownloadableGetters>(
    item: &T,
    modpack_root: &Path,
    r#type: &str,
    http_client: &CachedHttpClient,
) -> PathBuf {
    let content = http_client
        .get_async(item.get_location())
        .await
        .expect(&format!("Failed to download '{}'!", item.get_name()))
        .bytes()
        .await
        .unwrap();
    let dist = match r#type {
        "mod" => modpack_root.join(Path::new("mods")),
        "resourcepack" => modpack_root.join(Path::new("resourcepacks")),
        "shaderpack" => modpack_root.join(Path::new("shaderpacks")),
        _ => panic!("Unsupported 'ModrinthCompatible' item '{}'???", r#type),
    };
    fs::create_dir_all(&dist).expect(&format!(
        "Failed to create '{}' directory",
        &dist.to_str().unwrap()
    ));
    let final_dist = dist.join(Path::new(item.get_location().split('/').last().expect(
        &format!(
            "Could not determine file name for ddl: '{}'!",
            item.get_location()
        ),
    )));
    fs::write(&final_dist, content).expect("Failed to write ddl item!");
    final_dist
}

async fn download_from_modrinth<T: Downloadable + DownloadableGetters>(
    item: &T,
    modpack_root: &Path,
    loader_type: &str,
    r#type: &str,
    http_client: &CachedHttpClient,
) -> PathBuf {
    let resp = http_client
        .get_async(format!(
            "https://api.modrinth.com/v2/project/{}/version",
            item.get_location()
        ))
        .await
        .expect(&format!("Failed to download '{}'!", item.get_name()))
        .text()
        .await
        .unwrap();
    let resp_obj: Vec<ModrinthObject> =
        serde_json::from_str(&resp).expect("Failed to parse modrinth response!");
    let dist = match r#type {
        "mod" => modpack_root.join(Path::new("mods")),
        "resourcepack" => modpack_root.join(Path::new("resourcepacks")),
        "shaderpack" => modpack_root.join(Path::new("shaderpacks")),
        _ => panic!("Unsupported 'ModrinthCompatible' item '{}'???", r#type),
    };
    fs::create_dir_all(&dist).expect(&format!(
        "Failed to create '{}' directory",
        &dist.to_str().unwrap()
    ));
    for _mod in resp_obj {
        if &_mod.version_number == item.get_version()
            && (_mod.loaders.contains(&String::from("minecraft"))
                || _mod.loaders.contains(&String::from(loader_type))
                || r#type == "shaderpack")
        {
            let content = http_client
                .get_async(&_mod.files[0].url)
                .await
                .expect(&format!("Failed to download '{}'!", item.get_name()))
                .bytes()
                .await
                .unwrap();
            let final_dist = dist.join(Path::new(&_mod.files[0].filename));
            fs::write(&final_dist, content).expect("Failed to write modrinth item!");
            return final_dist;
        }
    }
    panic!("No items returned from modrinth!")
}

fn get_app_data() -> PathBuf {
    if env::consts::OS == "linux" {
        dirs::home_dir().unwrap()
    } else if env::consts::OS == "windows" || env::consts::OS == "macos" {
        dirs::config_dir().unwrap()
    } else {
        panic!("Unsupported os '{}'!", env::consts::OS)
    }
}

fn get_multimc_folder(multimc: &str) -> Result<PathBuf, std::io::Error> {
    let path = get_app_data().join(multimc);
    match path.metadata() {
        Ok(metadata) => {
            if metadata.is_dir() {
                Ok(path)
            } else {
                panic!("MultiMC directory is not a directort!");
            }
        }
        Err(metadata) => Err(metadata),
    }
}

fn get_minecraft_folder() -> PathBuf {
    if env::consts::OS == "macos" {
        get_app_data().join("minecraft")
    } else {
        get_app_data().join(".minecraft")
    }
}

fn get_modpack_root(launcher: &Launcher, uuid: &String) -> PathBuf {
    match launcher {
        Launcher::Vanilla(root) => {
            let root = root.join(Path::new(&format!(".WC_OVHL/{}", uuid)));
            fs::create_dir_all(&root).expect("Failed to create modpack folder");
            root
        }
        Launcher::MultiMC(root) => {
            let root = root.join(Path::new(&format!("instances/{}/.minecraft", uuid)));
            fs::create_dir_all(&root).expect("Failed to create modpack folder");
            root
        }
    }
}

fn image_to_base64(img: &DynamicImage) -> String {
    let mut image_data: Vec<u8> = Vec::new();
    img.write_to(&mut Cursor::new(&mut image_data), ImageOutputFormat::Png)
        .unwrap();
    let res_base64 = engine::general_purpose::STANDARD.encode(image_data);
    format!("data:image/png;base64,{}", res_base64)
}

fn create_launcher_profile(installer_profile: &InstallerProfile, icon_img: Option<DynamicImage>) {
    let now = SystemTime::now();
    let now: DateTime<Utc> = now.into();
    let now = now.to_rfc3339();
    let manifest = &installer_profile.manifest;
    let modpack_root = get_modpack_root(
        installer_profile
            .launcher
            .as_ref()
            .expect("No launcher selected!"),
        &manifest.uuid,
    );
    // TODO(make this work on loaders other than fabric)
    match installer_profile
        .launcher
        .as_ref()
        .expect("Asked to create launcher profile without knowing launcher!")
    {
        Launcher::Vanilla(root) => {
            let icon = if manifest.icon {
                image_to_base64(
                    icon_img
                        .as_ref()
                        .expect("manifest.icon was true but no icon was supplied!"),
                )
            } else {
                String::from("Furnace")
            };
            let profile = LauncherProfile {
                lastUsed: now.to_string(),
                lastVersionId: format!(
                    "fabric-loader-{}-{}",
                    &manifest.loader.version, &manifest.loader.minecraft_version
                ),
                created: now,
                name: manifest.name.clone(),
                icon,
                r#type: String::from("custom"),
                gameDir: Some(modpack_root.to_str().unwrap().to_string()),
                javaDir: None,
                javaArgs: None,
                logConfig: None,
                logConfigIsXML: None,
                resolution: None,
            };
            let lp_file_path = root.join(Path::new("launcher_profiles.json"));
            let mut lp_obj: LauncherProfiles = serde_json::from_str(
                &fs::read_to_string(&lp_file_path)
                    .expect("Failed to read 'launcher_profiles.json'!"),
            )
            .expect("Failed to parse 'launcher_profiles.json'!");
            lp_obj.profiles.insert(manifest.uuid.clone(), profile);
            fs::write(
                lp_file_path,
                serde_json::to_string(&lp_obj)
                    .expect("Failed to create new 'launcher_profiles.json'!"),
            )
            .expect("Failed to write to 'launcher_profiles.json'");
        }
        Launcher::MultiMC(root) => {
            let pack = MMCPack {
                // TODO(Figure out how to get the correct components for the right loader and mc version)
                components: vec![
                    MMCComponent {
                        uid: String::from("org.lwjgl3"),
                        version: String::from("3.3.1"),
                        cachedVolatile: Some(true),
                        dependencyOnly: Some(true),
                        important: None,
                    },
                    MMCComponent {
                        uid: String::from("net.minecraft"),
                        version: manifest.loader.minecraft_version.to_string(),
                        cachedVolatile: None,
                        dependencyOnly: None,
                        important: Some(true),
                    },
                    MMCComponent {
                        uid: String::from("net.fabricmc.intermediary"),
                        version: manifest.loader.minecraft_version.to_string(),
                        cachedVolatile: Some(true),
                        dependencyOnly: Some(true),
                        important: None,
                    },
                    MMCComponent {
                        uid: String::from("net.fabricmc.fabric-loader"),
                        version: manifest.loader.version.to_string(),
                        cachedVolatile: None,
                        dependencyOnly: None,
                        important: None,
                    },
                ],
                formatVersion: 1,
            };
            fs::write(
                root.join(Path::new(&format!(
                    "instances/{}/mmc-pack.json",
                    manifest.uuid
                ))),
                serde_json::to_string(&pack).expect("Failed to create 'mmc-pack.json'"),
            )
            .expect("Failed to write to 'mmc-pack.json'");
            fs::write(
                root.join(Path::new(&format!(
                    "instances/{}/instance.cfg",
                    manifest.uuid
                ))),
                format!(
                    "iconKey={}
            name={}
            ",
                    manifest.uuid, manifest.name
                ),
            )
            .expect("Failed to write to 'instance.cfg'");
            if manifest.icon {
                icon_img
                    .expect("'icon' is 'true' but no icon was found")
                    .save(root.join(Path::new(&format!("icons/{}.png", manifest.uuid))))
                    .expect("Failed to write 'icon.png'");
            }
        }
    }
}

async fn install(installer_profile: InstallerProfile) {
    // Yes this is needed and no i wont change it
    // This might not be needed now to we use isahc
    // Especially now that we use 'InstallerProfile's
    // TODO(Remove unnecessary clone usage)
    let modpack_root = &get_modpack_root(
        installer_profile
            .launcher
            .as_ref()
            .expect("Launcher not selected!"),
        &installer_profile.manifest.uuid,
    );
    let manifest = &installer_profile.manifest;
    let http_client = &installer_profile.http_client;
    let loader_future = match installer_profile.launcher.as_ref().unwrap() {
        Launcher::Vanilla(root) => Some(manifest.loader.download(
            root,
            &manifest.loader.r#type,
            http_client,
        )),
        Launcher::MultiMC(_) => None,
    };
    let mods_w_path = futures::stream::iter(manifest.mods.clone().into_iter().map(|r#mod| async {
        if r#mod.path.is_none() && installer_profile.enabled_features.contains(&r#mod.id) {
            Mod {
                path: Some(
                    r#mod
                        .download(modpack_root, &manifest.loader.r#type, http_client)
                        .await,
                ),
                name: r#mod.name,
                source: r#mod.source,
                location: r#mod.location,
                version: r#mod.version,
                id: r#mod.id,
                authors: r#mod.authors,
            }
        } else {
            r#mod
        }
    }))
    .buffer_unordered(CONCURRENCY)
    .collect::<Vec<Mod>>()
    .await;
    let shaderpacks_w_path = futures::stream::iter(manifest.shaderpacks.clone().into_iter().map(
        |shaderpack| async {
            if shaderpack.path.is_none()
                && installer_profile.enabled_features.contains(&shaderpack.id)
            {
                Shaderpack {
                    path: Some(
                        shaderpack
                            .download(modpack_root, &manifest.loader.r#type, http_client)
                            .await,
                    ),
                    name: shaderpack.name,
                    source: shaderpack.source,
                    location: shaderpack.location,
                    version: shaderpack.version,
                    id: shaderpack.id,
                    authors: shaderpack.authors,
                }
            } else {
                shaderpack
            }
        },
    ))
    .buffer_unordered(CONCURRENCY)
    .collect::<Vec<Shaderpack>>()
    .await;
    let resourcepacks_w_path =
        futures::stream::iter(manifest.resourcepacks.clone().into_iter().map(
            |resourcepack| async {
                if resourcepack.path.is_none()
                    && installer_profile
                        .enabled_features
                        .contains(&resourcepack.id)
                {
                    Resourcepack {
                        path: Some(
                            resourcepack
                                .download(modpack_root, &manifest.loader.r#type, http_client)
                                .await,
                        ),
                        name: resourcepack.name,
                        source: resourcepack.source,
                        location: resourcepack.location,
                        version: resourcepack.version,
                        id: resourcepack.id,
                        authors: resourcepack.authors,
                    }
                } else {
                    resourcepack
                }
            },
        ))
        .buffer_unordered(CONCURRENCY)
        .collect::<Vec<Resourcepack>>()
        .await;
    if !manifest.include.is_empty() {
        // Include files exist
        let release: Vec<GithubRelease> = serde_json::from_str(
            http_client
                .get_async(
                    GH_API.to_owned() + installer_profile.modpack_source.as_str() + "releases",
                )
                .await
                .expect("Failed to retrieve releases!")
                .text()
                .await
                .unwrap()
                .as_str(),
        )
        .expect("Failed to parse release response!");

        let selected_rel = release
            .iter()
            .filter(|rel| rel.tag_name == installer_profile.modpack_branch)
            .collect::<Vec<&GithubRelease>>()
            .first()
            .cloned()
            .expect("Failed to retrieve release for selected branch!");
        for inc in &manifest.include {
            if !installer_profile.enabled_features.contains(&inc.id) {
                continue;
            }
            for asset in &selected_rel.assets {
                if asset.name == inc.id.clone() + ".zip" {
                    // download and unzip in modpack root
                    let content = http_client
                        .get_async(&asset.browser_download_url)
                        .await
                        .expect("Failed to download 'include.zip'")
                        .bytes()
                        .await
                        .unwrap();
                    let zipfile_path = modpack_root.join(Path::new(&asset.name));
                    fs::write(&zipfile_path, content).expect("Failed to write 'include.zip'!");
                    let zipfile = fs::File::open(&zipfile_path).unwrap();
                    let mut archive = zip::ZipArchive::new(zipfile).unwrap();
                    // modified from https://github.com/zip-rs/zip/blob/e32db515a2a4c7d04b0bf5851912a399a4cbff68/examples/extract.rs#L19
                    for i in 0..archive.len() {
                        let mut file = archive.by_index(i).unwrap();
                        let outpath = match file.enclosed_name() {
                            Some(path) => modpack_root.join(path),
                            None => continue,
                        };
                        if (*file.name()).ends_with('/') {
                            fs::create_dir_all(&outpath).unwrap();
                        } else {
                            if let Some(p) = outpath.parent() {
                                if !p.exists() {
                                    fs::create_dir_all(p).unwrap();
                                }
                            }
                            let mut outfile = fs::File::create(&outpath).unwrap();
                            std::io::copy(&mut file, &mut outfile).unwrap();
                        }
                    }
                    fs::remove_file(&zipfile_path).expect("Failed to remove tmp 'include.zip'!");
                    break;
                }
            }
        }
    }
    let local_manifest = Manifest {
        manifest_version: manifest.manifest_version,
        modpack_version: manifest.modpack_version.clone(),
        name: manifest.name.clone(),
        subtitle: manifest.subtitle.clone(),
        description: manifest.subtitle.clone(),
        icon: manifest.icon,
        uuid: manifest.uuid.clone(),
        loader: manifest.loader.clone(),
        mods: mods_w_path,
        shaderpacks: shaderpacks_w_path,
        resourcepacks: resourcepacks_w_path,
        include: manifest.include.clone(),
        features: manifest.features.clone(),
    };
    fs::write(
        modpack_root.join(Path::new("manifest.json")),
        serde_json::to_string(&local_manifest).expect("Failed to parse 'manifest.json'!"),
    )
    .expect("Failed to save a local copy of 'manifest.json'!");
    let icon_img = if manifest.icon {
        Some(
            ImageReader::new(Cursor::new(
                http_client
                    .get_async(
                        GH_RAW.to_owned()
                            + installer_profile.modpack_source.as_str()
                            + installer_profile.modpack_branch.as_str()
                            + "/icon.png",
                    )
                    .await
                    .expect("Failed to download icon")
                    .bytes()
                    .await
                    .unwrap(),
            ))
            .with_guessed_format()
            .expect("Could not guess icon.png format????????")
            .decode()
            .expect("Failed to decode icon!"),
        )
    } else {
        None
    };
    create_launcher_profile(&installer_profile, icon_img);
    if loader_future.is_some() {
        loader_future.unwrap().await;
    }
}

macro_rules! remove_items {
    ($items:expr, $predicate:expr) => {
        $items.iter().filter($predicate).for_each(|x| {
            fs::remove_file(x.path.as_ref().expect(&format!(
                "Missing 'path' field on installed {} '{}'!",
                stringify!($items),
                x.name
            )))
            .expect(&format!(
                "Failed to delete outdated {} '{}'!",
                stringify!($items),
                x.name
            ));
        });
    };
}
// Why haven't I split this into multiple files? That's a good question. I forgot, and I can't be bothered to do it now.
// TODO(Split project into multiple files to improve maintainability)
async fn update(installer_profile: InstallerProfile) {
    // TODO(figure out how to handle 'include' updates) current behaviour is writing over existing includes and files
    // TODO(change this to be idiomatic and good) im not sure if the 'remove_items' macro should exist and if it should then maybe the filtering could be turned into a macro too
    let local_manifest: Manifest = match fs::read_to_string(
        get_modpack_root(
            installer_profile
                .launcher
                .as_ref()
                .expect("Launcher not selected!"),
            &installer_profile.manifest.uuid,
        )
        .join(Path::new("manifest.json")),
    ) {
        Ok(contents) => match serde_json::from_str(&contents) {
            Ok(parsed) => parsed,
            Err(err) => panic!("Failed to parse local manifest: {}", err),
        },
        Err(err) => panic!("Failed to read local manifest: {}", err),
    };

    let new_mods: Vec<Mod> = installer_profile
        .manifest
        .mods
        .iter()
        .filter_map(|r#mod| {
            local_manifest
                .mods
                .iter()
                .find(|installed_mod| installed_mod.name == r#mod.name)
                .map_or_else(
                    || Some(r#mod.clone()),
                    |installed_mod| Some(installed_mod.clone()),
                )
        })
        .collect();
    remove_items!(local_manifest.mods, |x| { !new_mods.contains(x) });
    let new_shaderpacks: Vec<Shaderpack> = installer_profile
        .manifest
        .shaderpacks
        .iter()
        .filter_map(|shaderpack| {
            local_manifest
                .shaderpacks
                .iter()
                .find(|installed_sp| installed_sp.name == shaderpack.name)
                .map_or_else(
                    || Some(shaderpack.clone()),
                    |installed_sp| Some(installed_sp.clone()),
                )
        })
        .collect();
    remove_items!(local_manifest.shaderpacks, |x| {
        !new_shaderpacks.contains(x)
    });
    let new_resourcepacks: Vec<Resourcepack> = installer_profile
        .manifest
        .resourcepacks
        .iter()
        .filter_map(|resourcepack| {
            local_manifest
                .resourcepacks
                .iter()
                .find(|installed_rp| installed_rp.name == resourcepack.name)
                .map_or_else(
                    || Some(resourcepack.clone()),
                    |installed_rp| Some(installed_rp.clone()),
                )
        })
        .collect();
    remove_items!(local_manifest.resourcepacks, |x| {
        !new_resourcepacks.contains(x)
    });
    if installer_profile.manifest.loader != local_manifest.loader {
        fs::remove_dir_all(
            get_modpack_root(
                installer_profile
                    .launcher
                    .as_ref()
                    .expect("Launcher not selected!"),
                &installer_profile.manifest.uuid,
            )
            .join(Path::new(&format!(
                "versions/fabric-loader-{}-{}",
                &local_manifest.loader.version, &local_manifest.loader.minecraft_version
            ))),
        )
        .expect("Could not delete old fabric version!");
    }
    install(InstallerProfile {
        manifest: Manifest {
            manifest_version: installer_profile.manifest.manifest_version,
            modpack_version: installer_profile.manifest.modpack_version.clone(),
            name: installer_profile.manifest.name.clone(),
            icon: installer_profile.manifest.icon,
            uuid: installer_profile.manifest.uuid.clone(),
            loader: installer_profile.manifest.loader.clone(),
            mods: new_mods,
            shaderpacks: new_shaderpacks,
            resourcepacks: new_resourcepacks,
            include: installer_profile.manifest.include.clone(),
            features: installer_profile.manifest.features.clone(),
            description: installer_profile.manifest.description.clone(),
            subtitle: installer_profile.manifest.subtitle.clone(),
        },
        http_client: installer_profile.http_client,
        installed: installer_profile.installed,
        update_available: installer_profile.update_available,
        modpack_source: installer_profile.modpack_source,
        modpack_branch: installer_profile.modpack_branch,
        enabled_features: installer_profile.enabled_features,
        launcher: installer_profile.launcher,
    })
    .await;
}

fn get_launcher(string_representation: &str) -> Launcher {
    let launcher = string_representation.split('-').collect::<Vec<_>>();
    match *launcher.first().unwrap() {
        "vanilla" => Launcher::Vanilla(get_minecraft_folder()),
        "multimc" => Launcher::MultiMC(
            get_multimc_folder(launcher.last().expect("Invalid MultiMC!"))
                .expect("Invalid MultiMC!"),
        ),
        &_ => {
            panic!("Invalid launcher!")
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        // Multiple arguments detected entering CLI mode
        let args = CLIArgs::parse();
        let branch = if args.branch.is_none() {
            let repo: GithubRepo = serde_json::from_str(
                isahc::get(GH_API.to_owned() + &args.modpack)
                    .expect("Failed to gather modpack repo info!")
                    .text()
                    .unwrap()
                    .as_str(),
            )
            .expect("Could not retrieve default branch try specifying it using '--branch'!");
            repo.default_branch
        } else {
            args.branch.unwrap()
        };
        let launcher = get_launcher(&args.launcher);
        let installer_profile = futures::executor::block_on(init(&args.modpack, &branch, launcher));
        match args.action.as_str() {
            "install" => {
                if installer_profile.installed {
                    return;
                }
                futures::executor::block_on(install(installer_profile))
            }
            "update" => {
                if !installer_profile.installed || !installer_profile.update_available {
                    return;
                }
                futures::executor::block_on(update(installer_profile))
            }
            "play" => (),
            _ => (),
        };
        println!("Success!");
    } else {
        // Only the executable was present in arguments entering GUI mode
        let icon = image::load_from_memory(include_bytes!("assets/icon.png")).unwrap();
        let branches: Vec<GithubBranch> = serde_json::from_str(
            build_http_client()
                .get(GH_API.to_owned() + "Commander07/modpack-test/" + "branches")
                .expect("Failed to retrive branches!")
                .text()
                .unwrap()
                .as_str(),
        )
        .expect("Failed to parse branches!");
        let config_path = env::temp_dir().join(".WC_OVHL/config.json");
        let config: Config;
        if config_path.exists() {
            config =
                serde_json::from_slice(&fs::read(&config_path).expect("Failed to read config!"))
                    .expect("Failed to load config!");
        } else {
            config = Config {
                launcher: String::from("vanilla"),
            };
            fs::create_dir_all(config_path.parent().unwrap())
                .expect("Failed to create config dir!");
            fs::write(&config_path, serde_json::to_vec(&config).unwrap())
                .expect("Failed to write config!");
        }
        dioxus_desktop::launch_with_props(
            gui::App,
            gui::AppProps {
                branches,
                modpack_source: String::from("Commander07/modpack-test/"),
                config,
                config_path,
            },
            DioxusConfig::new()
                .with_window(
                    WindowBuilder::new()
                        .with_resizable(false)
                        .with_title("Wynncraft Overhaul Installer")
                        .with_inner_size(LogicalSize::new(1280, 720)),
                )
                .with_icon(
                    Icon::from_rgba(icon.to_rgba8().to_vec(), icon.width(), icon.height()).unwrap(),
                )
                .with_data_directory(env::temp_dir().join(".WC_OVHL")),
        );
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Launcher {
    Vanilla(PathBuf),
    MultiMC(PathBuf),
}

#[derive(Debug, Clone)]
struct InstallerProfile {
    manifest: Manifest,
    http_client: CachedHttpClient,
    installed: bool,
    update_available: bool,
    modpack_source: String,
    modpack_branch: String,
    enabled_features: Vec<String>,
    launcher: Option<Launcher>,
}

#[derive(Parser)]
struct CLIArgs {
    #[arg(short, long, verbatim_doc_comment)]
    /// Available actions:
    ///     - install: Install modpack if it does not already exist
    ///     - update: Updates modpack if it is installed
    ///     - play: Launches the modpack
    action: String,
    #[arg(short, long)]
    /// Github user and repo for modpack formmated like "<user>/<repo>/"
    modpack: String,
    #[arg(short, long)]
    /// Github branch defaults to default branch as specified on github
    branch: Option<String>,
    #[arg(short, long)]
    /// Launcher to install profile on:
    ///     - vanilla
    ///     - multimc-<data_dir_name>
    launcher: String,
}

async fn init(modpack_source: &str, modpack_branch: &str, launcher: Launcher) -> InstallerProfile {
    let http_client = CachedHttpClient::new();
    let manifest: Manifest = serde_json::from_str(
        http_client
            .get_async(GH_RAW.to_owned() + modpack_source + modpack_branch + "/manifest.json")
            .await
            .expect("Failed to retrieve manifest!")
            .text()
            .await
            .unwrap()
            .as_str(),
    )
    .expect("Failed to parse json!");
    // TODO(Figure out a way to support older manifest versions)
    assert!(
        CURRENT_MANIFEST_VERSION == manifest.manifest_version,
        "Unsupported manifest version '{}'!",
        manifest.manifest_version
    );
    let modpack_root = get_modpack_root(&launcher, &manifest.uuid);
    let installed = modpack_root.join(Path::new("manifest.json")).exists();
    let update_available: bool;
    if installed {
        let local_manifest: Manifest = serde_json::from_str(
            &fs::read_to_string(modpack_root.join(Path::new("manifest.json")))
                .expect("Failed to read local manifest!"),
        )
        .expect("Failed to parse local manifest!");
        if manifest.modpack_version != local_manifest.modpack_version {
            update_available = true;
        } else {
            update_available = false;
        }
    } else {
        update_available = false;
    }
    InstallerProfile {
        manifest,
        http_client,
        installed,
        update_available,
        modpack_source: modpack_source.to_owned(),
        modpack_branch: modpack_branch.to_owned(),
        enabled_features: vec![String::from("default")],
        launcher: Some(launcher),
    }
}
