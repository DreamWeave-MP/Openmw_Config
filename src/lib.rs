// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Dave Corley (S3kshun8)

//! Parser, resolver, and serializer for `OpenMW` configuration chains.
//!
//! `OpenMW` loads one or more `openmw.cfg` files in a chain: the root config can reference
//! additional configs via `config=` entries, and each file can accumulate or override settings
//! from its parent.  This crate walks that chain, resolves token substitutions
//! (`?local?`, `?global?`, `?userdata?`, `?userconfig?`), normalises paths, and exposes the composed result as
//! [`OpenMWConfiguration`].
//!
//! # Quick start
//!
//! ```no_run
//! use openmw_config::OpenMWConfiguration;
//!
//! // Load from the platform-default location (or OPENMW_CONFIG / OPENMW_CONFIG_DIR env vars)
//! let config = OpenMWConfiguration::from_env()?;
//!
//! // Iterate content files in load order
//! for plugin in config.content_files_iter() {
//!     println!("{}", plugin.value());
//! }
//! # Ok::<(), openmw_config::ConfigError>(())
//! ```
//!
//! # Configuration sources
//!
//! See the [OpenMW path documentation](https://openmw.readthedocs.io/en/latest/reference/modding/paths.html)
//! for platform-specific default locations.  The environment variables `OPENMW_CONFIG` (path to
//! an `openmw.cfg` file) and `OPENMW_CONFIG_DIR` (directory containing `openmw.cfg`) override the
//! platform default.

mod config;
mod platform_paths;
#[cfg(feature = "lua")]
pub mod lua;

pub use config::{
    ConfigChainEntry,
    ConfigChainStatus,
    OpenMWConfiguration,
    directorysetting::DirectorySetting,
    encodingsetting::{EncodingSetting, EncodingType},
    error::ConfigError,
    filesetting::FileSetting,
    gamesetting::GameSettingType,
    genericsetting::GenericSetting,
};

#[cfg(feature = "lua")]
pub use lua::create_lua_module;

pub(crate) trait GameSetting: std::fmt::Display {
    fn meta(&self) -> &GameSettingMeta;
}

/// Source-tracking metadata attached to every setting value.
///
/// Records which config file defined the setting and any comment lines that
/// immediately preceded it in the file, so that [`OpenMWConfiguration`]'s
/// `Display` implementation can round-trip comments faithfully.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GameSettingMeta {
    source_config: std::path::PathBuf,
    comment: String,
}

impl GameSettingMeta {
    #[must_use]
    pub fn source_config(&self) -> &std::path::Path {
        &self.source_config
    }

    #[must_use]
    pub fn comment(&self) -> &str {
        &self.comment
    }
}

const NO_CONFIG_DIR: &str = "FAILURE: COULD NOT READ CONFIG DIRECTORY";
const NO_LOCAL_DIR: &str = "FAILURE: COULD NOT READ LOCAL DIRECTORY";
const NO_GLOBAL_DIR: &str = "FAILURE: COULD NOT READ GLOBAL DIRECTORY";
const DEFAULT_FLATPAK_APP_ID: &str = "org.openmw.OpenMW";

fn has_flatpak_info_file() -> bool {
    use std::sync::OnceLock;

    static HAS_FLATPAK_INFO: OnceLock<bool> = OnceLock::new();
    *HAS_FLATPAK_INFO.get_or_init(|| std::path::Path::new("/.flatpak-info").exists())
}

fn flatpak_mode_enabled() -> bool {
    if std::env::var_os("OPENMW_CONFIG_USING_FLATPAK").is_some() {
        return true;
    }

    std::env::var_os("FLATPAK_ID").is_some() || has_flatpak_info_file()
}

fn flatpak_app_id() -> String {
    std::env::var("OPENMW_FLATPAK_ID")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            std::env::var("FLATPAK_ID")
                .ok()
                .filter(|value| !value.trim().is_empty())
        })
        .unwrap_or_else(|| DEFAULT_FLATPAK_APP_ID.to_string())
}

fn flatpak_userconfig_path() -> Result<std::path::PathBuf, ConfigError> {
    platform_paths::home_dir()
        .map(|home| {
            home.join(".var")
                .join("app")
                .join(flatpak_app_id())
                .join("config")
                .join("openmw")
        })
}

fn flatpak_userdata_path() -> Result<std::path::PathBuf, ConfigError> {
    platform_paths::home_dir().map(|home| {
        home.join(".var")
            .join("app")
            .join(flatpak_app_id())
            .join("data")
            .join("openmw")
    })
}

/// Fallible variant of [`default_config_path`].
///
/// Resolution precedence:
/// 1. Flatpak mode path (`$HOME/.var/app/<app-id>/config/openmw`) when Flatpak mode is enabled.
/// 2. Platform default path from platform-specific resolvers.
///
/// Flatpak mode is enabled when `OPENMW_CONFIG_USING_FLATPAK` is set to any value, or
/// auto-detected via `FLATPAK_ID` / `/.flatpak-info`.
///
/// # Errors
/// Returns [`ConfigError::PlatformPathUnavailable`] if no platform config directory can be discovered.
pub fn try_default_config_path() -> Result<std::path::PathBuf, ConfigError> {
    #[cfg(target_os = "android")]
    return Ok(std::path::PathBuf::from("/storage/emulated/0/Alpha3/config"));

    #[cfg(not(target_os = "android"))]
    {
        if flatpak_mode_enabled() {
            return flatpak_userconfig_path();
        }

        platform_paths::config_dir().map_err(|_| ConfigError::PlatformPathUnavailable("config"))
    }
}

/// Path to input bindings and core configuration
/// These functions are not expected to fail and should they fail, indicate either:
/// a severe issue with the system
/// or that an unsupported system is being used.
///
/// # Panics
/// Panics if the platform config directory cannot be determined (unsupported system).
#[must_use]
pub fn default_config_path() -> std::path::PathBuf {
    try_default_config_path().expect(NO_CONFIG_DIR)
}

/// Fallible variant of [`default_userdata_path`].
///
/// Resolution precedence:
/// 1. Flatpak mode path (`$HOME/.var/app/<app-id>/data/openmw`) when Flatpak mode is enabled.
/// 2. Platform default path from platform-specific resolvers.
///
/// Flatpak mode is enabled when `OPENMW_CONFIG_USING_FLATPAK` is set to any value, or
/// auto-detected via `FLATPAK_ID` / `/.flatpak-info`.
///
/// # Errors
/// Returns [`ConfigError::PlatformPathUnavailable`] if no platform userdata directory can be discovered.
pub fn try_default_userdata_path() -> Result<std::path::PathBuf, ConfigError> {
    #[cfg(target_os = "android")]
    return Ok(std::path::PathBuf::from("/storage/emulated/0/Alpha3"));

    #[cfg(not(target_os = "android"))]
    {
        if flatpak_mode_enabled() {
            return flatpak_userdata_path();
        }

        platform_paths::data_dir().map_err(|_| ConfigError::PlatformPathUnavailable("userdata"))
    }
}

/// Path to save storage, screenshots, navmeshdb, and data-local
/// These functions are not expected to fail and should they fail, indicate either:
/// a severe issue with the system
/// or that an unsupported system is being used.
///
/// # Panics
/// Panics if the platform data directory cannot be determined (unsupported system).
#[must_use]
pub fn default_userdata_path() -> std::path::PathBuf {
    try_default_userdata_path().expect("FAILURE: COULD NOT READ USERDATA DIRECTORY")
}

/// Path to the `data-local` directory as defined by the engine's defaults.
///
/// This directory is loaded last and therefore overrides all other data sources
/// in the VFS load order.
#[must_use]
pub fn default_data_local_path() -> std::path::PathBuf {
    default_userdata_path().join("data")
}

/// Fallible variant of [`default_local_path`].
///
/// Resolves the `?local?` token target.
///
/// - On macOS app bundles, this is the `Contents/Resources` directory.
/// - On other platforms, this is the directory containing the running executable.
///
/// # Errors
/// Returns [`ConfigError::PlatformPathUnavailable`] if the local path cannot be determined.
pub fn try_default_local_path() -> Result<std::path::PathBuf, ConfigError> {
    let exe = std::env::current_exe()?;

    #[cfg(target_os = "macos")]
    {
        if let Some(macos_dir) = exe.parent()
            && macos_dir.file_name() == Some(std::ffi::OsStr::new("MacOS"))
            && let Some(contents_dir) = macos_dir.parent()
            && contents_dir.file_name() == Some(std::ffi::OsStr::new("Contents"))
        {
            return Ok(contents_dir.join("Resources"));
        }
    }

    exe.parent()
        .map(std::path::Path::to_path_buf)
        .ok_or(ConfigError::PlatformPathUnavailable("local"))
}

/// Path that backs the `?local?` token.
///
/// # Panics
/// Panics if the local path cannot be determined.
#[must_use]
pub fn default_local_path() -> std::path::PathBuf {
    try_default_local_path().expect(NO_LOCAL_DIR)
}

/// Fallible variant of [`default_global_path`].
///
/// Resolves the `?global?` token target.
///
/// Resolution order:
/// 1. `OPENMW_GLOBAL_PATH` when set.
/// 2. Flatpak default (`/app/share/games`) when Flatpak mode is active.
/// 3. Platform default (`/usr/share/games` on Unix-like systems, `/Library/Application Support` on macOS).
///
/// Flatpak app id selection is: `OPENMW_FLATPAK_ID` > `FLATPAK_ID` > `org.openmw.OpenMW`.
///
/// # Errors
/// Returns [`ConfigError::PlatformPathUnavailable`] on unsupported platforms.
pub fn try_default_global_path() -> Result<std::path::PathBuf, ConfigError> {
    if let Ok(value) = std::env::var("OPENMW_GLOBAL_PATH")
        && !value.trim().is_empty()
    {
        return Ok(std::path::PathBuf::from(value));
    }

    if cfg!(windows) {
        return Err(ConfigError::PlatformPathUnavailable("global"));
    }

    if cfg!(target_os = "macos") {
        return Ok(std::path::PathBuf::from("/Library/Application Support"));
    }

    if flatpak_mode_enabled() {
        return Ok(std::path::PathBuf::from("/app/share/games"));
    }

    Ok(std::path::PathBuf::from("/usr/share/games"))
}

/// Path that backs the `?global?` token.
///
/// # Panics
/// Panics if the global path cannot be determined.
#[must_use]
pub fn default_global_path() -> std::path::PathBuf {
    try_default_global_path().expect(NO_GLOBAL_DIR)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_default_data_local_path_is_userdata_data_child() {
        assert_eq!(
            default_data_local_path(),
            default_userdata_path().join("data")
        );
    }

    #[test]
    #[cfg(windows)]
    fn test_windows_default_paths_contract() {
        let cfg = default_config_path();
        let cfg_str = cfg.to_string_lossy().to_lowercase();
        assert!(cfg_str.contains("my games"));
        assert!(cfg_str.contains("openmw"));
        assert_eq!(default_userdata_path(), cfg);
    }

    #[test]
    fn test_try_default_config_path_returns_path_or_error() {
        let _ = try_default_config_path();
    }

    #[test]
    fn test_try_default_local_path_returns_path_or_error() {
        let _ = try_default_local_path();
    }

    #[test]
    fn test_flatpak_env_flag_forces_flatpak_paths() {
        let _guard = ENV_LOCK.lock().expect("env lock poisoned");
        let Ok(home) = platform_paths::home_dir() else {
            return;
        };

        // SAFETY: guarded by a process-wide mutex in tests to prevent concurrent env mutation.
        unsafe {
            std::env::set_var("OPENMW_CONFIG_USING_FLATPAK", "bananas");
            std::env::remove_var("OPENMW_FLATPAK_ID");
            std::env::remove_var("FLATPAK_ID");
        }

        let cfg = try_default_config_path().expect("flatpak config path should resolve");
        let data = try_default_userdata_path().expect("flatpak userdata path should resolve");

        assert_eq!(
            cfg,
            home.join(".var")
                .join("app")
                .join(DEFAULT_FLATPAK_APP_ID)
                .join("config")
                .join("openmw")
        );
        assert_eq!(
            data,
            home.join(".var")
                .join("app")
                .join(DEFAULT_FLATPAK_APP_ID)
                .join("data")
                .join("openmw")
        );

        // SAFETY: guarded by a process-wide mutex in tests to prevent concurrent env mutation.
        unsafe {
            std::env::remove_var("OPENMW_CONFIG_USING_FLATPAK");
        }
    }

    #[test]
    fn test_flatpak_app_id_override_precedence() {
        let _guard = ENV_LOCK.lock().expect("env lock poisoned");
        let Ok(home) = platform_paths::home_dir() else {
            return;
        };

        // SAFETY: guarded by a process-wide mutex in tests to prevent concurrent env mutation.
        unsafe {
            std::env::set_var("OPENMW_CONFIG_USING_FLATPAK", "enabled");
            std::env::set_var("OPENMW_FLATPAK_ID", "org.example.Override");
            std::env::set_var("FLATPAK_ID", "org.example.ShouldNotWin");
        }

        let cfg = try_default_config_path().expect("flatpak config path should resolve");
        assert_eq!(
            cfg,
            home.join(".var")
                .join("app")
                .join("org.example.Override")
                .join("config")
                .join("openmw")
        );

        // SAFETY: guarded by a process-wide mutex in tests to prevent concurrent env mutation.
        unsafe {
            std::env::remove_var("OPENMW_CONFIG_USING_FLATPAK");
            std::env::remove_var("OPENMW_FLATPAK_ID");
            std::env::remove_var("FLATPAK_ID");
        }
    }

    #[test]
    fn test_flatpak_auto_detect_via_flatpak_id() {
        let _guard = ENV_LOCK.lock().expect("env lock poisoned");
        let Ok(home) = platform_paths::home_dir() else {
            return;
        };

        // SAFETY: guarded by a process-wide mutex in tests to prevent concurrent env mutation.
        unsafe {
            std::env::remove_var("OPENMW_CONFIG_USING_FLATPAK");
            std::env::remove_var("OPENMW_FLATPAK_ID");
            std::env::set_var("FLATPAK_ID", "org.example.AutoDetect");
        }

        let data = try_default_userdata_path().expect("flatpak userdata path should resolve");
        assert_eq!(
            data,
            home.join(".var")
                .join("app")
                .join("org.example.AutoDetect")
                .join("data")
                .join("openmw")
        );

        // SAFETY: guarded by a process-wide mutex in tests to prevent concurrent env mutation.
        unsafe {
            std::env::remove_var("FLATPAK_ID");
        }
    }

    #[test]
    fn test_global_path_env_override_has_precedence() {
        let _guard = ENV_LOCK.lock().expect("env lock poisoned");
        let expected = std::path::PathBuf::from("/opt/openmw/global");

        // SAFETY: guarded by a process-wide mutex in tests to prevent concurrent env mutation.
        unsafe {
            std::env::set_var("OPENMW_GLOBAL_PATH", expected.as_os_str());
        }

        assert_eq!(
            try_default_global_path().expect("global override should be used"),
            expected
        );

        // SAFETY: guarded by a process-wide mutex in tests to prevent concurrent env mutation.
        unsafe {
            std::env::remove_var("OPENMW_GLOBAL_PATH");
        }
    }

    #[test]
    #[cfg(not(windows))]
    fn test_global_path_default_is_platform_or_flatpak_value() {
        let _guard = ENV_LOCK.lock().expect("env lock poisoned");

        // SAFETY: guarded by a process-wide mutex in tests to prevent concurrent env mutation.
        unsafe {
            std::env::remove_var("OPENMW_GLOBAL_PATH");
            std::env::remove_var("OPENMW_CONFIG_USING_FLATPAK");
            std::env::remove_var("FLATPAK_ID");
        }

        if cfg!(target_os = "macos") {
            assert_eq!(
                try_default_global_path().expect("macOS global path should resolve"),
                std::path::PathBuf::from("/Library/Application Support")
            );
        } else if flatpak_mode_enabled() {
            assert_eq!(
                try_default_global_path().expect("flatpak global path should resolve"),
                std::path::PathBuf::from("/app/share/games")
            );
        } else {
            assert_eq!(
                try_default_global_path().expect("unix global path should resolve"),
                std::path::PathBuf::from("/usr/share/games")
            );
        }
    }

    #[test]
    #[cfg(windows)]
    fn test_global_path_is_unavailable_on_windows_without_override() {
        let _guard = ENV_LOCK.lock().expect("env lock poisoned");

        // SAFETY: guarded by a process-wide mutex in tests to prevent concurrent env mutation.
        unsafe {
            std::env::remove_var("OPENMW_GLOBAL_PATH");
            std::env::remove_var("OPENMW_CONFIG_USING_FLATPAK");
            std::env::remove_var("FLATPAK_ID");
        }

        assert!(matches!(
            try_default_global_path(),
            Err(ConfigError::PlatformPathUnavailable("global"))
        ));
    }
}
