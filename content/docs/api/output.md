+++
title = "Saving and serialization"
description = "save_user, save_subconfig, save_to_path, save_resolved_to_path, Display and to_resolved_string."
weight = 60

[extra]
kind = "api"
+++

Preserving output keeps each setting as it was written; flattened output writes what it resolved to.
[Saving and exporting](@/docs/saving.md) explains when to use which, and what a save keeps and
changes.

Every save writes to a temporary file in the destination's directory and renames it over the target.
All of them can fail with `NotWritable` when the destination cannot be written, and with `Io` when
the write itself fails.

## save_user

{{ api_signature(value="fn save_user(&self) -> Result<(), ConfigError>") }}

Writes `user_config_path()/openmw.cfg`: the settings attributed to it, in order, with their
comments. Creates the directory if needed.

## save_subconfig

{{ api_signature(value="fn save_subconfig(&self, target_dir: &Path) -> Result<(), ConfigError>") }}

Writes `target_dir/openmw.cfg` the same way. `target_dir` must be a `config=` directory in effect,
by resolved path or as written; otherwise it fails with `SubconfigNotLoaded` and writes nothing.

## save_to_path

{{ api_signature(value="fn save_to_path(&self, path: impl AsRef<Path>) -> Result<(), ConfigError>") }}

Writes the whole chain, preserved, to `path`. Relative paths in it resolve against wherever it is
written. The directory must exist.

## save_resolved_to_path

{{ api_signature(value="fn save_resolved_to_path(&self, path: impl AsRef<Path>) -> Result<(), ConfigError>") }}

Writes `to_resolved_string()` to `path`. The directory must exist.

## Display

{{ api_signature(value="impl Display for OpenMWConfiguration") }}

The whole chain, preserved, as one `openmw.cfg`: every setting in order with its comments,
`config=` and `replace=` included, ending with `# OpenMW-Config Serializer Version: <version>`. The
data directories loading added are left out. `to_string()` and `format!("{config}")` give it as a
`String`.

## to_resolved_string

{{ api_signature(value="fn to_resolved_string(&self) -> String") }}

The whole chain, flattened: `data=`, `resources=`, `user-data=` and `data-local=` hold resolved
paths; `config=` and `replace=` are left out, as are the data directories loading added. Ends with
the same version comment.
