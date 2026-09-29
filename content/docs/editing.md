+++
title = "Editing"
description = "Content files, data directories, archives, fallback settings, unknown keys and single-valued settings, and which file each change belongs to."
weight = 40

[extra]
kind = "guide"
+++

Every change you make happens to the whole configuration in memory. Saving is a separate step,
and it writes files, not the configuration: [Saving and exporting](@/docs/saving.md) covers that
side. What connects the two is attribution: each entry belongs to one file, and `save_user()`
writes the entries that belong to the user's.

## Where changes go

Everything the editing methods add is attributed to the user's `openmw.cfg`, the last file in the
chain. `new_empty()` configurations attribute to the `openmw.cfg` in their own directory.

Replacing or removing entries that a parent config defined is different: `save_user()` never
touches the parent's file. So the user's config replaces the list. `save_user()` writes a
`replace=` line for that kind of entry, then the entries the parents still contribute to the list,
then the user's own:

```ini
# The root config lists Morrowind.esm and Tribunal.esm; the user's lists Mod.esp.
# After remove_content_file("Tribunal.esm") and save_user(), the user's openmw.cfg holds:
replace=content
content=Morrowind.esm
content=Mod.esp
```

When the chain loads again, the `replace=` discards the root's list and the user's copy takes its
place, so what you had in memory is what you get back. The root's file is unchanged. The copies
leave the parent's comments behind, in the parent's file.

| Method | Makes the user's config replace the list when |
|---|---|
| `set_content_files`, `set_fallback_archives`, `set_data_directories`, `set_game_settings`, `set_generic_settings` | Any of the old entries came from a parent |
| `remove_content_file`, `remove_groundcover_file`, `remove_archive_file`, `remove_data_directory` | An entry it removes came from a parent |

In memory, the parents' remaining entries stay theirs: `meta().source_config()` still names the
file that defines them, and [`save_subconfig`](@/docs/saving.md#saving-the-user-s-config) on that
file writes them back to it. So a change to an intermediate config in the chain can be saved there
instead, or as well.

Two kinds of change are not carried over this way. `clear_resources()` and the other `clear_*`
methods remove the last definition, which lets a parent's value take effect again. When that last
definition is itself a parent's, `save_user()` cannot undo it: `replace=` works on lists only, and
OpenMW has no way for a later file to unset a single value. Saving the parent's own file with
`save_subconfig()` can. `clear_matching()` removes settings in memory only: a parent's setting it
removes is back on the next load.

## Content files, groundcover and archives

Three lists of file names, looked up by OpenMW in its data directories. Order is load order.

| | Content files | Groundcover | Archives |
|---|---|---|---|
| Iterate | `content_files_iter()` | `groundcover_iter()` | `fallback_archives_iter()` |
| Check | `has_content_file(name)` | `has_groundcover_file(name)` | `has_archive_file(name)` |
| Append | `add_content_file(name)` | `add_groundcover_file(name)` | `add_archive_file(name)` |
| Remove | `remove_content_file(name)` | `remove_groundcover_file(name)` | `remove_archive_file(name)` |
| Replace all | `set_content_files(list)` | | `set_fallback_archives(list)` |

- `add_*` appends to the end of the list, and fails if the name is already listed anywhere in the
  chain.
- `remove_*` removes every entry with that name. Removing a name that is not there does nothing.
- `set_*` replaces the whole list with the new one, in the given order. It does not check for
  duplicates. `None` leaves the list empty.
- Names compare exactly, case included.

## Data directories

`data_directories_iter()` yields each directory in load order as a `DirectorySetting`: `parsed()`
is the resolved path to use, `original()` the text as written. Nothing checks that the directories
exist; OpenMW does not either.

- `add_data_directory(path)` appends a directory. The path is read like a value in the file, so
  tokens work, and a relative path is anchored to the user's config directory. It does not check
  for duplicates.
- `remove_data_directory(path)` removes every `data=` entry whose resolved path, or whose original
  text, equals `path`. The `resources/vfs` and `data-local` directories go only with their
  settings.
- `has_data_dir(path)` compares against the resolved paths, and accepts `/` or `\` in the query.
- `set_data_directories(list)` replaces every `data=` entry. `None` leaves none. The
  `resources/vfs` and `data-local` directories stay, first and last, since no `data=` line
  declares them.

Replacing the data directories in the user's config copies every one the parents listed into the
user's file, the package's own included. A relative path is copied resolved, because in the user's
file it would be read against the user's directory; tokens and absolute paths are copied as
written. That is what makes the result reload correctly, and it is why a tool that only adds
directories should use `add_data_directory`.

## Fallback settings

`fallback=Key,Value` lines carry Morrowind.ini's settings: weather, lighting, interface colors,
level-up messages. Each key can be defined many times along the chain; the last definition is the
value.

- `game_settings()` yields each key once, with its current value. The key defined most recently
  comes first.
- `get_game_setting(key)` returns the current value of one key, or `None`. Keys are case-sensitive.
- `set_game_setting("Key,Value", source, comment)` appends a new definition. The earlier ones stay,
  and the new one wins because it is last. `source` is the file to attribute it to, or `None` for
  the user's config.
- `set_game_settings(list)` replaces every fallback setting with the list, each written as
  `Key,Value`. If one fails to parse, it returns the error with the old settings removed and none
  of the new ones added.

Values are typed by their text. Three whole numbers from 0 to 255 are a `Color`; a number with a `.`
is a `Float`; a whole number is an `Int`; anything else, `1e5` included, is a `String`. The value
keeps its exact text either way, so `1.50` is still `1.50` when it is written back.

The `comment` argument of `set_game_setting` is written above the line. Each line of it that is
not blank and does not start with `#` gets a `# `, so `"Brighter dawns"` becomes
`# Brighter dawns`. The method empties the string you pass.

```rust
use openmw_config::{ConfigError, OpenMWConfiguration};

fn main() -> Result<(), ConfigError> {
    let mut config = OpenMWConfiguration::from_env_or_user_config()?;

    let mut comment = String::from("Brighter dawns");
    config.set_game_setting("Weather_Clear_Sky_Sunrise_Color,160,170,190", None, &mut comment)?;

    let dawn = config.get_game_setting("Weather_Clear_Sky_Sunrise_Color").unwrap();
    assert_eq!(dawn.value(), "160,170,190");

    config.save_user()
}
```

## Unknown keys

Lines the crate does not recognize are kept as generic settings: OpenMW, the launcher and other
tools add keys over time, and a round trip must not lose them.

- `generic_settings_iter()` yields them in order, each with `key()` and `value()`.
- `add_generic_setting(key, value)` appends one.
- `set_generic_settings(key, list)` replaces every entry with that key; `None` removes them. A
  parent's entries are replaced through `replace=<key>`, which loading honors for any key.

## Single-valued settings

`resources=`, `user-data=`, `data-local=` and `encoding=` hold one value each. The getters,
`resources()`, `userdata()`, `data_local()` and `encoding()`, return the last definition, or
`None`.

- `set_resources_path(path)`, `set_user_data_path(path)` and `set_data_local_path(path)` set the
  value in the user's config: they replace its definition when it is the last one, and otherwise
  add one, which wins. A parent's definition stays where it is. The path is read like a value in
  the file.
- `clear_resources()`, `clear_user_data()` and `clear_data_local()` remove the last definition
  only. If a parent config defined the setting too, its value takes over.
- `set_encoding(Some(setting))` and `set_encoding(None)` do the same for `encoding=`. The
  [Lua method](@/docs/lua/config.md#setencoding) takes the name directly.

Changing `resources=` or `data-local=` moves the data directories they add, as a reload would:
`resources/vfs` stays first and `data-local` last, and clearing the setting removes its
directory.

## Anything, by predicate

`settings_matching(predicate)` yields every setting the predicate accepts, and
`clear_matching(predicate)` removes them from memory. The predicate sees each setting's source and
comment through `meta()`, and its line through `to_string()`, which is how to ask where something
came from:

```rust
use openmw_config::{ConfigError, OpenMWConfiguration};

fn main() -> Result<(), ConfigError> {
    let config = OpenMWConfiguration::from_env_or_user_config()?;
    let user_file = config.user_config_path().join("openmw.cfg");

    for setting in config.settings_matching(|setting| setting.meta().source_config() == user_file) {
        print!("{setting}");
    }
    Ok(())
}
```

The setting is a [`SettingValue`](@/docs/api/types.md#settingvalue), one variant per kind of line,
so a predicate can also match on its kind: `|setting| matches!(setting, SettingValue::ContentFile(_))`.
