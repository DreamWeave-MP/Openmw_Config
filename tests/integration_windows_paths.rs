#![cfg(windows)]

mod common;

use common::{temp_dir, write_cfg};
use openmw_config::OpenMWConfiguration;
use std::path::PathBuf;

fn known_documents_dir() -> PathBuf {
    use std::os::windows::ffi::OsStringExt;
    use std::ptr::null_mut;
    use windows_sys::Win32::System::Com::CoTaskMemFree;
    use windows_sys::Win32::UI::Shell::{FOLDERID_Documents, SHGetKnownFolderPath};

    let mut raw_path: windows_sys::core::PWSTR = null_mut();
    let status = unsafe { SHGetKnownFolderPath(&FOLDERID_Documents, 0, null_mut(), &mut raw_path) };
    assert_eq!(status, 0, "SHGetKnownFolderPath(FOLDERID_Documents) failed");
    assert!(!raw_path.is_null(), "Known folder API returned null path");

    let mut len = 0_usize;
    unsafe {
        while *raw_path.add(len) != 0 {
            len += 1;
        }
    }

    let slice = unsafe { std::slice::from_raw_parts(raw_path, len) };
    let os = std::ffi::OsString::from_wide(slice);
    unsafe {
        CoTaskMemFree(raw_path.cast());
    }

    PathBuf::from(os)
}

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

#[test]
fn test_windows_default_paths_match_known_documents_folder() {
    let documents = known_documents_dir();
    let expected = documents.join("My Games").join("openmw");

    assert_eq!(openmw_config::try_default_config_path().unwrap(), expected);
    assert_eq!(
        openmw_config::try_default_userdata_path().unwrap(),
        expected
    );
}

#[test]
fn test_windows_save_to_path_overwrites_read_only_file() {
    let dir = temp_dir("win_save_readonly");
    write_cfg(&dir, "no-sound=1\n");
    let config = OpenMWConfiguration::new(Some(dir.clone())).unwrap();

    let out = dir.join("export.cfg");
    std::fs::write(&out, "old=content\n").unwrap();

    let mut permissions = std::fs::metadata(&out).unwrap().permissions();
    permissions.set_readonly(true);
    std::fs::set_permissions(&out, permissions).unwrap();

    config.save_to_path(&out).unwrap();

    let saved = std::fs::read_to_string(&out).unwrap();
    assert!(saved.contains("no-sound=1"));
    assert!(!saved.contains("old=content"));
}
