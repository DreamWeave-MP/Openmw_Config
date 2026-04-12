// This file is part of Openmw_Config.
// Openmw_Config is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// Openmw_Config is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
// You should have received a copy of the GNU General Public License along with Openmw_Config. If not, see <https://www.gnu.org/licenses/>.

use std::{borrow::Cow, fmt};

use crate::{ConfigError, GameSetting, GameSettingMeta, bail_config};

#[derive(Debug, Clone)]
pub struct ColorGameSetting {
    meta: GameSettingMeta,
    key: String,
    value: (u8, u8, u8),
}

impl std::fmt::Display for ColorGameSetting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (r, g, b) = self.value;
        write!(f, "{}fallback={},{r},{g},{b}", self.meta.comment, self.key)
    }
}

#[derive(Debug, Clone)]
pub struct StringGameSetting {
    meta: GameSettingMeta,
    key: String,
    value: String,
}

impl std::fmt::Display for StringGameSetting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}fallback={},{}", self.meta.comment, self.key, self.value)
    }
}

#[derive(Debug, Clone)]
pub struct FloatGameSetting {
    meta: GameSettingMeta,
    key: String,
    value: f64,
}

impl std::fmt::Display for FloatGameSetting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}fallback={},{}", self.meta.comment, self.key, self.value)
    }
}

#[derive(Debug, Clone)]
pub struct IntGameSetting {
    meta: GameSettingMeta,
    key: String,
    value: i64,
}

impl std::fmt::Display for IntGameSetting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}fallback={},{}", self.meta.comment, self.key, self.value)
    }
}

#[derive(Debug, Clone)]
pub enum GameSettingType {
    Color(ColorGameSetting),
    String(StringGameSetting),
    Float(FloatGameSetting),
    Int(IntGameSetting),
}

impl GameSettingType {
    /// Returns the setting key — the text before the first comma in a `fallback=Key,Value` entry.
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use openmw_config::GameSettingType;
    /// let setting = GameSettingType::try_from(
    ///     ("iMaxLevel,50".to_string(), PathBuf::default(), &mut String::new())
    /// ).unwrap();
    /// assert_eq!(setting.key(), "iMaxLevel");
    /// ```
    #[must_use]
    pub fn key(&self) -> &String {
        match self {
            GameSettingType::Color(setting) => &setting.key,
            GameSettingType::String(setting) => &setting.key,
            GameSettingType::Float(setting) => &setting.key,
            GameSettingType::Int(setting) => &setting.key,
        }
    }

    /// Returns the setting value — the text after the first comma in a `fallback=Key,Value` entry.
    ///
    /// ```
    /// use std::path::PathBuf;
    /// use openmw_config::GameSettingType;
    /// let setting = GameSettingType::try_from(
    ///     ("iMaxLevel,50".to_string(), PathBuf::default(), &mut String::new())
    /// ).unwrap();
    /// assert_eq!(setting.value(), "50");
    /// ```
    #[must_use]
    pub fn value(&self) -> Cow<'_, str> {
        match self {
            GameSettingType::Color(setting) => {
                let (r, g, b) = setting.value;
                Cow::Owned(format!("{r},{g},{b}"))
            }
            GameSettingType::String(setting) => Cow::Borrowed(&setting.value),
            GameSettingType::Float(setting) => Cow::Owned(setting.value.to_string()),
            GameSettingType::Int(setting) => Cow::Owned(setting.value.to_string()),
        }
    }
}

impl std::fmt::Display for GameSettingType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GameSettingType::Color(s) => write!(f, "{s}"),
            GameSettingType::Float(s) => write!(f, "{s}"),
            GameSettingType::String(s) => write!(f, "{s}"),
            GameSettingType::Int(s) => write!(f, "{s}"),
        }
    }
}

impl GameSetting for GameSettingType {
    fn meta(&self) -> &GameSettingMeta {
        match self {
            GameSettingType::Color(s) => &s.meta,
            GameSettingType::String(s) => &s.meta,
            GameSettingType::Float(s) => &s.meta,
            GameSettingType::Int(s) => &s.meta,
        }
    }
}

impl PartialEq for GameSettingType {
    fn eq(&self, other: &Self) -> bool {
        use GameSettingType::{Color, String, Float, Int};

        match (self, other) {
            (Color(a), Color(b)) => a.key == b.key,
            (String(a), String(b)) => a.key == b.key,
            (Float(a), Float(b)) => a.key == b.key,
            (Int(a), Int(b)) => a.key == b.key,
            // Mismatched types should never be considered equal
            _ => false,
        }
    }
}

impl PartialEq<&str> for GameSettingType {
    fn eq(&self, other: &&str) -> bool {
        use GameSettingType::{Color, String, Float, Int};

        match self {
            Color(a) => a.key == *other,
            String(a) => a.key == *other,
            Float(a) => a.key == *other,
            Int(a) => a.key == *other,
        }
    }
}

impl Eq for GameSettingType {}

impl TryFrom<(String, std::path::PathBuf, &mut String)> for GameSettingType {
    type Error = ConfigError;

    fn try_from(
        (original_value, source_config, queued_comment): (String, std::path::PathBuf, &mut String),
    ) -> Result<Self, ConfigError> {
        let tokens: Vec<&str> = original_value.splitn(2, ',').collect();

        if tokens.len() < 2 {
            bail_config!(invalid_game_setting, original_value, source_config);
        }

        let key = tokens[0].to_string();
        let value = tokens[1].to_string();

        let meta = GameSettingMeta {
            source_config,
            comment: queued_comment.clone(),
        };

        queued_comment.clear();

        if let Some(color) = parse_color_value(&value) {
            return Ok(GameSettingType::Color(ColorGameSetting {
                meta,
                key,
                value: color,
            }));
        }

        if value.contains('.')
            && let Ok(f) = value.parse::<f64>() {
                return Ok(GameSettingType::Float(FloatGameSetting {
                    meta,
                    key,
                    value: f,
                }));
            }

        if let Ok(i) = value.parse::<i64>() {
            return Ok(GameSettingType::Int(IntGameSetting {
                meta,
                key,
                value: i,
            }));
        }

        Ok(GameSettingType::String(StringGameSetting {
            meta,
            key,
            value,
        }))
    }
}

fn parse_color_value(value: &str) -> Option<(u8, u8, u8)> {
    let parts: Vec<_> = value
        .split(',')
        .map(str::trim)
        .filter_map(|s| s.parse::<u8>().ok())
        .collect();

    match parts.as_slice() {
        [r, g, b] => Some((*r, *g, *b)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn default_meta() -> GameSettingMeta {
        GameSettingMeta {
            source_config: PathBuf::default(),
            comment: String::default(),
        }
    }

    #[test]
    fn test_value_string_setting() {
        let setting = GameSettingType::String(StringGameSetting {
            meta: default_meta(),
            key: "greeting".into(),
            value: "hello world".into(),
        });

        assert_eq!(setting.value(), "hello world");
    }

    #[test]
    fn test_value_int_setting() {
        let setting = GameSettingType::Int(IntGameSetting {
            meta: default_meta(),
            key: "MaxEyesOfTodd".into(),
            value: 3,
        });

        assert_eq!(setting.value(), "3");
    }

    #[test]
    fn test_value_float_setting() {
        let setting = GameSettingType::Float(FloatGameSetting {
            meta: default_meta(),
            key: "FLightAttenuationEnfuckulation".into(),
            value: 0.75,
        });

        assert_eq!(setting.value(), "0.75");
    }

    #[test]
    fn test_value_color_setting() {
        let setting = GameSettingType::Color(ColorGameSetting {
            meta: default_meta(),
            key: "hud_color".into(),
            value: (255, 128, 64),
        });

        assert_eq!(setting.value(), "255,128,64");
    }

    #[test]
    fn test_to_string_for_string_setting() {
        let setting = GameSettingType::String(StringGameSetting {
            meta: default_meta(),
            key: "sGreeting".into(),
            value: "Hello, Nerevar.".into(),
        });

        assert_eq!(setting.to_string(), "fallback=sGreeting,Hello, Nerevar.");
    }

    #[test]
    fn test_to_string_for_int_setting() {
        let setting = GameSettingType::Int(IntGameSetting {
            meta: default_meta(),
            key: "iMaxSpeed".into(),
            value: 42,
        });

        assert_eq!(setting.to_string(), "fallback=iMaxSpeed,42");
    }

    #[test]
    fn test_to_string_for_float_setting() {
        let setting = GameSettingType::Float(FloatGameSetting {
            meta: default_meta(),
            key: "fJumpHeight".into(),
            value: 1.75,
        });

        assert_eq!(setting.to_string(), "fallback=fJumpHeight,1.75");
    }

    #[test]
    fn test_to_string_for_color_setting() {
        let setting = GameSettingType::Color(ColorGameSetting {
            meta: default_meta(),
            key: "iHUDColor".into(),
            value: (128, 64, 255),
        });

        assert_eq!(setting.to_string(), "fallback=iHUDColor,128,64,255");
    }

    #[test]
    fn test_commented_string() {
        let setting = GameSettingType::Color(ColorGameSetting {
            meta: GameSettingMeta { source_config: PathBuf::from("$HOME/.config/openmw/openmw.cfg"), comment: String::from("#Monochrome UI Settings\n#\n#\n#\n#######\n##\n##\n##\n") },
            key: "iHUDColor".into(),
            value: (128, 64, 255),
        });

        assert_eq!(setting.to_string(), "#Monochrome UI Settings\n#\n#\n#\n#######\n##\n##\n##\nfallback=iHUDColor,128,64,255");
    }

    // --- TryFrom parsing ---

    fn parse(s: &str) -> Result<GameSettingType, crate::ConfigError> {
        GameSettingType::try_from((s.to_string(), PathBuf::default(), &mut String::new()))
    }

    #[test]
    fn test_parse_string_value() {
        let setting = parse("sMyKey,hello world").unwrap();
        assert!(matches!(setting, GameSettingType::String(_)));
        assert_eq!(setting.key(), "sMyKey");
        assert_eq!(setting.value(), "hello world");
    }

    #[test]
    fn test_parse_integer_value() {
        let setting = parse("iSpeed,42").unwrap();
        assert!(matches!(setting, GameSettingType::Int(_)));
        assert_eq!(setting.value(), "42");
    }

    #[test]
    fn test_parse_negative_integer() {
        let setting = parse("iDepth,-100").unwrap();
        assert!(matches!(setting, GameSettingType::Int(_)));
        assert_eq!(setting.value(), "-100");
    }

    #[test]
    fn test_parse_float_value() {
        let setting = parse("fGravity,9.81").unwrap();
        assert!(matches!(setting, GameSettingType::Float(_)));
        assert_eq!(setting.value(), "9.81");
    }

    #[test]
    fn test_parse_color_value() {
        let setting = parse("iSkyColor,100,149,237").unwrap();
        assert!(matches!(setting, GameSettingType::Color(_)));
        assert_eq!(setting.value(), "100,149,237");
    }

    #[test]
    fn test_parse_missing_comma_errors() {
        assert!(parse("NoCommaAtAll").is_err());
    }

    #[test]
    fn test_parse_value_with_comma_stays_string() {
        // A string value that contains a comma should not be misidentified as color
        let setting = parse("sMessage,Hello, traveller").unwrap();
        assert!(matches!(setting, GameSettingType::String(_)));
        assert_eq!(setting.value(), "Hello, traveller");
    }

    #[test]
    fn test_parse_ambiguous_two_number_value_is_string() {
        // Two comma-separated numbers is NOT a valid color (needs 3), must fall back to String
        let setting = parse("sKey,10,20").unwrap();
        assert!(matches!(setting, GameSettingType::String(_)));
    }

    #[test]
    fn test_parse_color_out_of_u8_range_is_string() {
        // Values > 255 can't be u8 so the whole thing should parse as String
        let setting = parse("sBig,256,0,0").unwrap();
        assert!(matches!(setting, GameSettingType::String(_)));
    }

    #[test]
    fn test_parse_comment_consumed() {
        let mut comment = String::from("# some note\n");
        let setting = GameSettingType::try_from((
            "iVal,1".to_string(),
            PathBuf::default(),
            &mut comment,
        )).unwrap();
        assert_eq!(setting.meta().comment, "# some note\n");
        assert!(comment.is_empty(), "comment should be consumed");
    }

    // --- Equality ---

    #[test]
    fn test_same_key_same_type_are_equal() {
        let a = parse("iKey,1").unwrap();
        let b = parse("iKey,2").unwrap();
        assert_eq!(a, b, "equality is key-only within the same type");
    }

    #[test]
    fn test_different_keys_not_equal() {
        let a = parse("iKey,1").unwrap();
        let b = parse("iOther,1").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn test_mismatched_types_not_equal() {
        // "1" parses as Int; "1.0" parses as Float — same logical key, different types
        let int_setting = parse("iKey,1").unwrap();
        let float_setting = parse("iKey,1.0").unwrap();
        assert_ne!(int_setting, float_setting);
    }

    #[test]
    fn test_eq_with_str_key() {
        let setting = parse("iMaxLevel,50").unwrap();
        assert_eq!(setting, "iMaxLevel");
        assert_ne!(setting, "iOtherKey");
    }
}
