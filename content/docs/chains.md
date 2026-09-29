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
depth first: a file's first `config=`, and every file that one names, before its second. The last
directory in the chain is the user's config directory. A root naming `a` then `b`, where `a` names
`c`, loads `root`, `a`, `c`, `b`, and `b` is the user's config.

That is what OpenMW's loader, `ConfigurationManager::readConfiguration`, does: it keeps the
`config=` entries still to load on a stack. OpenMW's
[paths documentation](https://openmw.readthedocs.io/en/latest/reference/modding/paths.html#configuration-sources)
describes the same example level by level, `root`, `a`, `b`, `c`. Where the two disagree, the crate
does what OpenMW's code does.

- A `config=` directory without an `openmw.cfg` is not an error. Nothing loads from it, and the
  chain records it as skipped, but it stays in the chain, as it does in OpenMW's
  `readConfiguration`: its `config=` entry stays in memory and in every save, and when it is the
  last directory it is the user's config directory. On a fresh install, where the root's
  `config="?userconfig?"` names a directory with no `openmw.cfg` yet, `save_user()` creates the
  directory and the file there, as OpenMW's launcher does on its first save.
- A directory the chain has already tried is skipped, as OpenMW skips it: a config that names
  itself, directly or through others, loads once, and a directory two files name loads where the
  walk first reaches it. Directories compare by their resolved paths, not canonicalized, so a
  symlink and its target are two directories.
- More than 16 levels of `config=` below the root fails with `ConfigError::MaxDepthExceeded`.
  OpenMW sets no such limit.

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
`Better Balmora.esp`, and `user_config_path()` is `/games/root/../user/missing`: the last directory
in the chain, which has no `openmw.cfg` yet, and whose path is joined as written, not normalized.

## Which definition wins

| Setting | Across the chain |
|---|---|
| `content`, `groundcover`, `fallback-archive` | Lists, in load order. The same name twice fails with `DuplicateContentFile`, `DuplicateGroundcoverFile` or `DuplicateArchiveFile`, unless a `replace=` in a later config discarded the first. |
| `data` | A list, in load order. Duplicates are allowed. |
| `fallback` | Every definition is kept; the last one of each key is the value. |
| `resources`, `user-data`, `data-local`, `encoding` | The last definition wins. |
| Unknown keys | All kept, in order. |

Names compare exactly: `Morrowind.esm` and `morrowind.esm` are different plugins to this crate.

## replace=

`replace=` works per config, not per line. OpenMW reads a whole `openmw.cfg`, then merges it over
the configs loaded before it (`mergeComposingVariables`): a config's `replace=content` discards the
`content=` entries of every config before it, and keeps all of its own, wherever the `replace=`
line sits. In a chain of one file, it discards nothing. The configs loaded after it are untouched.

```ini
# root/openmw.cfg
content=Morrowind.esm
config=../user

# user/openmw.cfg
content=Mod.esp
replace=content
content=Other.esp
```

The content files are `Mod.esp` and `Other.esp`: the root's list is gone, the user's is whole.

The value is the setting's key, which is OpenMW's name for the option, spelled exactly: OpenMW
compares it case included, so `replace=Content` names no option and discards nothing. The archives'
key is `fallback-archive`, singular: OpenMW has no `fallback-archives` option, so
`replace=fallback-archives` leaves the archives alone.

| Line | Discards, from every config before its own |
|---|---|
| `replace=content` | Every `content=` entry |
| `replace=groundcover` | Every `groundcover=` entry |
| `replace=fallback-archive` | Every `fallback-archive=` entry |
| `replace=data` | Every `data=` entry |
| `replace=fallback` | Every `fallback=` entry |
| `replace=resources`, `replace=user-data`, `replace=data-local`, `replace=encoding` | Nothing: these hold one value, and OpenMW's `replace=` only reaches lists. The last file that sets one still wins |
| `replace=config` | In a config after the root, every config loaded before it except the root, with all their settings. In the root, nothing |
| `replace=replace` | Every `replace=` line, so those configs' `replace=` lines no longer reach the configs before them. `replace` is one of the lists in OpenMW's engine; its launcher applies each `replace=` as it reads it, and there `replace=replace` does nothing. The crate follows the engine. It cannot bring back a config a `replace=config` dropped: that happened while loading |
| `replace=<key>`, for any other key | Every unknown-key entry with exactly that key. OpenMW ignores keys it does not know; the crate keeps them as lists, and discards them as it does OpenMW's |

The line itself is kept, and written back where it was, so a saved file still means what it did.

`replace=config` starts the chain over from the root, as OpenMW's `readConfiguration` does. The
root stays, and so does everything in the file that says it, whichever line it is on: its
`config=` entries load, those above the `replace=` included. So does every `config=` entry still
waiting to load, from the files it dropped too. A directory it dropped has been tried, so it is not
read again when a later `config=` names it. In the root file, `replace=config` does nothing; what
stops the root's `config=` entries in OpenMW is `--replace=config` on its command line, which this
crate does not read.

`sub_configs()` leaves out the entries naming a config it dropped, but `config_chain()` still
records the configs, because they were read.

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
- `sub_configs()`: the `config=` entries still in effect, as directories, those naming a directory
  without an `openmw.cfg` included.
- `user_config_path()`: the last directory in the chain, whether or not it holds an `openmw.cfg`.

Set `CFG_DEBUG` to any value and the crate prints its progress to standard output as it loads:
each file it starts, each config it skips, and the settings it ended with.
