// This file is part of Openmw_Config.
// Openmw_Config is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// Openmw_Config is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
// You should have received a copy of the GNU General Public License along with Openmw_Config. If not, see <https://www.gnu.org/licenses/>.

use crate::{GameSetting, GameSettingMeta};
use std::fmt;

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
    pub fn new(value: &str, source_config: &std::path::Path, comment: &mut String) -> Self {
        Self {
            meta: GameSettingMeta {
                source_config: source_config.to_path_buf(),
                comment: std::mem::take(comment),
            },
            value: value.to_string(),
        }
    }

    #[must_use] 
    pub fn value(&self) -> &String {
        &self.value
    }
}
