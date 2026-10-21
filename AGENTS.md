# Agent guidance for openmw-config

The package is the Luau library at the repository root: `jess.toml`, `src/`, `tests/`, `bench/`.
The website beside it (`config.toml`, `content/`, `sass/brand.sass`, `static/img`) is its project
site, built by the DreamWeave Mod Template through jess (`jess-workspace.toml` names the template;
`jess.lock` pins it). The Rust crate lives on the `rust` branch and is not developed here.

## Verification

```sh
jess workspace sync --locked
jess workspace test
jess workspace run --trusted dreamweave-check
```

`content/home/mod.toml` is the project: `format = "jpk"`, `jess` names the package, `[[releases]]`
oldest first, each version matching `jess.toml` at its tag `openmw_config-<version>`. Never edit
`content/home/mod.lock`; CI writes it.
