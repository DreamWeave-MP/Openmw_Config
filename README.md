# openmw_config

**openmw_config** is a lightweight Rust crate that provides a simple, idiomatic API for reading,
composing, and writing [OpenMW](https://openmw.org/) configuration files. It closely matches
OpenMW's own configuration parser, supporting configuration chains, directory tokens, and value
replacement semantics. For comprehensive VFS coverage, combine with
[vfstool_lib](https://crates.io/crates/vfstool_lib).

## Features

- **Accurate parsing** — mirrors OpenMW's config resolution, including `config=`, `replace=`, and
  tokens like `?userdata?` and `?userconfig?`.
- **Multi-file chains** — multiple `openmw.cfg` files are merged according to OpenMW's rules;
  last-defined wins.
- **Round-trip serialization** — `Display` on `OpenMWConfiguration` emits a valid `openmw.cfg`,
  preserving comments.
- **Minimal dependencies** — only [`dirs`](https://crates.io/crates/dirs) and
  [`shellexpand`](https://crates.io/crates/shellexpand).

## Quick Start

```toml
[dependencies]
openmw-config = "1"
```

For Lua bindings with vendored LuaJIT + 5.2 compatibility:

```toml
[dependencies]
openmw-config = { version = "1", features = ["lua"] }
```

```rust,no_run
use openmw_config::OpenMWConfiguration;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load the default config chain for the current platform
    let config = OpenMWConfiguration::from_env()?;

    for plugin in config.content_files_iter() {
        println!("{}", plugin.value());
    }

    for dir in config.data_directories_iter() {
        println!("{}", dir.parsed().display());
    }

    Ok(())
}
```

## Loading a specific config

`new()` accepts either a directory containing `openmw.cfg` or a direct path to the file:

```rust,no_run
use std::path::PathBuf;
use openmw_config::OpenMWConfiguration;

// From a directory
let config = OpenMWConfiguration::new(Some(PathBuf::from("/home/user/.config/openmw")))?;

// From a file path
let config = OpenMWConfiguration::new(Some(PathBuf::from("/home/user/.config/openmw/openmw.cfg")))?;
# Ok::<(), openmw_config::ConfigError>(())
```

## Modifying and saving

```rust,no_run
use std::path::PathBuf;
use openmw_config::OpenMWConfiguration;

let mut config = OpenMWConfiguration::new(None)?;

// Replace all content files
config.set_content_files(Some(vec!["MyMod.esp".into(), "Another.esp".into()]));

// Add a single plugin (errors if already present)
config.add_content_file("Extra.esp")?;

// Replace all data directories
config.set_data_directories(Some(vec![PathBuf::from("/path/to/Data Files")]));

// Replace all fallback archives
config.set_fallback_archives(Some(vec!["Morrowind.bsa".into()]));

// Write the user config back to disk
config.save_user()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Serialization

`OpenMWConfiguration` implements `Display`, which produces a valid `openmw.cfg` string with
comments preserved:

```rust,no_run
use openmw_config::OpenMWConfiguration;

let config = OpenMWConfiguration::new(None)?;
println!("{config}");
# Ok::<(), openmw_config::ConfigError>(())
```

## API Overview

| `OpenMWConfiguration` methods | Description |
|---|---|
| `from_env()` | Load from `OPENMW_CONFIG` / `OPENMW_CONFIG_DIR`, then platform default |
| `new(path)` | Load from a specific file/directory path, or platform default when `None` |
| `root_config_file()` / `root_config_dir()` | Return the root `openmw.cfg` path or its parent directory |
| `is_user_config()` | Return `true` when root and user config resolve to the same directory |
| `user_config(self)` / `user_config_ref(&self)` | Load the highest-priority user config (consuming or non-consuming) |
| `user_config_path()` | Return the directory of the highest-priority user config |
| `sub_configs()` | Iterate effective `config=` entries after `replace=config` handling |
| `config_chain()` | Iterate parser-order chain events (`Loaded` / `SkippedMissing`) |
| `content_files_iter()` / `groundcover_iter()` / `fallback_archives_iter()` | Iterate loaded `content=`, `groundcover=`, and `fallback-archive=` entries |
| `data_directories_iter()` | Iterate loaded `data=` directories |
| `game_settings()` / `get_game_setting(key)` | Iterate deduplicated `fallback=` entries or look up one by key |
| `add_content_file(name)` / `add_groundcover_file(name)` / `add_archive_file(name)` | Append file entries (error on duplicates) |
| `add_data_directory(path)` | Append a `data=` directory entry |
| `remove_content_file(name)` / `remove_groundcover_file(name)` / `remove_archive_file(name)` | Remove matching file entries |
| `remove_data_directory(path)` | Remove a matching `data=` directory entry |
| `set_content_files(list)` / `set_fallback_archives(list)` / `set_data_directories(list)` | Replace full collections (`None` clears) |
| `set_game_setting(value, source, comment)` / `set_game_settings(list)` | Add/replace one or many `fallback=` entries |
| `userdata()` / `resources()` / `data_local()` / `encoding()` | Read singleton settings |
| `set_userdata(value)` / `set_resources(value)` / `set_data_local(value)` / `set_encoding(value)` | Replace singleton settings |
| `has_content_file(name)` / `has_groundcover_file(name)` / `has_archive_file(name)` / `has_data_dir(path)` | Presence checks for loaded entries |
| `save_user()` / `save_subconfig(path)` | Write user or selected loaded config back to disk |

| Free functions | Description |
|---|---|
| `default_config_path()` / `default_userdata_path()` / `default_data_local_path()` | Platform default paths (panic on unsupported platform path discovery failures) |
| `try_default_config_path()` / `try_default_userdata_path()` | Fallible variants of platform default path resolution |
| `create_lua_module(lua)` *(with `lua` feature)* | Build a Lua module table exposing camelCase userdata/functions |

| Setting helper methods | Description |
|---|---|
| `DirectorySetting::original()` / `DirectorySetting::original_str()` / `DirectorySetting::parsed()` | Access raw and parsed directory values |
| `FileSetting::value()` / `FileSetting::value_str()` | Access file value as `&String` or `&str` |
| `GameSettingType::key()` / `GameSettingType::key_str()` / `GameSettingType::value()` | Access `fallback=` key/value views |
| `ConfigChainEntry::path()` / `ConfigChainEntry::depth()` / `ConfigChainEntry::status()` | Inspect per-node chain traversal metadata |

## Advanced

- **Config chains** — `sub_configs()` walks the `config=` entries that were loaded. The last entry
  is the user config; everything above it is read-only from OpenMW's perspective.
- **Replace semantics** — `replace=content`, `replace=data`, etc. are honoured during load, exactly
  as OpenMW handles them. `replace=config` resets earlier settings and queued `config=` entries
  from the same parse scope before continuing.
- **Token expansion** — `?userdata?` and `?userconfig?` in `data=` paths are expanded to the
  platform-correct directories at load time.

`config_chain()` provides parser-order traversal details, including skipped missing subconfigs:

```rust,no_run
use openmw_config::{ConfigChainStatus, OpenMWConfiguration};

let config = OpenMWConfiguration::new(None)?;

for entry in config.config_chain() {
    let status = match entry.status() {
        ConfigChainStatus::Loaded => "loaded",
        ConfigChainStatus::SkippedMissing => "skipped-missing",
    };
    println!("[{status}] depth={} {}", entry.depth(), entry.path().display());
}
# Ok::<(), openmw_config::ConfigError>(())
```

## Lua Bindings (`mlua`)

- Public Lua methods/functions are intentionally **camelCase only**.
- `lua` feature: embeds vendored `LuaJIT` with 5.2 compatibility (`luajit52` + `vendored`).

Module exports (`openmwConfig`):

| Lua function | Returns | Notes |
|---|---|---|
| `fromEnv()` | `config` userdata | Loads using `OPENMW_CONFIG` / `OPENMW_CONFIG_DIR` semantics |
| `new(pathOrNil)` | `config` userdata | `pathOrNil` may be file path, dir path, or `nil` |
| `defaultConfigPath()` | `string` | Platform default config dir |
| `defaultUserDataPath()` | `string` | Platform default userdata dir |
| `defaultDataLocalPath()` | `string` | Platform default data-local dir |
| `tryDefaultConfigPath()` | `(string|nil, string|nil)` | Tuple-style success/error |
| `tryDefaultUserDataPath()` | `(string|nil, string|nil)` | Tuple-style success/error |
| `version` | `string` field | Crate version string |

`config` userdata methods:

| Lua method | Returns | Notes |
|---|---|---|
| `rootConfigFile()` / `rootConfigDir()` | `string` | Resolved root file/path |
| `isUserConfig()` | `boolean` | Whether root is already highest-priority config |
| `userConfigPath()` | `string` | Highest-priority config directory |
| `userConfig()` | `config` userdata | Returns a user-config-focused clone |
| `toString()` | `string` | Serialized `openmw.cfg` output |
| `subConfigs()` | `string[]` | Effective loaded `config=` directories |
| `configChain()` | `table[]` | Rows: `{ path, depth, status }`, status is `loaded` or `skippedMissing` |
| `contentFiles()` / `groundcoverFiles()` / `fallbackArchives()` | `string[]` | Collection snapshots |
| `dataDirectories()` | `string[]` | Resolved `data=` directories |
| `gameSettings()` | `table[]` | Rows: `{ key, value, kind }` |
| `getGameSetting(key)` | `table|nil` | Single row with `{ key, value, kind }` |
| `userData()` / `resources()` / `dataLocal()` / `encoding()` | `string|nil` | Singleton getters |
| `hasContentFile(name)` / `hasGroundcoverFile(name)` / `hasArchiveFile(name)` / `hasDataDir(path)` | `boolean` | Presence checks |
| `addContentFile(name)` / `addGroundcoverFile(name)` / `addArchiveFile(name)` / `addDataDirectory(path)` | `nil` | Mutating append operations |
| `removeContentFile(name)` / `removeGroundcoverFile(name)` / `removeArchiveFile(name)` / `removeDataDirectory(path)` | `nil` | Mutating remove operations |
| `setContentFiles(listOrNil)` / `setFallbackArchives(listOrNil)` / `setDataDirectories(listOrNil)` | `nil` | Replaces full collection, `nil` clears |
| `setGameSetting(value, sourcePathOrNil, commentOrNil)` / `setGameSettings(listOrNil)` | `nil` | Fallback setters |
| `setUserData(pathOrNil)` / `setResources(pathOrNil)` / `setDataLocal(pathOrNil)` / `setEncoding(valueOrNil)` | `nil` | Singleton setters, `nil` clears |
| `saveUser()` / `saveSubconfig(path)` | `nil` | Write to user config or loaded subconfig |

Error behavior:

- Most methods throw Lua runtime errors on invalid operations (`pcall`-friendly).
- `tryDefault*` helpers return tuple-style `(value, err)` and do not throw.

Example embedding usage:

```rust,ignore
use mlua::Lua;
use openmw_config::create_lua_module;

let lua = Lua::new();
let module = create_lua_module(&lua)?;
lua.globals().set("openmwConfig", module)?;

lua.load(r#"
  local cfg = openmwConfig.fromEnv()
  local files = cfg:contentFiles()
  print(#files)
"#).exec()?;
# Ok::<(), mlua::Error>(())
```

## Compatibility Guarantees

- Public APIs follow semver: breaking changes land only in a new major version.
- MSRV is declared in `Cargo.toml` and may change only in a semver-compatible release with notes.
- `openmw.cfg` behavior aims to match OpenMW docs for chain traversal and replace semantics.
- Unknown keys are preserved during parse/serialize roundtrips.

## Known Limitations

- `settings.cfg` handling is intentionally deferred to a post-1.0 release.
- This crate models `openmw.cfg` behavior only; it does not implement the entire OpenMW config stack.

## Reference

[OpenMW configuration documentation](https://openmw.readthedocs.io/en/latest/reference/modding/paths.html#configuration-sources)

---

See [CHANGELOG.md](CHANGELOG.md) for release history.

---

openmw-config is not affiliated with the OpenMW project.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
