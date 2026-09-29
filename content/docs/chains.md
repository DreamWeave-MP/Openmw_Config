+++
title = "Config chains"
description = "How config=, replace=, tokens, quoting and relative paths resolve, which definition wins, and how to see what was loaded."
weight = 30

[extra]
kind = "guide"
+++

A chain starts at one `openmw.cfg` and follows its `config=` entries. The crate reads it the way
OpenMW does, and keeps a record of every file it read and every file it could not find.

## Reading a file

Each line is `key=value`, split at the first `=`. Spaces around the key and the value are trimmed.

- Blank lines and lines starting with `#` are comments. They belong to the setting that follows
  them, and are written back above it; those after a file's last setting stay at its end.
- A line without an `=` fails with `ConfigError::InvalidLine`, with the file and line number.
- Known keys are parsed: `data`, `content`, `groundcover`, `fallback-archive`, `fallback`,
  `encoding`, `resources`, `user-data`, `data-local`, `config` and `replace`. Anything else is kept
  as a generic setting and written back unchanged.

## Following config=

A file's `config=` entries are read after the rest of the file, and the files they name are loaded
level by level: every file named by the root, in order, before any file those name. The last file
loaded is the user's config.

- A `config=` directory without an `openmw.cfg` is skipped, not an error. The chain records it as
  skipped.
- A chain deeper than 16 levels fails with `ConfigError::MaxDepthExceeded`. A config that names
  itself, directly or through others, hits this limit.

Three files, laid out side by side:

```ini
# root/openmw.cfg
data="Data Files"
content=Morrowind.esm
config=../mods
config=../user

# mods/openmw.cfg
data=Tamriel_Data
content=Tamriel_Data.esm

# user/openmw.cfg
content=Better Balmora.esp
config=missing
```

```rust
use openmw_config::{ConfigError, OpenMWConfiguration};

fn main() -> Result<(), ConfigError> {
    let config = OpenMWConfiguration::new(Some("/games/root".into()))?;

    for entry in config.config_chain() {
        println!("{:?} depth={} {}", entry.status(), entry.depth(), entry.path().display());
    }
    Ok(())
}
```

```text
Loaded depth=0 /games/root/openmw.cfg
Loaded depth=1 /games/root/../mods/openmw.cfg
Loaded depth=1 /games/root/../user/openmw.cfg
SkippedMissing depth=2 /games/root/../user/missing/openmw.cfg
```

The content files come out in load order, `Morrowind.esm`, `Tamriel_Data.esm`,
`Better Balmora.esp`, and `user_config_path()` is `/games/root/../user`: paths are joined as
written, not normalized.

## Which definition wins

| Setting | Across the chain |
|---|---|
| `content`, `groundcover`, `fallback-archive` | Lists, in load order. The same name twice fails with `DuplicateContentFile`, `DuplicateGroundcoverFile` or `DuplicateArchiveFile`, unless a `replace=` cleared the first. |
| `data` | A list, in load order. Duplicates are allowed. |
| `fallback` | Every definition is kept; the last one of each key is the value. |
| `resources`, `user-data`, `data-local`, `encoding` | The last definition wins. |
| Unknown keys | All kept, in order. |

Names compare exactly: `Morrowind.esm` and `morrowind.esm` are different plugins to this crate.

## replace=

`replace=` discards what the chain has loaded so far, for one kind of setting, and then reading
carries on. The value is the setting's key, which is OpenMW's name for the option, and is not
case-sensitive. The archives' key is `fallback-archive`, singular: OpenMW has no
`fallback-archives` option, so `replace=fallback-archives` leaves the archives alone.

| Line | Discards |
|---|---|
| `replace=content` | Every `content=` entry |
| `replace=groundcover` | Every `groundcover=` entry |
| `replace=fallback-archive` | Every `fallback-archive=` entry |
| `replace=data` | Every `data=` entry |
| `replace=fallback` | Every `fallback=` entry |
| `replace=resources`, `replace=user-data`, `replace=data-local` | The latest definition of that setting |
| `replace=config` | Every setting loaded so far, from every file, and every `config=` entry read so far that has not loaded yet |
| `replace=<key>`, for any other key | Every unknown-key entry with exactly that key |

The line itself is kept, and written back where it was, so a saved file still means what it did.

`replace=config` is the reset button: the file that contains it starts the configuration over.
`sub_configs()` stops listing the configs it discarded, but `config_chain()` still records them,
because they were read.

## Paths in values

`data`, `config`, `resources`, `user-data` and `data-local` hold paths. Each is read like this:

1. **Quotes.** A value starting with `"` runs to the next `"`. Inside, `&` escapes the next
   character: `&"` is a quote, `&&` an ampersand. Anything after the closing quote is ignored.
2. **Tokens.** A value starting with a token has it replaced by a directory:

   | Token | Directory |
   |---|---|
   | `?userconfig?` | The user config directory |
   | `?userdata?` | The user data directory |
   | `?local?` | The running executable's directory |
   | `?global?` | The shared data directory, such as `/usr/share/games` |

   [Paths and environment](@/docs/paths.md) lists each one per platform. A token the platform has
   no directory for, like `?global?` on Windows, stays as written.
3. **Separators.** `/` and `\` both work, and become the platform's own.
4. **Relative paths** are anchored to the directory of the file that contains them, not the
   current directory and not the root config's.

Every path setting keeps both forms: `original()` is the text as written, `parsed()` is the
result. Saving writes the original; exporting writes the result.

`content`, `groundcover` and `fallback-archive` are file names that OpenMW looks up in its data
directories, so they are never resolved.

## encoding=

`encoding=` accepts `win1250`, `win1251` and `win1252`, exactly as written. Anything else fails with
`ConfigError::BadEncoding`, with the file and line.

## Seeing what happened

- `config_chain()`: every file the loader tried, in the order it tried them, with its depth and
  whether it `Loaded` or was `SkippedMissing`.
- `sub_configs()`: the `config=` entries still in effect, as directories.
- `user_config_path()`: the directory of the last config loaded.

Set `CFG_DEBUG` to any value and the crate prints its progress to standard output as it loads:
each file it starts, each config it skips, and the settings it ended with.
