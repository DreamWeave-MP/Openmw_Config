// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Dave Corley (S3kshun8)

//! Parser, resolver, and serializer for `OpenMW` configuration chains.
//!
//! `OpenMW` loads one or more `openmw.cfg` files in a chain: the root config can reference
//! additional configs via `config=` entries, and each file can accumulate or override settings
//! from its parent.  This crate walks that chain, resolves token substitutions
//! (`?userdata?`, `?userconfig?`), normalises paths, and exposes the composed result as
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
#[cfg(any(feature = "lua", feature = "lua-module"))]
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

#[cfg(any(feature = "lua", feature = "lua-module"))]
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

/// Fallible variant of [`default_config_path`].
///
/// # Errors
/// Returns [`ConfigError::PlatformPathUnavailable`] if no platform config directory can be discovered.
pub fn try_default_config_path() -> Result<std::path::PathBuf, ConfigError> {
    #[cfg(target_os = "android")]
    return Ok(std::path::PathBuf::from("/storage/emulated/0/Alpha3/config"));

    #[cfg(not(target_os = "android"))]
    {
        if cfg!(windows) {
            dirs::document_dir()
                .ok_or(ConfigError::PlatformPathUnavailable("config"))
                .map(|p| p.join("My Games").join("openmw"))
        } else {
            dirs::preference_dir()
                .ok_or(ConfigError::PlatformPathUnavailable("config"))
                .map(|p| p.join("openmw"))
        }
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
/// # Errors
/// Returns [`ConfigError::PlatformPathUnavailable`] if no platform userdata directory can be discovered.
pub fn try_default_userdata_path() -> Result<std::path::PathBuf, ConfigError> {
    #[cfg(target_os = "android")]
    return Ok(std::path::PathBuf::from("/storage/emulated/0/Alpha3"));

    #[cfg(not(target_os = "android"))]
    {
        if cfg!(windows) {
            try_default_config_path()
        } else {
            dirs::data_dir()
                .ok_or(ConfigError::PlatformPathUnavailable("userdata"))
                .map(|p| p.join("openmw"))
        }
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
