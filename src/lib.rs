// SPDX-License-Identifier: GPL-3.0-or-later
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
//! // Load using OpenMW-style root config discovery.
//! // OPENMW_CONFIG and OPENMW_CONFIG_DIR override discovery.
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
//! an `openmw.cfg` file) and `OPENMW_CONFIG_DIR` (directory containing `openmw.cfg`) override root
//! config discovery. Without those, [`OpenMWConfiguration::from_env`] tries an `openmw.cfg` adjacent
//! to the running executable, then the platform global `OpenMW` config. User config is loaded only
//! when referenced by the root config, usually through `config="?userconfig?"`.
//!
//! Path helpers intentionally distinguish the user config path (`?userconfig?`,
//! [`try_default_config_path`]), the global config path ([`try_default_global_config_path`]), and
//! the global data-token path (`?global?`, [`try_default_global_path`]). Those are not synonyms.
//!
//! Serialization has two contracts. [`OpenMWConfiguration`]'s [`std::fmt::Display`] implementation
//! and preservation save APIs keep directory settings in their original spelling for round-trips.
//! [`OpenMWConfiguration::to_resolved_string`] and
//! [`OpenMWConfiguration::save_resolved_to_path`] emit flattened relocation-safe output, resolving
//! directory values and omitting chain-control entries such as `config=` and `replace=`.

mod config;
#[cfg(feature = "lua")]
pub mod lua;
mod platform_paths;

pub use config::{
    ConfigChainEntry, ConfigChainStatus, OpenMWConfiguration,
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
const NO_GLOBAL_CONFIG_DIR: &str = "FAILURE: COULD NOT READ GLOBAL CONFIG DIRECTORY";
const DEFAULT_FLATPAK_APP_ID: &str = "org.openmw.OpenMW";

#[cfg(target_os = "linux")]
fn has_flatpak_info_file() -> bool {
    use std::sync::OnceLock;

    static HAS_FLATPAK_INFO: OnceLock<bool> = OnceLock::new();
    *HAS_FLATPAK_INFO.get_or_init(|| std::path::Path::new("/.flatpak-info").exists())
}

fn flatpak_mode_enabled() -> bool {
    #[cfg(not(target_os = "linux"))]
    {
        return false;
    }

    #[cfg(target_os = "linux")]
    {
        if std::env::var_os("OPENMW_CONFIG_USING_FLATPAK").is_some() {
            return true;
        }

        std::env::var_os("FLATPAK_ID").is_some() || has_flatpak_info_file()
    }
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
    platform_paths::home_dir().map(|home| {
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
/// On Linux, Flatpak mode is enabled when `OPENMW_CONFIG_USING_FLATPAK` is set to any value, or
/// auto-detected via `FLATPAK_ID` / `/.flatpak-info`.
///
/// # Errors
/// Returns [`ConfigError::PlatformPathUnavailable`] if no platform config directory can be discovered.
pub fn try_default_config_path() -> Result<std::path::PathBuf, ConfigError> {
    #[cfg(target_os = "android")]
    return Ok(std::path::PathBuf::from(
        "/storage/emulated/0/Alpha3/config",
    ));

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
/// On Linux, Flatpak mode is enabled when `OPENMW_CONFIG_USING_FLATPAK` is set to any value, or
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

/// Find the default root `openmw.cfg` using `OpenMW`'s root config discovery order.
///
/// This is not the same as [`try_default_config_path`]. The latter resolves the `?userconfig?`
/// token. Root discovery starts from `OpenMW`'s baseline config: first an executable-adjacent
/// `openmw.cfg`, then the platform global config. The user config is loaded only if that root
/// config references it, normally via `config="?userconfig?"`.
///
/// # Errors
/// Returns [`ConfigError`] if platform paths cannot be resolved or no root `openmw.cfg` exists.
pub fn try_default_root_config_path() -> Result<std::path::PathBuf, ConfigError> {
    let local_dir = try_default_local_path()?;
    let local = local_dir.join("openmw.cfg");
    if local.is_file() {
        return Ok(local);
    }

    let global_config_dir = try_default_global_config_path()?;
    let global = global_config_dir.join("openmw.cfg");
    if global.is_file() {
        return Ok(global);
    }

    Err(ConfigError::CannotFindRootConfig { local, global })
}

/// Path to the default root `openmw.cfg` discovered with `OpenMW` startup semantics.
///
/// # Panics
/// Panics if no root config can be found.
#[must_use]
pub fn default_root_config_path() -> std::path::PathBuf {
    try_default_root_config_path().expect("FAILURE: COULD NOT FIND ROOT CONFIG")
}

#[cfg(test)]
pub(crate) fn discover_root_config_path(
    local_dir: &std::path::Path,
    global_config_dir: &std::path::Path,
) -> Result<std::path::PathBuf, ConfigError> {
    let local = local_dir.join("openmw.cfg");
    if local.is_file() {
        return Ok(local);
    }

    let global = global_config_dir.join("openmw.cfg");
    if global.is_file() {
        return Ok(global);
    }

    Err(ConfigError::CannotFindRootConfig { local, global })
}

/// Fallible variant of [`default_global_config_path`].
///
/// Resolves `OpenMW`'s global **config** directory, not the `?global?` data-token target. On normal
/// Linux package installs this is `/etc/openmw`; in Flatpak mode it is `/app/etc/openmw`.
///
/// `OPENMW_GLOBAL_CONFIG_PATH`, when set to a non-empty value, overrides the detected directory.
/// This mirrors the explicit override style used by the other path helpers and keeps packager tests
/// from needing to write to `/etc`, which is a generally poor hobby.
///
/// # Errors
/// Returns [`ConfigError::PlatformPathUnavailable`] on platforms where `OpenMW` has no global config
/// directory concept.
pub fn try_default_global_config_path() -> Result<std::path::PathBuf, ConfigError> {
    if let Ok(value) = std::env::var("OPENMW_GLOBAL_CONFIG_PATH")
        && !value.trim().is_empty()
    {
        return Ok(std::path::PathBuf::from(value));
    }

    if cfg!(windows) || cfg!(target_os = "macos") {
        return Err(ConfigError::PlatformPathUnavailable("global_config"));
    }

    if flatpak_mode_enabled() {
        return Ok(std::path::PathBuf::from("/app/etc/openmw"));
    }

    Ok(std::path::PathBuf::from("/etc/openmw"))
}

/// Path to `OpenMW`'s global **config** directory.
///
/// This is distinct from [`default_global_path`], which backs the `?global?` data token.
///
/// # Panics
/// Panics if the global config directory cannot be determined.
#[must_use]
pub fn default_global_config_path() -> std::path::PathBuf {
    try_default_global_config_path().expect(NO_GLOBAL_CONFIG_DIR)
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

    // NOTE: Flatpak path behavior is intentionally Linux-only.
    // We are not fully certain whether OpenMW Flatpak builds should prefer a global
    // or local config path in all packaging variants, so we keep this conservative:
    // only Linux Flatpak mode maps ?global? to /app/share/games.
    if flatpak_mode_enabled() {
        return Ok(std::path::PathBuf::from("/app/share/games"));
    }

    if cfg!(target_os = "macos") {
        return Ok(std::path::PathBuf::from("/Library/Application Support"));
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
pub(crate) fn test_env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn snapshot_env(keys: &[&str]) -> Vec<(String, Option<OsString>)> {
        keys.iter()
            .map(|key| ((*key).to_string(), std::env::var_os(key)))
            .collect()
    }

    fn restore_env(snapshot: Vec<(String, Option<OsString>)>) {
        for (key, value) in snapshot {
            // SAFETY: guarded by a process-wide mutex in tests to prevent concurrent env mutation.
            unsafe {
                if let Some(value) = value {
                    std::env::set_var(&key, value);
                } else {
                    std::env::remove_var(&key);
                }
            }
        }
    }

    fn unique_temp_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "openmw_config_{name}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn test_default_data_local_path_is_userdata_data_child() {
        let _guard = crate::test_env_lock();
        let snapshot = snapshot_env(&[
            "OPENMW_CONFIG_USING_FLATPAK",
            "OPENMW_FLATPAK_ID",
            "FLATPAK_ID",
            "OPENMW_GLOBAL_PATH",
        ]);

        // SAFETY: guarded by a process-wide mutex in tests to prevent concurrent env mutation.
        unsafe {
            std::env::remove_var("OPENMW_CONFIG_USING_FLATPAK");
            std::env::remove_var("OPENMW_FLATPAK_ID");
            std::env::remove_var("FLATPAK_ID");
            std::env::remove_var("OPENMW_GLOBAL_PATH");
        }

        assert_eq!(
            default_data_local_path(),
            default_userdata_path().join("data")
        );

        restore_env(snapshot);
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
        let _guard = crate::test_env_lock();
        let snapshot = snapshot_env(&[
            "OPENMW_CONFIG_USING_FLATPAK",
            "OPENMW_FLATPAK_ID",
            "FLATPAK_ID",
        ]);
        let _ = try_default_config_path();
        restore_env(snapshot);
    }

    #[test]
    fn test_try_default_local_path_returns_path_or_error() {
        let _guard = crate::test_env_lock();
        let snapshot = snapshot_env(&[
            "OPENMW_CONFIG_USING_FLATPAK",
            "OPENMW_FLATPAK_ID",
            "FLATPAK_ID",
        ]);
        let _ = try_default_local_path();
        restore_env(snapshot);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_flatpak_env_flag_forces_flatpak_paths() {
        let _guard = crate::test_env_lock();
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
    #[cfg(target_os = "linux")]
    fn test_flatpak_app_id_override_precedence() {
        let _guard = crate::test_env_lock();
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
    #[cfg(target_os = "linux")]
    fn test_flatpak_auto_detect_via_flatpak_id() {
        let _guard = crate::test_env_lock();
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
        let _guard = crate::test_env_lock();
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
    fn test_root_config_discovery_uses_local_before_global_config() {
        let local_dir = unique_temp_dir("root_local");
        let global_dir = unique_temp_dir("root_global");
        std::fs::create_dir_all(&local_dir).unwrap();
        std::fs::create_dir_all(&global_dir).unwrap();
        let local_cfg = local_dir.join("openmw.cfg");
        let global_cfg = global_dir.join("openmw.cfg");
        std::fs::write(&local_cfg, "content=Local.esm\n").unwrap();
        std::fs::write(&global_cfg, "content=Global.esm\n").unwrap();

        assert_eq!(
            discover_root_config_path(&local_dir, &global_dir).unwrap(),
            local_cfg
        );

        let _ = std::fs::remove_file(global_cfg);
        let _ = std::fs::remove_file(local_cfg);
        let _ = std::fs::remove_dir_all(global_dir);
        let _ = std::fs::remove_dir_all(local_dir);
    }

    #[test]
    fn test_root_config_discovery_uses_global_config_when_local_missing() {
        let local_dir = unique_temp_dir("root_local_missing");
        let global_dir = unique_temp_dir("root_global_present");
        std::fs::create_dir_all(&local_dir).unwrap();
        std::fs::create_dir_all(&global_dir).unwrap();
        let global_cfg = global_dir.join("openmw.cfg");
        std::fs::write(&global_cfg, "content=Global.esm\n").unwrap();

        assert_eq!(
            discover_root_config_path(&local_dir, &global_dir).unwrap(),
            global_cfg
        );

        let _ = std::fs::remove_file(global_dir.join("openmw.cfg"));
        let _ = std::fs::remove_dir_all(global_dir);
        let _ = std::fs::remove_dir_all(local_dir);
    }

    #[test]
    fn test_root_config_discovery_errors_without_user_fallback() {
        let local_dir = unique_temp_dir("root_none_local");
        let global_dir = unique_temp_dir("root_none_global");
        std::fs::create_dir_all(&local_dir).unwrap();
        std::fs::create_dir_all(&global_dir).unwrap();

        assert!(matches!(
            discover_root_config_path(&local_dir, &global_dir),
            Err(ConfigError::CannotFindRootConfig { .. })
        ));

        let _ = std::fs::remove_dir_all(global_dir);
        let _ = std::fs::remove_dir_all(local_dir);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_global_config_path_is_not_global_data_token_path() {
        let _guard = crate::test_env_lock();

        // SAFETY: guarded by a process-wide mutex in tests to prevent concurrent env mutation.
        unsafe {
            std::env::remove_var("OPENMW_GLOBAL_PATH");
            std::env::remove_var("OPENMW_GLOBAL_CONFIG_PATH");
            std::env::remove_var("OPENMW_CONFIG_USING_FLATPAK");
            std::env::remove_var("FLATPAK_ID");
        }

        assert_eq!(
            try_default_global_path().unwrap(),
            std::path::PathBuf::from("/usr/share/games")
        );
        assert_eq!(
            try_default_global_config_path().unwrap(),
            std::path::PathBuf::from("/etc/openmw")
        );
    }

    #[test]
    fn test_global_config_path_env_override_has_precedence() {
        let _guard = crate::test_env_lock();
        let expected = std::path::PathBuf::from("/opt/openmw/etc/openmw");

        // SAFETY: guarded by a process-wide mutex in tests to prevent concurrent env mutation.
        unsafe {
            std::env::set_var("OPENMW_GLOBAL_CONFIG_PATH", expected.as_os_str());
        }

        assert_eq!(
            try_default_global_config_path().expect("global config override should be used"),
            expected
        );

        // SAFETY: guarded by a process-wide mutex in tests to prevent concurrent env mutation.
        unsafe {
            std::env::remove_var("OPENMW_GLOBAL_CONFIG_PATH");
        }
    }

    #[test]
    #[cfg(not(windows))]
    fn test_global_path_default_is_platform_or_flatpak_value() {
        let _guard = crate::test_env_lock();

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
        let _guard = crate::test_env_lock();

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

    #[test]
    #[cfg(not(target_os = "linux"))]
    fn test_flatpak_mode_is_ignored_off_linux() {
        let _guard = crate::test_env_lock();

        // SAFETY: guarded by a process-wide mutex in tests to prevent concurrent env mutation.
        unsafe {
            std::env::set_var("OPENMW_CONFIG_USING_FLATPAK", "1");
            std::env::set_var("FLATPAK_ID", "org.example.Flatpak");
            std::env::remove_var("OPENMW_GLOBAL_PATH");
        }

        assert!(!flatpak_mode_enabled());

        assert_eq!(
            try_default_config_path().ok(),
            platform_paths::config_dir().ok()
        );
        assert_eq!(
            try_default_userdata_path().ok(),
            platform_paths::data_dir().ok()
        );

        if cfg!(windows) {
            assert!(matches!(
                try_default_global_path(),
                Err(ConfigError::PlatformPathUnavailable("global"))
            ));
        } else if cfg!(target_os = "macos") {
            assert_eq!(
                try_default_global_path().expect("macOS global path should resolve"),
                std::path::PathBuf::from("/Library/Application Support")
            );
        }

        // SAFETY: guarded by a process-wide mutex in tests to prevent concurrent env mutation.
        unsafe {
            std::env::remove_var("OPENMW_CONFIG_USING_FLATPAK");
            std::env::remove_var("FLATPAK_ID");
            std::env::remove_var("OPENMW_GLOBAL_PATH");
        }
    }
}
