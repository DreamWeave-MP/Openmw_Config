+++
title = "Compatibility"
description = "What the version number promises, the supported Rust version, the Lua contract, what is tested, and what the crate does not do yet."
weight = 80

[extra]
kind = "reference"
+++

## Versions

openmw-config follows semantic versioning. A release that breaks the public API, Rust or Lua, is a
new major version; 2.0 was one, because the `lua` feature stopped choosing a Lua runtime.

The minimum supported Rust version is 1.88, declared as `rust-version` in `Cargo.toml`. It can rise
in any release that is otherwise compatible, and the release notes say so when it does.

## The Lua contract

Within a major version, the documented `openmwConfig` functions and configuration methods keep
their names, arguments and results, and the rows they return keep their shapes:

| Rows from | Fields |
|---|---|
| `configChain()` | `path`, `depth`, `status`: `"loaded"` or `"skippedMissing"` |
| `gameSettings()`, `getGameSetting()` | `key`, `value`, `kind`, `source`, `comment`; `kind` is `"Color"`, `"String"`, `"Float"` or `"Int"` |
| `genericSettings()` | `key`, `value`, `source`, `comment` |

`nil` clears in every setter that accepts it.

## Behaving like OpenMW

The crate aims to read a chain exactly as OpenMW does: discovery, `config=` traversal, `replace=`
and tokens, as the
[OpenMW paths documentation](https://openmw.readthedocs.io/en/latest/reference/modding/paths.html)
describes them. Where that documentation and OpenMW's code disagree, the crate does what the code
does: `config=` entries load depth first, as `ConfigurationManager::readConfiguration` walks them,
not level by level. Where the crate differs from OpenMW, that is a bug:
[report it](https://github.com/DreamWeave-MP/Openmw_Config/issues) with the files involved.

## What is tested

Every push runs [StroggForge](https://github.com/DreamWeave-MP/StroggForge)'s library workflow:

- the whole test suite on Windows, Linux, and macOS on both Apple silicon and Intel;
- Clippy at the pedantic level, with every warning an error;
- `rustfmt`, and `cargo audit` against the RustSec advisory database;
- a dry run of the crates.io publish.

The suite covers parsing, chain traversal and every `replace=` form, round trips of comments,
unknown keys and fallback values, save boundaries, and path resolution per platform. Property
tests generate random configs, chains and paths and check that parsing, indexing, saving and
reloading agree. The Lua tests drive the bindings through a real Luau state.

## Benchmarks

`cargo bench --bench parsing` runs the Criterion benchmarks: path parsing, loading configs from ten
to five hundred plugins, and lookups. Every release runs them in CI and attaches the results to its
[GitHub release](https://github.com/DreamWeave-MP/Openmw_Config/releases) as `BENCHMARKS.md`.

## Checking your own machine

Two ignored tests load the chain starting at your user config, and write what they found into the
repository so you can compare it with what OpenMW does:

```sh
# Every openmw.cfg in the chain, one absolute path per line, into real_config_chain_paths.txt
cargo test --test integration_manual_chain_dump -- --ignored --exact dump_real_config_chain_to_repo_local_file

# A copy of every file in the chain, into manual_chain_snapshot/
cargo test --test integration_manual_chain_mirror -- --ignored --exact mirror_real_config_chain_to_repo_snapshot
```

Both outputs are ignored by git.

## Not done yet

- **Only `openmw.cfg`.** `settings.cfg` and the rest of OpenMW's configuration are out of scope for
  now.
- **No virtual file system.** The crate resolves data directories; it does not look inside them.
  [vfstool_lib](https://crates.io/crates/vfstool_lib) builds the VFS from them.
- **No checks against the disk.** Data directories, plugins and archives are not checked for
  existence, as OpenMW's config format does not require them to exist.
- **Exact names.** Plugin and archive names compare case included.
- **Parents stay as they are.** Saving writes the user's config, or one config you name, never a
  parent. `clear_matching()` and the single-valued `clear_*` methods cannot carry a parent's
  removal into the user's file. See [Editing](@/docs/editing.md#where-changes-go).
