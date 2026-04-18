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
