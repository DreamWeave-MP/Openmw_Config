+++
title = "Saving and exporting"
description = "Writing the user's config back, exporting a flattened copy, what each keeps and changes, and how writes stay safe."
weight = 50

[extra]
kind = "guide"
+++

The crate writes configurations in two ways, for two different jobs, and they are not
interchangeable.

- **Preserving** writes settings the way they were written: relative paths stay relative, tokens
  stay tokens, `config=` and `replace=` stay in place. The result means the same thing in the same
  place. This is how you save a user's config.
- **Flattening** writes the result: every path resolved, no chain entries, one file that means the
  same thing wherever it is written. This is how you export.

| Method | Writes | Where |
|---|---|---|
| `save_user()` | The user's config, preserved | `user_config_path()/openmw.cfg` |
| `save_subconfig(dir)` | One config from the chain, preserved | `dir/openmw.cfg` |
| `save_to_path(path)` | The whole chain in one file, preserved | `path` |
| `save_resolved_to_path(path)` | The whole chain, flattened | `path` |
| `to_string()`, `format!("{config}")` | The whole chain, preserved | A `String` |
| `to_resolved_string()` | The whole chain, flattened | A `String` |

## Saving the user's config

`save_user()` writes the settings attributed to the user's `openmw.cfg`, in order, each with the
comments above it. Parent configs are not touched. Their entries are copied only into a list the
user's config replaces, after its `replace=` line, which happens once you replace or remove what a
parent put in that list. The directory is created if it does not exist.

`save_subconfig(dir)` does the same for another config in the chain. `dir` must be the directory
of a `config=` entry that loaded, given as its resolved path or its text in the file; anything
else fails with `ConfigError::SubconfigNotLoaded`, so it cannot overwrite a file that is not part
of the chain. The root config is not a `config=` entry and cannot be saved this way.

```rust
use openmw_config::{ConfigError, OpenMWConfiguration};

fn main() -> Result<(), ConfigError> {
    let mut config = OpenMWConfiguration::from_env_or_user_config()?;
    config.remove_content_file("Unwanted.esp");
    config.save_user()
}
```

{% callout(kind="note", title="Only the user's file changes") %}
`save_user()` writes one file, and parent configs are never touched. When you replace or remove
entries a parent defined, the user's config replaces the list behind a `replace=` line, so the
change survives a reload. [Editing](@/docs/editing.md#where-changes-go) has the details and the two
exceptions.
{% end %}

## What a preserving save keeps

- Comments and blank lines, above each setting and at the end of each file.
- Every value's own spelling: quoted paths, tokens, relative paths, `1.50` rather than `1.5`.
- Unknown keys, and `replace=` lines where they were.

And what it changes:

- Spaces around `=` are removed: `content = A.esp` becomes `content=A.esp`.
- `config=` lines move to the end of their file, with their comments, because they are read after
  the rest of it.
- The `resources/vfs` and `data-local` entries that loading adds are never written.

`to_string()`, `save_to_path()` and the flattened forms end with a comment naming the version that
wrote them, `# OpenMW-Config Serializer Version: 2.0.1`. `save_user()` and `save_subconfig()` do
not add it, and loading drops it, so it never piles up.

## Exporting a flattened copy

`to_resolved_string()` and `save_resolved_to_path(path)` write the composed configuration as one
self-contained file:

- `data=`, `resources=`, `user-data=` and `data-local=` hold resolved, absolute paths.
- `config=` and `replace=` entries are left out: the chain has already been applied.
- Everything else, comments included, is written as in a preserving save.

Loading the export gives the same configuration as loading the chain did. Use it to hand a
configuration to another program, or to write one for another install:

```rust
use openmw_config::{ConfigError, OpenMWConfiguration};

fn main() -> Result<(), ConfigError> {
    let mut config = OpenMWConfiguration::load_optional("import/openmw.cfg")?;
    config.add_data_directory("/games/Morrowind/Data Files".as_ref());
    config.set_content_files(Some(vec!["Morrowind.esm".into(), "Tribunal.esm".into()]));
    config.save_resolved_to_path("export/openmw.cfg")
}
```

The destination's directory must already exist; unlike `save_user()`, exports do not create it.

`save_to_path(path)` writes the whole chain preserved instead. That keeps its spelling, but a
relative path in it now resolves against wherever you wrote the file, which is usually not what
you want from a copy.

## How writes stay safe

Every save checks that the destination can be written, and fails with `ConfigError::NotWritable`
if not. It then writes the text to a temporary file, `.openmw-config-tmp-<process>-<time>`, in the
destination's directory, flushes it to disk, and renames it over the target. A crash leaves the
old file or the new one, never half of either.

On Windows the crate deletes the old file before the rename, after clearing its read-only flag, so
there is a moment with no file in place.
