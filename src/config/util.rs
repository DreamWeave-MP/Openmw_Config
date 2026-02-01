// This file is part of Openmw_Config.
// Openmw_Config is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// Openmw_Config is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
// You should have received a copy of the GNU General Public License along with Openmw_Config. If not, see <https://www.gnu.org/licenses/>.

pub fn debug_log(message: String) {
    if std::env::var("CFG_DEBUG").is_ok() {
        println!("[CONFIG DEBUG]: {message}")
    }
}

pub fn user_config_path(
    sub_configs: &Vec<&std::path::PathBuf>,
    fallthrough_dir: &std::path::PathBuf,
) -> std::path::PathBuf {
    sub_configs
        .into_iter()
        .last()
        .unwrap_or(&fallthrough_dir)
        .to_path_buf()
}

pub fn is_writable(path: &std::path::PathBuf) -> bool {
    if path.exists() {
        match std::fs::OpenOptions::new().write(true).open(path) {
            Ok(_) => true,
            Err(e) => e.kind() != std::io::ErrorKind::PermissionDenied,
        }
    } else {
        match path.parent() {
            Some(parent) => {
                let test_path = parent.join(".write_test_tmp");
                match std::fs::File::create(&test_path) {
                    Ok(_) => {
                        let _ = std::fs::remove_file(&test_path);
                        true
                    }
                    Err(e) => e.kind() != std::io::ErrorKind::PermissionDenied,
                }
            }
            None => false,
        }
    }
}

pub fn validate_path(
    check_path: std::path::PathBuf,
) -> Result<std::path::PathBuf, crate::ConfigError> {
    if check_path.as_os_str().is_empty() {
        return Err(crate::ConfigError::NotFileOrDirectory(check_path));
    } else if check_path.is_absolute() {
        return Ok(check_path);
    } else if check_path.is_relative() {
        return Ok(std::fs::canonicalize(check_path)?);
    } else {
        return Err(crate::ConfigError::NotFileOrDirectory(check_path));
    }
}

/// Transposes an input directory or file path to an openmw.cfg path
/// Maybe could do with some additional validation
pub fn input_config_path(
    config_path: std::path::PathBuf,
) -> Result<std::path::PathBuf, crate::ConfigError> {
    let check_path = match validate_path(config_path) {
        Err(error) => return Err(error),
        Ok(path) => path,
    };

    match std::fs::metadata(&check_path) {
        Ok(metadata) => {
            if metadata.is_dir() {
                let maybe_config = check_path.join("openmw.cfg");

                if maybe_config.is_file() || maybe_config.is_symlink() {
                    Ok(maybe_config)
                } else {
                    crate::config::bail_config!(cannot_find, check_path);
                }
            } else if metadata.is_symlink() || metadata.is_file() {
                Ok(check_path)
            } else {
                crate::config::bail_config!(not_file_or_directory, check_path);
            }
        }
        Err(err) => {
            if err.kind() == std::io::ErrorKind::NotFound {
                crate::config::bail_config!(not_file_or_directory, check_path);
            } else {
                Err(crate::ConfigError::Io(err))
            }
        }
    }
}
