# Manifest Documentation

The `manifest.json` file is a JSON-formatted file used to define the contents of a Minecraft modpack. This document provides documentation on how to create a manifest.json file for your modpack.

If you want to see a modpack example go [here](https://github.com/Wynncraft-Overhaul/modpack-example).

## Header

The Header section contains metadata about the modpack, such as the name and the Minecraft version it targets.

- `manifest_version`: This field represents the manifest version the modpack was created for and should be automatically generated upon the creation of a new modpack.
- `modpack_version`: This field is a [semver](https://semver.org/) for your modpack.
- `name`: This field specifies the modpack name. This can be any string.
- `uuid`: This field is generated upon the creation of a new modpack and should be the same across all modpack versions. **Do not change this field**.
- `icon`: If this field is set to `true` the installer will look for an `icon.png` in the modpack root.

## Loader

The `loader` section specifies the target mod loader for the modpack.

- `type`: This field specifies the target mod loader. Currently supported values are: `fabric`.
- `version`: This field specifies the target mod loader version. Make sure this is compatible with your target Minecraft version.
- `minecraft_version`: This field specifies the target Minecraft version. Make sure that the loader version supports it.

## Mods

Mods is a list which contains mod objects for which the fields are:

- `name`: This field specifies the name of the mod. This does not have to match the actual mod name, but it's best to make sure it matches.
- `source`: This field specifies where the mod comes from. Currently supported values are: `modrinth`, `ddl`.
- `location`: If `source` is set to `modrinth`, then this should be set to the mod's slug (the part after `mod/` in the URL). If `source` is set to `ddl`, then this should be a direct download link. Note that links that redirect are not direct download links.
- `version`: If `source` is set to `modrinth`, then this must be set to exactly the same as the version number of the mod you want to download. However, if source is set to `ddl`, then this can be anything, but it's best to set it to the actual version.

## Shaderpacks

The `shaderpacks` section works exactly the same as the Mods section.

## Resourcepacks

The `resourcepacks` section works exactly the same as the Mods section.

## Include

`include` is a list of strings (paths) which contains all the files or folders you want to ship with the modpack. This is most useful for configs.
