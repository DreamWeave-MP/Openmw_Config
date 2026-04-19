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

#[test]
#[cfg(not(windows))]
fn test_from_env_expands_tilde_in_openmw_config() {
    let _guard = env_lock();
    let home = std::env::var("HOME").expect("HOME must be set on non-windows");
    let test_dir = std::path::Path::new(&home).join(".openmw_cfg_tilde_env_file");
    std::fs::create_dir_all(&test_dir).unwrap();
    let cfg_path = write_cfg(&test_dir, "content=Tildefile.esm\n");

    let relative_cfg = cfg_path
        .strip_prefix(&home)
        .expect("test cfg should live under HOME")
        .to_string_lossy()
        .trim_start_matches(std::path::MAIN_SEPARATOR)
        .to_string();
    let openmw_config = format!("~/{relative_cfg}");

    unsafe {
        std::env::set_var("OPENMW_CONFIG", openmw_config);
        std::env::remove_var("OPENMW_CONFIG_DIR");
    }

    let config = OpenMWConfiguration::from_env().unwrap();

    unsafe {
        std::env::remove_var("OPENMW_CONFIG");
    }

    std::fs::remove_dir_all(&test_dir).unwrap();
    assert!(config.has_content_file("Tildefile.esm"));
}

#[test]
#[cfg(not(windows))]
fn test_from_env_expands_tilde_in_openmw_config_dir_list() {
    let _guard = env_lock();
    let home = std::env::var("HOME").expect("HOME must be set on non-windows");
    let test_dir = std::path::Path::new(&home).join(".openmw_cfg_tilde_env_dir");
    std::fs::create_dir_all(&test_dir).unwrap();
    write_cfg(&test_dir, "content=Tildedir.esm\n");

    let missing = temp_dir("tilde_missing");
    let sep = ':';
    let relative_dir = test_dir
        .strip_prefix(&home)
        .expect("test dir should live under HOME")
        .to_string_lossy()
        .trim_start_matches(std::path::MAIN_SEPARATOR)
        .to_string();
    let tilde_dir = format!("~/{relative_dir}");
    let var_value = format!("{}{}{}", missing.display(), sep, tilde_dir);

    unsafe {
        std::env::set_var("OPENMW_CONFIG_DIR", var_value);
        std::env::remove_var("OPENMW_CONFIG");
    }

    let config = OpenMWConfiguration::from_env().unwrap();

    unsafe {
        std::env::remove_var("OPENMW_CONFIG_DIR");
    }

    std::fs::remove_dir_all(&test_dir).unwrap();
    assert!(config.has_content_file("Tildedir.esm"));
}
