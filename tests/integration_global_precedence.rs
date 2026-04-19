mod common;

use common::{temp_dir, write_cfg};
use openmw_config::OpenMWConfiguration;
use std::{ffi::OsString, path::{Path, PathBuf}, sync::{Mutex, OnceLock}};

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

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

fn load_with_global_data_dir() -> OpenMWConfiguration {
    let dir = temp_dir("global_precedence");
    write_cfg(&dir, "data=?global?/mods\n");
    OpenMWConfiguration::new(Some(dir)).expect("config with ?global? should load")
}

#[test]
#[cfg(not(windows))]
fn test_global_token_prefers_openmw_global_path_override() {
    let _guard = env_lock();
    let snapshot = snapshot_env(&[
        "OPENMW_GLOBAL_PATH",
        "OPENMW_CONFIG_USING_FLATPAK",
        "OPENMW_FLATPAK_ID",
        "FLATPAK_ID",
    ]);

    let override_path = PathBuf::from("/tmp/openmw_global_override");

    // SAFETY: guarded by a process-wide mutex in tests to prevent concurrent env mutation.
    unsafe {
        std::env::set_var("OPENMW_GLOBAL_PATH", override_path.as_os_str());
        std::env::set_var("OPENMW_CONFIG_USING_FLATPAK", "totally-flatpak");
        std::env::set_var("FLATPAK_ID", "org.example.ShouldNotWin");
    }

    let config = load_with_global_data_dir();
    let expected = override_path.join("mods");
    assert!(config.has_data_dir(expected.to_string_lossy().as_ref()));

    restore_env(snapshot);
}

#[test]
#[cfg(not(windows))]
fn test_global_token_uses_flatpak_default_when_mode_enabled() {
    let _guard = env_lock();
    let snapshot = snapshot_env(&[
        "OPENMW_GLOBAL_PATH",
        "OPENMW_CONFIG_USING_FLATPAK",
        "OPENMW_FLATPAK_ID",
        "FLATPAK_ID",
    ]);

    // SAFETY: guarded by a process-wide mutex in tests to prevent concurrent env mutation.
    unsafe {
        std::env::remove_var("OPENMW_GLOBAL_PATH");
        std::env::set_var("OPENMW_CONFIG_USING_FLATPAK", "yes");
        std::env::set_var("FLATPAK_ID", "org.example.Flatpak");
    }

    let config = load_with_global_data_dir();
    assert!(config.has_data_dir("/app/share/games/mods"));

    restore_env(snapshot);
}

#[test]
#[cfg(not(windows))]
fn test_global_token_falls_back_without_override() {
    let _guard = env_lock();
    let snapshot = snapshot_env(&[
        "OPENMW_GLOBAL_PATH",
        "OPENMW_CONFIG_USING_FLATPAK",
        "OPENMW_FLATPAK_ID",
        "FLATPAK_ID",
    ]);

    // SAFETY: guarded by a process-wide mutex in tests to prevent concurrent env mutation.
    unsafe {
        std::env::remove_var("OPENMW_GLOBAL_PATH");
        std::env::remove_var("OPENMW_CONFIG_USING_FLATPAK");
        std::env::remove_var("OPENMW_FLATPAK_ID");
        std::env::remove_var("FLATPAK_ID");
    }

    let expected_base = if cfg!(target_os = "macos") {
        PathBuf::from("/Library/Application Support")
    } else if Path::new("/.flatpak-info").exists() {
        PathBuf::from("/app/share/games")
    } else {
        PathBuf::from("/usr/share/games")
    };

    let config = load_with_global_data_dir();
    let expected = expected_base.join("mods");
    assert!(config.has_data_dir(expected.to_string_lossy().as_ref()));

    restore_env(snapshot);
}
