+++
title = "ConfigError"
description = "Every error openmw-config returns: when it happens, what it carries, and what it says."
weight = 90

[extra]
kind = "api"
+++

{{ api_signature(value="enum ConfigError") }}

The one error type for the whole crate. It implements `Debug`, `Display` and `std::error::Error`,
and converts from `std::io::Error`. The enum is non-exhaustive: match with a `_` arm.

`line` fields are 1-based, and `Some` whenever the error came from reading a file.

## Reading a config

| Variant | When |
|---|---|
| `InvalidLine { value, config_path, line }` | A line that is not blank, not a comment and has no `=` |
| `InvalidGameSetting { value, config_path, line }` | A `fallback=` value without a comma, in a file or passed to `set_game_setting`, `set_game_settings` or `GameSettingType::try_from` |
| `BadEncoding { value, config_path, line }` | An `encoding=` other than `win1250`, `win1251` or `win1252` |
| `DuplicateContentFile { file, config_path, line }` | A `content=` name the chain already listed. `config_path` and `line` are the second occurrence |
| `DuplicateGroundcoverFile { file, config_path, line }` | The same, for `groundcover=` |
| `DuplicateArchiveFile { file, config_path, line }` | The same, for `fallback-archive=` |
| `MaxDepthExceeded(PathBuf)` | More than 16 levels of `config=`, usually a chain that loops |

## Finding a config

| Variant | When |
|---|---|
| `NotFileOrDirectory(PathBuf)` | A path that is empty or does not exist; for `new_empty`, one ending in `openmw.cfg` |
| `CannotFind(PathBuf)` | A directory with no `openmw.cfg` in it |
| `CannotFindRootConfig { local, global }` | Root discovery found nothing beside the executable or in the global config directory. `global` is empty on platforms that have none |
| `CannotFindAnyConfig { local, global, user }` | The same, and the user's `openmw.cfg` does not exist either |
| `PlatformPathUnavailable(&'static str)` | A platform directory the operation needs cannot be determined. The string names it: `config`, `userdata`, `home`, `documents`, `local`, `global` or `global_config` |

## Changing a config

| Variant | When |
|---|---|
| `CannotAddContentFile { file, config_path }` | `add_content_file` with a name already listed; `config_path` is where |
| `CannotAddGroundcoverFile { file, config_path }` | `add_groundcover_file`, likewise |
| `CannotAddArchiveFile { file, config_path }` | `add_archive_file`, likewise |

## Writing a config

| Variant | When |
|---|---|
| `NotWritable(PathBuf)` | The destination file or directory cannot be written |
| `SubconfigNotLoaded(PathBuf)` | `save_subconfig` with a directory that is not a `config=` entry in effect |
| `Io(std::io::Error)` | The operating system refused a read or write |

## Messages

`Display` writes one line meant for people, naming the file and line where there is one:

```text
Morrowind.esm has appeared in the content files list twice. Its second occurence was in: /home/you/.config/openmw/openmw.cfg at line 3
Invalid fallback setting 'broken' in config file '/home/you/.config/openmw/openmw.cfg'
OpenMW root config discovery found no openmw.cfg at local path C:\Tools\openmw.cfg, and no global config path on this platform
```

Match on the variant, not the message: the wording can change between releases.
