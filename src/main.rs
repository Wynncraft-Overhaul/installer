use async_trait::async_trait;
use base64::{engine, Engine};
use chrono::{DateTime, Utc};
use clap::Parser;
use futures::StreamExt;
use iced::{Application, Command};
use image::io::Reader as ImageReader;
use image::{DynamicImage, ImageOutputFormat};
use isahc::config::RedirectPolicy;
use isahc::prelude::Configurable;
use isahc::{AsyncReadResponseExt, HttpClient, ReadResponseExt};
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
const CONCURRENCY: usize = 14;
#[async_trait]
trait Downloadable {
    async fn download(
        &self,
        modpack_root: &PathBuf,
        loader_type: &String,
        http_client: &HttpClient,
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
        http_client: &HttpClient,
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
        http_client: &HttpClient,
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
        http_client: &HttpClient,
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
    async fn download(
        &self,
        modpack_root: &PathBuf,
        _: &String,
        http_client: &HttpClient,
    ) -> PathBuf {
        match self.r#type.as_str() {
            "fabric" => download_fabric(&self, modpack_root, http_client).await,
            _ => panic!("Unsupported loader '{}'!", self.r#type.as_str()),
        }
    }
}
#[derive(Debug, Deserialize, Serialize, Clone)]
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
struct GithubRepo {
    // Theres a lot more fields but we only care about default_branch
    // https://docs.github.com/en/rest/repos/repos?apiVersion=2022-11-28#get-a-repository
    default_branch: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct GithubRelease {
    assets: Vec<GithubAsset>,
}

async fn download_fabric(
    loader: &Loader,
    modpack_root: &PathBuf,
    http_client: &HttpClient,
) -> PathBuf {
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
        .get_async(url.as_str())
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
    http_client: &HttpClient,
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
    http_client: &HttpClient,
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
        if &_mod.version_number == item.get_version() {
            if _mod.loaders.contains(&String::from("minecraft"))
                || _mod.loaders.contains(&String::from(loader_type))
                || r#type == "shaderpack"
            {
                let content = http_client
                    .get_async(&_mod.files[0].url)
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
    let icon = if manifest.icon {
        image_to_base64(&icon_img.expect("manifest.icon was true but no icon was supplied!"))
    } else {
        String::from("Furnace")
    };
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

async fn install(
    modpack_root: PathBuf,
    manifest: Manifest,
    http_client: HttpClient,
    modpack_source: String,
    modpack_branch: String,
) {
    // Yes this is needed and no i wont change it
    // This might not be needed now to we use isahc
    // TODO(Remove unnecessary clone usage)
    let modpack_root = &modpack_root;
    let manifest = &manifest;
    let http_client = &http_client;
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
    if manifest.include.len() > 0 {
        // Include files exist
        let release: GithubRelease = serde_json::from_str(
            http_client
                .get_async(GH_API.to_owned() + modpack_source.as_str() + "releases/latest")
                .await
                .expect("Failed to retrieve 'include' release from tag 'latest'!")
                .text()
                .await
                .unwrap()
                .as_str(),
        )
        .expect("Failed to parse release response!");
        for asset in release.assets {
            if asset.name == *"include.zip" {
                // download and unzip in modpack root
                let content = http_client
                    .get_async(asset.browser_download_url)
                    .await
                    .expect("Failed to download 'include.zip'")
                    .bytes()
                    .await
                    .unwrap();
                let zipfile_path = modpack_root.join(Path::new("include.zip"));
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
                    .get_async(
                        GH_RAW.to_owned()
                            + modpack_source.as_str()
                            + modpack_branch.as_str()
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
// Why haven't I split this into multiple files? That's a good question. I forgot, and I can't be bothered to do it now.
// TODO(Split project into multiple files to improve maintainability)
async fn update(
    modpack_root: PathBuf,
    manifest: Manifest,
    http_client: HttpClient,
    modpack_source: String,
    modpack_branch: String,
) {
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
        Manifest {
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
        modpack_source,
        modpack_branch,
    )
    .await;
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        // Multiple arguments detected entering CLI mode
        let args = CLIArgs::parse();
        let branch: String;
        if args.branch.is_none() {
            let repo: GithubRepo = serde_json::from_str(
                isahc::get(GH_API.to_owned() + &args.modpack)
                    .expect("Failed to gather modpack repo info!")
                    .text()
                    .unwrap()
                    .as_str(),
            )
            .expect("Could not retrieve default branch try specifying it using '--branch'!");
            branch = repo.default_branch;
        } else {
            branch = args.branch.unwrap();
        }
        let installer_profile = futures::executor::block_on(init(&args.modpack, &branch));
        match args.action.as_str() {
            "install" => futures::executor::block_on(install(
                installer_profile.modpack_root,
                installer_profile.manifest,
                installer_profile.http_client,
                installer_profile.modpack_source,
                installer_profile.modpack_branch,
            )),
            "update" => futures::executor::block_on(update(
                installer_profile.modpack_root,
                installer_profile.manifest,
                installer_profile.http_client,
                installer_profile.modpack_source,
                installer_profile.modpack_branch,
            )),
            "play" => (),
            _ => (),
        };
        println!("Success!");
    } else {
        // Only the executable was present in arguments entering GUI mode
        InstallerGUI::run(iced::Settings::default()).expect("Failed to create GUI!");
    }
}

#[derive(Debug, Clone)]
struct InstallerProfile {
    manifest: Manifest,
    modpack_root: PathBuf,
    http_client: HttpClient,
    installed: bool,
    update_available: bool,
    modpack_source: String,
    modpack_branch: String,
}

#[derive(Debug, Clone)]
enum InstallerGUI {
    Loading,
    Loaded {
        manifest: Manifest,
        modpack_root: PathBuf,
        http_client: HttpClient,
        installed: bool,
        update_available: bool,
        modpack_source: String,
        modpack_branch: String,
    },
}
#[derive(Debug, Clone)]
enum Message {
    Install,
    Update,
    Play,
    Updated(()),
    Installed(()),
    Init(InstallerProfile),
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
}

async fn init(modpack_source: &str, modpack_branch: &str) -> InstallerProfile {
    let http_client = build_http_client();
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
    let modpack_root = get_modpack_root(&manifest.uuid);
    let installed = modpack_root.join(Path::new("manifest.json")).exists();
    let update_available: bool;
    if installed {
        let local_manifest: Manifest = serde_json::from_str(
            &fs::read_to_string(modpack_root.join(Path::new("manifest.json")))
                .expect("Failed to read local manifest!"),
        )
        .expect("Failed to parse local manifest!");
        if &manifest.modpack_version != &local_manifest.modpack_version {
            update_available = true;
        } else {
            update_available = false;
        }
    } else {
        update_available = false;
    }
    InstallerProfile {
        manifest,
        modpack_root,
        http_client,
        installed,
        update_available,
        modpack_source: modpack_source.to_owned(),
        modpack_branch: modpack_branch.to_owned(),
    }
}

impl Application for InstallerGUI {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Theme = iced::Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        // TODO(Ability to change 'modpack_source' and 'modpack_branch')
        (
            InstallerGUI::Loading,
            Command::perform(init("Commander07/modpack-test/", "main"), Message::Init),
        )
    }

    fn title(&self) -> String {
        let subtitle = match self {
            InstallerGUI::Loading => "Loading",
            InstallerGUI::Loaded { manifest, .. } => manifest.name.as_str(),
        };

        format!("{} | Wynncraft Overhaul Installer", subtitle)
    }

    fn update(&mut self, message: Self::Message) -> iced::Command<Self::Message> {
        match self {
            InstallerGUI::Loading => match message {
                Message::Init(profile) => {
                    *self = InstallerGUI::Loaded {
                        manifest: profile.manifest,
                        modpack_root: profile.modpack_root,
                        http_client: profile.http_client,
                        installed: profile.installed,
                        update_available: profile.update_available,
                        modpack_source: profile.modpack_source,
                        modpack_branch: profile.modpack_branch,
                    };
                    Command::none()
                }
                _ => Command::none(),
            },
            InstallerGUI::Loaded {
                manifest,
                modpack_root,
                http_client,
                installed,
                update_available,
                modpack_source,
                modpack_branch,
            } => match message {
                Message::Install => {
                    if !*installed {
                        return Command::perform(
                            install(
                                modpack_root.clone(),
                                manifest.clone(),
                                http_client.clone(),
                                modpack_source.clone(),
                                modpack_branch.clone(),
                            ),
                            Message::Installed,
                        );
                    }
                    Command::none()
                }
                Message::Play => Command::none(),
                Message::Update => {
                    if *update_available {
                        return Command::perform(
                            update(
                                modpack_root.clone(),
                                manifest.clone(),
                                http_client.clone(),
                                modpack_source.clone(),
                                modpack_branch.clone(),
                            ),
                            Message::Updated,
                        );
                    }
                    Command::none()
                }
                Message::Installed(()) => {
                    *installed = true;
                    Command::none()
                }
                _ => Command::none(),
            },
        }
    }

    fn view(&self) -> iced::Element<Message> {
        let content = match self {
            InstallerGUI::Loading => {
                iced::widget::column![iced::widget::text("Loading...").size(40),]
                    .width(iced::Length::Shrink)
            }
            InstallerGUI::Loaded {
                manifest,
                modpack_root,
                http_client,
                installed,
                update_available,
                modpack_source,
                modpack_branch,
            } => iced::widget::column![
                iced::widget::text(&manifest.name).size(40),
                iced::widget::button("Install").on_press(Message::Install),
                iced::widget::button("Update").on_press(Message::Update)
            ]
            .width(iced::Length::Shrink),
        };

        iced::widget::container(content)
            .width(iced::Length::Fill)
            .height(iced::Length::Fill)
            .center_x()
            .center_y()
            .into()
    }
}
