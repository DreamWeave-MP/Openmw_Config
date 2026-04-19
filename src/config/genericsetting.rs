// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Dave Corley (S3kshun8)

use crate::{GameSetting, GameSettingMeta};
use std::fmt;

#[derive(Debug, Clone)]
pub struct GenericSetting {
    meta: GameSettingMeta,
    key: String,
    value: String,
}

impl GameSetting for GenericSetting {
    fn meta(&self) -> &GameSettingMeta {
        &self.meta
    }
}

impl fmt::Display for GenericSetting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}={}", self.meta.comment, self.key, self.value)
    }
}

impl GenericSetting {
    pub fn new(
        key: &str,
        value: &str,
        source_config: &std::path::Path,
        comment: &mut String,
    ) -> Self {
        Self {
            meta: GameSettingMeta {
                source_config: source_config.to_path_buf(),
                comment: std::mem::take(comment),
            },
            key: key.to_string(),
            value: value.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_new_consumes_comment_and_tracks_source() {
        let source = PathBuf::from("/tmp/openmw.cfg");
        let mut comment = String::from("# heading\n");

        let setting = GenericSetting::new("unknown-key", "value", &source, &mut comment);

        assert_eq!(setting.meta().source_config, source);
        assert_eq!(setting.meta().comment, "# heading\n");
        assert!(comment.is_empty());
    }

    #[test]
    fn test_display_emits_comment_and_pair() {
        let source = PathBuf::from("/tmp/openmw.cfg");
        let mut comment = String::from("# comment\n");

        let setting = GenericSetting::new("foo", "bar", &source, &mut comment);
        assert_eq!(setting.to_string(), "# comment\nfoo=bar");
    }
}
