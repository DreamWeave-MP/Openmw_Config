mod common;

use common::{temp_dir, write_cfg};
use openmw_config::{ConfigChainStatus, EncodingSetting, OpenMWConfiguration};

fn load(contents: &str) -> OpenMWConfiguration {
    let dir = temp_dir("replace");
    write_cfg(&dir, contents);
    OpenMWConfiguration::new(Some(dir)).unwrap()
}

#[test]
fn test_replace_fallback_clears_prior_game_settings() {
    let config = load("fallback=iOld,1\nreplace=fallback\nfallback=iNew,2\n");
    assert!(config.get_game_setting("iOld").is_none());
    assert_eq!(config.get_game_setting("iNew").unwrap().value(), "2");
}

#[test]
fn test_replace_fallback_archives_clears_prior_archives() {
    let config =
        load("fallback-archive=Old.bsa\nreplace=fallback-archives\nfallback-archive=New.bsa\n");
    assert!(!config.has_archive_file("Old.bsa"));
    assert!(config.has_archive_file("New.bsa"));
}

#[test]
fn test_replace_groundcover_clears_prior_groundcover() {
    let config = load("groundcover=Old.esp\nreplace=groundcover\ngroundcover=New.esp\n");
    assert!(!config.has_groundcover_file("Old.esp"));
    assert!(config.has_groundcover_file("New.esp"));
}

#[test]
fn test_replace_singletons_clears_previous_values() {
    let config = load(
        "user-data=/old/user\nreplace=user-data\nuser-data=/new/user\nresources=/old/res\nreplace=resources\nresources=/new/res\ndata-local=/old/local\nreplace=data-local\ndata-local=/new/local\n",
    );

    assert_eq!(config.userdata().unwrap().original(), "/new/user");
    assert_eq!(config.resources().unwrap().original(), "/new/res");
    assert_eq!(config.data_local().unwrap().original(), "/new/local");
}

#[test]
fn test_replace_config_clears_prior_settings() {
    let config = load(
        "content=Old.esm\nfallback-archive=Old.bsa\nencoding=win1252\nreplace=config\ncontent=New.esm\n",
    );

    assert!(config.has_content_file("New.esm"));
    assert!(!config.has_content_file("Old.esm"));
    assert!(!config.has_archive_file("Old.bsa"));
    assert!(config.encoding().is_none());
}

#[test]
fn test_replace_config_clears_queued_subconfigs_in_same_file() {
    let root_dir = temp_dir("replace_config_queue_root");
    let a_dir = temp_dir("replace_config_queue_a");
    let b_dir = temp_dir("replace_config_queue_b");

    write_cfg(&a_dir, "content=A.esm\n");
    write_cfg(&b_dir, "content=B.esm\n");
    write_cfg(
        &root_dir,
        &format!(
            "content=Root.esm\nconfig={}\nreplace=config\nconfig={}\n",
            a_dir.display(),
            b_dir.display()
        ),
    );

    let config = OpenMWConfiguration::new(Some(root_dir)).unwrap();

    assert!(config.has_content_file("B.esm"));
    assert!(!config.has_content_file("A.esm"));

    let sub_paths: Vec<_> = config
        .sub_configs()
        .map(|setting| setting.parsed().to_path_buf())
        .collect();
    assert_eq!(sub_paths, vec![b_dir.clone()]);
    assert_eq!(config.user_config_path(), b_dir);
}

#[test]
fn test_singleton_setters_replace_and_clear_latest_entry() {
    let mut config = load("user-data=/u0\nresources=/r0\ndata-local=/d0\nencoding=win1251\n");

    let mut no_comment = String::new();
    let cfg_path = config.root_config_file().to_path_buf();

    config.set_userdata(Some(openmw_config::DirectorySetting::new(
        "/u1",
        cfg_path.clone(),
        &mut no_comment,
    )));
    config.set_resources(Some(openmw_config::DirectorySetting::new(
        "/r1",
        cfg_path.clone(),
        &mut no_comment,
    )));
    config.set_data_local(Some(openmw_config::DirectorySetting::new(
        "/d1",
        cfg_path,
        &mut no_comment,
    )));
    let encoding = EncodingSetting::try_from((
        "win1252".to_string(),
        config.root_config_file(),
        &mut no_comment,
    ))
    .unwrap();
    config.set_encoding(Some(encoding));

    assert_eq!(config.userdata().unwrap().original(), "/u1");
    assert_eq!(config.resources().unwrap().original(), "/r1");
    assert_eq!(config.data_local().unwrap().original(), "/d1");
    assert_eq!(
        config.encoding().unwrap().to_string().trim(),
        "encoding=win1252"
    );

    config.set_userdata(None);
    config.set_resources(None);
    config.set_data_local(None);
    config.set_encoding(None);

    assert!(config.userdata().is_none());
    assert!(config.resources().is_none());
    assert!(config.data_local().is_none());
    assert!(config.encoding().is_none());
}

#[test]
fn test_archive_and_groundcover_adders_append_unique_values() {
    let mut config = load("");

    config.add_archive_file("Morrowind.bsa").unwrap();
    config.add_groundcover_file("Grass.esp").unwrap();

    assert!(config.has_archive_file("Morrowind.bsa"));
    assert!(config.has_groundcover_file("Grass.esp"));
}

#[test]
fn test_set_fallback_archives_replaces_and_clears() {
    let mut config = load("fallback-archive=Old.bsa\n");

    config.set_fallback_archives(Some(vec!["New.bsa".to_string()]));
    assert!(!config.has_archive_file("Old.bsa"));
    assert!(config.has_archive_file("New.bsa"));

    config.set_fallback_archives(None);
    assert_eq!(config.fallback_archives_iter().count(), 0);
}

#[test]
fn test_set_game_settings_replaces_and_clears() {
    let mut config = load("fallback=iOld,1\n");

    config
        .set_game_settings(Some(vec!["iNew,2".to_string()]))
        .unwrap();
    assert!(config.get_game_setting("iOld").is_none());
    assert_eq!(config.get_game_setting("iNew").unwrap().value(), "2");

    config.set_game_settings(None).unwrap();
    assert_eq!(config.game_settings().count(), 0);
}

#[test]
fn test_set_game_settings_invalid_entry_returns_error_and_leaves_map_empty() {
    let mut config = load("fallback=iOld,1\n");

    let result = config.set_game_settings(Some(vec!["invalid".to_string()]));
    assert!(result.is_err());
    assert_eq!(config.game_settings().count(), 0);
}

#[test]
fn test_user_config_and_is_user_config_contract() {
    let root_dir = temp_dir("user_config_root");
    write_cfg(&root_dir, "content=Root.esm\n");
    let root_only = OpenMWConfiguration::new(Some(root_dir.clone())).unwrap();
    assert!(root_only.is_user_config());

    let sub_dir = temp_dir("user_config_sub");
    write_cfg(&sub_dir, "content=Sub.esm\n");
    write_cfg(
        &root_dir,
        &format!("content=Root.esm\nconfig={}\n", sub_dir.display()),
    );

    let chained = OpenMWConfiguration::new(Some(root_dir)).unwrap();
    assert!(!chained.is_user_config());

    let user_only_ref = chained.user_config_ref().unwrap();
    let user_only = chained.clone().user_config().unwrap();
    assert!(user_only.has_content_file("Sub.esm"));
    assert!(user_only_ref.has_content_file("Sub.esm"));
    assert!(!user_only.has_content_file("Root.esm"));
    assert!(!user_only_ref.has_content_file("Root.esm"));
}

#[test]
fn test_config_chain_reports_loaded_and_missing_entries() {
    let root_dir = temp_dir("chain_report_root");
    let loaded_dir = temp_dir("chain_report_loaded");
    let missing_dir = temp_dir("chain_report_missing");

    write_cfg(&loaded_dir, "content=Loaded.esm\n");
    write_cfg(
        &root_dir,
        &format!(
            "config={}\nconfig={}\n",
            loaded_dir.display(),
            missing_dir.display()
        ),
    );

    let config = OpenMWConfiguration::new(Some(root_dir)).unwrap();
    let chain: Vec<_> = config.config_chain().cloned().collect();

    assert_eq!(chain[0].status(), &ConfigChainStatus::Loaded);
    assert_eq!(chain[1].status(), &ConfigChainStatus::SkippedMissing);
    assert_eq!(chain[2].status(), &ConfigChainStatus::Loaded);
}
