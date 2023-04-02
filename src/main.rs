use base64::{engine, Engine};
use chrono::{DateTime, Utc};
use image::io::Reader as ImageReader;
use image::{DynamicImage, ImageOutputFormat};
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
    fn download(&self, modpack_root: &PathBuf);
}
#[derive(Debug, Deserialize, Serialize)]
struct Mod {
    name: String,
    source: String,
    location: String,
    version: String,
}
#[derive(Debug, Deserialize, Serialize)]
struct Shaderpack {
    name: String,
    source: String,
    location: String,
    version: String,
}
#[derive(Debug, Deserialize, Serialize)]
struct Resourcepack {
    name: String,
    source: String,
    location: String,
    version: String,
}
#[derive(Debug, Deserialize, Serialize)]
struct Loader {
    r#type: String,
    version: String,
    minecraft_version: String,
}
impl Downloadable for Loader {
    fn download(&self, modpack_root: &PathBuf) {
        match self.r#type.as_str() {
            "fabric" => download_fabric(&self, modpack_root),
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
    gamedir: Option<String>,
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

fn download_fabric(loader: &Loader, modpack_root: &PathBuf) {
    let url = format!(
        "https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json",
        loader.minecraft_version, loader.version
    );
    let loader_name = format!(
        "fabric-loader-{}-{}",
        &loader.version, &loader.minecraft_version
    );
    let resp = reqwest::blocking::get(url.as_str())
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

fn create_launcher_profile(manifest: &Manifest, modpack_root: &PathBuf) {
    let now = SystemTime::now();
    let now: DateTime<Utc> = now.into();
    let now = now.to_rfc3339();
    let version_id = format!(
        "fabric-loader-{}-{}",
        &manifest.loader.version, &manifest.loader.minecraft_version
    );
    let icon: String;
    if manifest.icon {
        icon = image_to_base64(
            &ImageReader::open("test/icon.png")
                .expect("Failed to read icon!")
                .decode()
                .expect("Failed to decode icon!"),
        )
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
        gamedir: Some(modpack_root.to_str().unwrap().to_string()),
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

fn main() {
    let json: String = fs::read_to_string(Path::new("test/manifest.json"))
        .expect("Failed to read 'test/manifest.json'!")
        .parse()
        .expect("Failed to parse 'test/manifest.json' as string!");
    let manifest: Manifest = serde_json::from_str(&json).expect("Failed to parse json!");
    // TODO(Figure out a way to support older manifest versions)
    assert!(
        CURRENT_MANIFEST_VERSION == manifest.manifest_version,
        "Unsupported manifest version '{}'!",
        manifest.manifest_version
    );

    let modpack_root = get_modpack_root(&manifest.uuid);
    manifest.loader.download(&modpack_root);
    create_launcher_profile(&manifest, &modpack_root)
}
