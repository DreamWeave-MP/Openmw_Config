// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Dave Corley (S3kshun8)

use crate::{GameSetting, GameSettingMeta};
use std::fmt;

/// A plain filename entry from an `openmw.cfg` file (`content=`, `fallback-archive=`, `groundcover=`).
///
/// Stores only the filename string — no path resolution is applied, since these entries name
/// files looked up through the VFS rather than direct filesystem paths.
///
/// `PartialEq` comparisons are value-only and ignore source metadata, making it straightforward
/// to check whether a particular file is present regardless of which config file defined it.
#[derive(Debug, Clone)]
pub struct FileSetting {
    meta: GameSettingMeta,
    value: String,
}

impl PartialEq for FileSetting {
    fn eq(&self, other: &Self) -> bool {
        &self.value == other.value()
    }
}

impl PartialEq<&str> for FileSetting {
    fn eq(&self, other: &&str) -> bool {
        self.value == *other
    }
}

impl PartialEq<str> for FileSetting {
    fn eq(&self, other: &str) -> bool {
        self.value == other
    }
}

impl PartialEq<&String> for FileSetting {
    fn eq(&self, other: &&String) -> bool {
        &self.value == *other
    }
}

impl GameSetting for FileSetting {
    fn meta(&self) -> &GameSettingMeta {
        &self.meta
    }
}

impl fmt::Display for FileSetting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl FileSetting {
    /// Creates a new `FileSetting` attributed to `source_config`.
    ///
    /// Consumes the accumulated `comment` string (via [`std::mem::take`]).
    pub fn new(value: &str, source_config: &std::path::Path, comment: &mut String) -> Self {
        Self {
            meta: GameSettingMeta {
                source_config: source_config.to_path_buf(),
                comment: std::mem::take(comment),
            },
            value: value.to_string(),
        }
    }

    /// The filename string as it appeared in the `openmw.cfg` file.
    #[must_use]
    pub fn value(&self) -> &String {
        &self.value
    }
}
