+++
title = "openmw-config"
description = "Read, compose and write OpenMW's openmw.cfg chains, as a Jess package in Luau."

[taxonomies]
tags = ["OpenMW", "Jess", "Luau", "Configuration"]

[extra]
sections = ["overview", "releases", "credits"]
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
- **Speaks Luau.** Behind the `luau` feature, a Rust host composes an l3i extension into its
  runtime plan and scripts get the same configuration through a camelCase API, typed.
- **Tells you where it went wrong.** Parse errors name the file and line; `config_chain()` lists
  every file it loaded and every `config=` target it skipped.

```lua
local config = require '@openmw-config'

local chain = config.new(rootConfigPath)   -- the root openmw.cfg; config= entries follow
for _, plugin in chain:values 'content' do
  print(plugin)
end

chain:append('content', 'Better Balmora.esp')
chain:saveUser()                           -- the user's file, and only it
```

## Documentation

The package's reference is `src/init.jsl`: every function is documented where it is written, and
`jess luau-types` prints the module's types. The guide that explained the Rust crate lives with it
on the `rust` branch; the Luau package's own guide is being written.

openmw-config models `openmw.cfg` only; `settings.cfg` is not handled yet. It is not affiliated
with the OpenMW project.
