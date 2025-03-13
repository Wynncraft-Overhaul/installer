{ pkgs ? import <nixpkgs> { }, pkg-config, openssl, glib, gtk3, webkitgtk_4_1, xdotool, wrapGAppsHook3, glib-networking }:
let manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
in
pkgs.rustPlatform.buildRustPackage rec {
  pname = manifest.name;
  version = manifest.version;

  cargoLock.lockFile = ./Cargo.lock;

  src = pkgs.lib.cleanSource ./.;
  buildType = "debug";

  nativeBuildInputs = [ pkg-config wrapGAppsHook3 ];
  buildInputs = [ openssl glib gtk3 webkitgtk_4_1 xdotool glib-networking ];
}
