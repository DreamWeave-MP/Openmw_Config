+++
title = "Configuration"
description = "Every method of the configuration object: where it came from, the chain, plugins, directories, settings, saving, and the rows it returns."
weight = 20

[extra]
kind = "api"
+++

The object the [loaders](@/docs/lua/module.md#loaders) return: a whole chain, loaded. Call its
methods with `:`. Each mirrors a Rust method, named in the last column; the
[Rust API](@/docs/api/_index.md) has the full behavior, and the [guides](@/docs/editing.md) the
traps.

```lua
local cfg = openmwConfig.fromEnvOrUserConfig()

for _, plugin in ipairs(cfg:contentFiles()) do
    print(plugin)
end

if not cfg:hasContentFile("MyPlugin.esp") then
    cfg:addContentFile("MyPlugin.esp")
end
cfg:saveUser()
```

## Where it came from

| Method | Returns | Rust |
|---|---|---|
| `rootConfigFile()` | The `openmw.cfg` the chain started at | `root_config_file` |
| `rootConfigDir()` | Its directory | `root_config_dir` |
| `userConfigPath()` | The user's config directory, where `saveUser()` writes | `user_config_path` |
| `isUserConfig()` | Whether the root is the user's config | `is_user_config` |
| `userConfig()` | A new configuration loaded from the user's config | `user_config_ref` |

## The chain

| Method | Returns | Rust |
|---|---|---|
| `configChain()` | Every file the loader tried, as [rows](#row-shapes) | `config_chain` |
| `subConfigs()` | The `config=` directories in effect, as strings | `sub_configs` |

## Plugins and archives

| Method | Does | Rust |
|---|---|---|
| `contentFiles()` | The `content=` names, in load order | `content_files_iter` |
| `groundcoverFiles()` | The `groundcover=` names | `groundcover_iter` |
| `fallbackArchives()` | The `fallback-archive=` names | `fallback_archives_iter` |
| `hasContentFile(name)`, `hasGroundcoverFile(name)`, `hasArchiveFile(name)` | Whether the name is listed | `has_*` |
| `addContentFile(name)`, `addGroundcoverFile(name)`, `addArchiveFile(name)` | Appends it; raises if it is already listed | `add_*` |
| `removeContentFile(name)`, `removeGroundcoverFile(name)`, `removeArchiveFile(name)` | Removes every entry with the name | `remove_*` |
| `setContentFiles(list)`, `setFallbackArchives(list)` | Replaces the whole list; `nil` empties it | `set_content_files`, `set_fallback_archives` |

Names compare exactly, case included.

## Directories

| Method | Does | Rust |
|---|---|---|
| `dataDirectories()` | The data directories, resolved, in load order | `data_directories_iter` |
| `hasDataDir(path)` | Whether a data directory resolves to `path` | `has_data_dir` |
| `addDataDirectory(path)` | Appends one | `add_data_directory` |
| `removeDataDirectory(path)` | Removes every `data=` entry resolving to, or written as, `path` | `remove_data_directory` |
| `setDataDirectories(list)` | Replaces the `data=` entries; `nil` leaves none. `resources/vfs` and `data-local` stay | `set_data_directories` |
| `userData()`, `resources()`, `dataLocal()` | The directory, resolved, or `nil` | `userdata`, `resources`, `data_local` |
| `setUserData(path)`, `setResources(path)`, `setDataLocal(path)` | Sets it; `nil` removes the last definition | `set_*_path`, `clear_*` |

## Settings

| Method | Does | Rust |
|---|---|---|
| `gameSettings()` | Every fallback key once, with its current value, as [rows](#row-shapes); most recently defined first | `game_settings` |
| `getGameSetting(key)` | One key's current value as a row, or `nil`. Case-sensitive | `get_game_setting` |
| `setGameSetting(value, sourcePath, comment)` | Adds a definition, `"Key,Value"`; see below | `set_game_setting` |
| `setGameSettings(list)` | Replaces every fallback setting; `nil` leaves none | `set_game_settings` |
| `genericSettings()` | Every line with a key the crate does not recognize, as rows | `generic_settings_iter` |
| `addGenericSetting(key, value)` | Appends `key=value` | `add_generic_setting` |
| `setGenericSettings(key, list)` | Replaces every entry with `key`; `nil` removes them | `set_generic_settings` |
| `encoding()` | `"win1250"`, `"win1251"`, `"win1252"` or `nil` | `encoding` |
| `setEncoding(name)` | Sets `encoding=`; see below | `set_encoding` |

### setGameSetting

`value` is written as it would be after `fallback=`: `"Weather_Clear_Sky_Sunrise_Color,160,170,190"`.
`sourcePath` is the `openmw.cfg` to attribute it to, `nil` for the user's. `comment`, or `nil`,
is written above the line, with `# ` added to each line that lacks it:

```lua
cfg:setGameSetting("Weather_Clear_Sky_Sunrise_Color,160,170,190", nil, "Brighter dawns")
```

Raises when `value` has no comma. The new definition wins because it is last; earlier ones stay in
their files.

### setEncoding

Takes `"win1250"`, `"win1251"` or `"win1252"`, exactly, attributed to the user's config, and
raises on anything else. `nil` removes the last `encoding=`.

## Saving

| Method | Does | Rust |
|---|---|---|
| `saveUser()` | Writes the user's `openmw.cfg`, preserved | `save_user` |
| `saveSubconfig(dir)` | Writes the `openmw.cfg` of a `config=` directory in the chain | `save_subconfig` |
| `saveToPath(path)` | Writes the whole chain, preserved, to `path` | `save_to_path` |
| `saveResolvedToPath(path)` | Writes the whole chain, flattened, to `path` | `save_resolved_to_path` |
| `toString()` | The whole chain, preserved, as a string | `Display` |
| `toResolvedString()` | The whole chain, flattened, as a string | `to_resolved_string` |

[Saving and exporting](@/docs/saving.md) explains preserved and flattened output. `tostring(cfg)`
does not serialize: call `cfg:toString()`.

## Row shapes

| Rows from | Fields |
|---|---|
| `configChain()` | `path`: string · `depth`: integer, 0 for the root · `status`: `"loaded"` or `"skippedMissing"` |
| `gameSettings()`, `getGameSetting(key)` | `key` · `value`: the text as written · `kind`: `"Color"`, `"Float"`, `"Int"` or `"String"` · `source`: the `openmw.cfg` it came from · `comment`: the comment lines above it, or `""` |
| `genericSettings()` | `key` · `value` · `source` · `comment` |

```lua
local dawn = cfg:getGameSetting("Weather_Clear_Sky_Sunrise_Color")
if dawn then
    print(dawn.kind, dawn.value, dawn.source)
    -- Color   117,141,164   /etc/openmw/openmw.cfg
end
```
