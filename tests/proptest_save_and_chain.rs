mod common;

use common::{temp_dir, write_cfg};
use openmw_config::{ConfigError, OpenMWConfiguration};
use proptest::prelude::*;
use std::collections::HashSet;
use std::fmt::Write as _;

fn unique_preserve_order(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();

    for value in values {
        if seen.insert(value.clone()) {
            out.push(value);
        }
    }

    out
}

fn cfg_with_content(values: &[String]) -> String {
    let mut cfg = String::new();
    for value in values {
        writeln!(&mut cfg, "content={value}").expect("writing to String cannot fail");
    }
    cfg
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(40))]

    #[test]
    fn prop_save_user_writes_only_user_owned_entries(
        root_values in prop::collection::vec("[A-Za-z0-9_-]{1,18}\\.esm", 0..10),
        user_values in prop::collection::vec("[A-Za-z0-9_-]{1,18}\\.esm", 0..10),
        additions in prop::collection::vec("[A-Za-z0-9_-]{1,18}\\.esp", 0..10),
    ) {
        let root_values = unique_preserve_order(root_values);
        let root_set: HashSet<String> = root_values.iter().cloned().collect();
        let user_values = unique_preserve_order(user_values)
            .into_iter()
            .filter(|v| !root_set.contains(v))
            .collect::<Vec<_>>();

        let root_dir = temp_dir("prop_save_user_root");
        let user_dir = temp_dir("prop_save_user_user");

        write_cfg(&user_dir, &cfg_with_content(&user_values));

        let mut root_cfg = cfg_with_content(&root_values);
        writeln!(&mut root_cfg, "config={}", user_dir.display())
            .expect("writing to String cannot fail");
        write_cfg(&root_dir, &root_cfg);

        let mut config = OpenMWConfiguration::new(Some(root_dir)).unwrap();
        let additions = unique_preserve_order(additions);
        let mut applied_additions = Vec::new();
        for plugin in additions {
            if !config.has_content_file(&plugin) {
                config.add_content_file(&plugin).unwrap();
                applied_additions.push(plugin);
            }
        }

        config.save_user().unwrap();
        let saved_user = std::fs::read_to_string(user_dir.join("openmw.cfg")).unwrap();

        for plugin in &user_values {
            let needle = format!("content={plugin}");
            prop_assert!(saved_user.contains(&needle));
        }
        for plugin in &applied_additions {
            let needle = format!("content={plugin}");
            prop_assert!(saved_user.contains(&needle));
        }
        for plugin in &root_values {
            if !user_values.contains(plugin) && !applied_additions.contains(plugin) {
                let needle = format!("content={plugin}");
                prop_assert!(!saved_user.contains(&needle));
            }
        }
    }

    #[test]
    fn prop_cross_file_duplicate_content_is_rejected(
        root_name in "[A-Za-z0-9_-]{1,18}\\.esm",
        sub_name in "[A-Za-z0-9_-]{1,18}\\.esm",
        duplicate in any::<bool>(),
    ) {
        let root_dir = temp_dir("prop_dup_content_root");
        let sub_dir = temp_dir("prop_dup_content_sub");

        let sub_final = if duplicate { root_name.clone() } else { sub_name.clone() };

        write_cfg(&sub_dir, &format!("content={sub_final}\n"));
        write_cfg(
            &root_dir,
            &format!("content={root_name}\nconfig={}\n", sub_dir.display()),
        );

        let result = OpenMWConfiguration::new(Some(root_dir));
        let is_duplicate = matches!(result, Err(ConfigError::DuplicateContentFile { .. }));
        if duplicate {
            prop_assert!(is_duplicate);
        } else {
            prop_assert!(result.is_ok());
        }
    }

    #[test]
    fn prop_cross_file_duplicate_archive_is_rejected(
        root_name in "[A-Za-z0-9_-]{1,18}\\.bsa",
        sub_name in "[A-Za-z0-9_-]{1,18}\\.bsa",
        duplicate in any::<bool>(),
    ) {
        let root_dir = temp_dir("prop_dup_archive_root");
        let sub_dir = temp_dir("prop_dup_archive_sub");

        let sub_final = if duplicate { root_name.clone() } else { sub_name.clone() };

        write_cfg(&sub_dir, &format!("fallback-archive={sub_final}\n"));
        write_cfg(
            &root_dir,
            &format!("fallback-archive={root_name}\nconfig={}\n", sub_dir.display()),
        );

        let result = OpenMWConfiguration::new(Some(root_dir));
        let is_duplicate = matches!(result, Err(ConfigError::DuplicateArchiveFile { .. }));
        if duplicate {
            prop_assert!(is_duplicate);
        } else {
            prop_assert!(result.is_ok());
        }
    }

    #[test]
    fn prop_cross_file_duplicate_groundcover_is_rejected(
        root_name in "[A-Za-z0-9_-]{1,18}\\.esp",
        sub_name in "[A-Za-z0-9_-]{1,18}\\.esp",
        duplicate in any::<bool>(),
    ) {
        let root_dir = temp_dir("prop_dup_ground_root");
        let sub_dir = temp_dir("prop_dup_ground_sub");

        let sub_final = if duplicate { root_name.clone() } else { sub_name.clone() };

        write_cfg(&sub_dir, &format!("groundcover={sub_final}\n"));
        write_cfg(
            &root_dir,
            &format!("groundcover={root_name}\nconfig={}\n", sub_dir.display()),
        );

        let result = OpenMWConfiguration::new(Some(root_dir));
        let is_duplicate = matches!(result, Err(ConfigError::DuplicateGroundcoverFile { .. }));
        if duplicate {
            prop_assert!(is_duplicate);
        } else {
            prop_assert!(result.is_ok());
        }
    }
}
