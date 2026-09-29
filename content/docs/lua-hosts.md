+++
title = "Embedding Lua"
description = "Give scripts in your Rust program an openmwConfig module: features, registration, errors, and what crosses the boundary."
weight = 70

[extra]
kind = "guide"
+++

The Lua bindings are for Rust programs that run scripts: a mod manager with user-written rules, a
build tool with a scripting step. Your program creates the Lua state, registers the module, and
scripts get the same configuration the Rust API sees, through camelCase names. They are not a
standalone Lua library: there is nothing to `require` until a host provides it.

## Features

| Feature | Gives you |
|---|---|
| `lua` | The bindings, built against `mlua` 0.12. No Lua runtime is chosen: your program picks one through its own `mlua` dependency. |
| `standalone-lua` | `lua` plus `mlua`'s Luau runtime. The crate's own tests and documentation use it. |

DreamWeave's hosts run [Luau](https://luau.org), and the bindings are tested on it. A host that
embeds Luau depends on both crates:

```toml
[dependencies]
openmw-config = { version = "2", features = ["lua"] }
mlua = { version = "0.12", default-features = false, features = ["luau"] }
```

## Registering the module

`create_lua_module(&lua)` builds the module table. Hand it to scripts as a global, as a module, or
both:

```rust
use mlua::Lua;
use openmw_config::create_lua_module;

fn main() -> mlua::Result<()> {
    let lua = Lua::new();
    let openmw = create_lua_module(&lua)?;

    // As a global:
    lua.globals().set("openmwConfig", openmw.clone())?;
    // As a module, for local openmwConfig = require("@openmwConfig"):
    lua.register_module("@openmwConfig", openmw)?;

    lua.load(r#"
        local cfg = openmwConfig.fromEnvOrUserConfig()
        if not cfg:hasContentFile("MyPlugin.esp") then
            cfg:addContentFile("MyPlugin.esp")
        end
        cfg:saveUser()
    "#).exec()
}
```

Luau module names start with `@`; `mlua` refuses anything else when it runs on Luau.

## Errors

A call that fails in Rust raises a Lua error carrying the `ConfigError` message, so `pcall`
catches it:

```lua
local ok, err = pcall(function()
    cfg:addContentFile("Morrowind.esm")
end)
if not ok then
    print(err) -- runtime error: Morrowind.esm cannot be added ... already defined by: /home/you/.config/openmw/openmw.cfg
end
```

The `try*` path helpers never raise. They return two values: the path and `nil`, or `nil` and the
error message.

```lua
local path, err = openmwConfig.tryDefaultGlobalPath()
if not path then
    print("No ?global? here: " .. err)
end
```

The `default*` helpers are `default_*` underneath, which panic where the platform has no such path.
Scripts should use the `try*` forms.

## What crosses the boundary

- **Paths are strings.** Directories come back resolved, as `parsed()` would give them in Rust.
  Directories you add or set are read like values in a file, so tokens work.
- **Lists are arrays.** `contentFiles()` returns `{ "Morrowind.esm", ... }`; `setContentFiles`
  takes one.
- **`nil` clears.** Every setter that takes a list or a value treats `nil` as "none":
  `setContentFiles(nil)` empties the list, `setResources(nil)` removes the last `resources=`
  definition.
- **Settings are rows.** Fallback settings, generic settings and chain entries come back as tables
  with named fields. The [Lua API](@/docs/lua/config.md#row-shapes) lists their shapes.
- **Each object is its own configuration.** A configuration object owns a copy. `userConfig()`
  returns a new object, and changing one never changes another.

The [Lua API](@/docs/lua/_index.md) lists every function and method.
