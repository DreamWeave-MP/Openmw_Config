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
