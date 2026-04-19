// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Dave Corley (S3kshun8)

use std::{fmt, path::PathBuf};

#[macro_export]
macro_rules! config_err {
    // InvalidGameSetting: value, path
    (invalid_game_setting, $value:expr, $path:expr) => {
        $crate::ConfigError::InvalidGameSetting {
            value: $value.to_string(),
            config_path: $path.to_path_buf(),
            line: None,
        }
    };

    (invalid_game_setting, $value:expr, $path:expr, $line:expr) => {
        $crate::ConfigError::InvalidGameSetting {
            value: $value.to_string(),
            config_path: $path.to_path_buf(),
            line: Some($line),
        }
    };

    (not_file_or_directory, $config_path:expr) => {
        $crate::ConfigError::NotFileOrDirectory($config_path.to_path_buf())
    };

    (cannot_find, $config_path:expr) => {
        $crate::ConfigError::CannotFind($config_path.to_path_buf())
    };

    (duplicate_content_file, $content_file:expr, $config_path:expr) => {
        $crate::ConfigError::DuplicateContentFile {
            file: $content_file,
            config_path: $config_path.to_path_buf(),
            line: None,
        }
    };

    (duplicate_content_file, $content_file:expr, $config_path:expr, $line:expr) => {
        $crate::ConfigError::DuplicateContentFile {
            file: $content_file,
            config_path: $config_path.to_path_buf(),
            line: Some($line),
        }
    };

    (duplicate_archive_file, $archive_file:expr, $config_path:expr) => {
        $crate::ConfigError::DuplicateArchiveFile {
            file: $archive_file,
            config_path: $config_path.to_path_buf(),
            line: None,
        }
    };

    (duplicate_archive_file, $archive_file:expr, $config_path:expr, $line:expr) => {
        $crate::ConfigError::DuplicateArchiveFile {
            file: $archive_file,
            config_path: $config_path.to_path_buf(),
            line: Some($line),
        }
    };

    (archive_already_defined, $content_file:expr, $config_path:expr) => {
        $crate::ConfigError::CannotAddArchiveFile {
            file: $content_file,
            config_path: $config_path.to_path_buf(),
        }
    };

    (content_already_defined, $content_file:expr, $config_path:expr) => {
        $crate::ConfigError::CannotAddContentFile {
            file: $content_file,
            config_path: $config_path.to_path_buf(),
        }
    };

    (groundcover_already_defined, $groundcover_file:expr, $config_path:expr) => {
        $crate::ConfigError::CannotAddGroundcoverFile {
            file: $groundcover_file,
            config_path: $config_path.to_path_buf(),
        }
    };

    (duplicate_groundcover_file, $groundcover_file:expr, $config_path:expr) => {
        $crate::ConfigError::DuplicateGroundcoverFile {
            file: $groundcover_file,
            config_path: $config_path.to_path_buf(),
            line: None,
        }
    };

    (duplicate_groundcover_file, $groundcover_file:expr, $config_path:expr, $line:expr) => {
        $crate::ConfigError::DuplicateGroundcoverFile {
            file: $groundcover_file,
            config_path: $config_path.to_path_buf(),
            line: Some($line),
        }
    };

    (bad_encoding, $encoding:expr, $config_path:expr) => {
        $crate::ConfigError::BadEncoding {
            value: $encoding,
            config_path: $config_path,
            line: None,
        }
    };

    (bad_encoding, $encoding:expr, $config_path:expr, $line:expr) => {
        $crate::ConfigError::BadEncoding {
            value: $encoding,
            config_path: $config_path,
            line: Some($line),
        }
    };

    (invalid_line, $value:expr, $config_path:expr) => {
        $crate::ConfigError::InvalidLine {
            value: $value,
            config_path: $config_path,
            line: None,
        }
    };

    (invalid_line, $value:expr, $config_path:expr, $line:expr) => {
        $crate::ConfigError::InvalidLine {
            value: $value,
            config_path: $config_path,
            line: Some($line),
        }
    };

    (not_writable, $path:expr) => {
        $crate::ConfigError::NotWritable($path.to_path_buf())
    };

    (subconfig_not_loaded, $path:expr) => {
        $crate::ConfigError::SubconfigNotLoaded($path.to_path_buf())
    };

    (max_depth_exceeded, $path:expr) => {
        $crate::ConfigError::MaxDepthExceeded($path.to_path_buf())
    };

    // Wrap std::io::Error
    (io, $err:expr) => {
        $crate::ConfigError::Io($err)
    };
}

#[macro_export]
macro_rules! bail_config {
    ($($tt:tt)*) => {
        {
        return Err($crate::config_err!($($tt)*));
    }
};
}

/// Errors that can arise while loading, mutating, or saving an `OpenMW` configuration.
#[derive(Debug)]
#[non_exhaustive]
pub enum ConfigError {
    /// A content file (`content=`) appeared twice in the configuration chain.
    DuplicateContentFile {
        file: String,
        config_path: PathBuf,
        line: Option<usize>,
    },
    /// A fallback-archive (`fallback-archive=`) appeared twice in the configuration chain.
    DuplicateArchiveFile {
        file: String,
        config_path: PathBuf,
        line: Option<usize>,
    },
    /// [`OpenMWConfiguration::add_content_file`](crate::OpenMWConfiguration::add_content_file)
    /// was called for a file that is already present.
    CannotAddContentFile { file: String, config_path: PathBuf },
    /// [`OpenMWConfiguration::add_archive_file`](crate::OpenMWConfiguration::add_archive_file)
    /// was called for an archive that is already present.
    CannotAddArchiveFile { file: String, config_path: PathBuf },
    /// A groundcover file (`groundcover=`) appeared twice in the configuration chain.
    DuplicateGroundcoverFile {
        file: String,
        config_path: PathBuf,
        line: Option<usize>,
    },
    /// [`OpenMWConfiguration::add_groundcover_file`](crate::OpenMWConfiguration::add_groundcover_file)
    /// was called for a file that is already present.
    CannotAddGroundcoverFile { file: String, config_path: PathBuf },
    /// A `fallback=` entry could not be parsed as a valid `Key,Value` pair.
    InvalidGameSetting {
        value: String,
        config_path: PathBuf,
        line: Option<usize>,
    },
    /// An `encoding=` entry contained an unrecognised encoding name.
    /// Only `win1250`, `win1251`, and `win1252` are valid.
    BadEncoding {
        value: String,
        config_path: PathBuf,
        line: Option<usize>,
    },
    /// A line in an `openmw.cfg` file did not match any recognised `key=value` format.
    InvalidLine {
        value: String,
        config_path: PathBuf,
        line: Option<usize>,
    },
    /// An I/O error occurred while reading or writing a config file.
    Io(std::io::Error),
    /// The supplied path could not be classified as a file or directory.
    NotFileOrDirectory(PathBuf),
    /// No `openmw.cfg` was found at the given path.
    CannotFind(PathBuf),
    /// The target path for a save operation is not writable.
    NotWritable(PathBuf),
    /// [`OpenMWConfiguration::save_subconfig`](crate::OpenMWConfiguration::save_subconfig)
    /// was called with a path that is not part of the loaded configuration chain.
    SubconfigNotLoaded(PathBuf),
    /// The `config=` chain exceeded the maximum nesting depth, likely due to a circular reference.
    MaxDepthExceeded(PathBuf),
    /// Could not resolve a platform default path via `dirs`.
    PlatformPathUnavailable(&'static str),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::InvalidGameSetting {
                value,
                config_path,
                line,
            } => {
                let line_suffix = line
                    .map(|line| format!(" at line {line}"))
                    .unwrap_or_default();
                write!(
                    f,
                    "Invalid fallback setting '{}' in config file '{}'{}",
                    value,
                    config_path.display(),
                    line_suffix
                )
            }
            ConfigError::Io(e) => write!(f, "IO error: {e}"),
            ConfigError::NotFileOrDirectory(config_path) => write!(
                f,
                "Unable to determine whether {} was a file or directory, refusing to read.",
                config_path.display()
            ),
            ConfigError::CannotFind(config_path) => {
                write!(
                    f,
                    "An openmw.cfg does not exist at: {}",
                    config_path.display()
                )
            }
            ConfigError::DuplicateContentFile {
                file,
                config_path,
                line,
            } => {
                let line_suffix = line
                    .map(|line| format!(" at line {line}"))
                    .unwrap_or_default();
                write!(
                    f,
                    "{file} has appeared in the content files list twice. Its second occurence was in: {}{}",
                    config_path.display(),
                    line_suffix
                )
            }
            ConfigError::CannotAddContentFile { file, config_path } => write!(
                f,
                "{file} cannot be added to the configuration map as a content file because it was already defined by: {}",
                config_path.display()
            ),
            ConfigError::DuplicateGroundcoverFile {
                file,
                config_path,
                line,
            } => {
                let line_suffix = line
                    .map(|line| format!(" at line {line}"))
                    .unwrap_or_default();
                write!(
                    f,
                    "{file} has appeared in the groundcover list twice. Its second occurence was in: {}{}",
                    config_path.display(),
                    line_suffix
                )
            }
            ConfigError::CannotAddGroundcoverFile { file, config_path } => write!(
                f,
                "{file} cannot be added to the configuration map as a groundcover plugin because it was already defined by: {}",
                config_path.display()
            ),
            ConfigError::DuplicateArchiveFile {
                file,
                config_path,
                line,
            } => {
                let line_suffix = line
                    .map(|line| format!(" at line {line}"))
                    .unwrap_or_default();
                write!(
                    f,
                    "{file} has appeared in the BSA/Archive list twice. Its second occurence was in: {}{}",
                    config_path.display(),
                    line_suffix
                )
            }
            ConfigError::CannotAddArchiveFile { file, config_path } => write!(
                f,
                "{file} cannot be added to the configuration map as a fallback-archive because it was already defined by: {}",
                config_path.display()
            ),
            ConfigError::BadEncoding {
                value,
                config_path,
                line,
            } => {
                let line_suffix = line
                    .map(|line| format!(" at line {line}"))
                    .unwrap_or_default();
                write!(
                    f,
                    "Invalid encoding type: {value} in config file {}{}",
                    config_path.display(),
                    line_suffix
                )
            }
            ConfigError::InvalidLine {
                value,
                config_path,
                line,
            } => {
                let line_suffix = line
                    .map(|line| format!(" at line {line}"))
                    .unwrap_or_default();
                write!(
                    f,
                    "Invalid pair in openmw.cfg {value} was defined by {}{}",
                    config_path.display(),
                    line_suffix
                )
            }
            ConfigError::NotWritable(path) => {
                write!(f, "Target path is not writable: {}", path.display())
            }
            ConfigError::SubconfigNotLoaded(path) => write!(
                f,
                "Cannot save to {}; it is not part of the loaded configuration chain",
                path.display()
            ),
            ConfigError::MaxDepthExceeded(path) => write!(
                f,
                "Maximum config= nesting depth exceeded while loading {}",
                path.display()
            ),
            ConfigError::PlatformPathUnavailable(kind) => {
                write!(f, "Failed to resolve platform default {kind} path")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(err: std::io::Error) -> Self {
        ConfigError::Io(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_messages_include_key_context() {
        let path = PathBuf::from("/tmp/openmw.cfg");

        let cannot_find = ConfigError::CannotFind(path.clone()).to_string();
        assert!(cannot_find.contains("openmw.cfg"));

        let duplicate = ConfigError::DuplicateContentFile {
            file: "Morrowind.esm".into(),
            config_path: path.clone(),
            line: None,
        }
        .to_string();
        assert!(duplicate.contains("Morrowind.esm"));

        let invalid_line = ConfigError::InvalidLine {
            value: "broken".into(),
            config_path: path,
            line: Some(42),
        }
        .to_string();
        assert!(invalid_line.contains("broken"));
        assert!(invalid_line.contains("line 42"));
    }

    #[test]
    fn test_from_io_error_wraps_variant() {
        let io = std::io::Error::other("boom");
        let converted: ConfigError = io.into();
        assert!(matches!(converted, ConfigError::Io(_)));
    }
}
