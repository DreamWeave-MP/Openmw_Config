# Notes for whoever works on the crate next

Only what is still open. What was fixed is in `git log`; the bug pass of 2026-09-29 runs from
`c4475eb` to the commit that rewrote this file.

## Reading `replace=` and `config=` differently from OpenMW

Found on 2026-09-29 while working out the `save_user` semantics against OpenMW's own code
(`components/files/configurationmanager.cpp`: `readConfiguration`, `mergeComposingVariables`;
the launcher's `components/config/gamesettings.cpp` reads files the same way). The site's
compatibility page calls every difference from OpenMW a bug, but these change documented, tested
loading behavior (the 1.0.1 notes present "`replace=` applied as it is read" as a feature), so
they need the author's decision first. What `save_user` writes means the same under both readings:
it puts `replace=` ahead of every entry of its list and spells the option in lower case.

- **`replace=` is per file in OpenMW, per line here.** OpenMW parses a whole file, then merges it
  over the lower-priority files: a file's `replace=content` discards the content of every file
  loaded before it and keeps all of its own `content=` lines, wherever the `replace=` sits. This
  crate applies it at its line, so `content=A.esp` then `replace=content` in one file drops
  `A.esp`, which OpenMW keeps. Most single-file tests in `tests/integration_chain_replace.rs`,
  `tests/proptest_replace.rs` and the `config.rs` unit tests encode the per-line reading.
- **Case.** OpenMW matches `replace=` values against option names exactly; `replace=Content` does
  nothing there. This crate ignores case.
- **Single values.** `replace=` only affects list options in OpenMW (`mergeComposingVariables`
  skips the rest), so `replace=resources`, `user-data`, `data-local` and `encoding` do nothing
  there. Here they remove the latest definition, which can bring an older file's value back.
- **`replace=config`.** OpenMW discards the configs parsed so far except the root (local or global)
  one, keeps the root's settings, and still follows every `config=` not yet read, the replacing
  file's own included; in the root file itself it does nothing (only `--replace=config` on the
  command line does). This crate discards everything loaded so far, the root included, every
  queued `config=`, and the file's own earlier `config=` lines.
- **Traversal order.** OpenMW's code walks `config=` with a stack: `dir1` with `config=dir2` then
  `config=dir3`, and `dir2` with `config=dir4`, loads `dir1, dir2, dir4, dir3`. Its documentation
  (`docs/source/reference/modding/paths.rst`) says `dir1, dir2, dir3, dir4`, which is what this
  crate does and what `test_config_chain_priority_order_*` assert. Which one to follow is a
  question for upstream as much as for this crate.

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
