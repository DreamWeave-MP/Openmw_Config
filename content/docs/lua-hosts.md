+++
title = "Embedding Luau"
description = "Give scripts in your Rust program the @dream/openmw-config module: the l3i extension, the plan and its policy, the toolchain, the type definitions, errors, and what crosses the boundary."
weight = 70

[extra]
kind = "guide"
+++

The Luau bindings are for Rust programs that run scripts: a mod manager with user-written rules, a
build tool with a scripting step. They are an [l3i](https://DreamWeave-MP.github.io/l3i/)
extension. The crate never creates a VM: your program composes the extension into an l3i
`RuntimePlan`, makes runtimes from the plan, and scripts get the same configuration the Rust API
sees, through camelCase names, with `require("@dream/openmw-config")`. They are not a standalone
Lua library: there is nothing to `require` until a host provides it.

## Features

| Feature | Gives you |
|---|---|
| `luau` | The `openmw_config::luau` module: the extension, the module and type keys, and the `Config` type a host can push itself. Adds [l3i](https://github.com/DreamWeave-MP/l3i) as a dependency. |
| `luau-analysis` | `luau`, with l3i's `analysis` feature: Luau's type checker. The crate's own tests use it to check the module's declared types; a host needs it only to run the analyzer itself. |

Neither is on by default. A host depends on both crates:

```toml
[dependencies]
openmw-config = { version = "3", features = ["luau"] }
l3i = "1"
```

## The toolchain

l3i builds Luau from source and links it with clang, lld and cross-language thin LTO; its
`build.rs` refuses any other toolchain, naming the missing piece. Cargo does not inherit a
dependency's `.cargo/config.toml`, so a crate that depends on l3i copies the `[env]` and
`rustflags` lines of l3i's `.cargo/config.toml` into its own `.cargo/config.toml`; this
repository's is that copy. clang and rustc must be on the same LLVM major: `rustc -vV` prints
rustc's, `clang++ --version` prints clang's. l3i's
[Start here](https://DreamWeave-MP.github.io/l3i/docs/start-here/) lists the lines, the packages
to install on each platform, and `L3I_UNVERIFIED_TOOLCHAIN` for a build that only needs to
compile.

## Composing the plan

`openmw_config::luau::extension()` is the extension, id `dream.openmw-config`. Hand it to
`RuntimePlan::builder()`, finalize the plan once, and make as many runtimes from it as you need:

```rust
use l3i::Runtime;
use l3i::extension::RuntimePlan;

fn main() -> l3i::Result<()> {
    let plan = RuntimePlan::builder()
        .extension(openmw_config::luau::extension())
        .finalize()?;
    let runtime = Runtime::from_plan(&plan)?;

    runtime.exec(
        r#"
        local openmwConfig = require("@dream/openmw-config")
        local cfg = openmwConfig.fromEnvOrUserConfig()
        if not cfg:hasContentFile("MyPlugin.esp") then
            cfg:addContentFile("MyPlugin.esp")
        end
        cfg:saveUser()
        "#,
    )
}
```

`finalize` runs the extension's description and resolves the whole composition: module paths,
type names, tags, atoms and direct-access slots. Anything the VM would reject fails there, so
`Runtime::from_plan` has nothing left to discover, and every runtime made from one plan has the
same shape. The plan takes any number of extensions beside this one; `finalize` refuses two that
claim one module path or one type. The module is frozen once installed: a script cannot assign
`openmwConfig.new`.

### The policy

`RuntimePolicy` is the VM configuration and what scripts may do: the standard libraries,
`sandbox(true)` for read-only globals and Luau's safe environment, `limits` for execution time
and memory, the profiler, and native code generation with l3i's `jit` feature. Give it to the
builder with `.policy(..)`; `RuntimePolicy::new()` is the defaults. Two settings matter to this
module:

- `compat_global("@dream/openmw-config", "openmwConfig")` also exposes the module as the global
  `openmwConfig`, for scripts written against the versions before 3.0, which had that global and
  nothing to `require`. It is host policy: the module itself declares no global. A module
  exposed this way also becomes a compiler-known library, so its members are typed at compile
  time and `version` folds to a constant.
- `sandbox(true)` makes the globals read-only after installation. `require` still works, and so
  does everything the module returns.

```rust
use l3i::extension::{RuntimePlan, RuntimePolicy};

let policy = RuntimePolicy::new()
    .compat_global("@dream/openmw-config", "openmwConfig")
    .sandbox(true);
let plan = RuntimePlan::builder()
    .policy(policy)
    .extension(openmw_config::luau::extension())
    .finalize()?;
```

l3i's [Extensions](https://DreamWeave-MP.github.io/l3i/docs/extensions/) page lists every
setting of the policy and everything `finalize` checks.

### A configuration the host loaded

`openmw_config::luau::Config` is public. A host that loaded an `OpenMWConfiguration` itself, or
wants to hand scripts one it has already changed, wraps it in `Config::new` and pushes it as
`Owned(config)`, as a global or as the result of a function it binds:

```rust
use l3i::userdata::Owned;
use openmw_config::OpenMWConfiguration;
use openmw_config::luau::Config;

let config = OpenMWConfiguration::from_env_or_user_config().map_err(|e| l3i::Error::runtime(e.to_string()))?;
runtime.set_global("cfg", &Owned(Config::new(config)))?;
runtime.exec("for _, name in cfg:contentFiles() do print(name) end")?;
```

`Config::with` and `Config::with_mut` read and mutate the configuration from Rust afterwards;
`with_mut` marks every row a script holds stale, as the script's own mutators do.

## Type definitions

The plan renders a `.d.luau` for everything it composes, `plan.type_definitions()`, in Luau's
`declare extern type` grammar: the configuration as `dream_openmw_Config`, the string list view as
`dream_openmw_Strings`, the setting views and rows as `dream_openmw_GameSettings`,
`dream_openmw_GameSetting`, `dream_openmw_GenericSettings` and `dream_openmw_GenericSetting`, and
the chain as `dream_openmw_ConfigChain` and `dream_openmw_ChainEntry`. Every member is typed, and
each view names its element type, so a `--!strict` script reads `#content`, `content[1]`,
`for i, name in content` and `content:toTable()` as strings without a cast:

```luau
--!strict
local openmwConfig = require("@dream/openmw-config")
local cfg: dream_openmw_Config = openmwConfig.fromEnvOrUserConfig()
local content: dream_openmw_Strings = cfg:contentFiles()
local first: string? = content[1]
for i, name in content do
    print(i, name)
end
local setting = cfg:getGameSetting("iMaxLevel")
if setting then
    print(setting.kind, setting.typed)
end
```

Give the text to Luau's analyzer or an editor's language server, or to l3i's own `analysis`
frontend. `plan.check_definitions()`, behind l3i's `analysis` feature (this crate's
`luau-analysis`), loads the definitions into Luau's frontend and type checks a strict script that
requires every module; the crate's tests run it, so the declared API and the runtime cannot
drift apart.

## Errors

A call that fails in Rust raises a Luau error carrying the `ConfigError` message, so `pcall`
catches it:

```lua
local ok, err = pcall(function()
    cfg:addContentFile("Morrowind.esm")
end)
if not ok then
    print(err) -- Morrowind.esm cannot be added ... already defined by: /home/you/.config/openmw/openmw.cfg
end
```

An argument of the wrong type raises too: `cfg:hasContentFile(5)` is an error, not `false`. So
does reading a row after its configuration changed; the message says the row is stale, and
fetching it again gives the current one.

The `try*` path helpers never raise. They return two values: the path and `nil`, or `nil` and the
error message.

```lua
local path, err = openmwConfig.tryDefaultGlobalPath()
if not path then
    print("No ?global? here: " .. err)
end
```

The `default*` helpers are `default_*` underneath, which panic where the platform has no such path,
and a panic inside a Luau call aborts the process. Scripts should use the `try*` forms.

## What crosses the boundary

- **Paths are strings.** Directories come back resolved, as `parsed()` would give them in Rust.
  Directories you add or set are read like values in a file, so tokens work.
- **Lists are live views.** `contentFiles()`, `groundcoverFiles()`, `fallbackArchives()`,
  `dataDirectories()` and `subConfigs()` return a `dream.openmw.Strings` view over the
  configuration: `#list`, `list[i]` (1-based, `nil` past the end), `for i, name in list do`, and
  `list:toTable()` for a plain table. Nothing is copied until an element is read, and a view sees
  every change made after it was taken. `ipairs`, `pairs` and `table.*` need `:toTable()`. The
  setters take plain tables: `setContentFiles({ "Morrowind.esm" })`.
- **`nil` clears.** Every setter that takes a list or a value treats `nil` as "none":
  `setContentFiles(nil)` empties the list, `setResources(nil)` removes the last `resources=`
  definition.
- **Settings are rows.** `gameSettings()`, `getGameSetting(key)`, `genericSettings()` and
  `configChain()` return views of userdata rows with named fields, read from the live
  configuration when a field is read. A row taken before the configuration changed raises when
  read, rather than returning a shifted entry. The [Lua API](@/docs/lua/config.md#row-shapes)
  lists their fields.
- **Counts and `isUserConfig` are fields.** `cfg.contentFileCount` and `cfg.isUserConfig`, not
  calls.
- **Each object is its own configuration.** A configuration object owns its copy. `userConfig()`
  returns a new object, and changing one never changes another. Views and rows share the object
  they came from.

The [Lua API](@/docs/lua/_index.md) lists every function, method and field.
