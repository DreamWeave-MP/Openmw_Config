+++
title = "Documentation"
description = "How openmw-config reads, changes and writes OpenMW's configuration chains, and its complete Rust and Lua API."
template = "docs/section.html"
page_template = "docs/page.html"
sort_by = "weight"

[extra]
docs_root = true
docs_project_name = "openmw-config"
docs_short_title = "openmw-config docs"
docs_project_path = "@/home/index.md"
docs_repository_url = "https://github.com/DreamWeave-MP/Openmw_Config/tree/main/content/docs"
docs_sidebar_label = "Documentation"
hide_child_cards = true
kind = "guide"
+++

openmw-config turns OpenMW's chain of `openmw.cfg` files into one configuration you can read,
change and save, from Rust or from Lua running inside a Rust program. These pages explain how it
behaves; the two references list every call.

## Learn it

- **[Start here](@/docs/start-here.md)**: add the crate, load the chain, add a plugin and save
  it. Five minutes.
- **[Loading](@/docs/loading.md)**: the five ways to get a configuration, and which one your tool
  wants. Most bugs in OpenMW tools start by picking the wrong one.
- **[Config chains](@/docs/chains.md)**: how `config=`, `replace=`, tokens and relative paths
  resolve, and how to see what was loaded.

## Use it

- **[Editing](@/docs/editing.md)**: content files, data directories, archives, fallback settings,
  unknown keys, and which file each change belongs to.
- **[Saving and exporting](@/docs/saving.md)**: writing the user's config back, exporting a
  flattened copy, and why those are different things.
- **[Paths and environment](@/docs/paths.md)**: where configs live on each platform, what each
  token means, and every environment variable the crate reads.
- **[Embedding Lua](@/docs/lua-hosts.md)**: giving scripts in your Rust program an
  `openmwConfig` module.
- **[Compatibility](@/docs/compatibility.md)**: what the version number promises, the supported
  Rust version, what is tested, and what the crate does not do yet.

## Look it up

- **[Rust API](@/docs/api/_index.md)**: `OpenMWConfiguration` method by method, the setting
  types, the path functions and every `ConfigError`.
- **[Lua API](@/docs/lua/_index.md)**: the `openmwConfig` module and the configuration object
  scripts receive.
