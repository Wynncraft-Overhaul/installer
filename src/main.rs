use semver::{BuildMetadata, Prerelease, Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

const CURRENT_MANIFEST_VERSION: Version = Version {
    major: 0,
    minor: 1,
    patch: 1,
    pre: Prerelease::EMPTY,
    build: BuildMetadata::EMPTY,
};

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
    manifest_version: String,
    modpack_version: String,
    name: String,
    uuid: String,
    loader: Loader,
    mods: Vec<Mod>,
    shaderpacks: Vec<Shaderpack>,
    resourcepacks: Vec<Resourcepack>,
    include: Vec<String>,
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

fn main() {
    let json: String = fs::read_to_string(Path::new("test/manifest.json"))
        .expect("Failed to read 'test/manifest.json'!")
        .parse()
        .expect("Failed to parse 'test/manifest.json' as string!");
    let manifest: Manifest = serde_json::from_str(&json).expect("Failed to parse json!");
    let req = VersionReq::parse(&(">=".to_owned() + &manifest.manifest_version))
        .expect("Invalid manifest version");
    // TODO(Better manifest version handling)
    assert!(
        req.matches(&CURRENT_MANIFEST_VERSION),
        "Unsupported manifest version '{}'!",
        manifest.manifest_version
    );

    let modpack_root = get_modpack_root(&manifest.uuid);
    manifest.loader.download(&modpack_root);
}
