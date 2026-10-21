# openmw-config

Read, compose and write OpenMW's `openmw.cfg` chains, as a Jess package in Luau:
`org.dreamweave.openmw-config`, module `@openmw-config`.

```toml
[dependencies]
"org.dreamweave.openmw-config" = "4"
```

The site at <https://dreamweave-mp.github.io/Openmw_Config/> is this repository, built by the
DreamWeave Mod Template through jess on every push; every release is a JPK on this repository's
GitHub Releases, and `jess workspace sync` takes it from there.

The Rust crate `openmw-config`, versions 3.0.2 and below, lives on the `rust` branch and stays on
crates.io. It is no longer developed here.

## Developing

`jess workspace test` runs the package's suites. `jess workspace run --trusted dreamweave-check`
validates the site; `dreamweave-site -- public <url>` renders it. Releases are tags
`openmw_config-<version>`, matching `version` in `jess.toml` and a `[[releases]]` entry in
`content/home/mod.toml`.
