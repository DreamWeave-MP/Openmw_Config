#![cfg(windows)]

mod common;

use common::{temp_dir, write_cfg};
use openmw_config::OpenMWConfiguration;

#[test]
fn test_windows_drive_path_is_treated_as_rooted_data_dir() {
    let dir = temp_dir("win_drive");
    write_cfg(&dir, "data=C:\\Games\\OpenMW\\Data Files\n");

    let config = OpenMWConfiguration::new(Some(dir)).unwrap();
    assert!(config.has_data_dir("C:/Games/OpenMW/Data Files"));
}

#[test]
fn test_windows_has_data_dir_matches_mixed_separators() {
    let dir = temp_dir("win_sep");
    write_cfg(&dir, "data=C:/mods/Example Mod\n");

    let config = OpenMWConfiguration::new(Some(dir)).unwrap();
    assert!(config.has_data_dir("C:\\mods\\Example Mod"));
}
