+++
title = "Loading"
description = "The five ways to get a configuration, how OpenMW finds its root config, and which one your tool wants."
weight = 20

[extra]
kind = "guide"
+++

Every constructor returns an `OpenMWConfiguration` holding a whole chain of `openmw.cfg` files.
What differs is where the chain starts. Start in the wrong place and everything after it is
subtly wrong: a user config read on its own misses the data directories and settings a packaged
OpenMW puts in its root config.

| Your program | Use | Lua | Starts from |
|---|---|---|---|
| Behaves like OpenMW, or ships beside it | `from_env()` | `fromEnv()` | OpenMW's root config |
| Is an external tool: a mod manager, an installer, a patcher | `from_env_or_user_config()` | `fromEnvOrUserConfig()` | The root config, or the user's when there is none |
| Was given a path | `new(Some(path))` | `new(path)` | That file or directory |
| Wants the user's config specifically | `new(None)` | `new(nil)` | `?userconfig?/openmw.cfg` |
| Builds a config from nothing | `new_empty(dir)` | `newEmpty(dir)` | Nothing; saves go to `dir/openmw.cfg` |
| Reads input that may not exist | `load_optional(path)` | `loadOptional(path)` | The file if it exists, otherwise nothing |

## Root discovery

`from_env()` finds the root config the way OpenMW does at startup, and stops at the first match:

1. **`OPENMW_CONFIG`**, the path of an `openmw.cfg` file. A leading `~` is expanded and a relative
   path is taken from the current directory. When it is set, it is the only candidate: an empty
   value is an error, not a reason to keep looking.
2. **`OPENMW_CONFIG_DIR`**, a list of directories separated by `;` on Windows and `:` elsewhere.
   The first one that contains an `openmw.cfg` wins. If none does, discovery carries on.
3. **Beside the executable**: an `openmw.cfg` in the directory of the running program. Inside a
   macOS app bundle this is `Contents/Resources`.
4. **The global config**: `/etc/openmw/openmw.cfg` on Linux, `/app/etc/openmw/openmw.cfg` inside
   Flatpak. Windows and macOS have no global config, so they skip this step.

If nothing matches, it fails with `ConfigError::CannotFindRootConfig`, naming the paths it tried.
It never falls back to the user's config by itself.

{% callout(kind="note", title="Beside the executable means beside your program") %}
Step 3 looks next to the program that is running, which for an external tool is the tool, not
OpenMW. That is right for utilities that ship inside OpenMW's directory and wrong for everything
else, which is what `from_env_or_user_config()` is for.
{% end %}

`from_env_or_user_config()` runs the same discovery. When it ends in `CannotFindRootConfig`, it
loads the user's `openmw.cfg` instead; if that does not exist either, it fails with
`CannotFindAnyConfig`, naming all three candidates. Every other error, such as a bad
`OPENMW_CONFIG`, is returned as it is: the fallback is for missing configs, not broken ones.

## Why not just open the user's config

On a packaged install, OpenMW's root config holds the baseline: the engine's resources, data
directories and `fallback=` defaults. It ends with
`config="?userconfig?"`, which chains to the user's file, where the launcher writes plugins and
mods. A tool that opens the user's file directly sees only half the configuration, and computes a
load order OpenMW will never use.

`new(None)` does exactly that, deliberately: it starts at `?userconfig?/openmw.cfg`, for the rare
program that means to work on the user's file and whatever it chains to. Use it only when that is
what you mean.

## Loading from a path

`new(Some(path))` accepts either a directory that contains an `openmw.cfg` or the path of the file
itself, and loads the chain from there.

- A directory without an `openmw.cfg` fails with `CannotFind`.
- A path that does not exist, or an empty one, fails with `NotFileOrDirectory`.
- A relative path is made absolute from the current directory. When the shell's `PWD` names the
  same directory, it is used, so a path through a symlinked directory keeps its spelling.

## Starting empty

Importers and generators often want a configuration that does not come from any file yet.

`new_empty(dir)` reads nothing and needs nothing to exist. `dir` is a config directory, not a file:
an empty path or one ending in `openmw.cfg` fails with `NotFileOrDirectory`. Everything you add is
attributed to `dir/openmw.cfg`, and `save_user()` creates the directory and writes that file.

`load_optional(path)` loads `path` like `new()` when there is something to load, and otherwise
starts empty in the same place:

| `path` is | Result |
|---|---|
| An existing `openmw.cfg`, or a directory containing one | Loaded, as by `new()` |
| An existing directory without an `openmw.cfg` | Empty, saving to that directory |
| A missing file named `openmw.cfg` | Empty, saving to its parent directory |
| Any other missing path | Empty, treating the path as a directory |
| Empty | `NotFileOrDirectory` |

Nothing is written until you save.

## What loading adds

Two entries in the data directory list do not come from any `data=` line, because OpenMW adds
them too:

- With `resources=` set, `resources/vfs` becomes the **first** data directory.
- With `data-local=` set, that directory becomes the **last** one. If it does not exist, loading
  creates it. This is the one way loading writes to disk.

They appear in `data_directories_iter()` like any other entry, and no save or export ever writes
them back, so reloading never duplicates them. `new_empty()` adds neither.

## Which file is the user's

The last file the chain loads is the user's: the one the launcher edits, and the one `save_user()`
writes.

- `user_config_path()` is its directory: the last `config=` directory that loaded, or the root's own
  directory when nothing chained.
- `is_user_config()` is true when the root config is that file.
- `user_config()` and `user_config_ref()` load a new configuration starting from that directory.
  The first consumes the configuration you have, the second leaves it alone. When this
  configuration already starts at the user's config, the first returns it unchanged and the second
  returns a clone.
- `root_config_file()` and `root_config_dir()` are where this configuration started.

[Config chains](@/docs/chains.md) covers how the chain in between is read, and the
[loading reference](@/docs/api/loading.md) lists every error each constructor can return.
