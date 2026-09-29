mod common;

use common::{temp_dir, write_cfg};
use openmw_config::{ConfigChainStatus, ConfigError, EncodingSetting, OpenMWConfiguration};
use std::fmt::Write as _;
use std::path::PathBuf;

fn load(contents: &str) -> OpenMWConfiguration {
    let dir = temp_dir("replace");
    write_cfg(&dir, contents);
    OpenMWConfiguration::new(Some(dir)).unwrap()
}

/// A chain of files, root first, each ending in a `config=` line that names the next; returns
/// their directories and the loaded chain.
fn linear_chain(tag: &str, files: &[&str]) -> (Vec<PathBuf>, OpenMWConfiguration) {
    let dirs: Vec<PathBuf> = (0..files.len())
        .map(|index| temp_dir(&format!("{tag}_{index}")))
        .collect();
    for (index, text) in files.iter().enumerate() {
        let mut text = (*text).to_owned();
        if let Some(next) = dirs.get(index + 1) {
            writeln!(text, "config={}", next.display()).unwrap();
        }
        write_cfg(&dirs[index], &text);
    }
    let config = OpenMWConfiguration::new(Some(dirs[0].clone())).unwrap();
    (dirs, config)
}

fn content(config: &OpenMWConfiguration) -> Vec<String> {
    config
        .content_files_iter()
        .map(|file| file.value().clone())
        .collect()
}

#[test]
fn test_replace_matches_option_names_exactly() {
    // mergeComposingVariables (components/files/configurationmanager.cpp) looks each option's
    // name up in the replace= values as written: in another case they name no option.
    let (_, config) = linear_chain(
        "replace_case",
        &[
            "content=Root.esm\ngroundcover=Root.esp\nfallback-archive=Root.bsa\ndata=/root/data\nfallback=iRoot,1\n",
            "replace=Content\nreplace=GROUNDCOVER\nreplace=Fallback-Archive\nreplace=Data\nreplace=FALLBACK\ncontent=User.esp\n",
        ],
    );

    assert_eq!(content(&config), ["Root.esm", "User.esp"]);
    assert!(config.has_groundcover_file("Root.esp"));
    assert!(config.has_archive_file("Root.bsa"));
    assert!(config.has_data_dir("/root/data"));
    assert_eq!(config.get_game_setting("iRoot").unwrap().value(), "1");
}

#[test]
fn test_replace_config_matches_exactly() {
    // hasReplaceConfig (configurationmanager.cpp) compares each replace= value with "config".
    let (_, config) = linear_chain(
        "replace_config_case",
        &[
            "content=Root.esm\n",
            "content=Mid.esm\n",
            "replace=Config\nreplace=CONFIG\ncontent=User.esp\n",
        ],
    );

    assert_eq!(content(&config), ["Root.esm", "Mid.esm", "User.esp"]);
}

#[test]
fn test_replace_fallback_clears_prior_game_settings() {
    let (_, config) = linear_chain(
        "replace_fallback",
        &["fallback=iOld,1\n", "replace=fallback\nfallback=iNew,2\n"],
    );
    assert!(config.get_game_setting("iOld").is_none());
    assert_eq!(config.get_game_setting("iNew").unwrap().value(), "2");
}

#[test]
fn test_replace_fallback_archive_clears_prior_archives() {
    let (_, config) = linear_chain(
        "replace_archives",
        &[
            "fallback-archive=Old.bsa\n",
            "replace=fallback-archive\nfallback-archive=New.bsa\n",
        ],
    );
    assert!(!config.has_archive_file("Old.bsa"));
    assert!(config.has_archive_file("New.bsa"));
}

#[test]
fn test_replace_groundcover_clears_prior_groundcover() {
    let (_, config) = linear_chain(
        "replace_groundcover",
        &[
            "groundcover=Old.esp\n",
            "replace=groundcover\ngroundcover=New.esp\n",
        ],
    );
    assert!(!config.has_groundcover_file("Old.esp"));
    assert!(config.has_groundcover_file("New.esp"));
}

#[test]
fn test_replace_is_per_file_and_keeps_the_files_own_entries() {
    // OpenMW parses a whole file before mergeComposingVariables applies its replace= lines to
    // the files before it: in a file alone they discard nothing, wherever they sit.
    let config = load(
        "content=Old.esm\nreplace=content\ncontent=New.esm\n\
         groundcover=Old.esp\nreplace=groundcover\ngroundcover=New.esp\n\
         fallback-archive=Old.bsa\nreplace=fallback-archive\nfallback-archive=New.bsa\n\
         data=/old\nreplace=data\ndata=/new\n\
         fallback=iOld,1\nfallback=iBoth,1\nreplace=fallback\nfallback=iBoth,2\n\
         custom=old\nreplace=custom\ncustom=new\n",
    );

    assert_eq!(content(&config), ["Old.esm", "New.esm"]);
    assert!(config.has_groundcover_file("Old.esp") && config.has_groundcover_file("New.esp"));
    assert!(config.has_archive_file("Old.bsa") && config.has_archive_file("New.bsa"));
    assert!(config.has_data_dir("/old") && config.has_data_dir("/new"));
    assert_eq!(config.get_game_setting("iOld").unwrap().value(), "1");
    assert_eq!(config.get_game_setting("iBoth").unwrap().value(), "2");
    let custom: Vec<_> = config
        .generic_settings_iter()
        .map(|setting| setting.value().to_owned())
        .collect();
    assert_eq!(custom, ["old", "new"]);
}

#[test]
fn test_replace_discards_the_lists_of_every_file_before_it() {
    let (_, config) = linear_chain(
        "replace_per_file",
        &[
            "content=Root.esm\ngroundcover=Root.esp\nfallback-archive=Root.bsa\ndata=/root\nfallback=iRoot,1\nfallback=iKey,1\ncustom=root\n",
            "content=Mid.esm\ngroundcover=Mid.esp\nfallback-archive=Mid.bsa\ndata=/mid\nfallback=iMid,1\ncustom=mid\n",
            "content=User1.esp\nfallback=iKey,2\nreplace=content\nreplace=groundcover\nreplace=fallback-archive\nreplace=data\nreplace=fallback\nreplace=custom\ncontent=User2.esp\ncustom=user\n",
        ],
    );

    assert_eq!(content(&config), ["User1.esp", "User2.esp"]);
    assert_eq!(config.groundcover_iter().count(), 0);
    assert_eq!(config.fallback_archives_iter().count(), 0);
    assert_eq!(config.data_directories_iter().count(), 0);
    assert!(config.get_game_setting("iRoot").is_none());
    assert!(config.get_game_setting("iMid").is_none());
    assert_eq!(config.get_game_setting("iKey").unwrap().value(), "2");
    let custom: Vec<_> = config
        .generic_settings_iter()
        .map(|setting| setting.value().to_owned())
        .collect();
    assert_eq!(custom, ["user"]);
}

#[test]
fn test_replace_leaves_the_files_after_it_alone() {
    let (_, config) = linear_chain(
        "replace_middle",
        &[
            "content=Root.esm\ndata=/root\n",
            "content=Mid1.esm\nreplace=content\ncontent=Mid2.esm\n",
            "content=User.esp\n",
        ],
    );

    assert_eq!(content(&config), ["Mid1.esm", "Mid2.esm", "User.esp"]);
    assert!(config.has_data_dir("/root"));
}

#[test]
fn test_a_name_a_replace_discards_may_come_back() {
    // The root's A.esm is gone before OpenMW looks for a name listed twice.
    let (_, config) = linear_chain(
        "replace_duplicate_ok",
        &["content=A.esm\n", "content=A.esm\nreplace=content\n"],
    );

    assert_eq!(content(&config), ["A.esm"]);
}

#[test]
fn test_a_name_twice_around_a_replace_in_one_file_is_a_duplicate() {
    // Both stay, as OpenMW keeps both and then refuses a content file listed twice.
    let dir = temp_dir("replace_duplicate_fails");
    let cfg = write_cfg(&dir, "content=A.esm\nreplace=content\ncontent=A.esm\n");

    match OpenMWConfiguration::new(Some(dir)) {
        Err(ConfigError::DuplicateContentFile {
            file,
            config_path,
            line,
        }) => {
            assert_eq!(file, "A.esm");
            assert_eq!(config_path, cfg);
            assert_eq!(line, Some(3));
        }
        other => panic!("expected DuplicateContentFile, got {other:?}"),
    }
}

/// A root config holding `Root.bsa` that chains to a user config with `user`'s contents; the
/// archives the chain loads.
fn archives_after_user_config(tag: &str, user: &str) -> Vec<String> {
    let root_dir = temp_dir(&format!("{tag}_root"));
    let user_dir = temp_dir(&format!("{tag}_user"));
    write_cfg(&user_dir, user);
    write_cfg(
        &root_dir,
        &format!("fallback-archive=Root.bsa\nconfig={}\n", user_dir.display()),
    );
    OpenMWConfiguration::new(Some(root_dir))
        .unwrap()
        .fallback_archives_iter()
        .map(|archive| archive.value().clone())
        .collect()
}

#[test]
fn test_replace_names_the_archive_option_as_openmw_does() {
    // `replace=` takes an option name, and OpenMW's is `fallback-archive`.
    let archives = archives_after_user_config(
        "replace_archive_option",
        "replace=fallback-archive\nfallback-archive=User.bsa\n",
    );
    assert_eq!(archives, ["User.bsa"]);

    // No option is called `fallback-archives`, so OpenMW keeps the parent's archives.
    let archives = archives_after_user_config(
        "replace_archive_plural",
        "replace=fallback-archives\nfallback-archive=User.bsa\n",
    );
    assert_eq!(archives, ["Root.bsa", "User.bsa"]);
}

#[test]
fn test_replace_leaves_single_values_alone() {
    // mergeComposingVariables (configurationmanager.cpp) only merges the composing options,
    // the lists: resources=, user-data=, data-local= and encoding= take the last file's value
    // whatever replace= says.
    let (_, config) = linear_chain(
        "replace_single_chain",
        &[
            "resources=/root/res\nuser-data=/root/user\ndata-local=/root/local\nencoding=win1250\n",
            "resources=/mid/res\nuser-data=/mid/user\ndata-local=/mid/local\nencoding=win1251\n",
            "replace=resources\nreplace=user-data\nreplace=data-local\nreplace=encoding\n",
        ],
    );

    assert_eq!(config.resources().unwrap().original(), "/mid/res");
    assert_eq!(config.userdata().unwrap().original(), "/mid/user");
    assert_eq!(config.data_local().unwrap().original(), "/mid/local");
    assert_eq!(
        config.encoding().unwrap().to_string().trim(),
        "encoding=win1251"
    );
}

#[test]
fn test_replace_of_a_single_value_keeps_the_files_own() {
    let config = load(
        "user-data=/old/user\nreplace=user-data\nresources=/old/res\nreplace=resources\ndata-local=/old/local\nreplace=data-local\n",
    );

    assert_eq!(config.userdata().unwrap().original(), "/old/user");
    assert_eq!(config.resources().unwrap().original(), "/old/res");
    assert_eq!(config.data_local().unwrap().original(), "/old/local");
}

#[test]
fn test_replace_config_in_the_root_does_nothing() {
    // readConfiguration checks replace=config in the files after the root, and on the command
    // line; the root's own follows its config= entries and keeps its settings.
    let config = load(
        "content=Old.esm\nfallback-archive=Old.bsa\nencoding=win1252\nreplace=config\ncontent=New.esm\n",
    );

    assert_eq!(content(&config), ["Old.esm", "New.esm"]);
    assert!(config.has_archive_file("Old.bsa"));
    assert_eq!(
        config.encoding().unwrap().to_string().trim(),
        "encoding=win1252"
    );
}

#[test]
fn test_replace_config_in_the_root_follows_every_config_entry() {
    let root_dir = temp_dir("replace_config_root_root");
    let a_dir = temp_dir("replace_config_root_a");
    let b_dir = temp_dir("replace_config_root_b");

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

    assert_eq!(content(&config), ["Root.esm", "A.esm", "B.esm"]);
    let sub_paths: Vec<_> = config
        .sub_configs()
        .map(|setting| setting.parsed().to_path_buf())
        .collect();
    assert_eq!(sub_paths, [a_dir, b_dir.clone()]);
    assert_eq!(config.user_config_path(), b_dir);
}

#[test]
fn test_replace_config_keeps_the_root_and_the_config_entries_still_to_load() {
    // readConfiguration: a file after the root that says replace=config drops the configs read
    // before it except the root, then follows its own config= entries, those before the
    // replace= line included, and every one still on the stack.
    let root_dir = temp_dir("replace_config_keep_root");
    let a_dir = temp_dir("replace_config_keep_a");
    let b_dir = temp_dir("replace_config_keep_b");
    let b_child_dir = temp_dir("replace_config_keep_b_child");
    let c_dir = temp_dir("replace_config_keep_c");

    write_cfg(
        &root_dir,
        &format!(
            "content=Root.esm\nconfig={}\nconfig={}\nconfig={}\n",
            a_dir.display(),
            b_dir.display(),
            c_dir.display()
        ),
    );
    write_cfg(&a_dir, "content=A.esm\nfallback-archive=A.bsa\n");
    write_cfg(
        &b_dir,
        &format!(
            "content=B1.esp\nconfig={}\nreplace=config\ncontent=B2.esp\n",
            b_child_dir.display()
        ),
    );
    write_cfg(&b_child_dir, "content=BChild.esp\n");
    write_cfg(&c_dir, "content=C.esp\n");

    let config = OpenMWConfiguration::new(Some(root_dir.clone())).unwrap();

    assert_eq!(
        content(&config),
        ["Root.esm", "B1.esp", "B2.esp", "BChild.esp", "C.esp"]
    );
    assert!(!config.has_archive_file("A.bsa"));
    assert_eq!(config.user_config_path(), c_dir);

    // The configs the chain dropped are no config= entries in effect; the chain still lists
    // every file it read.
    let sub_paths: Vec<_> = config
        .sub_configs()
        .map(|setting| setting.parsed().to_path_buf())
        .collect();
    assert_eq!(
        sub_paths,
        [b_dir.clone(), c_dir.clone(), b_child_dir.clone()]
    );
    let chain: Vec<_> = config
        .config_chain()
        .map(|entry| entry.path().to_path_buf())
        .collect();
    assert_eq!(
        chain,
        [
            root_dir.join("openmw.cfg"),
            a_dir.join("openmw.cfg"),
            b_dir.join("openmw.cfg"),
            b_child_dir.join("openmw.cfg"),
            c_dir.join("openmw.cfg"),
        ]
    );
}

#[test]
fn test_replace_config_right_after_the_root_drops_nothing() {
    let (_, config) = linear_chain(
        "replace_config_first",
        &["content=Root.esm\n", "replace=config\ncontent=User.esp\n"],
    );

    assert_eq!(content(&config), ["Root.esm", "User.esp"]);
}

#[test]
fn test_a_config_replace_config_dropped_is_not_read_again() {
    // readConfiguration's set of directories it has tried outlives replace=config.
    let root_dir = temp_dir("replace_config_again_root");
    let a_dir = temp_dir("replace_config_again_a");
    let b_dir = temp_dir("replace_config_again_b");

    write_cfg(
        &root_dir,
        &format!("config={}\nconfig={}\n", a_dir.display(), b_dir.display()),
    );
    write_cfg(&a_dir, "content=A.esm\n");
    write_cfg(
        &b_dir,
        &format!(
            "replace=config\ncontent=B.esm\nconfig={}\n",
            a_dir.display()
        ),
    );

    let config = OpenMWConfiguration::new(Some(root_dir)).unwrap();

    assert_eq!(content(&config), ["B.esm"]);
    let sub_paths: Vec<_> = config
        .sub_configs()
        .map(|setting| setting.parsed().to_path_buf())
        .collect();
    assert_eq!(sub_paths, std::slice::from_ref(&b_dir));
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

    let config = OpenMWConfiguration::new(Some(root_dir.clone())).unwrap();
    let chain: Vec<_> = config
        .config_chain()
        .map(|entry| (entry.path().to_path_buf(), entry.status().clone()))
        .collect();

    // In the order readConfiguration tries them: each config= entry when the walk reaches it.
    assert_eq!(
        chain,
        [
            (root_dir.join("openmw.cfg"), ConfigChainStatus::Loaded),
            (loaded_dir.join("openmw.cfg"), ConfigChainStatus::Loaded),
            (
                missing_dir.join("openmw.cfg"),
                ConfigChainStatus::SkippedMissing
            ),
        ]
    );
}

#[test]
fn test_a_config_two_files_name_loads_once() {
    // readConfiguration remembers every directory it has tried and skips a repeated one, so
    // the second config= naming `shared` loads nothing.
    let root_dir = temp_dir("repeated_root");
    let first_dir = temp_dir("repeated_first");
    let second_dir = temp_dir("repeated_second");
    let shared_dir = temp_dir("repeated_shared");
    let missing_dir = temp_dir("repeated_missing");
    write_cfg(
        &root_dir,
        &format!(
            "config={}\nconfig={}\nconfig={}\n",
            first_dir.display(),
            second_dir.display(),
            missing_dir.display()
        ),
    );
    write_cfg(
        &first_dir,
        &format!(
            "content=First.esp\nconfig={}\nconfig={}\n",
            shared_dir.display(),
            missing_dir.display()
        ),
    );
    write_cfg(
        &second_dir,
        &format!("content=Second.esp\nconfig={}\n", shared_dir.display()),
    );
    write_cfg(&shared_dir, "content=Shared.esp\n");

    let config = OpenMWConfiguration::new(Some(root_dir.clone())).unwrap();

    assert_eq!(content(&config), ["First.esp", "Shared.esp", "Second.esp"]);
    let chain: Vec<_> = config
        .config_chain()
        .map(|entry| entry.path().to_path_buf())
        .collect();
    assert_eq!(
        chain,
        [
            root_dir.join("openmw.cfg"),
            first_dir.join("openmw.cfg"),
            shared_dir.join("openmw.cfg"),
            missing_dir.join("openmw.cfg"),
            second_dir.join("openmw.cfg"),
        ]
    );
    assert_eq!(config.user_config_path(), second_dir);
}

#[test]
fn test_config_entries_load_depth_first() {
    // readConfiguration pushes a file's config= entries on a stack, first on top, so the first
    // one and everything it names load before the second.
    let root_dir = temp_dir("depth_first_root");
    let first_dir = temp_dir("depth_first_first");
    let second_dir = temp_dir("depth_first_second");
    let nested_dir = temp_dir("depth_first_nested");
    write_cfg(
        &root_dir,
        &format!(
            "content=Root.esm\nconfig={}\nconfig={}\n",
            first_dir.display(),
            second_dir.display()
        ),
    );
    write_cfg(
        &first_dir,
        &format!("content=First.esp\nconfig={}\n", nested_dir.display()),
    );
    write_cfg(&second_dir, "content=Second.esp\n");
    write_cfg(&nested_dir, "content=Nested.esp\n");

    let config = OpenMWConfiguration::new(Some(root_dir)).unwrap();

    assert_eq!(
        content(&config),
        ["Root.esm", "First.esp", "Nested.esp", "Second.esp"]
    );
    assert_eq!(config.user_config_path(), second_dir);
}
