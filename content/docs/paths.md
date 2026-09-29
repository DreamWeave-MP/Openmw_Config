+++
title = "Paths and environment"
description = "Where configs and data live on each platform, what each token means, Flatpak, and every environment variable the crate reads."
weight = 60

[extra]
kind = "reference"
+++

OpenMW's paths have similar names and different meanings. The crate keeps them apart, and so should
you: the user config directory, the global config directory and the global data directory are
three different places.

## Per platform

| | Linux | Linux, Flatpak | macOS | Windows |
|---|---|---|---|---|
| User config, `?userconfig?` | `$XDG_CONFIG_HOME/openmw`, or `~/.config/openmw` | `~/.var/app/<app-id>/config/openmw` | `~/Library/Preferences/openmw` | `Documents\My Games\openmw` |
| User data, `?userdata?` | `$XDG_DATA_HOME/openmw`, or `~/.local/share/openmw` | `~/.var/app/<app-id>/data/openmw` | `~/Library/Application Support/openmw` | `Documents\My Games\openmw` |
| Default `data-local` | `?userdata?/data` | `?userdata?/data` | `?userdata?/data` | `?userdata?/data` |
| `?local?` | The running program's directory | The running program's directory | `Contents/Resources` in an app bundle, otherwise the program's directory | The running program's directory |
| `?global?` | `/usr/share/games` | `/app/share/games` | `/Library/Application Support` | None |
| Global config directory | `/etc/openmw` | `/app/etc/openmw` | None | None |

- `~` is `HOME`. On Windows it is `USERPROFILE`, then `HOMEDRIVE` and `HOMEPATH` together, then
  `HOME`.
- Windows' Documents folder comes from the Known Folder API, so a Documents folder moved elsewhere,
  or into OneDrive, is found where it is.
- On Android the user config is `/storage/emulated/0/Alpha3/config` and the user data
  `/storage/emulated/0/Alpha3`.
- With no global config directory, root discovery looks only beside the executable. With no
  `?global?`, the token stays unresolved where it is written.

## Flatpak

Flatpak mode exists on Linux only. It is on when any of these holds:

- `OPENMW_CONFIG_USING_FLATPAK` is set, to anything;
- `FLATPAK_ID` is set, which Flatpak does for every app it runs;
- `/.flatpak-info` exists.

The app id is `OPENMW_FLATPAK_ID`, then `FLATPAK_ID`, then `org.openmw.OpenMW`, skipping blank
values. A tool running in its own Flatpak and reading OpenMW's config should set
`OPENMW_FLATPAK_ID=org.openmw.OpenMW`, or it will look in its own sandbox's directories.

## Environment variables

| Variable | Effect |
|---|---|
| `OPENMW_CONFIG` | The root `openmw.cfg` for `from_env()`. The only candidate when set. |
| `OPENMW_CONFIG_DIR` | Directories to search for the root `openmw.cfg`, separated by `;` on Windows and `:` elsewhere. |
| `OPENMW_GLOBAL_CONFIG_PATH` | Replaces the global config directory. A crate setting for packagers and tests, not an OpenMW one. |
| `OPENMW_GLOBAL_PATH` | Replaces `?global?`. |
| `OPENMW_CONFIG_USING_FLATPAK` | Turns Flatpak mode on. |
| `OPENMW_FLATPAK_ID`, `FLATPAK_ID` | The Flatpak app id. `FLATPAK_ID` also turns Flatpak mode on. |
| `XDG_CONFIG_HOME`, `XDG_DATA_HOME` | Linux user config and data roots. Empty values are ignored. |
| `HOME`, `USERPROFILE`, `HOMEDRIVE`, `HOMEPATH` | The home directory. |
| `PWD` | When a relative path is made absolute and `PWD` names the current directory, it is used as written, so paths through symlinked directories keep their spelling. |
| `CFG_DEBUG` | Set to anything: print what loading does to standard output. |

## Functions

Each location has a pair of functions. `try_default_*` returns a `Result` and fails with
`ConfigError::PlatformPathUnavailable` where the platform has no such place. `default_*` returns
the path and panics instead: use it only where the location must exist.

| Resolves | Fallible | Panicking |
|---|---|---|
| User config directory, `?userconfig?` | `try_default_config_path()` | `default_config_path()` |
| User `openmw.cfg` | `try_default_user_config_file()` | `default_user_config_file()` |
| User data, `?userdata?` | `try_default_userdata_path()` | `default_userdata_path()` |
| Default `data-local` | | `default_data_local_path()` |
| `?local?` | `try_default_local_path()` | `default_local_path()` |
| `?global?` | `try_default_global_path()` | `default_global_path()` |
| Global config directory | `try_default_global_config_path()` | `default_global_config_path()` |
| Root `openmw.cfg`, beside the executable or global | `try_default_root_config_path()` | `default_root_config_path()` |
| Root `openmw.cfg`, else the user's | `try_default_root_or_user_config_path()` | `default_root_or_user_config_path()` |

The root functions run steps 3 and 4 of [root discovery](@/docs/loading.md#root-discovery) and
ignore `OPENMW_CONFIG` and `OPENMW_CONFIG_DIR`, which only `from_env()` reads. The
[path reference](@/docs/api/paths.md) has each function's errors.
