// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Dave Corley (S3kshun8)

use std::fmt;

use crate::{ConfigError, GameSetting, GameSettingMeta, bail_config};

/// The text encoding used to interpret byte strings in plugin data.
///
/// `OpenMW` supports exactly three Windows code-page encodings; any other value in an
/// `encoding=` line is rejected with [`ConfigError::BadEncoding`](crate::ConfigError::BadEncoding).
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum EncodingType {
    /// Windows code page 1250 — Central European (Polish, Czech, Slovak, …).
    WIN1250,
    /// Windows code page 1251 — Cyrillic (Russian, Ukrainian, …).
    WIN1251,
    /// Windows code page 1252 — Western European (English, French, German, …). The default.
    WIN1252,
}

impl std::fmt::Display for EncodingType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            EncodingType::WIN1250 => "win1250",
            EncodingType::WIN1251 => "win1251",
            EncodingType::WIN1252 => "win1252",
        };

        writeln!(f, "{value}")
    }
}

/// A parsed `encoding=` entry from an `openmw.cfg` file.
///
/// At most one `encoding=` entry is meaningful per resolved configuration chain (singleton
/// semantics). The encoding affects how byte strings in plugin records are decoded.
#[derive(Debug, Clone)]
pub struct EncodingSetting {
    meta: GameSettingMeta,
    encoding: EncodingType,
}

impl EncodingSetting {
    /// The parsed encoding type.
    #[must_use]
    pub fn value(&self) -> EncodingType {
        self.encoding
    }
}

impl PartialEq for EncodingSetting {
    fn eq(&self, other: &Self) -> bool {
        self.encoding == other.encoding
    }
}

impl GameSetting for EncodingSetting {
    fn meta(&self) -> &GameSettingMeta {
        &self.meta
    }
}

impl fmt::Display for EncodingSetting {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}encoding={}", self.meta.comment, self.encoding)
    }
}

impl<P: AsRef<std::path::Path>> TryFrom<(String, P, &mut String)> for EncodingSetting {
    type Error = ConfigError;

    fn try_from(
        (value, source_config, comment): (String, P, &mut String),
    ) -> Result<Self, Self::Error> {
        let source_config = source_config.as_ref().to_path_buf();

        let encoding = match value.as_str() {
            "win1250" => EncodingType::WIN1250,
            "win1251" => EncodingType::WIN1251,
            "win1252" => EncodingType::WIN1252,
            _ => bail_config!(bad_encoding, value, source_config),
        };

        let meta = GameSettingMeta {
            source_config,
            comment: comment.to_owned(),
        };
        comment.clear();

        Ok(EncodingSetting { meta, encoding })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn dummy_path() -> PathBuf {
        PathBuf::from("/tmp/test.cfg")
    }

    fn dummy_comment() -> String {
        "#OpenMW-Config Test Suite\n\n\n\n#EncodingSetting\n\n\n\n".into()
    }

    #[test]
    fn test_valid_encodings() {
        let encodings = vec![
            ("win1250", EncodingType::WIN1250),
            ("win1251", EncodingType::WIN1251),
            ("win1252", EncodingType::WIN1252),
        ];

        for (input, expected_variant) in encodings {
            let setting =
                EncodingSetting::try_from((input.to_string(), dummy_path(), &mut dummy_comment()))
                    .unwrap();

            assert_eq!(setting.encoding, expected_variant);
        }
    }

    #[test]
    fn test_invalid_encoding() {
        let err =
            EncodingSetting::try_from(("utf8".to_string(), dummy_path(), &mut dummy_comment()));

        assert!(matches!(err, Err(ConfigError::BadEncoding { .. })));
    }

    #[test]
    fn test_empty_encoding_string() {
        let err = EncodingSetting::try_from((String::new(), dummy_path(), &mut dummy_comment()));
        assert!(matches!(err, Err(ConfigError::BadEncoding { .. })));
    }

    #[test]
    fn test_source_path_preservation() {
        let path = PathBuf::from("/some/path/to/config.cfg");
        let setting = EncodingSetting::try_from((
            "win1251".to_string(),
            path.as_path(),
            &mut dummy_comment(),
        ))
        .unwrap();

        assert_eq!(setting.meta.source_config, path);
    }

    #[test]
    fn test_display_trait_output() {
        let setting = EncodingSetting::try_from((
            "win1250".to_string(),
            dummy_path().as_path(),
            &mut dummy_comment(),
        ))
        .unwrap();

        let rendered = setting.to_string();
        assert_eq!(
            rendered.trim(),
            format!("{}encoding=win1250", dummy_comment())
        );
    }
}
