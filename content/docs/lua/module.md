+++
title = "openmwConfig"
description = "The module table: loaders, default path helpers, fallible path helpers and version."
weight = 10

[extra]
kind = "api"
+++

The module `require("@dream/openmw-config")` returns, frozen. The examples name it `openmwConfig`,
as `local openmwConfig = require("@dream/openmw-config")` does; a host's policy may also expose it
as a global of that name.

## Loaders

Each returns a [configuration](@/docs/lua/config.md) and raises on failure.
[Loading](@/docs/loading.md) explains which one a script wants.

{{ api_signature(value="openmwConfig.fromEnv() → config") }}

OpenMW's root discovery: `OPENMW_CONFIG`, `OPENMW_CONFIG_DIR`, beside the executable, the global
config. Raises when there is no root config. Rust: `from_env()`.

{{ api_signature(value="openmwConfig.fromEnvOrUserConfig() → config") }}

`fromEnv()`, falling back to the user's `openmw.cfg` when there is no root config. The one external
tools want. Rust: `from_env_or_user_config()`.

{{ api_signature(value="openmwConfig.new(path?) → config") }}

The chain starting at `path`, a file or a directory holding an `openmw.cfg`. `nil` starts at the
user's `openmw.cfg`, skipping OpenMW's root config. Rust: `new()`.

{{ api_signature(value="openmwConfig.newEmpty(userConfigDir) → config") }}

An empty configuration that saves to `userConfigDir/openmw.cfg`. Reads nothing. Rust: `new_empty()`.

{{ api_signature(value="openmwConfig.loadOptional(path) → config") }}

`new(path)` if there is a config at `path`, otherwise an empty configuration that saves there.
Rust: `load_optional()`.

## Path helpers

Strings, from the Rust [path functions](@/docs/api/paths.md).
[Paths and environment](@/docs/paths.md) lists what each is on every platform.

| Returns the path | Or `path, err` without raising | Resolves |
|---|---|---|
| `defaultConfigPath()` | `tryDefaultConfigPath()` | The user config directory, `?userconfig?` |
| `defaultUserConfigFile()` | `tryDefaultUserConfigFile()` | The user's `openmw.cfg` |
| `defaultUserDataPath()` | `tryDefaultUserDataPath()` | The user data directory, `?userdata?` |
| `defaultDataLocalPath()` | | The default `data-local` directory |
| `defaultLocalPath()` | `tryDefaultLocalPath()` | `?local?`, the running program's directory |
| `defaultGlobalPath()` | `tryDefaultGlobalPath()` | `?global?`; Windows has none |
| | `tryDefaultRootOrUserConfigPath()` | The root `openmw.cfg`, else the user's |

The `try*` forms return two values, `path, nil` or `nil, message`, and never raise. The `default*`
forms fail where the platform has no such path, as a Rust panic inside the host, which aborts the
process: prefer the `try*` forms in scripts.

```lua
local config, err = openmwConfig.tryDefaultUserConfigFile()
print(config or ("no user config directory: " .. err))
```

## version

{{ api_signature(value="openmwConfig.version → string") }}

The crate's version, like `"3.0.1"`. A constant, not a function; the compiler folds it when the
host exposes the module as a global.
