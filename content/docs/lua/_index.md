+++
title = "Lua API"
description = "The openmwConfig module and the configuration object scripts receive, with the Rust call that builds them."
template = "docs/section.html"
page_template = "docs/page.html"
sort_by = "weight"
weight = 100

[extra]
kind = "api"
hide_child_cards = true
+++

The Lua API is the Rust API with camelCase names, for scripts running inside a Rust program. A host
builds the module with one call; [Embedding Lua](@/docs/lua-hosts.md) shows how to register it.

## create_lua_module

{{ api_signature(value="fn create_lua_module(lua: &mlua::Lua) -> mlua::Result<mlua::Table>") }}

Rust, behind the `lua` feature. Builds the `openmwConfig` table in `lua`: the loaders, the path
helpers and `version`. Fails only if `mlua` cannot create the table or its functions.

## Pages

| Page | Covers |
|---|---|
| [openmwConfig](@/docs/lua/module.md) | The module: loaders that return a configuration, path helpers, `version` |
| [Configuration](@/docs/lua/config.md) | The object the loaders return: reading, changing and saving, and the rows it returns |

## Conventions

- **camelCase only.** `cfg:addContentFile`, never `add_content_file`.
- **Methods use `:`.** `cfg:saveUser()`; the module's functions use `.`: `openmwConfig.fromEnv()`.
- **Failures raise.** A Lua error with the Rust error's message; catch it with `pcall`. The
  `try*` path helpers are the exception: they return `path, nil` or `nil, message`.
- **`nil` clears.** A setter given `nil` empties a list or removes a setting.
- **Paths are strings,** resolved on the way out.
- **Each configuration object is independent.** It owns its copy; there is no shared state
  between objects, or between an object and the files until you save.
