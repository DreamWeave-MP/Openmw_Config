# Notes for whoever works on the crate next

Only what is still open. What was fixed is in `git log`; the bug pass of 2026-09-29 runs from
`c4475eb` to the commit that rewrote this file.

## Where loading still differs from OpenMW

The crate now loads as `components/files/configurationmanager.cpp` does (`readConfiguration`,
`mergeComposingVariables`; checked against upstream `46bd459920`). These differences are left,
each for the author's decision:

- **A `config=` directory without an `openmw.cfg`.** `readConfiguration` adds it to
  `mActiveConfigPaths` whether or not it loads, so a missing `?userconfig?` is still OpenMW's user
  config directory, where the engine and the launcher write. The crate skips it: its
  `user_config_path()` is the last config that loaded, so on a fresh install `save_user()` writes
  the root's directory, and the `config=` line naming the missing directory is dropped from
  memory, so a save of the file that holds it loses the line. Changing it moves where
  `save_user()` writes, and `user_config()`/`user_config_ref()` would need a missing file to load.
- **`replace=replace`.** In the engine, `replace` is itself a composing option, so a config's
  `replace=replace` cancels the `replace=` lines of the configs before it for the configs before
  those. The launcher (`gamesettings.cpp`, `readFile`) applies each file's `replace=` as it reads
  it, so there it does nothing. The two readers disagree; the crate does what the launcher does.
- **Unknown keys.** OpenMW ignores keys it does not register, and treats the ones it registers
  outside this crate's model (`start=`, `skip-menu=`, `no-sound=` and so on) as single values.
  The crate keeps every unknown key as a list, and `replace=<key>` discards the configs before it
  as for OpenMW's lists, which `set_generic_settings` relies on to take a parent's entries over.
- **A single value twice in one file.** Boost's `store` refuses a non-composing option given twice
  in one source (`multiple_occurrences`; read from Boost, not run); the crate takes the last.

## The site's Lua pages still describe `mlua`

Left from the Stage 1 l3i migration. `content/docs/lua-hosts.md`, `content/docs/lua/_index.md`,
`content/docs/lua/module.md`, the feature table in `content/docs/api/_index.md` and
`content/docs/start-here.md` still name the `lua` and `standalone-lua` features,
`create_lua_module`, the `openmwConfig` global and `mlua`. Since `498fa6f` the crate has the `luau`
and `luau-analysis` features, `openmw_config::luau::extension()` and the
`@dream/openmw-config` module.

## A Luau boundary figure to remeasure

The Stage 1 l3i cleanup pass measured `gameSettings` iterated with `for` over 500 rows at 0.44 and
0.55 µs per row (10 to 20 percent wide intervals, machine load 5 to 15), against 0.23 µs for
3.0.0. Remeasure quietly (`cargo bench --features luau --bench luau_boundary -- --warm-up-time 1
--measurement-time 2`) before treating it as a regression; if it holds, l3i's by-value
`Owned<T>` iterator path is the place to look. The README's numbers were left alone for the same
reason.

## The next release is not declared

`content/home/mod.toml` lists nothing past 2.0.1, which is tagged, while `Cargo.toml` says 3.0.1.
When 3.x is declared, its notes need the l3i breaks (`498fa6f`) and, from the bug passes of
2026-09-28 and 29:

- fixed: `add_archive_file` returns `CannotAddArchiveFile`; `set_game_setting` writes its comment
  as comment lines; the comments that end a file are kept, and stay at its end when settings are
  added; `set_*` and `remove_*` of what a parent defined persist through `save_user`, which writes
  `replace=` and the parents' remaining entries into the user config (relative data directories
  resolved) while those entries stay the parents' in memory; `replace=fallback-archive` is the
  archive option's name, as in OpenMW, and `replace=fallback-archives` no longer means anything;
  the `resources/vfs` and `data-local` data directories follow their settings and survive
  `set_data_directories`; setting `resources=`, `user-data=`, `data-local=` or `encoding=` leaves a
  parent's definition in place; `ConfigError::source()` returns the I/O error.
- added: `SettingValue` and `TrailingComment` are exported; typed game setting values and `meta()`
  on every setting type.
- breaking: `config_err!`, `bail_config!` and `impl_singleton_setting!` are no longer exported.
- breaking, loading as OpenMW's code does (`components/files/configurationmanager.cpp`), which
  wins where OpenMW's documentation disagrees:
  - `replace=` values match option names exactly: `replace=Content` discards nothing, and
    `replace=Config` resets nothing.
  - `replace=resources`, `replace=user-data`, `replace=data-local` and `replace=encoding` do
    nothing: `replace=` only reaches lists. They used to remove the latest definition, which
    could bring an older file's value back.
  - `config=` entries load depth first, as `readConfiguration`'s stack walks them: `dir1` naming
    `dir2` then `dir3`, and `dir2` naming `dir4`, loads `dir1, dir2, dir4, dir3`, not level by
    level as OpenMW's paths documentation says. `config_chain()` lists a missing `config=` target
    when the walk reaches it, and `user_config_path()` is the last config loaded, which is no
    longer always the last entry of `sub_configs()`.
  - a `config=` directory the chain already tried is skipped, as `readConfiguration` skips a
    "Repeated config dir": a config that names itself, or a loop, loads each file once instead of
    failing with `MaxDepthExceeded`, and a directory two files name loads once. Paths compare
    with `Path`'s equality, which ignores `.` components and trailing separators where
    `std::filesystem::path::compare` counts them: `config=.` is the file's own directory here, and
    to OpenMW's set a new directory each time (read from the code, not run).
  - `replace=config` in a file after the root drops the configs loaded before it except the root,
    and the chain still loads every `config=` entry waiting on the stack and all of the file's own,
    those above the `replace=` included; a dropped directory is not read again. In the root it
    does nothing. It used to drop everything loaded so far, the root included, every queued
    `config=`, and the file's own earlier lines.
  - `replace=` is per config, as `mergeComposingVariables` merges whole files: a config's
    `replace=content` discards the `content=` entries of every config before it and keeps all of
    its own, wherever the line sits; in a single file it discards nothing. The 1.0 notes
    presented "`replace=` applied as it is read"; that reading is gone. A name that stays twice
    fails as a duplicate even when a `replace=` sits between the two in one file, and one a
    later `replace=` discards may come back.
