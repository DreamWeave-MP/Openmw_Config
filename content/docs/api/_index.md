+++
title = "Rust API"
description = "Every public type, method and function in openmw-config, grouped by what you are doing."
template = "docs/section.html"
page_template = "docs/page.html"
sort_by = "weight"
weight = 90

[extra]
kind = "api"
hide_child_cards = true
+++

Everything is exported from the crate root:

```rust
use openmw_config::{ConfigError, OpenMWConfiguration};
```

`OpenMWConfiguration` is the configuration: a whole chain, loaded, with every setting tagged with
the file it came from. Its methods are grouped here by what they are for. Every fallible call
returns `Result<_, ConfigError>`.

| Page | Covers |
|---|---|
| [Loading](@/docs/api/loading.md) | `from_env`, `from_env_or_user_config`, `new`, `new_empty`, `load_optional`, and the root and user config accessors |
| [Chain](@/docs/api/chain.md) | `config_chain`, `sub_configs`, `ConfigChainEntry`, `ConfigChainStatus` |
| [Content files and archives](@/docs/api/files.md) | `content=`, `groundcover=` and `fallback-archive=`: iterate, check, add, remove, replace |
| [Directories](@/docs/api/directories.md) | `data=`, and the single-valued `resources=`, `user-data=` and `data-local=` |
| [Settings](@/docs/api/settings.md) | `fallback=`, `encoding=`, unknown keys, and any setting by predicate |
| [Saving and serialization](@/docs/api/output.md) | `save_user`, `save_subconfig`, `save_to_path`, `save_resolved_to_path`, `Display`, `to_resolved_string` |
| [Setting types](@/docs/api/types.md) | `FileSetting`, `DirectorySetting`, `GameSettingType`, `GenericSetting`, `EncodingSetting`, `EncodingType`, `GameSettingMeta` |
| [Path functions](@/docs/api/paths.md) | The `default_*` and `try_default_*` functions |
| [ConfigError](@/docs/api/errors.md) | Every error, when it happens, and what it says |

`OpenMWConfiguration` implements `Clone`, `Debug`, `Default` and `Display`. `Default` is an empty
configuration with no root path; start from `new_empty()` instead, which knows where it would
save.

## Features

| Feature | Adds |
|---|---|
| `lua` | `create_lua_module` and the `lua` module. See the [Lua API](@/docs/lua/_index.md). |
| `standalone-lua` | `lua`, with `mlua`'s Luau runtime selected. |

Neither is on by default.
