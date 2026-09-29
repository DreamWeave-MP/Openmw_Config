//! `set_*` and `remove_*` against `save_user()`: what is in memory after a mutation is what a
//! reload of the saved chain yields, parents included, and parent files are never touched.

mod common;

use common::{temp_dir, write_cfg};
use openmw_config::{FileSetting, OpenMWConfiguration};
use std::path::PathBuf;

/// A root config chaining to a user config, both with the given contents; returns the two
/// directories and the loaded chain.
fn chain(tag: &str, root: &str, user: &str) -> (PathBuf, PathBuf, OpenMWConfiguration) {
    let root_dir = temp_dir(&format!("wb_{tag}_root"));
    let user_dir = temp_dir(&format!("wb_{tag}_user"));
    write_cfg(&user_dir, user);
    write_cfg(&root_dir, &format!("{root}config={}\n", user_dir.display()));
    let config = OpenMWConfiguration::new(Some(root_dir.clone())).unwrap();
    (root_dir, user_dir, config)
}

fn content(config: &OpenMWConfiguration) -> Vec<&str> {
    config
        .content_files_iter()
        .map(FileSetting::value_str)
        .collect()
}

fn user_file(user_dir: &std::path::Path) -> String {
    std::fs::read_to_string(user_dir.join("openmw.cfg")).unwrap()
}

#[test]
fn set_content_files_over_a_parent_list_persists_and_reloads_without_duplicates() {
    let (root_dir, user_dir, mut config) = chain("set_content", "content=Morrowind.esm\n", "");
    config.set_content_files(Some(vec!["Morrowind.esm".into(), "Mod.esp".into()]));
    assert_eq!(content(&config), ["Morrowind.esm", "Mod.esp"]);
    config.save_user().unwrap();

    let saved = user_file(&user_dir);
    assert_eq!(
        saved,
        "replace=content\ncontent=Morrowind.esm\ncontent=Mod.esp\n"
    );
    assert_eq!(
        std::fs::read_to_string(root_dir.join("openmw.cfg")).unwrap(),
        format!("content=Morrowind.esm\nconfig={}\n", user_dir.display()),
        "the parent is never rewritten"
    );

    let reloaded = OpenMWConfiguration::new(Some(root_dir)).unwrap();
    assert_eq!(content(&reloaded), ["Morrowind.esm", "Mod.esp"]);
}

#[test]
fn remove_content_file_of_a_parent_entry_persists() {
    let (root_dir, user_dir, mut config) = chain(
        "remove_content",
        "content=Morrowind.esm\ncontent=Tribunal.esm\n",
        "content=User.esp\n",
    );
    config.remove_content_file("Morrowind.esm");
    assert_eq!(content(&config), ["Tribunal.esm", "User.esp"]);
    config.save_user().unwrap();

    assert_eq!(
        user_file(&user_dir),
        "replace=content\ncontent=Tribunal.esm\ncontent=User.esp\n"
    );
    let reloaded = OpenMWConfiguration::new(Some(root_dir)).unwrap();
    assert_eq!(content(&reloaded), ["Tribunal.esm", "User.esp"]);
}

#[test]
fn removing_a_user_entry_leaves_the_parent_list_alone() {
    let (root_dir, user_dir, mut config) = chain(
        "remove_user",
        "content=Morrowind.esm\n",
        "content=User.esp\ncontent=Other.esp\n",
    );
    config.remove_content_file("User.esp");
    config.save_user().unwrap();
    assert_eq!(user_file(&user_dir), "content=Other.esp\n");
    let reloaded = OpenMWConfiguration::new(Some(root_dir)).unwrap();
    assert_eq!(content(&reloaded), ["Morrowind.esm", "Other.esp"]);
}

#[test]
fn a_single_config_writes_no_replace_line() {
    let dir = temp_dir("wb_single");
    write_cfg(&dir, "content=Morrowind.esm\n");
    let mut config = OpenMWConfiguration::new(Some(dir.clone())).unwrap();
    config.set_content_files(Some(vec!["A.esm".into()]));
    config.remove_content_file("A.esm");
    config.add_content_file("B.esm").unwrap();
    config.save_user().unwrap();
    assert_eq!(user_file(&dir), "content=B.esm\n");
}

#[test]
fn every_list_setter_and_remover_takes_the_parent_list_over() {
    let root = "fallback-archive=Root.bsa\ngroundcover=Root.esp\ndata=/root/data\nfallback=iRoot,1\nfallback=iBoth,1\ncustom=root\n";
    let (root_dir, user_dir, mut config) = chain("all", root, "fallback=iBoth,2\n");

    config.set_fallback_archives(Some(vec!["User.bsa".into()]));
    config.remove_groundcover_file("Root.esp");
    config.set_data_directories(Some(vec![PathBuf::from("/user/data")]));
    config
        .set_game_settings(Some(vec!["iUser,3".into()]))
        .unwrap();
    config.set_generic_settings("custom", Some(vec!["user".into()]));
    config.save_user().unwrap();

    let saved = user_file(&user_dir);
    for line in [
        "replace=fallback-archive\nfallback-archive=User.bsa\n",
        "replace=groundcover\n",
        "replace=data\ndata=/user/data\n",
        "replace=fallback\nfallback=iUser,3\n",
        "replace=custom\ncustom=user\n",
    ] {
        assert!(saved.contains(line), "missing {line:?} in:\n{saved}");
    }

    let reloaded = OpenMWConfiguration::new(Some(root_dir)).unwrap();
    let archives: Vec<_> = reloaded
        .fallback_archives_iter()
        .map(FileSetting::value_str)
        .collect();
    assert_eq!(archives, ["User.bsa"]);
    assert_eq!(reloaded.groundcover_iter().count(), 0);
    let dirs: Vec<_> = reloaded
        .data_directories_iter()
        .map(|dir| dir.parsed().to_path_buf())
        .collect();
    assert_eq!(dirs, [PathBuf::from("/user/data")]);
    let keys: Vec<_> = reloaded
        .game_settings()
        .map(|setting| setting.key_str().to_owned())
        .collect();
    assert_eq!(keys, ["iUser"]);
    let custom: Vec<_> = reloaded
        .generic_settings_iter()
        .filter(|setting| setting.key() == "custom")
        .map(|setting| setting.value().to_owned())
        .collect();
    assert_eq!(custom, ["user"]);
}

#[test]
fn remove_data_directory_and_archive_of_a_parent_persist() {
    let (root_dir, user_dir, mut config) = chain(
        "remove_dirs",
        "data=/root/a\ndata=/root/b\nfallback-archive=A.bsa\nfallback-archive=B.bsa\n",
        "",
    );
    config.remove_data_directory(&PathBuf::from("/root/a"));
    config.remove_archive_file("B.bsa");
    config.save_user().unwrap();
    assert_eq!(
        user_file(&user_dir),
        "replace=data\ndata=/root/b\nreplace=fallback-archive\nfallback-archive=A.bsa\n"
    );
    let reloaded = OpenMWConfiguration::new(Some(root_dir)).unwrap();
    let dirs: Vec<_> = reloaded
        .data_directories_iter()
        .map(|dir| dir.parsed().to_path_buf())
        .collect();
    assert_eq!(dirs, [PathBuf::from("/root/b")]);
    let archives: Vec<_> = reloaded
        .fallback_archives_iter()
        .map(FileSetting::value_str)
        .collect();
    assert_eq!(archives, ["A.bsa"]);
}

#[test]
fn a_second_set_reuses_the_replace_line() {
    let (_root_dir, user_dir, mut config) = chain("set_twice", "content=Morrowind.esm\n", "");
    config.set_content_files(Some(vec!["A.esm".into()]));
    config.set_content_files(Some(vec!["B.esm".into()]));
    config.save_user().unwrap();
    assert_eq!(user_file(&user_dir), "replace=content\ncontent=B.esm\n");
}

#[test]
fn replace_of_a_generic_key_is_honoured_on_load() {
    let dir = temp_dir("wb_generic_replace");
    write_cfg(
        &dir,
        "custom=1\ncustom=2\nreplace=custom\ncustom=3\nother=x\n",
    );
    let config = OpenMWConfiguration::new(Some(dir)).unwrap();
    let values: Vec<_> = config
        .generic_settings_iter()
        .map(|setting| format!("{}={}", setting.key(), setting.value()))
        .collect();
    assert_eq!(values, ["custom=3", "other=x"]);
}

/// root -> mid -> user, where `mid` holds `mid` and is a sub-configuration `save_subconfig` can
/// write; returns the three directories and the loaded chain.
fn three_level_chain(
    tag: &str,
    mid: &str,
    user: &str,
) -> (PathBuf, PathBuf, PathBuf, OpenMWConfiguration) {
    let root_dir = temp_dir(&format!("wb_{tag}_root"));
    let mid_dir = temp_dir(&format!("wb_{tag}_mid"));
    let user_dir = temp_dir(&format!("wb_{tag}_user"));
    write_cfg(&user_dir, user);
    write_cfg(&mid_dir, &format!("{mid}config={}\n", user_dir.display()));
    write_cfg(&root_dir, &format!("config={}\n", mid_dir.display()));
    let config = OpenMWConfiguration::new(Some(root_dir.clone())).unwrap();
    (root_dir, mid_dir, user_dir, config)
}

#[test]
fn removing_a_parent_entry_leaves_the_parents_other_entries_in_its_file() {
    let (root_dir, mid_dir, user_dir, mut config) = three_level_chain(
        "remove_mid",
        "content=A.esm\n# the modlist's pick\ncontent=B.esm\n",
        "content=User.esp\n",
    );
    config.remove_content_file("A.esm");

    let mid_cfg = mid_dir.join("openmw.cfg");
    let kept = config
        .content_files_iter()
        .find(|file| file.value() == "B.esm")
        .unwrap();
    assert_eq!(
        kept.meta().source_config(),
        mid_cfg,
        "B.esm is still the modlist's"
    );

    // Saving the modlist alone persists the removal in the modlist.
    config.save_subconfig(&mid_dir).unwrap();
    assert_eq!(
        std::fs::read_to_string(&mid_cfg).unwrap(),
        format!(
            "# the modlist's pick\ncontent=B.esm\nconfig={}\n",
            user_dir.display()
        )
    );
    let reloaded = OpenMWConfiguration::new(Some(root_dir.clone())).unwrap();
    assert_eq!(content(&reloaded), ["B.esm", "User.esp"]);

    // Saving the user's config takes the list over, without the modlist's comment.
    config.save_user().unwrap();
    assert_eq!(
        user_file(&user_dir),
        "replace=content\ncontent=B.esm\ncontent=User.esp\n"
    );
    let reloaded = OpenMWConfiguration::new(Some(root_dir)).unwrap();
    assert_eq!(content(&reloaded), ["B.esm", "User.esp"]);
}
