+++
title = "Path functions"
description = "The default_* and try_default_* functions: user config and data, tokens, the global config, and root discovery."
weight = 80

[extra]
kind = "api"
+++

Free functions, exported from the crate root. [Paths and environment](@/docs/paths.md) lists what
each resolves to on every platform, and the environment variables that change it.

Each location comes as a pair. `try_default_*` returns `Result<PathBuf, ConfigError>`.
`default_*` returns the `PathBuf` and panics where its `try_` form would fail; use it only when the
location must exist.

## User config and data

{{ api_signature(value="fn try_default_config_path() -> Result<PathBuf, ConfigError>") }}

{{ api_signature(value="fn default_config_path() -> PathBuf") }}

The user config directory, which `?userconfig?` stands for. Fails with `PlatformPathUnavailable`
when there is no home or Documents directory to find it in.

{{ api_signature(value="fn try_default_user_config_file() -> Result<PathBuf, ConfigError>") }}

{{ api_signature(value="fn default_user_config_file() -> PathBuf") }}

The user's `openmw.cfg`: the user config directory joined with `openmw.cfg`. It need not exist.

{{ api_signature(value="fn try_default_userdata_path() -> Result<PathBuf, ConfigError>") }}

{{ api_signature(value="fn default_userdata_path() -> PathBuf") }}

The user data directory, `?userdata?`: saves, screenshots, the navmesh cache.

{{ api_signature(value="fn default_data_local_path() -> PathBuf") }}

OpenMW's default `data-local` directory, `data` inside the user data directory. It has no `try_`
form; it panics where `default_userdata_path()` would.

## Tokens

{{ api_signature(value="fn try_default_local_path() -> Result<PathBuf, ConfigError>") }}

{{ api_signature(value="fn default_local_path() -> PathBuf") }}

`?local?`: the running program's directory, or `Contents/Resources` inside a macOS app bundle. Fails
with `Io` when the executable's path cannot be read.

{{ api_signature(value="fn try_default_global_path() -> Result<PathBuf, ConfigError>") }}

{{ api_signature(value="fn default_global_path() -> PathBuf") }}

`?global?`: `OPENMW_GLOBAL_PATH` when set, otherwise the platform's shared data directory. Fails
with `PlatformPathUnavailable` on Windows, which has none.

## Global config

{{ api_signature(value="fn try_default_global_config_path() -> Result<PathBuf, ConfigError>") }}

{{ api_signature(value="fn default_global_config_path() -> PathBuf") }}

The directory of OpenMW's global config: `OPENMW_GLOBAL_CONFIG_PATH` when set, otherwise
`/etc/openmw` on Linux or `/app/etc/openmw` inside Flatpak. Fails with `PlatformPathUnavailable`
on Windows and macOS. Not the same place as `?global?`.

## Root discovery

{{ api_signature(value="fn try_default_root_config_path() -> Result<PathBuf, ConfigError>") }}

{{ api_signature(value="fn default_root_config_path() -> PathBuf") }}

The root `openmw.cfg`: beside the running executable, else in the global config directory where
the platform has one. Fails with `CannotFindRootConfig` when neither exists; its `global` field is
empty on platforms without a global config. Unlike `from_env()`, it ignores `OPENMW_CONFIG` and
`OPENMW_CONFIG_DIR`.

{{ api_signature(value="fn try_default_root_or_user_config_path() -> Result<PathBuf, ConfigError>") }}

{{ api_signature(value="fn default_root_or_user_config_path() -> PathBuf") }}

The root `openmw.cfg`, or the user's when there is no root. Fails with `CannotFindAnyConfig` when
neither exists.
