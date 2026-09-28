mod common;

use common::{temp_dir, write_cfg};
use openmw_config::{FileSetting, OpenMWConfiguration};

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

#[test]
fn test_save_user_keeps_replace_entries() {
    let root_dir = temp_dir("save_user_replace_root");
    let user_dir = temp_dir("save_user_replace_user");

    write_cfg(
        &user_dir,
        "# start over\nreplace=content\ncontent=UserOnly.esp\n",
    );
    write_cfg(
        &root_dir,
        &format!("content=RootOnly.esm\nconfig={}\n", user_dir.display()),
    );

    let config = OpenMWConfiguration::new(Some(root_dir.clone())).unwrap();
    assert!(
        config
            .to_string()
            .contains("# start over\nreplace=content\ncontent=UserOnly.esp")
    );
    assert!(!config.to_resolved_string().contains("replace="));
    config.save_user().unwrap();

    let user_saved = std::fs::read_to_string(user_dir.join("openmw.cfg")).unwrap();
    assert_eq!(
        user_saved,
        "# start over\nreplace=content\ncontent=UserOnly.esp\n"
    );

    let reloaded = OpenMWConfiguration::new(Some(root_dir)).unwrap();
    let content: Vec<_> = reloaded
        .content_files_iter()
        .map(FileSetting::value_str)
        .collect();
    assert_eq!(content, ["UserOnly.esp"]);
}

#[test]
fn test_injected_resources_vfs_directory_is_never_saved() {
    let dir = temp_dir("save_resources_vfs");
    let resources = temp_dir("save_resources_vfs_engine");
    let vfs = resources.join("vfs");

    write_cfg(
        &dir,
        &format!(
            "# engine files\nresources={}\ndata=Data Files\n",
            resources.display()
        ),
    );

    let config = OpenMWConfiguration::new(Some(dir.clone())).unwrap();
    let first = config.data_directories_iter().next().unwrap();
    assert_eq!(first.parsed(), vfs);

    for serialized in [config.to_string(), config.to_resolved_string()] {
        assert_eq!(serialized.matches("data=").count(), 1, "{serialized}");
        assert_eq!(
            serialized.matches("# engine files").count(),
            1,
            "{serialized}"
        );
    }

    config.save_user().unwrap();
    let reloaded = OpenMWConfiguration::new(Some(dir)).unwrap();
    let copies = reloaded
        .data_directories_iter()
        .filter(|data_dir| data_dir.parsed() == vfs)
        .count();
    assert_eq!(copies, 1);
}
