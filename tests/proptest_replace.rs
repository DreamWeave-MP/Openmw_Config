mod common;

use common::{temp_dir, write_cfg};
use openmw_config::OpenMWConfiguration;
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

fn load_cfg(tag: &str, cfg: &str) -> OpenMWConfiguration {
    let dir = temp_dir(tag);
    write_cfg(&dir, cfg);
    OpenMWConfiguration::new(Some(dir)).unwrap()
}

fn content_values(config: &OpenMWConfiguration) -> Vec<String> {
    config
        .content_files_iter()
        .map(|f| f.value().clone())
        .collect()
}

fn archive_values(config: &OpenMWConfiguration) -> Vec<String> {
    config
        .fallback_archives_iter()
        .map(|f| f.value().clone())
        .collect()
}

fn groundcover_values(config: &OpenMWConfiguration) -> Vec<String> {
    config
        .groundcover_iter()
        .map(|f| f.value().clone())
        .collect()
}

fn fallback_map(config: &OpenMWConfiguration) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    for setting in config.game_settings() {
        map.insert(setting.key().clone(), setting.value().to_string());
    }
    map
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn prop_replace_content_resets_prior_content_only(
        pre_content in prop::collection::vec("[A-Za-z0-9_-]{1,24}\\.esm", 0..12),
        post_content in prop::collection::vec("[A-Za-z0-9_-]{1,24}\\.esm", 0..12),
        archives in prop::collection::vec("[A-Za-z0-9_-]{1,24}\\.bsa", 0..12),
        ground in prop::collection::vec("[A-Za-z0-9_-]{1,24}\\.esp", 0..12),
        fallback in prop::collection::vec(("[A-Za-z][A-Za-z0-9_]{0,20}", "[A-Za-z][A-Za-z0-9]{0,6}"), 0..12),
    ) {
        let pre_content = unique_preserve_order(pre_content);
        let post_content = unique_preserve_order(post_content);
        let archives = unique_preserve_order(archives);
        let ground = unique_preserve_order(ground);

        let mut cfg = String::new();
        for item in &pre_content {
            writeln!(&mut cfg, "content={item}").expect("writing to String cannot fail");
        }
        for item in &archives {
            writeln!(&mut cfg, "fallback-archive={item}").expect("writing to String cannot fail");
        }
        for item in &ground {
            writeln!(&mut cfg, "groundcover={item}").expect("writing to String cannot fail");
        }
        for (k, v) in &fallback {
            writeln!(&mut cfg, "fallback={k},{v}").expect("writing to String cannot fail");
        }

        cfg.push_str("replace=content\n");
        for item in &post_content {
            writeln!(&mut cfg, "content={item}").expect("writing to String cannot fail");
        }

        let loaded = load_cfg("prop_replace_content", &cfg);

        prop_assert_eq!(content_values(&loaded), post_content);
        prop_assert_eq!(archive_values(&loaded), archives);
        prop_assert_eq!(groundcover_values(&loaded), ground);
    }

    #[test]
    fn prop_replace_fallback_resets_prior_fallback_only(
        pre_fallback in prop::collection::vec(("[A-Za-z][A-Za-z0-9_]{0,20}", "[A-Za-z][A-Za-z0-9]{0,6}"), 0..12),
        post_fallback in prop::collection::vec(("[A-Za-z][A-Za-z0-9_]{0,20}", "[A-Za-z][A-Za-z0-9]{0,6}"), 0..12),
        content in prop::collection::vec("[A-Za-z0-9_-]{1,24}\\.esm", 0..12),
        archives in prop::collection::vec("[A-Za-z0-9_-]{1,24}\\.bsa", 0..12),
    ) {
        let content = unique_preserve_order(content);
        let archives = unique_preserve_order(archives);

        let mut cfg = String::new();
        for item in &content {
            writeln!(&mut cfg, "content={item}").expect("writing to String cannot fail");
        }
        for item in &archives {
            writeln!(&mut cfg, "fallback-archive={item}").expect("writing to String cannot fail");
        }
        for (k, v) in &pre_fallback {
            writeln!(&mut cfg, "fallback={k},{v}").expect("writing to String cannot fail");
        }

        cfg.push_str("replace=fallback\n");
        for (k, v) in &post_fallback {
            writeln!(&mut cfg, "fallback={k},{v}").expect("writing to String cannot fail");
        }

        let loaded = load_cfg("prop_replace_fallback", &cfg);

        let mut expected = std::collections::HashMap::new();
        for (k, v) in &post_fallback {
            expected.insert(k.clone(), v.clone());
        }

        prop_assert_eq!(fallback_map(&loaded), expected);
        prop_assert_eq!(content_values(&loaded), content);
        prop_assert_eq!(archive_values(&loaded), archives);
    }

    #[test]
    fn prop_replace_groundcover_resets_prior_groundcover_only(
        pre_ground in prop::collection::vec("[A-Za-z0-9_-]{1,24}\\.esp", 0..12),
        post_ground in prop::collection::vec("[A-Za-z0-9_-]{1,24}\\.esp", 0..12),
        content in prop::collection::vec("[A-Za-z0-9_-]{1,24}\\.esm", 0..12),
    ) {
        let pre_ground = unique_preserve_order(pre_ground);
        let post_ground = unique_preserve_order(post_ground);
        let content = unique_preserve_order(content);

        let mut cfg = String::new();
        for item in &content {
            writeln!(&mut cfg, "content={item}").expect("writing to String cannot fail");
        }
        for item in &pre_ground {
            writeln!(&mut cfg, "groundcover={item}").expect("writing to String cannot fail");
        }
        cfg.push_str("replace=groundcover\n");
        for item in &post_ground {
            writeln!(&mut cfg, "groundcover={item}").expect("writing to String cannot fail");
        }

        let loaded = load_cfg("prop_replace_ground", &cfg);
        prop_assert_eq!(groundcover_values(&loaded), post_ground);
        prop_assert_eq!(content_values(&loaded), content);
    }

    #[test]
    fn prop_replace_fallback_archives_resets_prior_archives_only(
        pre_archives in prop::collection::vec("[A-Za-z0-9_-]{1,24}\\.bsa", 0..12),
        post_archives in prop::collection::vec("[A-Za-z0-9_-]{1,24}\\.bsa", 0..12),
        content in prop::collection::vec("[A-Za-z0-9_-]{1,24}\\.esm", 0..12),
    ) {
        let pre_archives = unique_preserve_order(pre_archives);
        let post_archives = unique_preserve_order(post_archives);
        let content = unique_preserve_order(content);

        let mut cfg = String::new();
        for item in &content {
            writeln!(&mut cfg, "content={item}").expect("writing to String cannot fail");
        }
        for item in &pre_archives {
            writeln!(&mut cfg, "fallback-archive={item}").expect("writing to String cannot fail");
        }
        cfg.push_str("replace=fallback-archives\n");
        for item in &post_archives {
            writeln!(&mut cfg, "fallback-archive={item}").expect("writing to String cannot fail");
        }

        let loaded = load_cfg("prop_replace_archives", &cfg);
        prop_assert_eq!(archive_values(&loaded), post_archives);
        prop_assert_eq!(content_values(&loaded), content);
    }
}
