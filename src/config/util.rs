// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025 Dave Corley (S3kshun8)

use std::path::{Path, PathBuf};

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
        return crate::platform_paths::home_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from(path));
    }

    if let Some(rest) = path.strip_prefix("~/").or_else(|| path.strip_prefix("~\\"))
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

pub(crate) fn canonical_path_if_exists(path: &Path) -> Option<PathBuf> {
    std::fs::canonicalize(path).ok()
}

pub(crate) fn paths_equivalent(left: &Path, right: &Path) -> bool {
    if left == right {
        return true;
    }

    match (
        canonical_path_if_exists(left),
        canonical_path_if_exists(right),
    ) {
        (Some(left), Some(right)) => left == right,
        _ => false,
    }
}

fn display_preserving_current_dir() -> Result<PathBuf, crate::ConfigError> {
    let real_current_dir = std::env::current_dir()?;

    let Some(pwd) = std::env::var_os("PWD")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
    else {
        return Ok(real_current_dir);
    };

    match std::fs::canonicalize(&pwd) {
        Ok(canonical_pwd) if canonical_pwd == real_current_dir => Ok(pwd),
        _ => Ok(real_current_dir),
    }
}

pub(crate) fn display_preserving_absolute(path: &Path) -> Result<PathBuf, crate::ConfigError> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    display_preserving_current_dir().map(|current_dir| current_dir.join(path))
}

pub fn validate_path(check_path: PathBuf) -> Result<PathBuf, crate::ConfigError> {
    if check_path.as_os_str().is_empty() {
        Err(crate::ConfigError::NotFileOrDirectory(check_path))
    } else if check_path.is_absolute() {
        Ok(check_path)
    } else if check_path.is_relative() {
        if check_path.exists() {
            display_preserving_absolute(&check_path)
        } else {
            Err(crate::ConfigError::NotFileOrDirectory(check_path))
        }
    } else {
        Err(crate::ConfigError::NotFileOrDirectory(check_path))
    }
}

/// Transposes an input directory or file path to an openmw.cfg path
/// Maybe could do with some additional validation
pub fn input_config_path(config_path: PathBuf) -> Result<PathBuf, crate::ConfigError> {
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
    use super::{display_preserving_absolute, expand_leading_tilde, paths_equivalent};
    use std::path::PathBuf;

    fn unique_temp_dir(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "openmw_config_util_{name}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn test_expand_tilde_leaves_regular_path_unchanged() {
        let value = "/tmp/example";
        assert_eq!(expand_leading_tilde(value), std::path::PathBuf::from(value));
    }

    #[test]
    fn test_expand_tilde_does_not_expand_named_user_syntax() {
        let value = "~alice/mods";
        assert_eq!(expand_leading_tilde(value), PathBuf::from(value));
    }

    #[test]
    fn test_expand_tilde_home_variants() {
        let home = crate::platform_paths::home_dir().expect("home directory required for test");

        assert_eq!(expand_leading_tilde("~"), home);
        assert_eq!(expand_leading_tilde("~/mods"), home.join("mods"));
        assert_eq!(expand_leading_tilde("~\\mods"), home.join("mods"));
    }

    #[test]
    #[cfg(unix)]
    fn test_paths_equivalent_treats_symlinked_directories_as_same() {
        let root = unique_temp_dir("symlink_equivalence");
        let real_home = root.join("real_home");
        let linked_home = root.join("linked_home");
        let real_config = real_home.join(".config").join("openmw");
        let linked_config = linked_home.join(".config").join("openmw");

        std::fs::create_dir_all(&real_config).unwrap();
        std::os::unix::fs::symlink(&real_home, &linked_home).unwrap();

        assert!(paths_equivalent(&linked_config, &real_config));

        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn test_display_preserving_absolute_leaves_absolute_paths_alone() {
        let path = if cfg!(windows) {
            PathBuf::from(r"C:\Users\Example\openmw.cfg")
        } else {
            PathBuf::from("/home/example/openmw.cfg")
        };

        assert_eq!(display_preserving_absolute(&path).unwrap(), path);
    }
}
