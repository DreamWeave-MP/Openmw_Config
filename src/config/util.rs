// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Dave Corley (S3kshun8)

pub fn debug_log(message: &str) {
    if std::env::var("CFG_DEBUG").is_ok() {
        println!("[CONFIG DEBUG]: {message}");
    }
}

pub fn is_writable(path: &std::path::Path) -> bool {
    if path.exists() {
        std::fs::OpenOptions::new().write(true).open(path).is_ok()
    } else {
        match path.parent() {
            Some(parent) => {
                let test_path = parent.join(".write_test_tmp");
                match std::fs::File::create(&test_path) {
                    Ok(_) => {
                        let _ = std::fs::remove_file(&test_path);
                        true
                    }
                    Err(_) => false,
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
        Err(crate::ConfigError::NotFileOrDirectory(check_path))
    } else if check_path.is_absolute() {
        Ok(check_path)
    } else if check_path.is_relative() {
        if check_path.exists() {
            Ok(std::fs::canonicalize(check_path)?)
        } else {
            Err(crate::ConfigError::NotFileOrDirectory(check_path))
        }
    } else {
        Err(crate::ConfigError::NotFileOrDirectory(check_path))
    }
}

/// Transposes an input directory or file path to an openmw.cfg path
/// Maybe could do with some additional validation
pub fn input_config_path(
    config_path: std::path::PathBuf,
) -> Result<std::path::PathBuf, crate::ConfigError> {
    let check_path = validate_path(config_path)?;

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
            }
            Err(crate::ConfigError::Io(err))
        }
    }
}
