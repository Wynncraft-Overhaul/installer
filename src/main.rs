use base64::{engine, Engine};
use bytes::Bytes;
use chrono::{DateTime, Utc};
use image::io::Reader as ImageReader;
use image::{DynamicImage, ImageOutputFormat};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{
    env, fs,
    io::Cursor,
    path::{Path, PathBuf},
    time::SystemTime,
};

const CURRENT_MANIFEST_VERSION: i32 = 1;
trait Downloadable {
    fn download(&self, modpack_root: &PathBuf, loader_type: &String, http_client: &Client);
}
trait DownloadableGetters {
    fn get_name(&self) -> &String;
    fn get_location(&self) -> &String;
    fn get_version(&self) -> &String;
}
#[derive(Debug, Deserialize, Serialize)]
struct Mod {
    name: String,
    source: String,
    location: String,
    version: String,
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
impl Downloadable for Mod {
    fn download(&self, modpack_root: &PathBuf, loader_type: &String, http_client: &Client) {
        match self.source.as_str() {
            "modrinth" => {
                download_from_modrinth(self, modpack_root, loader_type, "mod", http_client)
            }
            "ddl" => download_from_ddl(self, modpack_root, "mod", http_client),
            _ => panic!("Unsupported source '{}'!", self.source.as_str()),
        }
    }
}
#[derive(Debug, Deserialize, Serialize)]
struct Shaderpack {
    name: String,
    source: String,
    location: String,
    version: String,
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
impl Downloadable for Shaderpack {
    fn download(&self, modpack_root: &PathBuf, loader_type: &String, http_client: &Client) {
        match self.source.as_str() {
            "modrinth" => {
                download_from_modrinth(self, modpack_root, loader_type, "shaderpack", http_client)
            }
            "ddl" => download_from_ddl(self, modpack_root, "shaderpack", http_client),
            _ => panic!("Unsupported source '{}'!", self.source.as_str()),
        }
    }
}
#[derive(Debug, Deserialize, Serialize)]
struct Resourcepack {
    name: String,
    source: String,
    location: String,
    version: String,
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
impl Downloadable for Resourcepack {
    fn download(&self, modpack_root: &PathBuf, loader_type: &String, http_client: &Client) {
        match self.source.as_str() {
            "modrinth" => {
                download_from_modrinth(self, modpack_root, loader_type, "resourcepack", http_client)
            }
            "ddl" => download_from_ddl(self, modpack_root, "resourcepack", http_client),
            _ => panic!("Unsupported source '{}'!", self.source.as_str()),
        }
    }
}
#[derive(Debug, Deserialize, Serialize)]
struct Loader {
    r#type: String,
    version: String,
    minecraft_version: String,
}
impl Downloadable for Loader {
    fn download(&self, modpack_root: &PathBuf, _: &String, http_client: &Client) {
        match self.r#type.as_str() {
            "fabric" => download_fabric(&self, modpack_root, http_client),
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

fn download_fabric(loader: &Loader, modpack_root: &PathBuf, http_client: &Client) {
    let url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json",
        loader.minecraft_version, loader.version
    );
    let loader_name = format!(
        "fabric-loader-{}-{}",
        &loader.version, &loader.minecraft_version
    );
    let resp = http_client
        .get(url.as_str())
        .send()
        .expect("Failed to download fabric loader!")
        .text()
        .unwrap();
    let fabric_path = modpack_root.join(&Path::new(&format!("versions/{}", &loader_name)));
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
}

fn download_from_ddl<T: Downloadable + DownloadableGetters>(
    item: &T,
    modpack_root: &PathBuf,
    r#type: &str,
    http_client: &Client,
) {
    let content = http_client
        .get(item.get_location())
        .send()
        .expect(&format!("Failed to download '{}'!", item.get_name()))
        .bytes()
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
    fs::write(
        dist.join(Path::new(item.get_location().split("/").last().expect(
            &format!(
                "Could not determine file name for ddl: '{}'!",
                item.get_location()
            ),
        ))),
        &content,
    )
    .expect("Failed to write ddl item!");
}

fn download_from_modrinth<T: Downloadable + DownloadableGetters>(
    item: &T,
    modpack_root: &PathBuf,
    loader_type: &String,
    r#type: &str,
    http_client: &Client,
) {
    let resp = http_client
        .get(format!(
            "https://api.modrinth.com/v2/project/{}/version",
            item.get_location()
        ))
        .send()
        .expect(&format!("Failed to download '{}'!", item.get_name()))
        .text()
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
                    .expect(&format!("Failed to download '{}'!", item.get_name()))
                    .bytes()
                    .unwrap();
                fs::write(dist.join(Path::new(&_mod.files[0].filename)), &content)
                    .expect("Failed to write modrinth item!");
            }
        }
    }
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

fn read_gh(file: GitHubFile, http_client: &Client) -> Vec<ReadFile> {
    match file.r#type.as_str() {
        "file" => vec![read_gh_file(file, http_client)],
        "dir" => read_gh_dir(file, http_client),
        _ => panic!("Unsupported GiHub type '{}'", file.r#type),
    }
}

fn read_gh_dir(file: GitHubFile, http_client: &Client) -> Vec<ReadFile> {
    let resp = http_client
        .get(&file.url)
        .send()
        .expect(&format!("Failed to get include item '{}!'", file.path))
        .text()
        .unwrap();
    let mut files: Vec<ReadFile> = vec![];
    for new_file in read_gh_init(resp) {
        files.append(&mut read_gh(new_file, http_client));
    }
    return files;
}

fn read_gh_file(file: GitHubFile, http_client: &Client) -> ReadFile {
    ReadFile {
        content: http_client
            .get(file.download_url.as_ref().unwrap())
            .send()
            .expect(&format!(
                "Failed to download include file '{}'",
                file.download_url.as_ref().unwrap()
            ))
            .bytes()
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

fn main() {
    let gh_api = String::from("https://api.github.com/repos/");
    let gh_raw = String::from("https://raw.githubusercontent.com/");
    let modpack_source = "Commander07/modpack-test/";
    let modpack_branch = "main";
    let http_client = build_http_client();
    let manifest: Manifest = serde_json::from_str(
        http_client
            .get(gh_raw.clone() + &modpack_source + &modpack_branch + "/manifest.json")
            .send()
            .expect("Failed to retrieve manifest!")
            .text()
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
    fs::write(
        get_modpack_root(&manifest.uuid).join(Path::new("manifest.json")),
        serde_json::to_string(&manifest).expect("Failed to parse 'manifest.json'!"),
    )
    .expect("Failed to save a local copy of 'manifest.json'!");
    manifest
        .loader
        .download(&modpack_root, &manifest.loader.r#type, &http_client);
    for _mod in &manifest.mods {
        _mod.download(&modpack_root, &manifest.loader.r#type, &http_client);
    }
    for resourcepack in &manifest.resourcepacks {
        resourcepack.download(&modpack_root, &manifest.loader.r#type, &http_client);
    }
    for shaderpack in &manifest.shaderpacks {
        shaderpack.download(&modpack_root, &manifest.loader.r#type, &http_client)
    }
    for include in &manifest.include {
        let resp = http_client
            .get(gh_api.clone() + &modpack_source + "contents/" + include)
            .send()
            .expect(&format!("Failed to get include item '{}!'", include))
            .text()
            .unwrap();
        for file in read_gh_init(resp) {
            for read_file in read_gh(file, &http_client) {
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
    let icon_img: Option<DynamicImage>;
    if manifest.icon {
        icon_img = Some(
            ImageReader::new(Cursor::new(
                http_client
                    .get(gh_raw.clone() + &modpack_source + &modpack_branch + "/icon.png")
                    .send()
                    .expect("Failed to download icon")
                    .bytes()
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
}
