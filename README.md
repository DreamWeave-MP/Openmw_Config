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

| Method | Description |
|---|---|
| `OpenMWConfiguration::from_env()` | Load from `OPENMW_CONFIG` / `OPENMW_CONFIG_DIR` env vars, then platform default |
| `OpenMWConfiguration::new(path)` | Load from a specific path (file or directory), or platform default if `None` |
| `OpenMWConfiguration::config_chain()` | Iterator over chain traversal events (`Loaded` / `SkippedMissing`) |
| `content_files_iter()` | Iterator over loaded content files (`content=`) |
| `groundcover_iter()` | Iterator over groundcover plugins (`groundcover=`) |
| `fallback_archives_iter()` | Iterator over BSA/BA2 archives (`fallback-archive=`) |
| `data_directories_iter()` | Iterator over data directories (`data=`) |
| `game_settings()` | Iterator over `fallback=` entries, deduplicated by key (last-wins) |
| `get_game_setting(key)` | Look up a single `fallback=` entry by key |
| `sub_configs()` | Iterator over chained `config=` entries |
| `add_content_file(name)` | Append a content file; errors if duplicate |
| `remove_content_file(name)` | Remove a content file by name |
| `set_content_files(list)` | Replace all content files; `None` clears them |
| `set_data_directories(list)` | Replace all data directories; `None` clears them |
| `set_fallback_archives(list)` | Replace all fallback archives; `None` clears them |
| `set_game_settings(list)` | Replace all fallback entries; `None` clears them |
| `save_user()` | Write the user config (`last config= in the chain`) to disk |
| `save_subconfig(path)` | Write an arbitrary loaded sub-config to disk |
| `user_config_ref()` | Load the highest-priority user config without consuming `self` |
| `user_config_path()` | Directory of the highest-priority (user) config |
| `try_default_config_path()` | Fallible platform default config directory resolver |
| `try_default_userdata_path()` | Fallible platform default userdata directory resolver |

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
