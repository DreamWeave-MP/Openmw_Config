+++
title = "Directories"
description = "data= directories, and the single-valued resources=, user-data= and data-local=."
weight = 40

[extra]
kind = "api"
+++

Directory settings are [`DirectorySetting`](@/docs/api/types.md#directorysetting)s: `parsed()` is
the resolved path, `original()` the text as written. A path you add or set is read like a value in
a file: tokens resolve, and a relative path is anchored to the user's config directory. Paths you
look up or remove are compared as given.

## Data directories

{{ api_signature(value="fn data_directories_iter(&self) -> impl Iterator<Item = &DirectorySetting>") }}

The data directories in load order, which is VFS priority order: later directories override
earlier ones. Includes the entries loading adds: `resources/vfs` first when `resources=` is set,
`data-local` last when that is. Nothing checks that the directories exist.

{{ api_signature(value="fn has_data_dir(&self, file_name: &str) -> bool") }}

Whether a data directory resolves to `file_name`. `/` and `\` in the query both match the
platform's separator. The query is not resolved: pass an absolute path.

{{ api_signature(value="fn add_data_directory(&mut self, dir: &Path)") }}

Appends a data directory, attributed to the user's config. Does not check for duplicates.

{{ api_signature(value="fn remove_data_directory(&mut self, data_dir: &PathBuf)") }}

Removes every data directory whose resolved path equals `data_dir`, or whose text as written equals
it. Removing a parent's makes the user's config take the list over with `replace=data`, copying the
remaining parent directories into it.

{{ api_signature(value="fn set_data_directories(&mut self, dirs: Option<Vec<PathBuf>>)") }}

Removes every data directory, the added `resources/vfs` and `data-local` entries included, and adds
`dirs` in order, attributed to the user's config, behind `replace=data` when a parent had
contributed. `None` leaves none.

## resources=, user-data= and data-local=

Each holds one directory: the engine's resources, the user data root, and the highest-priority data
directory. The getter returns the last definition in the chain.

{{ api_signature(value="fn resources(&self) -> Option<&DirectorySetting>") }}

{{ api_signature(value="fn userdata(&self) -> Option<&DirectorySetting>") }}

{{ api_signature(value="fn data_local(&self) -> Option<&DirectorySetting>") }}

The config key is `user-data`; `?userdata?` is only a token, and there is no `userdata=` key.

### Setting a path

{{ api_signature(value="fn set_resources_path(&mut self, path: impl AsRef<Path>)") }}

{{ api_signature(value="fn set_user_data_path(&mut self, path: impl AsRef<Path>)") }}

{{ api_signature(value="fn set_data_local_path(&mut self, path: impl AsRef<Path>)") }}

Replaces the last definition in place, or adds one when there is none, attributed to the user's
config.

### Clearing

{{ api_signature(value="fn clear_resources(&mut self)") }}

{{ api_signature(value="fn clear_user_data(&mut self)") }}

{{ api_signature(value="fn clear_data_local(&mut self)") }}

Removes the last definition. When a parent config defined the setting as well, its value is the one
in effect afterwards.

### Setting a DirectorySetting

{{ api_signature(value="fn set_resources(&mut self, new: Option<DirectorySetting>)") }}

{{ api_signature(value="fn set_userdata(&mut self, new: Option<DirectorySetting>)") }}

{{ api_signature(value="fn set_data_local(&mut self, new: Option<DirectorySetting>)") }}

The general form of the two above: `Some` replaces the last definition or adds one, `None` removes
the last definition. The setting keeps the source you gave it in
[`DirectorySetting::new`](@/docs/api/types.md#directorysetting), and saving writes it to that file.

Changing `resources=` or `data-local=` after loading does not change the data directories loading
added for them.
