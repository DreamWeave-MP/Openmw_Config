+++
title = "Chain"
description = "What the loader read: config_chain, sub_configs, ConfigChainEntry and ConfigChainStatus."
weight = 20

[extra]
kind = "api"
+++

[Config chains](@/docs/chains.md) explains how a chain loads. These report what happened.

## config_chain

{{ api_signature(value="fn config_chain(&self) -> impl Iterator<Item = &ConfigChainEntry>") }}

Every `openmw.cfg` the loader tried, in the order it tried them: the root first, then depth first
through the `config=` entries, as OpenMW walks them. Files a `replace=config` later discarded are still listed, because they were
read. A configuration from `new_empty()` has none.

```rust
use openmw_config::{ConfigChainStatus, ConfigError, OpenMWConfiguration};

fn main() -> Result<(), ConfigError> {
    let config = OpenMWConfiguration::from_env_or_user_config()?;
    for entry in config.config_chain() {
        let status = match entry.status() {
            ConfigChainStatus::Loaded => "loaded",
            ConfigChainStatus::SkippedMissing => "skipped",
        };
        println!("{:indent$}{status} {}", "", entry.path().display(), indent = entry.depth() * 2);
    }
    Ok(())
}
```

## sub_configs

{{ api_signature(value="fn sub_configs(&self) -> impl Iterator<Item = &DirectorySetting>") }}

The `config=` entries still in effect, as directories: file by file in the order the files loaded,
each file's in its own order. Skipped entries are not here, and neither are entries a
`replace=config` discarded. The chain loads depth first, so the last entry need not be the user's
config: `user_config_path()` is.

## ConfigChainEntry

{{ api_signature(value="struct ConfigChainEntry") }}

One file the loader tried. `Clone`, `Debug`, `Eq`.

| Method | Returns |
|---|---|
| `path(&self) -> &Path` | The `openmw.cfg` path, joined as written: `..` components are kept |
| `depth(&self) -> usize` | 0 for the root, 1 for the files it names, and so on |
| `status(&self) -> &ConfigChainStatus` | Whether it loaded |

## ConfigChainStatus

{{ api_signature(value="enum ConfigChainStatus { Loaded, SkippedMissing }") }}

`Loaded`: the file was read. `SkippedMissing`: a `config=` entry named a directory without an
`openmw.cfg`, and the loader moved on. `Clone`, `Debug`, `Eq`.
