# Manifest Documentation

The `manifest.json` file is a JSON-formatted file used to define the contents of the modpack. This document provides documentation on how the file is structured. Before changing anything it is recommend to run the file contents through a [JSON linter](https://jsonlint.com/) to make sure the syntax is correct. Linting does **not** check that the structure of the manifest file is correct.

## Header

The Header section contains metadata about the modpack, such as the name, subtitle and jvm arguments.

- `manifest_version`: This field represents the manifest version the modpack was created for. (Current version is `3`)
- `modpack_version`: This field is the modpack version. It has to change for the installer to know an update is available.
- `name`: This field specifies the modpack name. This can be any string.
- `subtitle`: Name of modpack version
- `tab_group`: The id of the tab the version will appear in. The id can be any non negative number. `0` is the default tab.
- `tab_title`: The name of the tab.
- `tab_color`: The background color of the boxes containing the modpack versions. In `#rrbbgg` format.
- `tab_background`: The url of the background image for the tab.
- `popup_title`: Adds a title to the pre install popup.
- `popup_contents`: If specified a popup will appear before install with an option to cancel. This field contains the contents of that popup.
- `description`: This field is a html representation of the description show in the installer.
- `uuid`: This field is a [UUID4](https://www.uuidgenerator.net/) and should be the same across all modpack versions. But different across branches/alt versions.
- `icon`: If this field is set to `true` the installer will look for an `icon.png` in the modpack root.
- `max_mem`: Optional Xmx field (mb)
- `min_mem`: Optional Xms field (mb)
- `java_args`: Optional field for arguments to be passed to the jvm

## Loader

The `loader` section specifies the target mod loader for the modpack.

- `type`: This field specifies the target mod loader. Currently supported loaders are: `fabric` and `quilt`.
- `version`: This field specifies the target mod loader version. Make sure this is compatible with your target Minecraft version.
- `minecraft_version`: This field specifies the target Minecraft version. Make sure that the loader version supports it.

## Mods

Mods is a list which contains mod objects for which the fields are:

- `name`: This field specifies the name of the mod. This does not have to match the actual mod name, but it's best to make sure it matches.
- `source`: This field specifies where the mod comes from. Currently supported values are: `modrinth`, `ddl` and `mediafire`.
- `location`: If `source` is set to `modrinth`, then this should be set to the mod's slug (the part after `mod/` in the URL). If `source` is set to `ddl`, then this should be a direct download link. Note that links that redirect are not direct download links. For `mediafire` mods it should be the link to the download page.
- `version`: If `source` is set to `modrinth`, then this must be set to exactly the same as the version number of the mod you want to download. However, if source is set to `ddl` or `mediafire`, then this can be anything, but it's best to set it to the actual version to improve clarity. This is also used for checking if a mod needs to be updated, which means it needs to change between mod versions, to properly update.
- `id`: This is an optional field which defaults to `default` it is the id of the feature requried to be true in order to be included. (`default` is always true)
- `authors`: This is a list with objects which the following fields:
  - `name`: This field is the authors name.
  - `link`: This field is a link to the authors page.

## Shaderpacks

The `shaderpacks` section works exactly the same as the Mods section.

## Resourcepacks

The `resourcepacks` section works exactly the same as the Mods section.

## Include

Include is a list of include objects for which the fields are:

- `location`: Path of the file or folder you want to include
- `id`: This is an optional field which defaults to `default` it is the id of the feature requried to be true in order to be included. (`default` is always true)
- `name`: Optional but required for include to be listed in the credits screen. Name for the included file.
- `authors`: Optional but required for include to be listed in the credits screen. List with objects which have the following fields:
  - `name`: This field is the authors name.
  - `link`: This field is a link to the authors page.

## Features

Features is a list which contains feature objects for which the fields are:

- `name`: Name of the feature displayed in the installer
- `id`: Id of the feature
- `default`: This is a bool specifying if it should be on by default
- `hidden`: When set to true the feature wont be displayed in the installer. This can be used to section off the default includes to improve update speeds. This field can be omitted, which causes it to be visible.
