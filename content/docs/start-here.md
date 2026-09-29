+++
title = "Start here"
description = "Add the crate, load OpenMW's configuration, add a plugin and save it."
weight = 10

[extra]
kind = "tutorial"
+++

This page takes you from an empty Cargo project to a program that reads the configuration OpenMW
would use, adds a plugin to it, and saves the change where the OpenMW launcher would. It assumes
OpenMW is installed on the machine you run it on.

## Add the crate

```sh
cargo add openmw-config
```

The crate needs Rust 1.88 or newer. It has no required dependencies: on Windows it also uses
`windows-sys` to find the Documents folder, and the optional `lua` feature adds `mlua`.

## Load the configuration

```rust
use openmw_config::{ConfigError, OpenMWConfiguration};

fn main() -> Result<(), ConfigError> {
    let config = OpenMWConfiguration::from_env_or_user_config()?;

    println!("Loaded from {}", config.root_config_file().display());
    for entry in config.config_chain() {
        println!("  read {}", entry.path().display());
    }

    for directory in config.data_directories_iter() {
        println!("data    {}", directory.parsed().display());
    }
    for plugin in config.content_files_iter() {
        println!("content {}", plugin.value());
    }
    Ok(())
}
```

`from_env_or_user_config()` finds the configuration the way OpenMW does, and falls back to the
user's own `openmw.cfg` when no root config is found, which is what a tool running outside
OpenMW's directory usually needs. [Loading](@/docs/loading.md) explains the alternatives.

The data directories are resolved paths: tokens like `?userdata?` replaced, relative paths
anchored to the file that named them. The content files are names, in load order.

## Change it and save

```rust
use openmw_config::{ConfigError, OpenMWConfiguration};

fn main() -> Result<(), ConfigError> {
    let mut config = OpenMWConfiguration::from_env_or_user_config()?;

    if !config.has_content_file("Better Balmora.esp") {
        config.add_content_file("Better Balmora.esp")?;
    }

    config.save_user()
}
```

`add_content_file` appends the plugin to the end of the load order and attributes it to the user's
config, the last file in the chain. `save_user()` writes that one file back, with every comment it
had, and leaves the rest of the chain untouched. The write goes to a temporary file first, so a
crash cannot leave a half-written `openmw.cfg`.

{% callout(kind="warning", title="Try it on a copy") %}
`save_user()` writes the real user config. To experiment, point the crate at a copy instead:
`OpenMWConfiguration::new(Some("/path/to/copy".into()))` loads any directory that holds an
`openmw.cfg`.
{% end %}

## Next

- [Loading](@/docs/loading.md): pick the right constructor for your kind of tool.
- [Config chains](@/docs/chains.md): what happens between the root config and the user's.
- [Editing](@/docs/editing.md) and [Saving and exporting](@/docs/saving.md): the rest of the
  read-change-write cycle, and its traps.
- [Embedding Lua](@/docs/lua-hosts.md), if your program runs scripts.
