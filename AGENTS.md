# Agent guidance for openmw-config

The crate is the library (`src/`); the website beside it (`content/`, `templates/`, `sass/`,
`static/`, `tools/`, `buildSite`) is the documentation site and is not packaged. `AGENT-NOTES.md`
holds the running notes between agents; read it first, append to it when you leave.

## Building

The `luau` feature depends on [l3i](https://github.com/DreamWeave-MP/l3i), which builds
Luau with clang++, lld, and cross-language thin LTO and refuses any other toolchain. This
repository's `.cargo/config.toml` is a copy of l3i's and sets that policy; on a host without
clang, build inside the `l3i-tools44` toolbox: `toolbox run -c l3i-tools44 cargo test
--all-features`. l3i comes from crates.io.
Keep builds small: cap jobs (`CARGO_BUILD_JOBS=2`), one cargo command at a time, `cargo clean`
after a bench session.

## Gates

Every commit: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -W
clippy::pedantic -D warnings`, `cargo test --all-features`. `cargo test --all-features` also runs
the l3i definitions gate (`luau-analysis` feature): the extension's declared types are type
checked by Luau's frontend and a strict script that requires `@dream/openmw-config` must be clean.

## Luau bindings

`src/luau.rs` is the extension (`dream.openmw-config`, module `@dream/openmw-config`, type
`dream.openmw.Config`). The crate never creates a VM, never picks tags or atoms: tag policy is
declared per type (four tagged types: `Config`, `Strings`, `GameSetting`, `GameSettings`), the plan
assigns the numbers. Every member declares a Luau signature. Lists are `l3i::sequence` views over
the live configuration; rows hold the configuration and a position and go stale on mutation.
`benches/luau_boundary.rs` holds the frozen boundary scripts; run it with `cargo bench --features
luau --bench luau_boundary -- --warm-up-time 1 --measurement-time 2` while iterating.

## Commits

Subjects are `FEAT:`/`FIX:`/`CLEANUP:`/`TEST:`/`DOCS:`/`PERF:`/`BREAK:` plus one imperative
sentence, no scope, no trailing period, no trailers. One concern per commit, gates green, explicit
paths in `git add` (another agent may be working in the tree).
