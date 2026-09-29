+++
title = "Setting types"
description = "SettingValue, FileSetting, DirectorySetting, GameSettingType, GenericSetting, EncodingSetting, EncodingType, TrailingComment and GameSettingMeta."
weight = 70

[extra]
kind = "api"
+++

Each kind of line has its own type. The configuration hands them out by reference; you rarely build
one yourself, but every type has a public constructor for when you do.

A constructor's `comment` argument is the text written above the line when it is saved: whole
comment lines, `#` and newline included, or empty. The constructor takes it and leaves the string
empty.

## SettingValue

{{ api_signature(value="enum SettingValue") }}

One entry of the configuration: a line, or for `TrailingComment` the lines that end a file.
[`settings_matching`](@/docs/api/settings.md#any-setting) hands these out. Non-exhaustive, so a
`match` needs a `_` arm. `Clone`, `Debug`.

| Variant | Line | Holds |
|---|---|---|
| `ContentFile` | `content=` | `FileSetting` |
| `Groundcover` | `groundcover=` | `FileSetting` |
| `BethArchive` | `fallback-archive=` | `FileSetting` |
| `DataDirectory` | `data=`, and the directories loading adds | `DirectorySetting` |
| `SubConfiguration` | `config=` | `DirectorySetting` |
| `Resources`, `UserData`, `DataLocal` | `resources=`, `user-data=`, `data-local=` | `DirectorySetting` |
| `Encoding` | `encoding=` | `EncodingSetting` |
| `GameSetting` | `fallback=` | `GameSettingType` |
| `Replace` | `replace=` | `GenericSetting` |
| `Generic` | Any other `key=value` | `GenericSetting` |
| `TrailingComment` | Comment and blank lines after a file's last setting | `TrailingComment` |

| Item | |
|---|---|
| `meta(&self) -> &GameSettingMeta` | Its source file and comment |
| `Display` | Its line as it would be saved, comment included |

## FileSetting

{{ api_signature(value="struct FileSetting") }}

A file name from `content=`, `groundcover=` or `fallback-archive=`. Names are not resolved: OpenMW
looks them up in its data directories. `Clone`, `Debug`.

| Item | |
|---|---|
| `new(value: &str, source_config: &Path, comment: &mut String) -> FileSetting` | Attributed to `source_config`, an `openmw.cfg` path |
| `value(&self) -> &String` | The name as written |
| `value_str(&self) -> &str` | The same, as `&str` |
| `Display` | The name alone, without key or comment |
| `PartialEq` with `FileSetting`, `str`, `&str`, `&String` | Compares the name only, not where it came from |

## DirectorySetting

{{ api_signature(value="struct DirectorySetting { pub meta: GameSettingMeta, .. }") }}

A path from `data=`, `config=`, `resources=`, `user-data=` or `data-local=`, kept both as written
and resolved. `Clone`, `Debug`.

| Item | |
|---|---|
| `new<S: Into<String>>(value: S, source_config: PathBuf, comment: &mut String) -> DirectorySetting` | Resolves `value` as a line in `source_config`: a relative path is anchored to the directory of `source_config` when it ends in `openmw.cfg`, and to `source_config` itself otherwise |
| `parsed(&self) -> &Path` | The resolved path: quotes removed, tokens replaced, separators normalized, anchored to its file's directory |
| `original(&self) -> &String` | The value as written, quotes and tokens included |
| `original_str(&self) -> &str` | The same, as `&str` |
| `meta` | Its source file and comment, as a public field |
| `Display` | `original()` and a newline |

Use `parsed()` with the file system and `original()` when writing a config back.

## GameSettingType

{{ api_signature(value="enum GameSettingType { Color(..), String(..), Float(..), Int(..) }") }}

A `fallback=Key,Value` line, typed by its value's text. The first rule that fits wins:

| Variant | Value |
|---|---|
| `Color` | Three comma-separated whole numbers, each 0 to 255: `117,141,164` |
| `Float` | Contains a `.` and parses as a number: `1.5`, `.5` |
| `Int` | A whole number: `-7` |
| `String` | Anything else: `1e5`, `256,0,0`, `hello, world` |

The variants hold types the crate does not export, so match on the variant to learn the kind, and
use the methods for the rest. The enum is non-exhaustive. `Clone`, `Debug`, `Eq`.

| Item | |
|---|---|
| `key(&self) -> &String` | The text before the first comma |
| `key_str(&self) -> &str` | The same, as `&str` |
| `value(&self) -> Cow<'_, str>` | The text after the first comma, exactly as written: `1.50` stays `1.50` |
| `Display` | The whole line, comment included: `fallback=Key,Value` |
| `PartialEq` | Same variant and same key; the value is ignored |
| `PartialEq<&str>` | The key equals the string |
| `TryFrom<(String, PathBuf, &mut String)>` | Parses `Key,Value`, attributed to the path. Fails with `InvalidGameSetting` without a comma |

## GenericSetting

{{ api_signature(value="struct GenericSetting") }}

A line whose key the crate does not recognize, kept so a round trip does not lose it. `Clone`,
`Debug`.

| Item | |
|---|---|
| `new(key: &str, value: &str, source_config: &Path, comment: &mut String) -> GenericSetting` | Attributed to `source_config` |
| `key(&self) -> &str` | The key |
| `value(&self) -> &str` | The value, as written |
| `Display` | The comment and `key=value` |

## EncodingSetting

{{ api_signature(value="struct EncodingSetting") }}

An `encoding=` line. `Clone`, `Debug`.

| Item | |
|---|---|
| `TryFrom<(String, P, &mut String)> where P: AsRef<Path>` | Parses `win1250`, `win1251` or `win1252`, exactly, attributed to the path. Fails with `BadEncoding` otherwise |
| `value(&self) -> EncodingType` | The encoding |
| `PartialEq` | Same encoding |
| `Display` | The comment and `encoding=<name>` |

## EncodingType

{{ api_signature(value="enum EncodingType { WIN1250, WIN1251, WIN1252 }") }}

The code page OpenMW decodes plugin text with: `WIN1250` Central European, `WIN1251` Cyrillic,
`WIN1252` Western European and the default. `Display` gives the config spelling, `win1252`.
`Clone`, `Copy`, `Debug`, `Eq`. Non-exhaustive.

## TrailingComment

{{ api_signature(value="struct TrailingComment") }}

The comment and blank lines after a file's last setting, kept so saving writes them back at the
end of the file. `meta()` gives the file, and the lines as its comment; `Display` writes them.
`Clone`, `Debug`.

## GameSettingMeta

{{ api_signature(value="struct GameSettingMeta") }}

Where a setting came from. `Clone`, `Debug`, `Eq`.

| Method | Returns |
|---|---|
| `source_config(&self) -> &Path` | The `openmw.cfg` the setting is attributed to; empty for entries loading added |
| `comment(&self) -> &str` | The comment lines above it, blank lines included |

Reach it through `DirectorySetting::meta`, or through `meta()` on the settings
[`settings_matching`](@/docs/api/settings.md#any-setting) yields. `FileSetting`, `GenericSetting`,
`GameSettingType` and `EncodingSetting` do not expose theirs directly.
