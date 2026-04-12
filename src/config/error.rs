// This file is part of Openmw_Config.
// Openmw_Config is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// Openmw_Config is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
// You should have received a copy of the GNU General Public License along with Openmw_Config. If not, see <https://www.gnu.org/licenses/>.

use std::{fmt, path::PathBuf};

#[macro_export]
macro_rules! config_err {
    // InvalidGameSetting: value, path
    (invalid_game_setting, $value:expr, $path:expr) => {
        $crate::ConfigError::InvalidGameSetting {
            value: $value.to_string(),
            config_path: $path.to_path_buf(),
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
        }
    };

    (duplicate_archive_file, $archive_file:expr, $config_path:expr) => {
        $crate::ConfigError::DuplicateArchiveFile {
            file: $archive_file,
            config_path: $config_path.to_path_buf(),
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
        }
    };

    (bad_encoding, $encoding:expr, $config_path:expr) => {
        $crate::ConfigError::BadEncoding {
            value: $encoding,
            config_path: $config_path,
        }
    };

    (invalid_line, $value:expr, $config_path:expr) => {
        $crate::ConfigError::InvalidLine {
            value: $value,
            config_path: $config_path,
        }
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

#[derive(Debug)]
pub enum ConfigError {
    DuplicateContentFile { file: String, config_path: PathBuf },
    DuplicateArchiveFile { file: String, config_path: PathBuf },
    CannotAddContentFile { file: String, config_path: PathBuf },
    CannotAddArchiveFile { file: String, config_path: PathBuf },
    DuplicateGroundcoverFile { file: String, config_path: PathBuf },
    CannotAddGroundcoverFile { file: String, config_path: PathBuf },
    InvalidGameSetting { value: String, config_path: PathBuf },
    BadEncoding { value: String, config_path: PathBuf },
    InvalidLine { value: String, config_path: PathBuf },
    Io(std::io::Error),
    NotFileOrDirectory(PathBuf),
    CannotFind(PathBuf),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::InvalidGameSetting { value, config_path } => {
                write!(
                    f,
                    "Invalid fallback setting '{}' in config file '{}'",
                    value,
                    config_path.display()
                )
            }
            ConfigError::Io(e) => write!(f, "IO error: {e}"),
            ConfigError::NotFileOrDirectory(config_path) => write!(
                f,
                "Unable to determine whether {} was a file or directory, refusing to read.",
                config_path.display()
            ),
            ConfigError::CannotFind(config_path) => {
                write!(f, "An openmw.cfg does not exist at: {}", config_path.display())
            }
            ConfigError::DuplicateContentFile { file, config_path } => write!(
                f,
                "{file} has appeared in the content files list twice. Its second occurence was in: {}",
                config_path.display()
            ),
            ConfigError::CannotAddContentFile { file, config_path } => write!(
                f,
                "{file} cannot be added to the configuration map as a content file because it was already defined by: {}",
                config_path.display()
            ),
            ConfigError::DuplicateGroundcoverFile { file, config_path } => write!(
                f,
                "{file} has appeared in the groundcover list twice. Its second occurence was in: {}",
                config_path.display()
            ),
            ConfigError::CannotAddGroundcoverFile { file, config_path } => write!(
                f,
                "{file} cannot be added to the configuration map as a groundcover plugin because it was already defined by: {}",
                config_path.display()
            ),
            ConfigError::DuplicateArchiveFile { file, config_path } => write!(
                f,
                "{file} has appeared in the BSA/Archive list twice. Its second occurence was in: {}",
                config_path.display()
            ),
            ConfigError::CannotAddArchiveFile { file, config_path } => write!(
                f,
                "{file} cannot be added to the configuration map as a fallback-archive because it was already defined by: {}",
                config_path.display()
            ),
            ConfigError::BadEncoding { value, config_path } => write!(
                f,
                "Invalid encoding type: {value} in config file {}",
                config_path.display()
            ),
            ConfigError::InvalidLine { value, config_path } => write!(
                f,
                "Invalid pair in openmw.cfg {value} was defined by {}",
                config_path.display()
            ),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<std::io::Error> for ConfigError {
    fn from(err: std::io::Error) -> Self {
        ConfigError::Io(err)
    }
}
