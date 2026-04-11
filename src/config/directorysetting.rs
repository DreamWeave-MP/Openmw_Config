// This file is part of Openmw_Config.
// Openmw_Config is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// Openmw_Config is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
// You should have received a copy of the GNU General Public License along with Openmw_Config. If not, see <https://www.gnu.org/licenses/>.

use crate::config::strings;
use std::{fmt, path::PathBuf};

#[derive(Debug, Clone)]
pub struct DirectorySetting {
    pub meta: crate::GameSettingMeta,
    original: String,
    parsed: PathBuf,
}

/// This is tricky.
/// The trait implementation for GameSetting necessitates that all settings have a Display method.
/// However, DirectorySetting is reused interchangeably amongst variants that use a different key. So really the key should just be skipped here,
/// And handled by the upper SettingValue implementation?
/// But that, also, is fucked off, because then we wouldn't be able to handle comments.
/// So the hope I guess is that the SettingValue itself can have an implementation to account for this.
/// That seems fair?
/// And then we just assume data= is the default in here.
impl std::fmt::Display for DirectorySetting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", self.original)
    }
}

impl crate::GameSetting for DirectorySetting {
    fn meta(&self) -> &crate::GameSettingMeta {
        &self.meta
    }
}

/// Refactor to clone less shit
/// Use std::mem::take for the comment and change parse_data_directory to accept &str
impl DirectorySetting {
    pub fn new<S: Into<String>>(value: S, source_config: PathBuf, comment: &mut String) -> Self {
        let original = value.into();
        let parsed = strings::parse_data_directory(&source_config, original.clone());

        let meta = crate::GameSettingMeta {
            source_config: source_config,
            comment: comment.clone(),
        };
        comment.clear();

        Self {
            original,
            parsed,
            meta,
        }
    }

    pub fn original(&self) -> &String {
        &self.original
    }

    pub fn parsed(&self) -> &PathBuf {
        &self.parsed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_directory_setting_basic_construction() {
        let config_path = PathBuf::from("/my/config");
        let mut comment = "some comment".to_string();

        let setting = DirectorySetting::new("data", config_path.clone(), &mut comment);

        assert_eq!(setting.original, "data");
        assert_eq!(setting.parsed, config_path.join("data"));
        assert_eq!(setting.meta.source_config, config_path);
        assert_eq!(setting.meta.comment, "some comment");
        assert!(comment.is_empty()); // Should have been cleared
    }

    #[test]
    fn test_directory_setting_with_user_data_token() {
        let config_path = PathBuf::from("/irrelevant");
        let mut comment = String::new();

        let setting = DirectorySetting::new("?userdata?/foo", config_path, &mut comment);

        let expected_prefix = crate::default_userdata_path();
        assert!(setting.parsed.starts_with(expected_prefix));
        assert!(setting.parsed.ends_with("foo/"));
    }

    #[test]
    fn test_directory_setting_with_user_config_token() {
        let config_path = PathBuf::from("/config/dir");
        let mut comment = String::new();

        let setting = DirectorySetting::new("?userconfig?/bar", config_path, &mut comment);
        dbg!(setting.parsed());

        let expected_prefix = crate::default_config_path();
        assert!(setting.parsed.starts_with(expected_prefix));
        assert!(setting.parsed.ends_with("bar"));
    }

    #[test]
    fn test_directory_setting_quoted_path() {
        let config_path = PathBuf::from("/my/config");
        let mut comment = String::new();

        let setting =
            DirectorySetting::new("\"path/with spaces\"", config_path.clone(), &mut comment);

        assert_eq!(setting.original, "\"path/with spaces\"");
        assert_eq!(setting.parsed, config_path.join("path").join("with spaces"));
    }

    #[test]
    fn test_directory_setting_relative_path_normalization() {
        let config_path = PathBuf::from("/my/config");
        let mut comment = String::new();

        let setting = DirectorySetting::new("subdir\\nested", config_path.clone(), &mut comment);

        let expected = config_path.join("subdir").join("nested");
        assert_eq!(setting.parsed, expected);
    }

    fn mock_path(path: &str) -> PathBuf {
        PathBuf::from(path)
    }

    #[test]
    fn test_dot_component_is_removed() {
        let config = mock_path("/etc/openmw");
        let mut comment = String::from("comment");
        let setting = DirectorySetting::new("./data", config.clone(), &mut comment);
        assert_eq!(setting.parsed(), &config.join("data"));
    }

    #[test]
    fn test_double_dot_not_normalized() {
        // OpenMW does not normalize .. — the raw joined path is preserved
        let config = mock_path("/home/user/.config/openmw");
        let mut comment = String::from("comment");
        let setting = DirectorySetting::new("../common", config.clone(), &mut comment);
        let expected = config.join("../common");
        assert_eq!(setting.parsed(), &expected);
    }

    #[test]
    fn test_dot_components_not_normalized() {
        // OpenMW does not normalize . or .. in the middle of a path
        let config = mock_path("/opt/game/config");
        let mut comment = String::new();
        let setting = DirectorySetting::new("foo/./bar/../baz", config.clone(), &mut comment);
        let expected = config.join("foo/./bar/../baz");
        assert_eq!(setting.parsed(), &expected);
    }
}
