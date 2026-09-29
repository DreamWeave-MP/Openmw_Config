//! A `config=` directory without an `openmw.cfg` is still part of the chain, as `OpenMW`'s
//! `ConfigurationManager::readConfiguration` has it: the directory goes into
//! `mActiveConfigPaths` whether or not a file loads from it, and the last one is the user
//! config directory. On first run the engine creates that directory, and the launcher writes
//! the user's `openmw.cfg` into it.

mod common;

use common::{temp_dir, write_cfg};
use openmw_config::{ConfigChainStatus, ConfigError, OpenMWConfiguration};
use std::path::{Path, PathBuf};

fn content(config: &OpenMWConfiguration) -> Vec<String> {
    config
        .content_files_iter()
        .map(|file| file.value().clone())
        .collect()
}

fn sub_configs(config: &OpenMWConfiguration) -> Vec<PathBuf> {
    config
        .sub_configs()
        .map(|dir| dir.parsed().to_path_buf())
        .collect()
}

fn statuses(config: &OpenMWConfiguration) -> Vec<ConfigChainStatus> {
    config
        .config_chain()
        .map(|entry| entry.status().clone())
        .collect()
}

/// A directory that does not exist, as `?userconfig?` does not on a fresh install.
fn missing_dir(tag: &str) -> PathBuf {
    temp_dir(tag).join("not-created-yet")
}

fn read(dir: &Path) -> String {
    std::fs::read_to_string(dir.join("openmw.cfg")).unwrap()
}

#[test]
fn the_last_config_directory_is_the_users_without_an_openmw_cfg() {
    // A directory that does not exist, and one that exists without an openmw.cfg.
    for user_dir in [missing_dir("missing_last"), temp_dir("empty_last")] {
        let root_dir = temp_dir("missing_last_root");
        write_cfg(
            &root_dir,
            &format!("content=Morrowind.esm\nconfig={}\n", user_dir.display()),
        );

        let config = OpenMWConfiguration::new(Some(root_dir)).unwrap();

        assert_eq!(config.user_config_path(), user_dir);
        assert!(!config.is_user_config());
        assert_eq!(sub_configs(&config), std::slice::from_ref(&user_dir));
        assert_eq!(
            statuses(&config),
            [ConfigChainStatus::Loaded, ConfigChainStatus::SkippedMissing]
        );
        assert!(
            config
                .to_string()
                .contains(&format!("config={}\n", user_dir.display()))
        );

        // The user's configuration on its own is empty, rooted where OpenMW writes it.
        for user in [
            config.user_config_ref().unwrap(),
            config.clone().user_config().unwrap(),
        ] {
            assert!(user.is_user_config());
            assert_eq!(user.root_config_dir(), user_dir);
            assert_eq!(user.content_file_count(), 0);
        }
    }
}

#[test]
fn a_missing_directory_in_the_middle_stays_in_the_chain() {
    let root_dir = temp_dir("missing_middle_root");
    let missing = missing_dir("missing_middle");
    let user_dir = temp_dir("missing_middle_user");
    write_cfg(&user_dir, "content=User.esp\n");
    write_cfg(
        &root_dir,
        &format!(
            "content=Morrowind.esm\nconfig={}\nconfig={}\n",
            missing.display(),
            user_dir.display()
        ),
    );

    let config = OpenMWConfiguration::new(Some(root_dir)).unwrap();

    assert_eq!(content(&config), ["Morrowind.esm", "User.esp"]);
    assert_eq!(config.user_config_path(), user_dir);
    assert_eq!(sub_configs(&config), [missing, user_dir]);
    assert_eq!(
        statuses(&config),
        [
            ConfigChainStatus::Loaded,
            ConfigChainStatus::SkippedMissing,
            ConfigChainStatus::Loaded
        ]
    );
}

#[test]
fn replace_config_drops_a_missing_directory_only_once_a_config_loaded() {
    // readConfiguration truncates mActiveConfigPaths with parsedConfigs, and only when more
    // than the root has loaded.
    let missing = missing_dir("missing_replace_kept");
    let replacing = temp_dir("missing_replace_kept_user");
    write_cfg(&replacing, "replace=config\n");
    let root_dir = temp_dir("missing_replace_kept_root");
    write_cfg(
        &root_dir,
        &format!(
            "config={}\nconfig={}\n",
            missing.display(),
            replacing.display()
        ),
    );
    let config = OpenMWConfiguration::new(Some(root_dir)).unwrap();
    assert_eq!(sub_configs(&config), [missing, replacing.clone()]);
    assert_eq!(config.user_config_path(), replacing);

    let loaded = temp_dir("missing_replace_dropped_loaded");
    write_cfg(&loaded, "content=Loaded.esp\n");
    let missing = missing_dir("missing_replace_dropped");
    let replacing = temp_dir("missing_replace_dropped_user");
    write_cfg(&replacing, "replace=config\n");
    let root_dir = temp_dir("missing_replace_dropped_root");
    write_cfg(
        &root_dir,
        &format!(
            "config={}\nconfig={}\nconfig={}\n",
            loaded.display(),
            missing.display(),
            replacing.display()
        ),
    );
    let config = OpenMWConfiguration::new(Some(root_dir)).unwrap();
    assert_eq!(sub_configs(&config), std::slice::from_ref(&replacing));
    assert_eq!(config.user_config_path(), replacing);
}

#[test]
fn save_user_on_a_fresh_install_creates_the_users_openmw_cfg() {
    let root_dir = temp_dir("fresh_install_root");
    let user_dir = missing_dir("fresh_install");
    let root_text = format!(
        "content=Morrowind.esm\ncontent=Tribunal.esm\nconfig={}\n",
        user_dir.display()
    );
    write_cfg(&root_dir, &root_text);

    let mut config = OpenMWConfiguration::new(Some(root_dir.clone())).unwrap();
    config.remove_content_file("Tribunal.esm");
    config.add_content_file("Mod.esp").unwrap();
    config.save_user().unwrap();

    // As the launcher does on first run: the directory is created and openmw.cfg written in it,
    // holding the user's settings. The root is not touched.
    assert_eq!(
        read(&user_dir),
        "replace=content\ncontent=Morrowind.esm\ncontent=Mod.esp\n"
    );
    assert_eq!(read(&root_dir), root_text);

    let reloaded = OpenMWConfiguration::new(Some(root_dir)).unwrap();
    assert_eq!(content(&reloaded), content(&config));
    assert_eq!(reloaded.user_config_path(), user_dir);
    assert_eq!(
        statuses(&reloaded),
        [ConfigChainStatus::Loaded, ConfigChainStatus::Loaded]
    );
}

#[test]
fn saving_the_file_that_names_a_missing_directory_keeps_the_line() {
    let root_dir = temp_dir("dangling_root");
    let modlist_dir = temp_dir("dangling_modlist");
    let user_dir = temp_dir("dangling_user");
    let missing = missing_dir("dangling");
    write_cfg(&user_dir, "content=User.esp\n");
    let modlist_text = format!(
        "content=Modlist.esp\n# not there yet\nconfig={}\n",
        missing.display()
    );
    write_cfg(&modlist_dir, &modlist_text);
    write_cfg(
        &root_dir,
        &format!(
            "config={}\nconfig={}\n",
            modlist_dir.display(),
            user_dir.display()
        ),
    );

    let config = OpenMWConfiguration::new(Some(root_dir)).unwrap();
    assert_eq!(config.user_config_path(), user_dir);

    config.save_subconfig(&modlist_dir).unwrap();
    assert_eq!(read(&modlist_dir), modlist_text);
}

#[test]
fn loading_creates_the_user_config_directory_as_the_engine_does() {
    // readConfiguration ends with create_directories(getUserConfigPath()): the last directory
    // of the chain, with the directories above it. It writes no openmw.cfg there.
    let root_dir = temp_dir("create_user_root");
    let middle = missing_dir("create_user_middle");
    let user_dir = missing_dir("create_user").join("nested");
    write_cfg(
        &root_dir,
        &format!(
            "config={}\nconfig={}\n",
            middle.display(),
            user_dir.display()
        ),
    );

    let config = OpenMWConfiguration::new(Some(root_dir)).unwrap();

    assert_eq!(config.user_config_path(), user_dir);
    assert!(user_dir.is_dir());
    assert!(!user_dir.join("openmw.cfg").exists());
    assert!(
        !middle.exists(),
        "only the user config directory is created"
    );
    assert_eq!(
        statuses(&config),
        [
            ConfigChainStatus::Loaded,
            ConfigChainStatus::SkippedMissing,
            ConfigChainStatus::SkippedMissing
        ]
    );
}

#[test]
fn loading_fails_when_the_user_config_directory_cannot_be_created() {
    // std::filesystem::create_directories throws there, and OpenMW stops with a fatal error.
    let root_dir = temp_dir("create_user_fails_root");
    let blocker = temp_dir("create_user_fails").join("a-file");
    std::fs::write(&blocker, "").unwrap();
    let user_dir = blocker.join("user");
    write_cfg(
        &root_dir,
        &format!("content=Morrowind.esm\nconfig={}\n", user_dir.display()),
    );

    match OpenMWConfiguration::new(Some(root_dir)) {
        Err(ConfigError::NotWritable(path)) => assert_eq!(path, user_dir),
        other => panic!("expected NotWritable, got {other:?}"),
    }
}
