+++
title = "Lua API"
description = "The @dream/openmw-config module and the configuration object scripts receive, with the Rust extension that provides them."
template = "docs/section.html"
page_template = "docs/page.html"
sort_by = "weight"
weight = 100

[extra]
kind = "api"
hide_child_cards = true
+++

The Lua API is the Rust API with camelCase names, for Luau scripts running inside a Rust program.
A host composes one [l3i](https://DreamWeave-MP.github.io/l3i/) extension into its runtime plan;
[Embedding Luau](@/docs/lua-hosts.md) shows how. Scripts start with:

```lua
local openmwConfig = require("@dream/openmw-config")
```

The examples on these pages assume that line.

## openmw_config::luau::extension

{{ api_signature(value="fn extension() -> OpenmwConfigExtension") }}

Rust, behind the `luau` feature. The extension `dream.openmw-config`, for
`RuntimePlan::builder().extension(..)`. It declares the module `@dream/openmw-config` with the
loaders, the path helpers and `version`; the configuration type `dream.openmw.Config`; the list
views `dream.openmw.Strings`, `dream.openmw.GameSettings`, `dream.openmw.GenericSettings` and
`dream.openmw.ConfigChain`; and the rows `dream.openmw.GameSetting`, `dream.openmw.GenericSetting`
and `dream.openmw.ChainEntry`. The plan renders type definitions for all of them, under the same
names with `.` as `_`: `dream_openmw_Config`, `dream_openmw_Strings` and so on. The `luau` module
exports each id and type key as a constant: `EXTENSION_ID`, `MODULE`, `CONFIG_TYPE`,
`STRINGS_TYPE`, `GAME_SETTING_TYPE`, `GAME_SETTINGS_TYPE`, `GENERIC_SETTING_TYPE`,
`GENERIC_SETTINGS_TYPE`, `CHAIN_ENTRY_TYPE` and `CONFIG_CHAIN_TYPE`.

{{ api_signature(value="fn Config::new(config: OpenMWConfiguration) -> Config") }}

The configuration as scripts hold it. A host that loaded a configuration itself pushes
`Owned(Config::new(config))` to a runtime; `Config::with` and `Config::with_mut` read and change
it from Rust.

## Pages

| Page | Covers |
|---|---|
| [openmwConfig](@/docs/lua/module.md) | The module: loaders that return a configuration, path helpers, `version` |
| [Configuration](@/docs/lua/config.md) | The object the loaders return: reading, changing and saving, the views and the rows it returns |

## Conventions

- **camelCase only.** `cfg:addContentFile`, never `add_content_file`.
- **Methods use `:`, fields use `.`.** `cfg:saveUser()`, `cfg.isUserConfig`,
  `cfg.contentFileCount`; the module's functions use `.`: `openmwConfig.fromEnv()`.
- **Lists are views.** `#list`, `list[i]`, `for i, item in list do` and `list:toTable()`; `ipairs`
  and `table.*` want the table.
- **Rows are live.** A row reads the configuration when a field is read; a row taken before the
  configuration changed raises when read.
- **Failures raise.** A Luau error with the Rust error's message; catch it with `pcall`. The
  `try*` path helpers are the exception: they return `path, nil` or `nil, message`.
- **`nil` clears.** A setter given `nil` empties a list or removes a setting.
- **Paths are strings,** resolved on the way out.
- **Each configuration object is independent.** It owns its copy; there is no shared state
  between objects, or between an object and the files until you save.
