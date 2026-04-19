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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_new_consumes_comment_and_sets_metadata() {
        let source = PathBuf::from("/tmp/openmw.cfg");
        let mut comment = String::from("# note\n");

        let setting = FileSetting::new("Morrowind.esm", &source, &mut comment);

        assert_eq!(setting.value(), "Morrowind.esm");
        assert_eq!(setting.meta().source_config, source);
        assert_eq!(setting.meta().comment, "# note\n");
        assert!(comment.is_empty());
    }

    #[test]
    fn test_display_outputs_only_file_value() {
        let source = PathBuf::from("/tmp/openmw.cfg");
        let mut comment = String::new();
        let setting = FileSetting::new("Tribunal.esm", &source, &mut comment);

        assert_eq!(setting.to_string(), "Tribunal.esm");
    }

    #[test]
    fn test_partial_eq_variants_compare_by_value_only() {
        let source = PathBuf::from("/tmp/openmw.cfg");
        let mut comment = String::from("# ignored\n");

        let lhs = FileSetting::new("Bloodmoon.esm", &source, &mut comment);
        let rhs = FileSetting::new("Bloodmoon.esm", &source, &mut String::new());
        let str_owned = String::from("Bloodmoon.esm");

        assert_eq!(lhs, rhs);
        assert_eq!(lhs, "Bloodmoon.esm");
        assert_eq!(lhs, str_owned.as_str());
        assert_eq!(lhs, &str_owned);
    }
}
