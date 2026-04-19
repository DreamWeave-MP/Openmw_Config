mod common;

use common::{temp_dir, write_cfg};
use openmw_config::OpenMWConfiguration;

#[test]
fn test_save_user_only_persists_user_owned_settings() {
    let root_dir = temp_dir("save_user_root");
    let user_dir = temp_dir("save_user_user");

    write_cfg(&user_dir, "content=UserBase.esm\n");
    write_cfg(
        &root_dir,
        &format!("content=RootOnly.esm\nconfig={}\n", user_dir.display()),
    );

    let mut config = OpenMWConfiguration::new(Some(root_dir.clone())).unwrap();
    config.add_content_file("UserAdded.esp").unwrap();
    config.save_user().unwrap();

    let user_saved = std::fs::read_to_string(user_dir.join("openmw.cfg")).unwrap();
    assert!(user_saved.contains("content=UserBase.esm"));
    assert!(user_saved.contains("content=UserAdded.esp"));
    assert!(!user_saved.contains("content=RootOnly.esm"));

    let root_saved = std::fs::read_to_string(root_dir.join("openmw.cfg")).unwrap();
    assert!(root_saved.contains("content=RootOnly.esm"));
    assert!(!root_saved.contains("content=UserAdded.esp"));
}

#[test]
fn test_save_subconfig_does_not_persist_settings_from_other_sources() {
    let root_dir = temp_dir("save_sub_root");
    let user_dir = temp_dir("save_sub_user");

    write_cfg(&user_dir, "content=UserBase.esm\n");
    write_cfg(
        &root_dir,
        &format!("content=RootOnly.esm\nconfig={}\n", user_dir.display()),
    );

    let mut config = OpenMWConfiguration::new(Some(root_dir.clone())).unwrap();
    config.add_content_file("UserAdded.esp").unwrap();
    config.save_subconfig(&user_dir).unwrap();

    let user_saved = std::fs::read_to_string(user_dir.join("openmw.cfg")).unwrap();
    assert!(user_saved.contains("content=UserBase.esm"));
    assert!(user_saved.contains("content=UserAdded.esp"));
    assert!(!user_saved.contains("content=RootOnly.esm"));
}
