// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Dave Corley (S3kshun8)

#[must_use]
pub fn debug_enabled() -> bool {
    std::env::var("CFG_DEBUG").is_ok()
}

pub fn debug_log(message: &str) {
    if debug_enabled() {
        println!("[CONFIG DEBUG]: {message}");
    }
}

pub fn debug_log_lazy<F>(message: F)
where
    F: FnOnce() -> String,
{
    if debug_enabled() {
        println!("[CONFIG DEBUG]: {}", message());
    }
}

#[must_use]
pub fn expand_leading_tilde(path: &str) -> std::path::PathBuf {
    if path == "~" {
        return crate::platform_paths::home_dir().unwrap_or_else(|_| std::path::PathBuf::from(path));
    }

    if let Some(rest) = path
        .strip_prefix("~/")
        .or_else(|| path.strip_prefix("~\\"))
        && let Ok(home) = crate::platform_paths::home_dir()
    {
        return home.join(rest);
    }

    std::path::PathBuf::from(path)
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

#[cfg(test)]
mod tests {
    use super::expand_leading_tilde;

    #[test]
    fn test_expand_tilde_leaves_regular_path_unchanged() {
        let value = "/tmp/example";
        assert_eq!(expand_leading_tilde(value), std::path::PathBuf::from(value));
    }

    #[test]
    fn test_expand_tilde_does_not_expand_named_user_syntax() {
        let value = "~alice/mods";
        assert_eq!(expand_leading_tilde(value), std::path::PathBuf::from(value));
    }

    #[test]
    fn test_expand_tilde_home_variants() {
        let home = crate::platform_paths::home_dir().expect("home directory required for test");

        assert_eq!(expand_leading_tilde("~"), home);
        assert_eq!(expand_leading_tilde("~/mods"), home.join("mods"));
        assert_eq!(expand_leading_tilde("~\\mods"), home.join("mods"));
    }
}
