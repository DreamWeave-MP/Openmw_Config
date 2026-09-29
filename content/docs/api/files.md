+++
title = "Content files and archives"
description = "content=, groundcover= and fallback-archive=: iterate, check, add, remove and replace."
weight = 30

[extra]
kind = "api"
+++

Three lists of file names, which OpenMW looks up in its data directories. Each iterates in load
order and yields [`FileSetting`](@/docs/api/types.md#filesetting)s. Names compare exactly, case
included.

Removing or replacing entries a parent config defined makes the user's config replace the list,
behind a `replace=` line, so `save_user()` persists the change: see
[where changes go](@/docs/editing.md#where-changes-go).

## Content files

`content=` lines: the plugins, `.esm`, `.esp`, `.omwaddon`, `.omwscripts` and the rest.

{{ api_signature(value="fn content_files_iter(&self) -> impl Iterator<Item = &FileSetting>") }}

{{ api_signature(value="fn has_content_file(&self, file_name: &str) -> bool") }}

{{ api_signature(value="fn add_content_file(&mut self, content_file: &str) -> Result<(), ConfigError>") }}

Appends to the end of the load order, attributed to the user's config. Fails with
`CannotAddContentFile`, naming the file that already lists it, if the name is anywhere in the
chain.

{{ api_signature(value="fn remove_content_file(&mut self, file_name: &str)") }}

Removes every entry with that name, whichever file it came from. Removing a parent's entry makes
the user's config replace the list with `replace=content` when saved.

{{ api_signature(value="fn set_content_files(&mut self, plugins: Option<Vec<String>>)") }}

Removes every `content=` entry and adds `plugins` in order, attributed to the user's config, behind
a `replace=content` line when a parent had contributed. `None` leaves the list empty. Does not
check for duplicates.

## Groundcover

`groundcover=` lines: plugins whose grass OpenMW instances instead of placing as objects.

{{ api_signature(value="fn groundcover_iter(&self) -> impl Iterator<Item = &FileSetting>") }}

{{ api_signature(value="fn has_groundcover_file(&self, file_name: &str) -> bool") }}

{{ api_signature(value="fn add_groundcover_file(&mut self, content_file: &str) -> Result<(), ConfigError>") }}

Appends, attributed to the user's config. Fails with `CannotAddGroundcoverFile` if the name is
already listed.

{{ api_signature(value="fn remove_groundcover_file(&mut self, file_name: &str)") }}

Removes every entry with that name. When one was a parent's, the user's config replaces the list
with `replace=groundcover` when saved. There is no `set_` form for groundcover.

## Archives

`fallback-archive=` lines: BSA archives, loaded in order.

{{ api_signature(value="fn fallback_archives_iter(&self) -> impl Iterator<Item = &FileSetting>") }}

{{ api_signature(value="fn has_archive_file(&self, file_name: &str) -> bool") }}

{{ api_signature(value="fn add_archive_file(&mut self, archive_file: &str) -> Result<(), ConfigError>") }}

Appends, attributed to the user's config. Fails with `CannotAddArchiveFile` if the name is already
listed.

{{ api_signature(value="fn remove_archive_file(&mut self, file_name: &str)") }}

Removes every entry with that name. When one was a parent's, the user's config replaces the list
with `replace=fallback-archive` when saved.

{{ api_signature(value="fn set_fallback_archives(&mut self, archives: Option<Vec<String>>)") }}

Removes every `fallback-archive=` entry and adds `archives` in order, attributed to the user's
config, behind `replace=fallback-archive` when a parent had contributed. `None` leaves none. Does
not check for duplicates.
