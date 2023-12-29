#![cfg_attr(
    all(target_os = "windows", not(debug_assertions),),
    windows_subsystem = "windows"
)]
use async_trait::async_trait;
use base64::{engine, Engine};
use cached::proc_macro::cached;
use cached::SizedCache;
use chrono::{DateTime, Utc};
use dioxus_desktop::tao::window::Icon;
use dioxus_desktop::{Config as DioxusConfig, LogicalSize, WindowBuilder};
use futures::StreamExt;
use image::io::Reader as ImageReader;
use image::{DynamicImage, ImageOutputFormat};
use isahc::config::RedirectPolicy;
use isahc::http::StatusCode;
use isahc::prelude::Configurable;
use isahc::{AsyncBody, AsyncReadResponseExt, HttpClient, ReadResponseExt, Request, Response};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::thread::sleep;
use std::time::Duration;
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
const ATTEMPTS: usize = 3;
const WAIT_BETWEEN_ATTEMPTS: Duration = Duration::from_secs(20);

fn default_id() -> String {
    String::from("default")
}

fn default_enabled_features() -> Vec<String> {
    vec![default_id()]
}

fn default_hidden() -> bool {
    false
}

macro_rules! add_headers {
    ($items:expr, $($headers:expr),*) => {
        $items.$(header($headers.next().unwrap().0, $headers.next().unwrap().1))*
    };
}

#[derive(Debug)]
struct CachedResponse {
    resp: Response<AsyncBody>,
    bytes: Vec<u8>,
}

fn resp_rebuilder(resp: &Response<AsyncBody>, bytes: &Vec<u8>) -> Response<AsyncBody> {
    let builder = Response::builder()
        .status(resp.status())
        .version(resp.version());
    let builder = add_headers!(builder, resp.headers().into_iter());
    builder.body(AsyncBody::from(bytes.to_owned())).unwrap()
}

impl CachedResponse {
    async fn new(mut resp: Response<AsyncBody>) -> Self {
        let bytes = resp.bytes().await.unwrap();

        Self {
            resp: resp_rebuilder(&resp, &bytes),
            bytes,
        }
    }
}

impl Clone for CachedResponse {
    fn clone(&self) -> Self {
        Self {
            resp: resp_rebuilder(&self.resp, &self.bytes),
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

    async fn get_async<T: Into<String> + Clone>(
        &self,
        url: T,
    ) -> Result<Response<AsyncBody>, isahc::Error> {
        let mut err = None;
        for _ in 0..ATTEMPTS {
            let resp = get_cached(&self.http_client, url.clone().into()).await;
            match resp {
                Ok(v) => return Ok(v.resp),
                Err(v) => err = Some(v),
            }
            sleep(WAIT_BETWEEN_ATTEMPTS);
        }
        Err(err.unwrap()) // unwrap can't fail
    }

    async fn get_nocache<T: Into<String> + Clone>(
        &self,
        url: T,
    ) -> Result<Response<AsyncBody>, isahc::Error> {
        let mut err = None;
        for _ in 0..ATTEMPTS {
            let resp = self.http_client.get_async(url.clone().into()).await;
            match resp {
                Ok(v) => return Ok(v),
                Err(v) => err = Some(v),
            }
            sleep(WAIT_BETWEEN_ATTEMPTS);
        }
        Err(err.unwrap()) // unwrap can't fail
    }

    async fn with_headers<T: Into<String>>(
        &self,
        url: T,
        headers: &[(&str, &str)],
    ) -> Result<Response<AsyncBody>, isahc::Error> {
        self.http_client
            .send_async(
                add_headers!(Request::get(url.into()), headers.into_iter())
                    .body(())
                    .unwrap(),
            )
            .await
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
        Ok(val) => Ok(CachedResponse::new(val).await),
        Err(err) => Err(err),
    }
}

#[cfg(debug_assertions)]
fn build_http_client() -> HttpClient {
    HttpClient::builder()
        .redirect_policy(RedirectPolicy::Limit(5))
        .default_headers(&[
            ("User-Agent", "wynncraft-overhaul/installer/0.1.0"),
            ("Authorization", &format!("Bearer {}", include_str!("pat"))),
        ])
        .build()
        .unwrap()
}
#[cfg(not(debug_assertions))]
fn build_http_client() -> HttpClient {
    HttpClient::builder()
        .redirect_policy(RedirectPolicy::Limit(5))
        .default_headers(&[("User-Agent", "wynncraft-overhaul/installer/0.1.0")])
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
    ) -> Result<PathBuf, DownloadError>;

    fn new(
        name: String,
        source: String,
        location: String,
        version: String,
        path: Option<PathBuf>,
        id: String,
        authors: Vec<Author>,
    ) -> Self;
    fn get_name(&self) -> &String;
    fn get_location(&self) -> &String;
    fn get_version(&self) -> &String;
    fn get_path(&self) -> &Option<PathBuf>;
    fn get_id(&self) -> &String;
    fn get_source(&self) -> &String;
    fn get_authors(&self) -> &Vec<Author>;
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
struct Included {
    md5: String,
    files: Vec<String>,
}

macro_rules! gen_downloadble_impl {
    ($item:ty, $type:literal) => {
        #[async_trait]
        impl Downloadable for $item {
            async fn download(
                &self,
                modpack_root: &Path,
                loader_type: &str,
                http_client: &CachedHttpClient,
            ) -> Result<PathBuf, DownloadError> {
                match self.source.as_str() {
                    "modrinth" => {
                        download_from_modrinth(self, modpack_root, loader_type, $type, http_client)
                            .await
                    }
                    "ddl" => download_from_ddl(self, modpack_root, $type, http_client).await,
                    "mediafire" => {
                        download_from_mediafire(self, modpack_root, $type, http_client).await
                    }
                    _ => panic!("Unsupported source '{}'!", self.source.as_str()),
                }
            }

            fn new(
                name: String,
                source: String,
                location: String,
                version: String,
                path: Option<PathBuf>,
                id: String,
                authors: Vec<Author>,
            ) -> Self {
                Self {
                    name,
                    source,
                    location,
                    version,
                    path,
                    id,
                    authors,
                }
            }

            fn get_name(&self) -> &String {
                &self.name
            }
            fn get_location(&self) -> &String {
                &self.location
            }
            fn get_version(&self) -> &String {
                &self.version
            }
            fn get_path(&self) -> &Option<PathBuf> {
                &self.path
            }
            fn get_id(&self) -> &String {
                &self.id
            }
            fn get_source(&self) -> &String {
                &self.source
            }
            fn get_authors(&self) -> &Vec<Author> {
                &self.authors
            }
        }
    };
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

gen_downloadble_impl!(Mod, "mod");
gen_downloadble_impl!(Shaderpack, "shaderpack");
gen_downloadble_impl!(Resourcepack, "resourcepack");
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
struct Loader {
    r#type: String,
    version: String,
    minecraft_version: String,
}

impl Loader {
    async fn download(&self, root: &Path, _: &str, http_client: &CachedHttpClient) -> PathBuf {
        match self.r#type.as_str() {
            "fabric" => {
                download_loader_json(
                    &format!(
                        "https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json",
                        self.minecraft_version, self.version
                    ),
                    &format!("fabric-loader-{}-{}", self.version, self.minecraft_version),
                    root,
                    http_client,
                )
                .await
            }
            "quilt" => {
                download_loader_json(
                    &format!(
                        "https://meta.quiltmc.org/v3/versions/loader/{}/{}/profile/json",
                        self.minecraft_version, self.version
                    ),
                    &format!("quilt-loader-{}-{}", self.version, self.minecraft_version),
                    root,
                    http_client,
                )
                .await
            }
            _ => panic!("Unsupported loader '{}'!", self.r#type.as_str()),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
struct Feature {
    id: String,
    name: String,
    default: bool,
    #[serde(default = "default_hidden")]
    hidden: bool,
    description: Option<String>,
}
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
struct Include {
    location: String,
    #[serde(default = "default_id")]
    id: String,
    name: Option<String>,
    authors: Option<Vec<Author>>,
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
    #[serde(default = "default_enabled_features")]
    enabled_features: Vec<String>,
    included_files: Option<HashMap<String, Included>>,
    source: Option<String>,
    installer_path: Option<String>,
    max_mem: Option<i32>,
    min_mem: Option<i32>,
    java_args: Option<String>,
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
    id: i32,
    browser_download_url: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct GithubRelease {
    tag_name: String,
    body: Option<String>,
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

#[derive(Debug)]
enum DownloadError {
    Non200StatusCode(String, u16),
    FailedToParseResponse(String, serde_json::Error),
    IoError(String, std::io::Error),
    HttpError(String, isahc::Error),
    MissingFilename(String),
    CouldNotFindItem(String),
    MedafireMissingDDL(String),
}

impl Display for DownloadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadError::Non200StatusCode(item, x) => write!(
                f,
                "Encountered '{x}' error code when attempting to download: '{item}'"
            ),

            DownloadError::FailedToParseResponse(item, e) => write!(
                f,
                "Failed to parse download response: '{e}' when attempting to download: '{item}'"
            ),
            DownloadError::IoError(item, e) => write!(
                f,
                "Encountered io error: '{e}' when attempting to download: '{item}'"
            ),
            DownloadError::HttpError(item, e) => write!(
                f,
                "Encountered http error: '{e}' when attempting to download: '{item}'"
            ),
            DownloadError::MissingFilename(item) => {
                write!(f, "Could not get filename for: '{item}'")
            }
            DownloadError::CouldNotFindItem(item) => {
                write!(f, "Could not find item: '{item}'")
            }
            DownloadError::MedafireMissingDDL(item) => {
                write!(f, "Could not get DDL link from Nediafire: '{item}'")
            }
        }
    }
}

impl std::error::Error for DownloadError {}

#[derive(Debug)]
enum LauncherProfileError {
    IoError(std::io::Error),
    InvalidJson(serde_json::Error),
    ProfilesNotObject,
    NoProfiles,
    RootNotObject,
    IconNotFound,
    InvalidIcon(image::error::ImageError),
}

impl Display for LauncherProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LauncherProfileError::IoError(e) => write!(
                f,
                "Encountered IO error when creating launcher profile: {e}"
            ),
            LauncherProfileError::InvalidJson(e) => {
                write!(f, "Invalid 'launcher_profiles.json': {e}")
            }
            LauncherProfileError::NoProfiles => {
                write!(f, "'launcher_profiles.json' missing 'profiles' key")
            }
            LauncherProfileError::ProfilesNotObject => {
                write!(f, "Expected 'launcher_profiles.profiles' to be 'object'")
            }
            LauncherProfileError::RootNotObject => {
                write!(f, "Expected 'launcher_profiles' to be 'object'")
            }
            LauncherProfileError::IconNotFound => {
                write!(f, "'manifest.icon' was set to true but no icon was found")
            }
            LauncherProfileError::InvalidIcon(e) => write!(
                f,
                "Encountered image error when creating launcher profile: {e}"
            ),
        }
    }
}

impl std::error::Error for LauncherProfileError {}

impl From<std::io::Error> for LauncherProfileError {
    fn from(value: std::io::Error) -> Self {
        LauncherProfileError::IoError(value)
    }
}

impl From<serde_json::Error> for LauncherProfileError {
    fn from(value: serde_json::Error) -> Self {
        LauncherProfileError::InvalidJson(value)
    }
}

impl From<image::error::ImageError> for LauncherProfileError {
    fn from(value: image::error::ImageError) -> Self {
        LauncherProfileError::InvalidIcon(value)
    }
}

async fn download_loader_json(
    url: &str,
    loader_name: &str,
    root: &Path,
    http_client: &CachedHttpClient,
) -> PathBuf {
    let loader_path = root.join(Path::new(&format!("versions/{}", &loader_name)));
    if loader_path
        .join(Path::new(&format!("{}.json", &loader_name)))
        .exists()
    {
        return PathBuf::new();
    }
    let resp = http_client
        .get_async(url)
        .await
        .expect("Failed to download loader!")
        .text()
        .await
        .unwrap();
    fs::create_dir_all(&loader_path).expect("Failed to create loader directory");
    fs::write(
        loader_path.join(Path::new(&format!("{}.json", &loader_name))),
        resp,
    )
    .expect("Failed to write loader json");
    fs::write(
        loader_path.join(Path::new(&format!("{}.jar", &loader_name))),
        "",
    )
    .expect("Failed to write loader dummy jar");
    loader_path
}

async fn download_from_ddl<T: Downloadable + Debug>(
    item: &T,
    modpack_root: &Path,
    r#type: &str,
    http_client: &CachedHttpClient,
) -> Result<PathBuf, DownloadError> {
    let mut resp = match http_client.get_nocache(item.get_location()).await {
        Ok(v) => v,
        Err(e) => return Err(DownloadError::HttpError(item.get_name().to_string(), e)),
    };
    let filename = if let Some(x) = resp.headers().get("content-disposition") {
        let x = x.to_str().unwrap();
        if x.contains("attachment") {
            let re = Regex::new(r#"filename="(.*?)""#).unwrap();
            match match re.captures(x) {
                Some(v) => Ok(v),
                None => Err(DownloadError::MissingFilename(item.get_name().to_string())),
            } {
                Ok(v) => v[1].to_string(),
                Err(e) => match item.get_location().split('/').last() {
                    Some(v) => v.to_string(),
                    None => {
                        return Err(e);
                    }
                }
                .to_string(),
            }
        } else {
            item.get_location()
                .split('/')
                .last()
                .unwrap() // this should be impossible to error because all locations will have "/"s in them and if they dont it gets caught earlier
                .to_string()
        }
    } else {
        item.get_location()
            .split('/')
            .last()
            .unwrap() // this should be impossible to error because all locations will have "/"s in them and if they dont it gets caught earlier
            .to_string()
    };
    let dist = match r#type {
        "mod" => modpack_root.join(Path::new("mods")),
        "resourcepack" => modpack_root.join(Path::new("resourcepacks")),
        "shaderpack" => modpack_root.join(Path::new("shaderpacks")),
        _ => panic!("Unsupported item type: '{}'???", r#type), // this should be impossible
    };
    match fs::create_dir_all(&dist) {
        Ok(_) => (),
        Err(e) => return Err(DownloadError::IoError(item.get_name().to_string(), e)),
    }
    let final_dist = dist.join(filename);
    match fs::write(&final_dist, resp.bytes().await.unwrap()) {
        Ok(_) => (),
        Err(e) => return Err(DownloadError::IoError(item.get_name().to_string(), e)),
    };
    Ok(final_dist)
}

async fn download_from_modrinth<T: Downloadable + Debug>(
    item: &T,
    modpack_root: &Path,
    loader_type: &str,
    r#type: &str,
    http_client: &CachedHttpClient,
) -> Result<PathBuf, DownloadError> {
    let mut resp = match http_client
        .get_nocache(format!(
            "https://api.modrinth.com/v2/project/{}/version",
            item.get_location()
        ))
        .await
    {
        Ok(v) => v,
        Err(e) => {
            return Err(DownloadError::HttpError(item.get_name().to_string(), e));
        }
    };
    if resp.status() != StatusCode::OK {
        return Err(DownloadError::Non200StatusCode(
            item.get_name().to_string(),
            resp.status().as_u16(),
        ));
    }
    let resp_text = match resp.text().await {
        Ok(v) => v,
        Err(e) => return Err(DownloadError::IoError(item.get_name().to_string(), e)),
    };
    let resp_obj: Vec<ModrinthObject> = match serde_json::from_str(&resp_text) {
        Ok(v) => v,
        Err(e) => {
            return Err(DownloadError::FailedToParseResponse(
                item.get_name().to_string(),
                e,
            ));
        }
    };
    let dist = match r#type {
        "mod" => modpack_root.join(Path::new("mods")),
        "resourcepack" => modpack_root.join(Path::new("resourcepacks")),
        "shaderpack" => modpack_root.join(Path::new("shaderpacks")),
        _ => panic!("Unsupported item type: '{}'???", r#type), // this should be impossible
    };
    match fs::create_dir_all(&dist) {
        Ok(_) => (),
        Err(e) => return Err(DownloadError::IoError(item.get_name().to_string(), e)),
    }
    for _mod in resp_obj {
        if &_mod.version_number == item.get_version()
            && (_mod.loaders.contains(&String::from("minecraft"))
                || _mod.loaders.contains(&String::from(loader_type))
                || r#type == "shaderpack")
        {
            let content = match http_client.get_nocache(&_mod.files[0].url).await {
                Ok(v) => v,
                Err(e) => return Err(DownloadError::HttpError(item.get_name().to_string(), e)),
            }
            .bytes()
            .await
            .unwrap();
            let final_dist = dist.join(Path::new(&_mod.files[0].filename));
            match fs::write(&final_dist, content) {
                Ok(_) => (),
                Err(e) => return Err(DownloadError::IoError(item.get_name().to_string(), e)),
            };
            return Ok(final_dist);
        }
    }
    Err(DownloadError::CouldNotFindItem(item.get_name().to_string()))
}

async fn download_from_mediafire<T: Downloadable + Debug>(
    item: &T,
    modpack_root: &Path,
    r#type: &str,
    http_client: &CachedHttpClient,
) -> Result<PathBuf, DownloadError> {
    let mut resp = match http_client.get_nocache(item.get_location()).await {
        Ok(v) => v,
        Err(e) => {
            return Err(DownloadError::HttpError(item.get_name().to_string(), e));
        }
    };
    if resp.status() != StatusCode::OK {
        return Err(DownloadError::Non200StatusCode(
            item.get_name().to_string(),
            resp.status().as_u16(),
        ));
    }
    let mediafire = match resp.text().await {
        Ok(v) => v,
        Err(e) => return Err(DownloadError::IoError(item.get_name().to_string(), e)),
    };
    let re = Regex::new(r#"Download file"\s*href="(.*?)""#).unwrap(); // wont error pattern is valid
    let ddl = &(match re.captures(&mediafire) {
        Some(v) => v,
        None => {
            return Err(DownloadError::MedafireMissingDDL(
                item.get_name().to_string(),
            ))
        }
    })[1];
    let mut resp = match http_client.get_nocache(ddl).await {
        Ok(v) => v,
        Err(e) => return Err(DownloadError::HttpError(item.get_name().to_string(), e)),
    };
    let cd_header = match std::str::from_utf8(
        match resp.headers().get("content-disposition") {
            Some(v) => v,
            None => return Err(DownloadError::MissingFilename(item.get_name().to_string())),
        }
        .as_bytes(),
    ) {
        Ok(v) => v,
        Err(_) => return Err(DownloadError::MissingFilename(item.get_name().to_string())),
    };
    let filename = if cd_header.contains("attachment") {
        match cd_header.split("filename=").last() {
            Some(v) => v,
            None => return Err(DownloadError::MissingFilename(item.get_name().to_string())),
        }
        .replace('"', "")
    } else {
        return Err(DownloadError::MissingFilename(item.get_name().to_string()));
    };
    let dist = match r#type {
        "mod" => modpack_root.join(Path::new("mods")),
        "resourcepack" => modpack_root.join(Path::new("resourcepacks")),
        "shaderpack" => modpack_root.join(Path::new("shaderpacks")),
        _ => panic!("Unsupported item type'{}'???", r#type), // this should be impossible
    };
    match fs::create_dir_all(&dist) {
        Ok(_) => (),
        Err(e) => return Err(DownloadError::IoError(item.get_name().to_string(), e)),
    };
    let final_dist = dist.join(filename);
    match fs::write(&final_dist, resp.bytes().await.unwrap()) {
        Ok(_) => (),
        Err(e) => return Err(DownloadError::IoError(item.get_name().to_string(), e)),
    };
    Ok(final_dist)
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

fn get_multimc_folder(multimc: &str) -> Result<PathBuf, String> {
    let path = match env::consts::OS {
        "linux" => get_app_data().join(format!(".local/share/{}", multimc)),
        "windows" | "macos" => get_app_data().join(multimc),
        _ => panic!("Unsupported os '{}'!", env::consts::OS),
    };
    match path.metadata() {
        Ok(metadata) => {
            if metadata.is_dir() {
                Ok(path)
            } else {
                Err(String::from("MultiMC directory is not a directory!"))
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

fn get_minecraft_folder() -> PathBuf {
    if env::consts::OS == "macos" {
        get_app_data().join("minecraft")
    } else {
        get_app_data().join(".minecraft")
    }
}

fn get_modpack_root(launcher: &Launcher, uuid: &str) -> PathBuf {
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

fn create_launcher_profile(
    installer_profile: &InstallerProfile,
    icon_img: Option<DynamicImage>,
) -> Result<(), LauncherProfileError> {
    let now = SystemTime::now();
    let now: DateTime<Utc> = now.into();
    let now = now.to_rfc3339();
    let manifest = &installer_profile.manifest;
    let modpack_root = get_modpack_root(
        installer_profile
            .launcher
            .as_ref()
            .expect("No launcher selected!"), // should be impossible
        &manifest.uuid,
    );
    match installer_profile
        .launcher
        .as_ref()
        .expect("Asked to create launcher profile without knowing launcher!") // should be impossible
    {
        Launcher::Vanilla(_) => {
            let icon = if manifest.icon && icon_img.is_some() {
                image_to_base64(
                    icon_img
                        .as_ref()
                        .unwrap()
                )
            } else {
                String::from("Furnace")
            };
            let mut jvm_args = String::new();
            if manifest.java_args.is_none()
                && (manifest.max_mem.is_some() || manifest.min_mem.is_some())
            {
                jvm_args += "XX:+UnlockExperimentalVMOptions -XX:+UseG1GC -XX:G1NewSizePercent=20 -XX:G1ReservePercent=20 -XX:MaxGCPauseMillis=50 -XX:G1HeapRegionSize=32M";
            }
            if let Some(x) = &manifest.java_args {
                jvm_args += &x
            }
            if let Some(x) = manifest.max_mem {
                jvm_args += &format!(" -Xmx{}M", x)
            }
            if let Some(x) = manifest.min_mem {
                jvm_args += &format!(" -Xms{}M", x)
            }
            let profile = LauncherProfile {
                lastUsed: now.to_string(),
                lastVersionId: match &manifest.loader.r#type[..] {
                    "fabric" => format!(
                        "fabric-loader-{}-{}",
                        &manifest.loader.version, &manifest.loader.minecraft_version
                    ),
                    "quilt" => format!(
                        "quilt-loader-{}-{}",
                        &manifest.loader.version, &manifest.loader.minecraft_version
                    ),
                    _ => panic!("Invalid loader"),
                },
                created: now,
                name: manifest.name.clone(),
                icon,
                r#type: String::from("custom"),
                gameDir: Some(modpack_root.to_str().unwrap().to_string()),
                javaDir: None,
                javaArgs: if jvm_args.is_empty() {
                    None
                } else {
                    Some(jvm_args)
                },
                logConfig: None,
                logConfigIsXML: None,
                resolution: None,
            };
            let lp_file_path = get_minecraft_folder().join(Path::new("launcher_profiles.json"));
            let mut lp_obj: JsonValue = serde_json::from_str(
                &fs::read_to_string(&lp_file_path)?,
            )?;
            match lp_obj {
                JsonValue::Object(ref obj) => match obj
                    .get("profiles")
                    .ok_or(LauncherProfileError::NoProfiles)?
                {
                    JsonValue::Object(_) => lp_obj.get_mut("profiles").unwrap().as_object_mut().unwrap().insert(manifest.uuid.clone(), serde_json::to_value(profile)?),
                    _ => return Err(LauncherProfileError::ProfilesNotObject),
                },
                _ => return Err(LauncherProfileError::RootNotObject),
            };
            fs::write(
                lp_file_path,
                serde_json::to_string(&lp_obj)?,
            )?;
        }
        Launcher::MultiMC(root) => {
            let pack = MMCPack {
                components: vec![
                    MMCComponent {
                        uid: String::from("net.minecraft"),
                        version: manifest.loader.minecraft_version.to_string(),
                        cachedVolatile: None,
                        dependencyOnly: None,
                        important: Some(true),
                    },
                    match &manifest.loader.r#type[..] {
                        "fabric" => MMCComponent {
                            uid: String::from("net.fabricmc.fabric-loader"),
                            version: manifest.loader.version.to_string(),
                            cachedVolatile: None,
                            dependencyOnly: None,
                            important: None,
                        },
                        "quilt" => MMCComponent {
                            uid: String::from("org.quiltmc.quilt-loader"),
                            version: manifest.loader.version.to_string(),
                            cachedVolatile: None,
                            dependencyOnly: None,
                            important: None,
                        },
                        _ => panic!("Invalid loader"),
                    },
                ],
                formatVersion: 1,
            };
            fs::write(
                root.join(Path::new(&format!(
                    "instances/{}/mmc-pack.json",
                    manifest.uuid
                ))),
                serde_json::to_string(&pack)?,
            )?;
            let jvm_args = match manifest.java_args.as_ref() {
                Some(v) => format!("\nJvmArgs={}\nOverrideJavaArgs=true", v),
                None => String::new(),
            };
            let max_mem = match manifest.max_mem {
                Some(v) => format!("\nMaxMemAlloc={}", v),
                None => String::new(),
            };
            let min_mem = match manifest.min_mem {
                Some(v) => format!("\nMinMemAlloc={}", v),
                None => String::new(),
            };
            let override_mem = if max_mem.is_empty() && min_mem.is_empty() {
                ""
            } else {
                "\nOverrideMemory=true"
            };
            fs::write(
                root.join(Path::new(&format!(
                    "instances/{}/instance.cfg",
                    manifest.uuid
                ))),
                format!(
                    "iconKey={}\nname={}{}{}{}{}",
                    manifest.uuid, manifest.name, max_mem, min_mem, override_mem, jvm_args
                ),
            )?;
            if manifest.icon {
                icon_img.ok_or(LauncherProfileError::IconNotFound)?
                    .save(root.join(Path::new(&format!("icons/{}.png", manifest.uuid))))?;
            }
        }
    };
    Ok(())
}

/// Panics:
///     If path is not located in modpack_root
macro_rules! validate_item_path {
    ($item:expr, $modpack_root:expr) => {
        if $item.get_path().is_some() {
            if $item
                .get_path()
                .as_ref()
                .unwrap()
                .parent()
                .expect("Illegal item file path!")
                .parent()
                .expect("Illegal item dir path!")
                == $modpack_root
            {
                $item
            } else {
                panic!("{:?}'s path was not located in modpack root!", $item);
            }
        } else {
            $item
        }
    };
}

fn uninstall(launcher: &Launcher, b64_id: &str) {
    let mut data_source_id = String::new();
    match launcher {
        Launcher::Vanilla(root) => {
            let root = root.join(".WC_OVHL/");
            for instance in fs::read_dir(root).unwrap() {
                let instance = instance.unwrap().path();
                if instance.join(b64_id).is_file() {
                    data_source_id = instance.file_name().unwrap().to_str().unwrap().to_owned();
                    fs::remove_dir_all(&instance).expect("Failed to uninstall modpack!");
                    fs::create_dir(instance).unwrap();
                }
            }
        }
        Launcher::MultiMC(root) => {
            let root = root.join("instances/");
            for instance in fs::read_dir(root).unwrap() {
                let instance = instance.unwrap().path();
                if instance.join(format!(".minecraft/{}", b64_id)).is_file() {
                    data_source_id = instance.file_name().unwrap().to_str().unwrap().to_owned();
                    fs::remove_dir_all(&instance).expect("Failed to uninstall modpack!");
                    fs::create_dir_all(instance.join(".minecraft/")).unwrap();
                }
            }
        }
    }
    let _ = isahc::post(
        "https://tracking.commander07.workers.dev/track",
        format!(
            "{{
        \"projectId\": \"55db8403a4f24f3aa5afd33fd1962888\",
        \"dataSourceId\": \"{}\",
        \"userAction\": \"uninstall\",
        \"additionalData\": {{}}
    }}",
            data_source_id
        ),
    );
}

async fn download_helper<T: Downloadable + Debug>(
    items: Vec<T>,
    enabled_features: &Vec<String>,
    modpack_root: &Path,
    loader_type: &str,
    http_client: &CachedHttpClient,
) -> Result<Vec<T>, DownloadError> {
    let results = futures::stream::iter(items.into_iter().map(|item| async {
        if item.get_path().is_none() && enabled_features.contains(item.get_id()) {
            let path = item
                .download(modpack_root, loader_type, http_client)
                .await?;
            Ok(T::new(
                item.get_name().to_owned(),
                item.get_source().to_owned(),
                item.get_location().to_owned(),
                item.get_version().to_owned(),
                Some(path),
                item.get_id().to_owned(),
                item.get_authors().to_owned(),
            ))
        } else {
            let item = validate_item_path!(item, modpack_root);
            let path;
            if !enabled_features.contains(item.get_id()) && item.get_path().is_some() {
                fs::remove_file(item.get_path().as_ref().unwrap())
                    .expect("Failed to remove old mod");
                path = None;
            } else {
                path = item.get_path().to_owned();
            }
            Ok(T::new(
                item.get_name().to_owned(),
                item.get_source().to_owned(),
                item.get_location().to_owned(),
                item.get_version().to_owned(),
                path,
                item.get_id().to_owned(),
                item.get_authors().to_owned(),
            ))
        }
    }))
    .buffer_unordered(CONCURRENCY)
    .collect::<Vec<Result<T, DownloadError>>>()
    .await;
    let mut return_vec = vec![];
    for res in results {
        match res {
            Ok(v) => return_vec.push(v),
            Err(e) => return Err(e),
        }
    }
    Ok(return_vec)
}

async fn install(installer_profile: InstallerProfile) -> Result<(), String> {
    let modpack_root = &get_modpack_root(
        installer_profile
            .launcher
            .as_ref()
            .expect("Launcher not selected!"),
        &installer_profile.manifest.uuid,
    );
    let manifest = &installer_profile.manifest;
    let http_client = &installer_profile.http_client;
    let minecraft_folder = get_minecraft_folder();
    let loader_future = match installer_profile.launcher.as_ref().unwrap() {
        Launcher::Vanilla(_) => Some(manifest.loader.download(
            &minecraft_folder,
            &manifest.loader.r#type,
            http_client,
        )),
        Launcher::MultiMC(_) => None,
    };
    let mods_w_path = match download_helper(
        manifest.mods.clone(),
        &installer_profile.enabled_features,
        modpack_root.as_path(),
        &manifest.loader.r#type,
        http_client,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => return Err(e.to_string()),
    };
    let shaderpacks_w_path = match download_helper(
        manifest.shaderpacks.clone(),
        &installer_profile.enabled_features,
        modpack_root.as_path(),
        &manifest.loader.r#type,
        http_client,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => return Err(e.to_string()),
    };
    let resourcepacks_w_path = match download_helper(
        manifest.resourcepacks.clone(),
        &installer_profile.enabled_features,
        modpack_root.as_path(),
        &manifest.loader.r#type,
        http_client,
    )
    .await
    {
        Ok(v) => v,
        Err(e) => return Err(e.to_string()),
    };
    let mut included_files: HashMap<String, Included> = HashMap::new();
    let inc_files = match installer_profile.local_manifest.clone() {
        Some(local_manifest) => match local_manifest.included_files {
            Some(files) => files,
            None => HashMap::new(),
        },
        None => HashMap::new(),
    };
    for inc in &inc_files {
        if !installer_profile
            .enabled_features
            .contains(&inc.0.replace(".zip", ""))
        {
            for file in &inc.1.files {
                fs::remove_file(file).expect("Failed to remove inc")
            }
        }
    }
    if !manifest.include.is_empty() {
        // Include files exist
        let release: GithubRelease = serde_json::from_str(
            http_client
                .get_async(
                    GH_API.to_owned()
                        + installer_profile.modpack_source.as_str()
                        + "releases/tags/"
                        + installer_profile.modpack_branch.as_str(),
                )
                .await
                .expect("Failed to retrieve releases!")
                .text()
                .await
                .unwrap()
                .as_str(),
        )
        .expect("Failed to parse release response!");
        let hash_pairs: HashMap<String, String> = serde_json::from_str(
            &release
                .body
                .as_ref()
                .expect("Missing body on modpack release!"),
        )
        .expect("Failed to parse hash pairs!");
        for inc in &manifest.include {
            if !installer_profile.enabled_features.contains(&inc.id) {
                continue;
            }
            'a: for asset in &release.assets {
                let inc_zip_name = inc.id.clone() + ".zip";
                if asset.name == inc_zip_name {
                    let md5 = hash_pairs
                        .get(&inc_zip_name)
                        .expect("Asset does not have hash in release body")
                        .to_owned();
                    match inc_files.get(&inc_zip_name) {
                        Some(local_inc) => {
                            if local_inc.md5 == md5 {
                                included_files.insert(inc_zip_name, local_inc.to_owned());
                                break 'a;
                            } else {
                                for file in &local_inc.files {
                                    let path = Path::new(file);
                                    assert!(
                                        path.starts_with(modpack_root),
                                        "Local include path was not located in modpack root!"
                                    );
                                    let _ = fs::remove_file(path);
                                }
                            }
                        }
                        None => (),
                    }
                    let mut files: Vec<String> = vec![];
                    // download and unzip in modpack root
                    let content = http_client
                        .with_headers(
                            format!(
                                "{}{}releases/assets/{}",
                                GH_API, installer_profile.modpack_source, asset.id
                            ),
                            &[("Accept", "application/octet-stream")],
                        )
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
                            files.push(outpath.to_str().unwrap().to_string());
                        }
                    }
                    fs::remove_file(&zipfile_path).expect("Failed to remove tmp 'include.zip'!");
                    included_files.insert(inc_zip_name.clone(), Included { md5, files });
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
        enabled_features: installer_profile.enabled_features.clone(),
        included_files: Some(included_files),
        source: Some(format!(
            "{}{}",
            installer_profile.modpack_source, installer_profile.modpack_branch
        )),
        installer_path: Some(
            env::current_exe()
                .unwrap()
                .canonicalize()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned()
                .replace("\\\\?\\", ""),
        ),
        max_mem: manifest.max_mem,
        min_mem: manifest.min_mem,
        java_args: manifest.java_args.clone(),
    };
    fs::write(
        modpack_root.join(
            engine::general_purpose::URL_SAFE_NO_PAD.encode(&installer_profile.modpack_source),
        ),
        "",
    )
    .expect("Failed to save b64_id");
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
    match create_launcher_profile(&installer_profile, icon_img) {
        Ok(_) => {}
        Err(e) => return Err(e.to_string()),
    };
    if loader_future.is_some() {
        loader_future.unwrap().await;
    }
    Ok(())
}

fn remove_old_items<T: Downloadable + PartialEq + Clone>(
    items: Vec<T>,
    installed_items: &Vec<T>,
) -> Vec<T> {
    let new_items: Vec<T> = items
        .iter()
        .filter_map(|item| {
            installed_items
                .iter()
                .find(|installed_item| installed_item.get_name() == item.get_name())
                .map_or_else(
                    || Some(item.clone()),
                    |installed_item| {
                        if installed_item.get_version() == item.get_version() {
                            Some(installed_item.clone())
                        } else {
                            let _ = fs::remove_file(installed_item.get_path().as_ref().expect(
                                &format!(
                                    "Missing 'path' field on installed {} '{}'!",
                                    stringify!(installed_item),
                                    installed_item.get_name()
                                ),
                            ));
                            Some(item.clone())
                        }
                    },
                )
        })
        .collect();
    installed_items
        .iter()
        .filter(|x| !new_items.contains(x))
        .for_each(|x| {
            let _ = fs::remove_file(x.get_path().as_ref().expect(&format!(
                "Missing 'path' field on installed {} '{}'!",
                stringify!(x),
                x.get_name()
            )));
        });
    new_items
}

// Why haven't I split this into multiple files? That's a good question. I forgot, and I can't be bothered to do it now.
// TODO(Split project into multiple files to improve maintainability)
async fn update(installer_profile: InstallerProfile) -> Result<(), String> {
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
    let new_mods = remove_old_items(installer_profile.manifest.mods, &local_manifest.mods);
    let new_shaderpacks = remove_old_items(
        installer_profile.manifest.shaderpacks,
        &local_manifest.shaderpacks,
    );
    let new_resourcepacks = remove_old_items(
        installer_profile.manifest.resourcepacks,
        &local_manifest.resourcepacks,
    );
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
            enabled_features: installer_profile.manifest.enabled_features,
            included_files: local_manifest.included_files.clone(),
            source: local_manifest.source.clone(),
            installer_path: local_manifest.installer_path.clone(),
            max_mem: installer_profile.manifest.max_mem,
            min_mem: installer_profile.manifest.min_mem,
            java_args: installer_profile.manifest.java_args,
        },
        http_client: installer_profile.http_client,
        installed: installer_profile.installed,
        update_available: installer_profile.update_available,
        modpack_source: installer_profile.modpack_source,
        modpack_branch: installer_profile.modpack_branch,
        enabled_features: installer_profile.enabled_features,
        launcher: installer_profile.launcher,
        local_manifest: Some(local_manifest),
    })
    .await
}

fn get_launcher(string_representation: &str) -> Result<Launcher, String> {
    let launcher = string_representation.split('-').collect::<Vec<_>>();
    match *launcher.first().unwrap() {
        "vanilla" => Ok(Launcher::Vanilla(get_app_data())),
        "multimc" => {
            let data_dir = get_multimc_folder(
                launcher
                    .last()
                    .expect("Missing data dir segement in MultiMC!"),
            );
            match data_dir {
                Ok(path) => Ok(Launcher::MultiMC(path)),
                Err(e) => Err(e),
            }
        }
        _ => Err(String::from("Invalid launcher!")),
    }
}

fn main() {
    let icon = image::load_from_memory(include_bytes!("assets/icon.png")).unwrap();
    let branches: Vec<GithubBranch> = serde_json::from_str(
        build_http_client()
            .get(GH_API.to_owned() + "Wynncraft-Overhaul/majestic-overhaul/" + "branches")
            .expect("Failed to retrive branches!")
            .text()
            .unwrap()
            .as_str(),
    )
    .expect("Failed to parse branches!");
    let config_path = env::temp_dir().join(".WC_OVHL/config.json");
    let config: Config;
    let style_css = include_str!("style.css");
    let style_css = style_css.replace(
        "Wynncraft_Game_Font.woff2.base64",
        include_str!("assets/Wynncraft_Game_Font.woff2.base64"),
    );
    if config_path.exists() {
        config = serde_json::from_slice(&fs::read(&config_path).expect("Failed to read config!"))
            .expect("Failed to load config!");
    } else {
        config = Config {
            launcher: String::from("vanilla"),
        };
        fs::create_dir_all(config_path.parent().unwrap()).expect("Failed to create config dir!");
        fs::write(&config_path, serde_json::to_vec(&config).unwrap())
            .expect("Failed to write config!");
    }

    dioxus_desktop::launch_with_props(
        gui::App,
        gui::AppProps {
            branches,
            modpack_source: String::from("Wynncraft-Overhaul/majestic-overhaul/"),
            config,
            config_path,
            style_css: Box::leak(style_css.into_boxed_str()), // this stops a memory leak from happening when switching between settings and start menu
        },
        DioxusConfig::new()
            .with_window(
                WindowBuilder::new()
                    .with_resizable(true)
                    .with_title("Majestic Overhaul Installer")
                    .with_inner_size(LogicalSize::new(960, 540)),
            )
            .with_icon(
                Icon::from_rgba(icon.to_rgba8().to_vec(), icon.width(), icon.height()).unwrap(),
            )
            .with_data_directory(env::temp_dir().join(".WC_OVHL")),
    );
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
    local_manifest: Option<Manifest>,
}

async fn init(
    modpack_source: String,
    modpack_branch: String,
    launcher: Launcher,
) -> Result<InstallerProfile, String> {
    let modpack_source = &modpack_source;
    let modpack_branch = &modpack_branch;
    let http_client = CachedHttpClient::new();
    let mut manifest_resp = match http_client
        .get_async(GH_RAW.to_owned() + modpack_source + modpack_branch + "/manifest.json")
        .await
    {
        Ok(val) => val,
        Err(e) => return Err(e.to_string()),
    };
    let manifest: Manifest =
        match serde_json::from_str(manifest_resp.text().await.unwrap().as_str()) {
            Ok(val) => val,
            Err(e) => return Err(e.to_string()),
        };

    // Its not guaranteed that a manifest with a different version manages to parse however we handle parsing failures and therefore we should be fine to just return an error here
    if CURRENT_MANIFEST_VERSION != manifest.manifest_version {
        return Err(format!(
            "Unsupported manifest version '{}'!",
            manifest.manifest_version
        ));
    }
    let modpack_root = get_modpack_root(&launcher, &manifest.uuid);
    let mut installed = modpack_root.join(Path::new("manifest.json")).exists();
    let local_manifest: Option<Result<Manifest, serde_json::Error>> = if installed {
        let local_manifest_content =
            match fs::read_to_string(modpack_root.join(Path::new("manifest.json"))) {
                Ok(val) => val,
                Err(e) => return Err(e.to_string()),
            };
        Some(serde_json::from_str(&local_manifest_content))
    } else {
        installed = false;
        None
    };
    let update_available = if installed {
        match local_manifest.as_ref().unwrap() {
            Ok(val) => manifest.modpack_version != val.modpack_version,
            Err(_) => false,
        }
    } else {
        false
    };
    let mut enabled_features = vec![default_id()];
    if !installed {
        for feat in &manifest.features {
            if feat.default {
                enabled_features.push(feat.id.clone());
            }
        }
    }
    Ok(InstallerProfile {
        manifest,
        http_client,
        installed,
        update_available,
        modpack_source: modpack_source.to_owned(),
        modpack_branch: modpack_branch.to_owned(),
        enabled_features,
        launcher: Some(launcher),
        local_manifest: if local_manifest.is_some() && local_manifest.as_ref().unwrap().is_ok() {
            Some(local_manifest.unwrap().unwrap())
        } else {
            None
        },
    })
}
