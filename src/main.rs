use semver::{BuildMetadata, Prerelease, Version, VersionReq};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

const CURRENT_MANIFEST_VERSION: Version = Version {
    major: 0,
    minor: 1,
    patch: 1,
    pre: Prerelease::EMPTY,
    build: BuildMetadata::EMPTY,
};

trait Downloadable {
    fn download(&self, modpack_root: &str);
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

#[derive(Debug, Deserialize, Serialize)]
struct Manifest {
    manifest_version: String,
    modpack_version: String,
    name: String,
    uuid: String,
    mods: Vec<Mod>,
    shaderpacks: Vec<Shaderpack>,
    resourcepacks: Vec<Resourcepack>,
    include: Vec<String>,
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
}
