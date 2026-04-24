# openmw_config

**openmw_config** is a lightweight Rust crate that provides a simple, idiomatic API for reading,
composing, and writing [OpenMW](https://openmw.org/) configuration files. It closely matches
OpenMW's own configuration parser, supporting configuration chains, directory tokens, and value
replacement semantics. For comprehensive VFS coverage, combine with
[vfstool_lib](https://crates.io/crates/vfstool_lib).

- [Why Use It](#why-use-it)
- [Features](#features)
- [Rust Quick Start](#rust-quick-start)
- [Lua Quick Start](#lua-quick-start)
- [Rust Usage](#rust-usage)
- [Lua Bindings (`mlua`)](#lua-bindings-mlua)
- [Advanced Behavior](#advanced-behavior)
- [Quality & Testing](#quality--testing)
- [Manual Diagnostics](#manual-diagnostics)
- [Compatibility Guarantees](#compatibility-guarantees)
- [Known Limitations](#known-limitations)

## Why Use It

- **OpenMW-accurate semantics** - models `config=` traversal, `replace=*` behavior, and token
  expansion (`?local?`, `?global?`, `?userdata?`, `?userconfig?`) to match real parser behavior.
- **Safe persistence model** - `save_user()`, `save_subconfig()`, and `save_to_path()` use atomic
  write semantics to avoid partial writes.
- **Integration-friendly API** - ergonomic Rust API plus embedded Lua host bindings via `mlua`,
  with a camelCase-only Lua surface.
- **Diagnostics and predictability** - line-aware parse errors, explicit chain introspection, and
  deterministic roundtrip serialization.

## Features

- **Accurate parsing** - mirrors OpenMW's config resolution, including `config=`, `replace=`, and
  tokens like `?local?`, `?global?`, `?userdata?`, and `?userconfig?`.
- **Multi-file chains** - multiple `openmw.cfg` files are merged according to OpenMW's rules;
  last-defined wins.
- **Round-trip serialization** - `Display` on `OpenMWConfiguration` emits a valid `openmw.cfg`,
  preserving comments.
- **Dependency-light core** - Unix/macOS path resolution and env expansion are implemented with
  `std`; Windows default paths use Known Folder APIs via `windows-sys` (Windows-only target dep).
  Lua support is optional via the `lua` feature.

## Rust Quick Start

Load the active config chain, inspect values, mutate, and save in a few lines:

```toml
[dependencies]
openmw-config = "1"
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

    // Mutate and persist to the user config with atomic write behavior
    let mut config = config;
    config.add_content_file("Extra.esp")?;
    config.save_user()?;

    // Or write the full composite config to an explicit path
    config.save_to_path("/tmp/openmw.cfg")?;

    Ok(())
}
```

See [Rust Usage](#rust-usage) and [API Overview](#api-overview) for more patterns.

## Lua Quick Start

Embed `openmwConfig` into a host-created Lua state:

```toml
[dependencies]
openmw-config = { version = "1", features = ["lua"] }
mlua = { version = "0.10", default-features = false, features = ["luajit52", "vendored"] }
```

```rust,ignore
use mlua::Lua;
use openmw_config::create_lua_module;

fn main() -> Result<(), mlua::Error> {
    let lua = Lua::new();
    let openmw = create_lua_module(&lua)?;
    lua.globals().set("openmwConfig", openmw)?;

    lua.load(r#"
      local cfg = openmwConfig.fromEnv()
      cfg:addContentFile("MyPlugin.esp")
      cfg:saveUser()
    "#).exec()?;

    Ok(())
}
```

This is embedded-host integration, not a standalone `require("openmw_config")` Lua module.
See [Lua Bindings (`mlua`)](#lua-bindings-mlua) for the full Lua API surface.

## Rust Usage

### Loading a specific config

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

### Modifying and saving

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

### Serialization

`OpenMWConfiguration` implements `Display`, which produces a valid `openmw.cfg` string with
comments preserved:

```rust,no_run
use openmw_config::OpenMWConfiguration;

let config = OpenMWConfiguration::new(None)?;
println!("{config}");
# Ok::<(), openmw_config::ConfigError>(())
```

## API Overview

| Core Rust API | Description |
|---|---|
| `OpenMWConfiguration::from_env()` / `OpenMWConfiguration::new(path)` | Load from env/defaults or explicit file/directory path |
| `root_config_file()` / `root_config_dir()` | Root config file and parent directory |
| `user_config_ref()` / `user_config_path()` | Resolve highest-priority user config |
| `sub_configs()` / `config_chain()` | Traverse effective subconfigs and parser-order chain events |
| `content_files_iter()` / `groundcover_iter()` / `fallback_archives_iter()` | Read loaded file collections |
| `data_directories_iter()` / `game_settings()` / `get_game_setting(key)` | Read resolved directories and `fallback=` settings |
| `generic_settings_iter()` | Read preserved generic `key=value` entries |
| `add_*` / `remove_*` / `set_*` methods | Mutate loaded values |
| `add_generic_setting()` / `set_generic_settings()` | Mutate preserved generic entries |
| `save_user()` / `save_subconfig(path)` / `save_to_path(path)` | Persist changes using atomic writes |
| `default_*` and `try_default_*` free functions | Resolve default config/user paths (panic or fallible variants) |
| `create_lua_module(lua)` *(with `lua` feature)* | Build a Lua table for embedded host integration |

For the complete API surface (including helper structs and all methods), see
[`docs.rs/openmw-config`](https://docs.rs/openmw-config).

Task-oriented map:

- **Load config state** - `OpenMWConfiguration::from_env()`, `OpenMWConfiguration::new(path)`
- **Inspect chain resolution** - `sub_configs()`, `config_chain()`, `user_config_path()`
- **Edit plugin/data lists** - `add_*`, `remove_*`, `set_*` method families
- **Read/write settings** - `game_settings()`, `get_game_setting(key)`, `generic_settings_iter()`, `set_game_setting(...)`
- **Persist safely** - `save_user()`, `save_subconfig(path)`, `save_to_path(path)`

## Advanced Behavior

- **Config chains** - `sub_configs()` walks the `config=` entries that were loaded. The last entry
  is the user config; everything above it is read-only from OpenMW's perspective.
- **Replace semantics** - `replace=content`, `replace=data`, etc. are honoured during load, exactly
  as OpenMW handles them. `replace=config` resets earlier settings and queued `config=` entries
  from the same parse scope before continuing.
- **Token expansion** - `?local?`, `?global?`, `?userdata?`, and `?userconfig?` in `data=` paths
  are expanded to platform-correct directories at load time.

Flatpak and token-resolution controls:

- `OPENMW_CONFIG_USING_FLATPAK` - if set to any value, Flatpak path mode is enabled.
- Auto-detection also enables Flatpak mode when `FLATPAK_ID` is set or `/.flatpak-info` exists.
- `OPENMW_FLATPAK_ID` - optional app-id override (falls back to `FLATPAK_ID`, then `org.openmw.OpenMW`).
- `OPENMW_GLOBAL_PATH` - optional override for the `?global?` token target.
- In Flatpak mode, `?userconfig?` and `?userdata?` resolve to `~/.var/app/<app-id>/config/openmw`
  and `~/.var/app/<app-id>/data/openmw` respectively.

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
| `defaultLocalPath()` | `string` | Path backing the `?local?` token |
| `defaultGlobalPath()` | `string` | Path backing the `?global?` token (throws on unsupported platforms) |
| `tryDefaultConfigPath()` | `(string|nil, string|nil)` | Tuple-style success/error |
| `tryDefaultUserDataPath()` | `(string|nil, string|nil)` | Tuple-style success/error |
| `tryDefaultLocalPath()` | `(string|nil, string|nil)` | Tuple-style success/error |
| `tryDefaultGlobalPath()` | `(string|nil, string|nil)` | Tuple-style success/error |
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
| `gameSettings()` | `table[]` | Rows: `{ key, value, kind, source, comment }` |
| `genericSettings()` | `table[]` | Rows: `{ key, value, source, comment }` |
| `getGameSetting(key)` | `table|nil` | Single row with `{ key, value, kind, source, comment }` |
| `userData()` / `resources()` / `dataLocal()` / `encoding()` | `string|nil` | Singleton getters |
| `hasContentFile(name)` / `hasGroundcoverFile(name)` / `hasArchiveFile(name)` / `hasDataDir(path)` | `boolean` | Presence checks |
| `addContentFile(name)` / `addGroundcoverFile(name)` / `addArchiveFile(name)` / `addDataDirectory(path)` | `nil` | Mutating append operations |
| `removeContentFile(name)` / `removeGroundcoverFile(name)` / `removeArchiveFile(name)` / `removeDataDirectory(path)` | `nil` | Mutating remove operations |
| `setContentFiles(listOrNil)` / `setFallbackArchives(listOrNil)` / `setDataDirectories(listOrNil)` | `nil` | Replaces full collection, `nil` clears |
| `setGenericSettings(key, listOrNil)` / `addGenericSetting(key, value)` | `nil` | Manage preserved generic entries |
| `setGameSetting(value, sourcePathOrNil, commentOrNil)` / `setGameSettings(listOrNil)` | `nil` | Fallback setters |
| `setUserData(pathOrNil)` / `setResources(pathOrNil)` / `setDataLocal(pathOrNil)` / `setEncoding(valueOrNil)` | `nil` | Singleton setters, `nil` clears |
| `saveUser()` / `saveSubconfig(path)` / `saveToPath(path)` | `nil` | Write to user config, loaded subconfig, or explicit path |

### Host Integration (Embedded Lua)

This crate's Lua support is host-embedded: your Rust application creates a Lua state and injects
the `openmwConfig` table for scripts to consume. For setup and registration, use
[Lua Quick Start](#lua-quick-start).

Typical mutation and persistence flow from Lua:

```lua
local cfg = openmwConfig.new(nil)
cfg:addContentFile("MyPlugin.esp")
cfg:setDataDirectories({"/path/to/data"})
cfg:setGameSetting("fJumpHeight,1.0", nil, nil)
cfg:saveUser()
```

#### Notes

- Lua API naming is `camelCase` only.
- Most method failures throw Lua runtime errors (`pcall`-friendly).
- `tryDefaultConfigPath()`, `tryDefaultUserDataPath()`, `tryDefaultLocalPath()`, and
  `tryDefaultGlobalPath()` return `(value, err)` tuples instead of throwing.
- This is not a standalone Lua module distribution (`require("openmw_config")`); integration is via Rust host registration.

### Lua Stability Contract

- Across 1.x releases, the documented `openmwConfig.*` constructors/default-path helpers and
  `cfg:*` read, mutate, and save method families are intended to remain stable.
- Table shapes are stable:
  - `configChain()` rows: `{ path, depth, status }`
  - `gameSettings()` / `getGameSetting()` rows: `{ key, value, kind, source, comment }`
  - `genericSettings()` rows: `{ key, value, source, comment }`
- `status` is one of `loaded` or `skippedMissing`.
- `kind` is one of `Color`, `String`, `Float`, or `Int`.
- `nil` inputs are used to clear optional settings in setter methods.

## Quality & Testing

- Unit and integration tests cover parser behavior across config chains, including
  `replace=config` queue semantics and missing subconfig traversal outcomes.
- Roundtrip behavior is validated to preserve comments and fallback lexical forms where relevant.
- Parse diagnostics include line context on malformed input variants for easier debugging.
- CI/local lint posture is strict: `cargo clippy --all-targets --features lua -- -W clippy::pedantic -D warnings`.
- Lua integration tests validate module exports, mutation flows, persistence, and error behavior.

## Manual Diagnostics

The repository includes an ignored integration test that dumps the resolved real-world config
chain to a local file for inspection.

- Test: `dump_real_config_chain_to_repo_local_file`
- Source: `tests/integration_manual_chain_dump.rs`
- Output file: `real_config_chain_paths.txt` (repo root)
- Purpose: verify chain resolution against your actual platform/user setup

Run it manually:

```bash
cargo test --test integration_manual_chain_dump -- --ignored --exact dump_real_config_chain_to_repo_local_file
```

Notes:

- Run this when validating chain resolution on an actual machine/profile setup.
- This test is intentionally ignored in normal test runs and CI.
- Output format is one absolute `openmw.cfg` path per line, in traversal order.
- It writes a local artifact intended for debugging and verification.

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

## Support

Has `openmw-config` been useful to you?

If so, please consider [amplifying the signal](https://ko-fi.com/magicaldave) through my ko-fi. 

Thank you for using `openmw-config`.

## License

Licensed under the GNU General Public License, version 3 or later:

- [LICENSE](LICENSE)
- <https://www.gnu.org/licenses/gpl-3.0.txt>

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this project is licensed as `GPL-3.0-or-later`.
