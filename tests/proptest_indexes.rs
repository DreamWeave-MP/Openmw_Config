mod common;

use common::{temp_dir, write_cfg};
use openmw_config::OpenMWConfiguration;
use proptest::prelude::*;
use std::collections::HashSet;
use std::path::PathBuf;

fn assert_public_consistency(config: &OpenMWConfiguration) {
    let content: HashSet<String> = config
        .content_files_iter()
        .map(|setting| setting.value().clone())
        .collect();
    let groundcover: HashSet<String> = config
        .groundcover_iter()
        .map(|setting| setting.value().clone())
        .collect();
    let archives: HashSet<String> = config
        .fallback_archives_iter()
        .map(|setting| setting.value().clone())
        .collect();
    let data_dirs: HashSet<PathBuf> = config
        .data_directories_iter()
        .map(|setting| setting.parsed().to_path_buf())
        .collect();

    for file in &content {
        assert!(config.has_content_file(file));
    }
    for file in &groundcover {
        assert!(config.has_groundcover_file(file));
    }
    for file in &archives {
        assert!(config.has_archive_file(file));
    }
    for dir in &data_dirs {
        assert!(config.has_data_dir(dir.to_string_lossy().as_ref()));
    }

    let game_settings: Vec<_> = config.game_settings().collect();
    for game_setting in game_settings {
        let key = game_setting.key();
        let from_lookup = config
            .get_game_setting(key)
            .expect("lookup by iterated key");
        assert_eq!(from_lookup.value(), game_setting.value());
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(80))]

    #[test]
    fn prop_mutation_sequences_keep_query_surface_consistent(
        ops in proptest::collection::vec((0u8..10, "[A-Za-z0-9_-]{1,12}", "[A-Za-z0-9_-]{1,12}"), 1..120)
    ) {
        let dir = temp_dir("prop_indexes");
        write_cfg(&dir, "");
        let mut config = OpenMWConfiguration::new(Some(dir)).unwrap();

        for (op, a, b) in ops {
            match op {
                0 => {
                    let _ = config.add_content_file(&format!("{a}.esp"));
                }
                1 => {
                    config.remove_content_file(&format!("{a}.esp"));
                }
                2 => {
                    let _ = config.add_groundcover_file(&format!("{a}.esp"));
                }
                3 => {
                    config.remove_groundcover_file(&format!("{a}.esp"));
                }
                4 => {
                    let _ = config.add_archive_file(&format!("{a}.bsa"));
                }
                5 => {
                    config.remove_archive_file(&format!("{a}.bsa"));
                }
                6 => {
                    config.add_data_directory(PathBuf::from(format!("/tmp/{a}/{b}")).as_path());
                }
                7 => {
                    config.remove_data_directory(&PathBuf::from(format!("/tmp/{a}/{b}")));
                }
                8 => {
                    let _ = config.set_game_settings(Some(vec![
                        format!("i{a},1"),
                        format!("i{a},2"),
                        format!("f{b},1.0"),
                    ]));
                }
                9 => {
                    config.set_content_files(None);
                }
                _ => unreachable!("op out of range"),
            }

            assert_public_consistency(&config);
        }
    }
}
