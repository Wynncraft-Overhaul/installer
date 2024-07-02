use std::fs;

extern crate winres;

fn main() {
    let _ = fs::create_dir("dist");
    fs::File::create("dist/__assets_head.html").unwrap();

    if cfg!(target_os = "windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("icon_256.ico");
        res.compile().unwrap();
    }
}
