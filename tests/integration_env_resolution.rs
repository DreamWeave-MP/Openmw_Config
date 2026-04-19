mod common;

use common::{temp_dir, write_cfg};
use openmw_config::{ConfigError, OpenMWConfiguration};
use std::sync::{Mutex, OnceLock};

fn env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[test]
fn test_from_env_both_vars_set_openmw_config_wins() {
    let _guard = env_lock();

    let dir_var_dir = temp_dir("env_dir_var");
    write_cfg(&dir_var_dir, "content=Morrowind.esm\n");

    let explicit_cfg_dir = temp_dir("env_cfg_var");
    let explicit_cfg = write_cfg(&explicit_cfg_dir, "content=Tribunal.esm\n");

    unsafe {
        std::env::set_var("OPENMW_CONFIG_DIR", &dir_var_dir);
        std::env::set_var("OPENMW_CONFIG", &explicit_cfg);
    }

    let config = OpenMWConfiguration::from_env().unwrap();

    unsafe {
        std::env::remove_var("OPENMW_CONFIG");
        std::env::remove_var("OPENMW_CONFIG_DIR");
    }

    assert!(config.has_content_file("Tribunal.esm"));
    assert!(!config.has_content_file("Morrowind.esm"));
}

#[test]
fn test_from_env_empty_openmw_config_errors() {
    let _guard = env_lock();

    unsafe {
        std::env::set_var("OPENMW_CONFIG", "");
        std::env::remove_var("OPENMW_CONFIG_DIR");
    }

    let result = OpenMWConfiguration::from_env();

    unsafe {
        std::env::remove_var("OPENMW_CONFIG");
    }

    assert!(matches!(result, Err(ConfigError::NotFileOrDirectory(_))));
}

#[test]
fn test_from_env_openmw_config_dir_path_list_uses_first_existing_openmw_cfg() {
    let _guard = env_lock();

    let missing = temp_dir("env_missing");
    let valid = temp_dir("env_valid");
    write_cfg(&valid, "content=Bloodmoon.esm\n");

    let sep = if cfg!(windows) { ';' } else { ':' };
    let var_value = format!("{}{}{}", missing.display(), sep, valid.display());

    unsafe {
        std::env::set_var("OPENMW_CONFIG_DIR", var_value);
        std::env::remove_var("OPENMW_CONFIG");
    }

    let config = OpenMWConfiguration::from_env().unwrap();

    unsafe {
        std::env::remove_var("OPENMW_CONFIG_DIR");
    }

    assert!(config.has_content_file("Bloodmoon.esm"));
}
