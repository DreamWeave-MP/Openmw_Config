+++
title = "Loading"
description = "Constructors that load a chain or start empty, and the accessors for the root and the user's config."
weight = 10

[extra]
kind = "api"
+++

Which constructor to use is a question about your program, not the API: [Loading](@/docs/loading.md)
answers it. Every constructor that reads files can also fail with the parse errors listed under
[reading](#errors-while-reading).

## from_env

{{ api_signature(value="fn from_env() -> Result<OpenMWConfiguration, ConfigError>") }}

Finds the root config the way OpenMW does at startup and loads its chain: `OPENMW_CONFIG`, then
`OPENMW_CONFIG_DIR`, then an `openmw.cfg` beside the running executable, then the global config
where the platform has one. Never falls back to the user's config by itself.

| Error | When |
|---|---|
| `NotFileOrDirectory` | `OPENMW_CONFIG` is set but empty, or names nothing |
| `CannotFind` | `OPENMW_CONFIG` names a directory without an `openmw.cfg` |
| `CannotFindRootConfig` | No step found a root config |
| `Io`, `PlatformPathUnavailable` | The running executable's directory, or the current directory for a relative `OPENMW_CONFIG`, cannot be read |

## from_env_or_user_config

{{ api_signature(value="fn from_env_or_user_config() -> Result<OpenMWConfiguration, ConfigError>") }}

`from_env()`, and when that fails with `CannotFindRootConfig`, the user's `openmw.cfg`
(`?userconfig?/openmw.cfg`) instead. Fails with `CannotFindAnyConfig` when that does not exist
either. Every other error from `from_env()` is returned unchanged.

This is the constructor for tools that run outside OpenMW's directory.

## new

{{ api_signature(value="fn new(path: Option<PathBuf>) -> Result<OpenMWConfiguration, ConfigError>") }}

Loads the chain starting at `path`, which is an `openmw.cfg` file or a directory containing one.
`None` starts at the user's config, `?userconfig?/openmw.cfg`, which skips OpenMW's root config:
see [why that matters](@/docs/loading.md#why-not-just-open-the-user-s-config).

A relative path is made absolute from the current directory. Symlinks are followed but not
resolved: the configuration keeps the path you gave.

| Error | When |
|---|---|
| `NotFileOrDirectory` | `path` is empty or does not exist |
| `CannotFind` | `path` is a directory without an `openmw.cfg` |
| `PlatformPathUnavailable` | `path` is `None` and the platform has no user config directory |

## new_empty

{{ api_signature(value="fn new_empty(user_config_dir: impl Into<PathBuf>) -> Result<OpenMWConfiguration, ConfigError>") }}

A configuration with no settings, rooted at `user_config_dir/openmw.cfg`. Reads nothing and needs
nothing to exist. New entries are attributed to that file, and `save_user()` creates the directory
and writes it. A relative directory is made absolute from the current directory.

Fails with `NotFileOrDirectory` when `user_config_dir` is empty or ends in `openmw.cfg`: it takes a
directory, the place a whole family of config files would live.

## load_optional

{{ api_signature(value="fn load_optional(path: impl Into<PathBuf>) -> Result<OpenMWConfiguration, ConfigError>") }}

`new(Some(path))` when there is a config at `path`, otherwise `new_empty()` in the same place:

- a directory without an `openmw.cfg`: empty, rooted in that directory;
- a missing file named `openmw.cfg`: empty, rooted in its parent;
- any other missing path: empty, treating it as a directory.

Fails with `NotFileOrDirectory` when `path` is empty, and with any error `new()` returns when
there is a config to load.

## root_config_file

{{ api_signature(value="fn root_config_file(&self) -> &Path") }}

The `openmw.cfg` the chain started at. For `from_env()` that is OpenMW's root config, usually not
the user's.

## root_config_dir

{{ api_signature(value="fn root_config_dir(&self) -> PathBuf") }}

The directory of `root_config_file()`. Panics if the root config path has no parent, which no
constructor produces.

## user_config_path

{{ api_signature(value="fn user_config_path(&self) -> PathBuf") }}

The directory of the user's config: the last `config=` directory in effect, whether or not it holds
an `openmw.cfg` yet, or `root_config_dir()` when nothing chained. `save_user()` writes `openmw.cfg`
here, and every entry the editing methods add is attributed to it. Loading creates the directory
when it is missing, as OpenMW's engine does, but writes no `openmw.cfg` in it.

## is_user_config

{{ api_signature(value="fn is_user_config(&self) -> bool") }}

Whether the root config is the user's config: true when nothing chained further. Paths that
resolve to the same file count as equal.

## user_config

{{ api_signature(value="fn user_config(self) -> Result<OpenMWConfiguration, ConfigError>") }}

A configuration loaded from `user_config_path()` as `load_optional()` loads it, replacing this one:
empty and rooted there when its `openmw.cfg` does not exist yet. When this configuration already
starts there, it is returned unchanged. Errors are those of `load_optional()`.

## user_config_ref

{{ api_signature(value="fn user_config_ref(&self) -> Result<OpenMWConfiguration, ConfigError>") }}

`user_config()` without consuming `self`: a new configuration loaded from the user's config, empty
when it does not exist yet, or a clone of this one when it already starts there.

## Errors while reading

Every constructor that reads files stops at the first problem, with the file and, where there is
one, the line:

| Error | Cause |
|---|---|
| `InvalidLine` | A line that is not blank, not a comment, and has no `=` |
| `InvalidGameSetting` | A `fallback=` value without a comma |
| `BadEncoding` | An `encoding=` other than `win1250`, `win1251` or `win1252` |
| `DuplicateContentFile`, `DuplicateGroundcoverFile`, `DuplicateArchiveFile` | The same name twice in what the chain keeps once each `replace=` has discarded the lists of the configs before its own |
| `CannotFind` | A file in the chain disappeared while loading |
| `MaxDepthExceeded` | More than 16 levels of `config=` below the root |
| `NotWritable` | The user config directory, the last of the chain, is missing and cannot be created |
| `Io` | The operating system refused to read a file |

[ConfigError](@/docs/api/errors.md) lists every variant's fields and messages.
