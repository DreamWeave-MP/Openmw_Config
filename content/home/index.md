+++
title = "openmw-config"
description = "Read, compose and write OpenMW's openmw.cfg chains from Rust, or from Lua embedded in a Rust host."

[taxonomies]
tags = ["OpenMW", "Rust", "Lua", "Configuration"]

[extra]
sections = ["overview", "install", "compatibility", "releases", "credits"]
+++

OpenMW does not read one `openmw.cfg`. It reads a chain of them: a root config beside the
executable or in the system's config directory, which points with `config=` at the user's, which
may point further. Each file adds data directories, content files and fallback settings, and any
of them can `replace=` what the files before it said. Tools that open the user's `openmw.cfg`
directly see only the last link, and get the load order wrong on every packaged install.

openmw-config reads the chain the way OpenMW does, and gives you the result as one configuration
you can inspect, change and save. Every setting remembers the file it came from and the comments
above it, so saving writes each change back to the file that owns it and leaves the rest of the
chain alone.

{{ schematic(data_path="data/schematics/chain.json") }}

## What it does

- **Loads like OpenMW.** Root discovery through `OPENMW_CONFIG`, `OPENMW_CONFIG_DIR`, the
  executable's directory and the global config; `config=` depth first, as OpenMW's loader walks
  it; every `replace=` form; the `?local?`, `?global?`, `?userdata?` and `?userconfig?` tokens,
  Flatpak included.
- **Saves without damage.** Comments, unknown keys and each value's own spelling survive a round
  trip. Every write goes to a temporary file beside the target, which then replaces it.
- **Exports for importers.** A flattened copy of the whole chain, with resolved paths and no
  chain entries, that means the same thing wherever you write it.
- **Speaks Lua.** Behind the `lua` feature, a Rust host hands scripts the same configuration
  through a camelCase API.
- **Tells you where it went wrong.** Parse errors name the file and line; `config_chain()` lists
  every file it loaded and every `config=` target it skipped.

```rust
use openmw_config::{ConfigError, OpenMWConfiguration};

fn main() -> Result<(), ConfigError> {
    let mut config = OpenMWConfiguration::from_env()?;

    for plugin in config.content_files_iter() {
        println!("{}", plugin.value());
    }

    config.add_content_file("Better Balmora.esp")?;
    config.save_user()
}
```

## Documentation

This site is the crate's documentation, its API reference included.

- **[Start here](@/docs/start-here.md)**: add the crate, load the chain, change it and save it.
- **[Guide](@/docs/_index.md)**: loading, how chains resolve, editing, saving, paths, and
  embedding Lua.
- **[Rust API](@/docs/api/_index.md)** and **[Lua API](@/docs/lua/_index.md)**: every type,
  method and function, with what it does to your files.

openmw-config models `openmw.cfg` only; `settings.cfg` is not handled yet. It is not affiliated
with the OpenMW project.
