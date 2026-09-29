# Notes for whoever works on the crate next

Found on 2026-09-28 while writing the documentation site against the source. The site describes
what the code does today. Each item below was reproduced with a small program against `main`
unless it says otherwise.

## Already fixed

These are committed; do not redo them.

- `41be144` FIX: Keep replace= entries when serializing a configuration.
- `bd5df7c` FIX: Stop persisting the injected resources/vfs data directory.
- `6993d97` FIX: Fall back to the user config on Windows and macOS.

## Bugs

- **`set_*` and `remove_*` change every file in memory, but `save_user()` only rewrites the user's
  file.** Entries a parent config defines come back on the next load. For content files this also
  breaks the saved chain. A root has `content=Morrowind.esm` and chains to a user config.
  `set_content_files(Some(vec!["Morrowind.esm".into(), "Mod.esp".into()]))` then `save_user()`
  writes both names into the user file. Reloading fails with `DuplicateContentFile`: "Morrowind.esm
  has appeared in the content files list twice". `remove_content_file("Morrowind.esm")` then
  `save_user()` reloads with Morrowind.esm still there. Nothing in the API writes a `replace=` line
  where it would take effect: `add_generic_setting("replace", "content")` appends a generic entry
  after the user's content lines. The same applies to `set_fallback_archives`,
  `set_data_directories`, `set_game_settings`, `set_generic_settings` and the other `remove_*`
  methods.
- **`add_archive_file` returns the wrong error.** On a duplicate it returns
  `DuplicateArchiveFile { line: None }` (the `duplicate_archive_file` arm of `bail_config!`), not
  `CannotAddArchiveFile`, which its doc comment and `ConfigError`'s promise. `CannotAddArchiveFile`
  is never constructed, and the `archive_already_defined` macro arm is unused.
- **Comments passed to `set_game_setting` are written verbatim.** The Rust `comment: &mut String`
  argument and the Lua `setGameSetting(value, source, comment)` argument go straight in front of the
  line, so `"my note"` serializes as `my notefallback=fJumpHeight,1.0`, a line OpenMW cannot parse.
  Callers must pass `"# my note\n"`. Either add the `#` and newline or reject text without them.
- **Comments at the end of a file are lost.** Blank lines and `#` lines after a file's last setting
  are queued for a setting that never comes, and are dropped. `# head\ncontent=A.esp\n\n# tail\n`
  saves as `# head\ncontent=A.esp\n`.
- **`benches/luau_boundary.rs:193` fails CI's Clippy.** `cargo clippy --workspace --all-targets
  --all-features -- -W clippy::pedantic -D warnings` stops at `semicolon_if_nothing_returned`
  (`b.iter(|| from_env.call::<bool>(()).unwrap())` needs a `;`). The file was untracked and in
  progress when this was written.

## API a caller cannot reach

- `SettingValue` is `pub` inside the private `config` module and not re-exported, so callers
  cannot name it. `settings_matching` and `clear_matching` hand predicates a `&SettingValue` they
  cannot match on; only `.meta()` and `Display` work.
- A setting's source file and comment are public on `DirectorySetting` (a `pub meta` field) but not
  on `FileSetting`, `GenericSetting`, `GameSettingType` or `EncodingSetting`. They carry them
  through the `pub(crate)` `GameSetting` trait only. From Rust the only way to read where a content
  file came from is `settings_matching(..)` and `.meta().source_config()`. The Lua bindings expose
  `source` and `comment` on every row.
- Typed fallback values are parsed and then unreachable. `ColorGameSetting`, `FloatGameSetting` and
  `IntGameSetting` store `(u8, u8, u8)`, `f64` and `i64`, but the structs are unnameable and
  `GameSettingType::value()` returns the raw text (`let _ = setting.value;`).
- `#[macro_export]` makes `config_err!`, `bail_config!` and `impl_singleton_setting!` public at the
  crate root.

## Smaller things

- `game_settings()` yields each key once, most recently defined first: `A,1 B,2 C,3 A,4` iterates
  as `A=4, C=3, B=2`. Nothing documents the order; the site now does. Decide whether it is intended.
- `ConfigError::PlatformPathUnavailable`'s doc comment says the path came "via `dirs`". The crate
  no longer depends on `dirs`.
- `impl std::error::Error for ConfigError` has no `source()`, so `Io` hides its `std::io::Error`
  from error chains.
- `clear_resources`, `clear_user_data` and `clear_data_local` remove only the last definition, so a
  parent's value becomes effective again. The injected `resources/vfs` and `data-local` data
  directories are computed once in `new()` and do not follow later changes to those settings.
  Both look deliberate. The site documents them as behavior.

## Stage 1 l3i migration (2026-09-28, the binder agent)

Everything below is committed on `main`; the site's Lua pages (`content/docs/lua/*`,
`content/docs/lua-hosts.md`, `content/home/mod.toml` if it names the `lua` feature or version 2)
describe the old `mlua` surface and need to follow.

### Bugs fixed (each with a regression test)

- `97e88a5` FIX: `add_archive_file` returns `CannotAddArchiveFile` on a duplicate.
- `cc09244` FIX: `set_game_setting` (Rust and Lua) writes its comment as comment lines: every
  non-blank line without `#` gets `# `, and the comment ends with a newline. `"my note"` →
  `# my note\nfallback=...`. Text already in `#` form passes through unchanged.
- `3d45e84` FIX: comment and blank lines after a file's last setting are kept (a
  `SettingValue::TrailingComment` entry attributed to that file) and written back; the serializer's
  own `# OpenMW-Config Serializer Version:` line is dropped on load so it never accumulates.
- `fdb40da` FIX: `set_*` and `remove_*` persist through `save_user()`. Replacing a list a parent
  contributed to (`set_content_files`, `set_fallback_archives`, `set_data_directories`,
  `set_game_settings`, `set_generic_settings(key, ..)`) records `replace=<name>` in the user config
  ahead of the new entries; removing a parent's entry (`remove_content_file`,
  `remove_groundcover_file`, `remove_archive_file`, `remove_data_directory`) makes the user config
  take the whole list over: `replace=<name>` plus the remaining entries re-attributed to the user
  file. A single-file chain writes no `replace=` line. Loading now honours `replace=<any key>` for
  generic entries, as OpenMW does. `save_user` never touches a parent file.
- `1fbcfe3` FIX: the singleton setters (`set_encoding`, `clear_user_data`, ...) and the injected
  `data=` entries rebuild the lookup indexes; before, `get_game_setting` could return a shifted
  entry after `clear_*`, and `has_data_dir` was false for the injected `resources/vfs` directory.
- The `benches/luau_boundary.rs` Clippy failure was fixed in `85ffeb3` before the bench landed.

### Rust core

- `b9fef28` PERF: per-kind position indexes (list iterators no longer scan the flat list),
  `game_settings()` no longer clones its index per call, `has_data_dir` allocates only when it
  rewrites separators, and `new()` no longer pretty-prints every setting on every load (that
  `format!` ran unconditionally: large config 6.65 ms → 1.21 ms).
- `a4fadeb` PERF: `FxHash` for the string indexes (`get_game_setting` miss 25 ns → 9 ns).
- `b980248` FEAT: `GameSettingType::{kind_name, int_value, float_value, color_value, value_str,
  meta}` and `meta()` on `FileSetting`, `GenericSetting`, `EncodingSetting` (two of the
  "API a caller cannot reach" items). `content_file_count()` and the other counts are public.

### Luau surface (`498fa6f`, version 3.0.0)

- Feature `lua` → `luau`; `standalone-lua` is gone (l3i owns Luau); `create_lua_module` is gone.
  `openmw_config::luau::extension()` is an l3i extension: id `dream.openmw-config`, module
  `@dream/openmw-config`, types `dream.openmw.Config`, `.Strings`, `.GameSetting`,
  `.GameSettings`, `.GenericSetting`, `.GenericSettings`, `.ChainEntry`, `.ConfigChain`.
  `luau-analysis` enables l3i's analysis frontend for the definitions gate in the tests.
- Scripts `require("@dream/openmw-config")`; the `openmwConfig` global is the host's
  (`RuntimePolicy::compat_global("@dream/openmw-config", "openmwConfig")`).
- Breaks: `cfg:isUserConfig()` → `cfg.isUserConfig`; the list methods return live views
  (`#list`, `list[i]`, `for i, v in list`, `list:toTable()`; `ipairs`/`pairs`/`table.*` need
  `:toTable()`); `gameSettings()`, `genericSettings()`, `configChain()` are views of userdata rows
  with the old field names (`pairs(row)` no longer works; a row read after the configuration was
  mutated raises "stale"); `ipairs(cfg:contentFiles())` errors.
- Additions: `row.typed` on game settings (Int → Luau integer, Float → number, Color → vector of
  the 0..255 components, String → string); direct fields `contentFileCount`,
  `groundcoverFileCount`, `archiveFileCount`, `dataDirectoryCount`, `gameSettingCount` (distinct
  keys), `genericSettingCount`, `subConfigCount`; `__tostring` on Config and rows; a non-string in
  a list argument names the entry (`list entry 2: ...`).
- Everything else keeps its name, arguments, return shape, and error behaviour.

### Boundary numbers (`benches/luau_boundary.rs`, per operation, same frozen scripts)

Before (mlua 0.12, commit 85ffeb3) → after (l3i): getGameSetting found 1.09/1.80/2.18 µs at
n=50/500/2000 → 0.21/0.22/0.18 µs; missing 159 → 94 ns; hasContentFile hit 232 → 94 ns, miss
267 → 99 ns; contentFiles 500 entries: materialise 96 → 86 ns/entry (`:toTable()`), `[i]` loop 191
→ 130 ns/entry, `for` 211 → 125 ns/entry (no allocation); gameSettings `for` over 500 rows
1.6 µs → 0.23 µs per row; `fromEnv` load (500 plugins, 500 fallbacks) 3.86 ms → 0.48 ms.
Measured with four agents building (load 40+); a later `nice -n 19` run under load read about
twice these figures across the board, contention, not code.
