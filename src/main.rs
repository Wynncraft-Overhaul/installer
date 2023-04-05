use async_recursion::async_recursion;
use async_trait::async_trait;
use base64::{engine, Engine};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use image::io::Reader as ImageReader;
use image::{DynamicImage, ImageOutputFormat};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{
    env, fs,
    io::Cursor,
    path::{Path, PathBuf},
    time::SystemTime,
};

const CURRENT_MANIFEST_VERSION: i32 = 1;
const GH_API: &str = "https://api.github.com/repos/";
const GH_RAW: &str = "https://raw.githubusercontent.com/";
const MODPACK_SOURCE: &str = "Commander07/modpack-test/";
const MODPACK_BRANCH: &str = "main";
const CONCURRENCY: usize = 14;
#[async_trait]
trait Downloadable {
    async fn download(
        &self,
        modpack_root: &PathBuf,
        loader_type: &String,
        http_client: &Client,
    ) -> PathBuf;
}
trait DownloadableGetters {
    fn get_name(&self) -> &String;
    fn get_location(&self) -> &String;
    fn get_version(&self) -> &String;
}
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
struct Mod {
    name: String,
    source: String,
    location: String,
    version: String,
    path: Option<PathBuf>,
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
        modpack_root: &PathBuf,
        loader_type: &String,
        http_client: &Client,
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
        modpack_root: &PathBuf,
        loader_type: &String,
        http_client: &Client,
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
        modpack_root: &PathBuf,
        loader_type: &String,
        http_client: &Client,
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
    async fn download(&self, modpack_root: &PathBuf, _: &String, http_client: &Client) -> PathBuf {
        match self.r#type.as_str() {
            "fabric" => download_fabric(&self, modpack_root, http_client).await,
            _ => panic!("Unsupported loader '{}'!", self.r#type.as_str()),
        }
    }
}
#[derive(Debug, Deserialize, Serialize)]
struct Manifest {
    manifest_version: i32,
    modpack_version: String,
    name: String,
    icon: bool,
    uuid: String,
    loader: Loader,
    mods: Vec<Mod>,
    shaderpacks: Vec<Shaderpack>,
    resourcepacks: Vec<Resourcepack>,
    include: Vec<String>,
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
struct GitHubFile {
    path: String,
    download_url: Option<String>,
    r#type: String,
    url: String,
}

struct ReadFile {
    path: PathBuf,
    content: Bytes,
}

async fn download_fabric(loader: &Loader, modpack_root: &PathBuf, http_client: &Client) -> PathBuf {
    let url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json",
        loader.minecraft_version, loader.version
    );
    let loader_name = format!(
        "fabric-loader-{}-{}",
        &loader.version, &loader.minecraft_version
    );
    let fabric_path = modpack_root.join(&Path::new(&format!("versions/{}", &loader_name)));
    if fabric_path
        .join(Path::new(&format!("{}.json", &loader_name)))
        .exists()
    {
        return PathBuf::new();
    }
    let resp = http_client
        .get(url.as_str())
        .send()
        .await
        .expect("Failed to download fabric loader!")
        .text()
        .await
        .unwrap();
    fs::create_dir_all(&fabric_path).expect("Failed to create fabric directory");
    fs::write(
        &fabric_path.join(Path::new(&format!("{}.json", &loader_name))),
        resp,
    )
    .expect("Failed to write fabric json");
    fs::write(
        &fabric_path.join(Path::new(&format!("{}.jar", &loader_name))),
        "",
    )
    .expect("Failed to write fabric dummy jar");
    return fabric_path;
}

async fn download_from_ddl<T: Downloadable + DownloadableGetters>(
    item: &T,
    modpack_root: &PathBuf,
    r#type: &str,
    http_client: &Client,
) -> PathBuf {
    let content = http_client
        .get(item.get_location())
        .send()
        .await
        .expect(&format!("Failed to download '{}'!", item.get_name()))
        .bytes()
        .await
        .unwrap();
    let dist: PathBuf;
    match r#type {
        "mod" => dist = modpack_root.join(Path::new("mods")),
        "resourcepack" => dist = modpack_root.join(Path::new("resourcepacks")),
        "shaderpack" => dist = modpack_root.join(Path::new("shaderpacks")),
        _ => panic!("Unsupported 'ModrinthCompatible' item '{}'???", r#type),
    };
    fs::create_dir_all(&dist).expect(&format!(
        "Failed to create '{}' directory",
        &dist.to_str().unwrap()
    ));
    let final_dist = dist.join(Path::new(item.get_location().split("/").last().expect(
        &format!(
            "Could not determine file name for ddl: '{}'!",
            item.get_location()
        ),
    )));
    fs::write(&final_dist, &content).expect("Failed to write ddl item!");
    final_dist
}

async fn download_from_modrinth<T: Downloadable + DownloadableGetters>(
    item: &T,
    modpack_root: &PathBuf,
    loader_type: &String,
    r#type: &str,
    http_client: &Client,
) -> PathBuf {
    let resp = http_client
        .get(format!(
            "https://api.modrinth.com/v2/project/{}/version",
            item.get_location()
        ))
        .send()
        .await
        .expect(&format!("Failed to download '{}'!", item.get_name()))
        .text()
        .await
        .unwrap();
    let resp_obj: Vec<ModrinthObject> =
        serde_json::from_str(&resp).expect("Failed to parse modrinth response!");
    let dist: PathBuf;
    match r#type {
        "mod" => dist = modpack_root.join(Path::new("mods")),
        "resourcepack" => dist = modpack_root.join(Path::new("resourcepacks")),
        "shaderpack" => dist = modpack_root.join(Path::new("shaderpacks")),
        _ => panic!("Unsupported 'ModrinthCompatible' item '{}'???", r#type),
    };
    fs::create_dir_all(&dist).expect(&format!(
        "Failed to create '{}' directory",
        &dist.to_str().unwrap()
    ));
    for _mod in resp_obj {
        if &_mod.version_number == item.get_version() {
            if _mod.loaders.contains(&String::from("minecraft"))
                || _mod.loaders.contains(&String::from(loader_type))
                || r#type == "shaderpack"
            {
                let content = http_client
                    .get(&_mod.files[0].url)
                    .send()
                    .await
                    .expect(&format!("Failed to download '{}'!", item.get_name()))
                    .bytes()
                    .await
                    .unwrap();
                let final_dist = dist.join(Path::new(&_mod.files[0].filename));
                fs::write(&final_dist, &content).expect("Failed to write modrinth item!");
                return final_dist;
            }
        }
    }
    panic!("No items returned from modrinth!")
}

fn get_minecraft_folder() -> PathBuf {
    if env::consts::OS == "linux" {
        Path::new(&(dirs::home_dir().unwrap().to_str().unwrap().to_owned() + "/.minecraft"))
            .to_owned()
    } else if env::consts::OS == "windows" {
        Path::new(&(dirs::config_dir().unwrap().to_str().unwrap().to_owned() + "\\.minecraft"))
            .to_owned()
    } else if env::consts::OS == "macos" {
        Path::new(&(dirs::config_dir().unwrap().to_str().unwrap().to_owned() + "/minecraft"))
            .to_owned()
    } else {
        panic!("Unsupported os '{}'!", env::consts::OS)
    }
}

fn get_modpack_root(modpack_uuid: &str) -> PathBuf {
    let root = get_minecraft_folder().join(Path::new(&format!(".WC_OVHL/{}", modpack_uuid)));
    fs::create_dir_all(&root).expect("Failed to create modpack folder");
    return root;
}

fn image_to_base64(img: &DynamicImage) -> String {
    let mut image_data: Vec<u8> = Vec::new();
    img.write_to(&mut Cursor::new(&mut image_data), ImageOutputFormat::Png)
        .unwrap();
    let res_base64 = engine::general_purpose::STANDARD.encode(image_data);
    format!("data:image/png;base64,{}", res_base64)
}

fn create_launcher_profile(
    manifest: &Manifest,
    modpack_root: &PathBuf,
    icon_img: Option<DynamicImage>,
) {
    let now = SystemTime::now();
    let now: DateTime<Utc> = now.into();
    let now = now.to_rfc3339();
    let version_id = format!(
        "fabric-loader-{}-{}",
        &manifest.loader.version, &manifest.loader.minecraft_version
    );
    let icon: String;
    if manifest.icon {
        icon = image_to_base64(&icon_img.expect("manifest.icon was true but no icon was supplied!"))
    } else {
        icon = String::from("Furnace");
    }
    let profile = LauncherProfile {
        lastUsed: now.to_string(),
        lastVersionId: version_id,
        created: now,
        name: manifest.name.clone(),
        icon: icon,
        r#type: String::from("custom"),
        gameDir: Some(modpack_root.to_str().unwrap().to_string()),
        javaDir: None,
        javaArgs: None,
        logConfig: None,
        logConfigIsXML: None,
        resolution: None,
    };
    let lp_file_path = get_minecraft_folder().join(Path::new("launcher_profiles.json"));
    let mut lp_obj: LauncherProfiles = serde_json::from_str(
        &fs::read_to_string(&lp_file_path).expect("Failed to read 'launcher_profiles.json'!"),
    )
    .expect("Failed to parse 'launcher_profiles.json'!");
    lp_obj.profiles.insert(manifest.uuid.clone(), profile);
    fs::write(
        lp_file_path,
        serde_json::to_string(&lp_obj).expect("Failed to create new 'launcher_profiles.json'!"),
    )
    .expect("Failed to write to 'launcher_profiles.json'");
}

#[async_recursion]
async fn read_gh(file: GitHubFile, http_client: &Client) -> Vec<ReadFile> {
    match file.r#type.as_str() {
        "file" => vec![read_gh_file(file, http_client).await],
        "dir" => read_gh_dir(file, http_client).await,
        _ => panic!("Unsupported GiHub type '{}'", file.r#type),
    }
}

async fn read_gh_dir(file: GitHubFile, http_client: &Client) -> Vec<ReadFile> {
    let resp = http_client
        .get(&file.url)
        .send()
        .await
        .expect(&format!("Failed to get include item '{}!'", file.path))
        .text()
        .await
        .unwrap();
    let mut files: Vec<ReadFile> = vec![];
    for new_file in read_gh_init(resp) {
        files.append(&mut read_gh(new_file, http_client).await);
    }
    return files;
}

async fn read_gh_file(file: GitHubFile, http_client: &Client) -> ReadFile {
    ReadFile {
        content: http_client
            .get(file.download_url.as_ref().unwrap())
            .send()
            .await
            .expect(&format!(
                "Failed to download include file '{}'",
                file.download_url.as_ref().unwrap()
            ))
            .bytes()
            .await
            .unwrap(),
        path: Path::new(&file.path).to_path_buf(),
    }
}

fn read_gh_init(resp: String) -> Vec<GitHubFile> {
    if resp.starts_with("[") {
        let dir: Vec<GitHubFile> =
            serde_json::from_str(&resp).expect("Failed to parse github directory api results!");
        dir
    } else {
        let file: GitHubFile =
            serde_json::from_str(&resp).expect("Failed to parse github file api results!");
        vec![file]
    }
}

fn build_http_client() -> Client {
    Client::builder()
        .user_agent("wynncraft-overhaul/installer/0.1.0 (commander#4392)")
        .build()
        .unwrap()
}

async fn install(modpack_root: &PathBuf, manifest: &Manifest, http_client: &Client) {
    let loader_future =
        manifest
            .loader
            .download(&modpack_root, &manifest.loader.r#type, &http_client);
    let mods_w_path =
        futures::stream::iter(manifest.mods.clone().into_iter().map(|r#mod| async move {
            if !r#mod.path.is_some() {
                Mod {
                    path: Some(
                        r#mod
                            .download(&modpack_root, &manifest.loader.r#type, &http_client)
                            .await,
                    ),
                    name: r#mod.name,
                    source: r#mod.source,
                    location: r#mod.location,
                    version: r#mod.version,
                }
            } else {
                r#mod
            }
        }))
        .buffer_unordered(CONCURRENCY)
        .collect::<Vec<Mod>>()
        .await;
    let shaderpacks_w_path = futures::stream::iter(manifest.shaderpacks.clone().into_iter().map(
        |shaderpack| async move {
            if !shaderpack.path.is_some() {
                Shaderpack {
                    path: Some(
                        shaderpack
                            .download(&modpack_root, &manifest.loader.r#type, &http_client)
                            .await,
                    ),
                    name: shaderpack.name,
                    source: shaderpack.source,
                    location: shaderpack.location,
                    version: shaderpack.version,
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
            |resourcepack| async move {
                if !resourcepack.path.is_some() {
                    Resourcepack {
                        path: Some(
                            resourcepack
                                .download(&modpack_root, &manifest.loader.r#type, &http_client)
                                .await,
                        ),
                        name: resourcepack.name,
                        source: resourcepack.source,
                        location: resourcepack.location,
                        version: resourcepack.version,
                    }
                } else {
                    resourcepack
                }
            },
        ))
        .buffer_unordered(CONCURRENCY)
        .collect::<Vec<Resourcepack>>()
        .await;
    // TODO(figure out how to handle asynchronous include downloads)
    for include in &manifest.include {
        let resp = http_client
            .get(
                GH_API.to_owned()
                    + MODPACK_SOURCE
                    + "contents/"
                    + include
                    + "?ref="
                    + MODPACK_BRANCH,
            )
            .send()
            .await
            .expect(&format!("Failed to get include item '{}!'", include))
            .text()
            .await
            .unwrap();
        for file in read_gh_init(resp) {
            for read_file in read_gh(file, &http_client).await {
                if let Some(p) = read_file.path.parent() {
                    fs::create_dir_all(modpack_root.join(p))
                        .expect("Failed to create include directory!")
                };
                fs::write(modpack_root.join(&read_file.path), read_file.content).expect(&format!(
                    "Failed to write to '{}'",
                    &read_file.path.to_str().unwrap()
                ));
            }
        }
    }
    let local_manifest = Manifest {
        manifest_version: manifest.manifest_version,
        modpack_version: manifest.modpack_version.clone(),
        name: manifest.name.clone(),
        icon: manifest.icon,
        uuid: manifest.uuid.clone(),
        loader: manifest.loader.clone(),
        mods: mods_w_path,
        shaderpacks: shaderpacks_w_path,
        resourcepacks: resourcepacks_w_path,
        include: manifest.include.clone(),
    };
    fs::write(
        get_modpack_root(&manifest.uuid).join(Path::new("manifest.json")),
        serde_json::to_string(&local_manifest).expect("Failed to parse 'manifest.json'!"),
    )
    .expect("Failed to save a local copy of 'manifest.json'!");
    let icon_img: Option<DynamicImage>;
    if manifest.icon {
        icon_img = Some(
            ImageReader::new(Cursor::new(
                http_client
                    .get(GH_RAW.to_owned() + MODPACK_SOURCE + MODPACK_BRANCH + "/icon.png")
                    .send()
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
        );
    } else {
        icon_img = None
    }
    create_launcher_profile(&manifest, &modpack_root, icon_img);
    loader_future.await;
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

async fn update(modpack_root: &PathBuf, manifest: &Manifest, http_client: &Client) {
    // TODO(figure out how to handle 'include' updates) current behaviour is writing over existing includes and files
    // TODO(change this to be idiomatic and good) im not sure if the 'remove_items' macro should exist and if it should then maybe the filtering could be turned into a macro too
    let local_manifest: Manifest =
        match fs::read_to_string(modpack_root.join(Path::new("manifest.json"))) {
            Ok(contents) => match serde_json::from_str(&contents) {
                Ok(parsed) => parsed,
                Err(err) => panic!("Failed to parse local manifest: {}", err),
            },
            Err(err) => panic!("Failed to read local manifest: {}", err),
        };

    let new_mods: Vec<Mod> = manifest
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
    let new_shaderpacks: Vec<Shaderpack> = manifest
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
    let new_resourcepacks: Vec<Resourcepack> = manifest
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
    if manifest.loader != local_manifest.loader {
        fs::remove_dir_all(modpack_root.join(&Path::new(&format!(
            "versions/{}",
            format!(
                "fabric-loader-{}-{}",
                &local_manifest.loader.version, &local_manifest.loader.minecraft_version
            )
        ))))
        .expect("Could not delete old fabric version!");
    }
    install(
        modpack_root,
        &Manifest {
            manifest_version: manifest.manifest_version,
            modpack_version: manifest.modpack_version.clone(),
            name: manifest.name.clone(),
            icon: manifest.icon,
            uuid: manifest.uuid.clone(),
            loader: manifest.loader.clone(),
            mods: new_mods,
            shaderpacks: new_shaderpacks,
            resourcepacks: new_resourcepacks,
            include: manifest.include.clone(),
        },
        http_client,
    )
    .await;
}

#[tokio::main]
async fn main() {
    let http_client = build_http_client();
    let manifest: Manifest = serde_json::from_str(
        http_client
            .get(GH_RAW.to_owned() + MODPACK_SOURCE + MODPACK_BRANCH + "/manifest.json")
            .send()
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
    let modpack_root = get_modpack_root(&manifest.uuid);
    let installed = modpack_root.join(Path::new("manifest.json")).exists();
    let mut update_available = false;
    if installed {
        let local_manifest: Manifest = serde_json::from_str(
            &fs::read_to_string(modpack_root.join(Path::new("manifest.json")))
                .expect("Failed to read local manifest!"),
        )
        .expect("Failed to parse local manifest!");
        if &manifest.modpack_version != &local_manifest.modpack_version {
            update_available = true
        }
    }
    if update_available {
        update(&modpack_root, &manifest, &http_client).await;
    } else if !installed {
        install(&modpack_root, &manifest, &http_client).await;
    }
    println!("Success!")
}
