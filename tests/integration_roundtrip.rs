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

fn load_from_contents(tag: &str, contents: &str) -> OpenMWConfiguration {
    let dir = temp_dir(tag);
    write_cfg(&dir, contents);
    OpenMWConfiguration::new(Some(dir)).unwrap()
}

fn content_values(config: &OpenMWConfiguration) -> Vec<String> {
    config
        .content_files_iter()
        .map(|f| f.value().clone())
        .collect::<Vec<_>>()
}

fn archive_values(config: &OpenMWConfiguration) -> Vec<String> {
    config
        .fallback_archives_iter()
        .map(|f| f.value().clone())
        .collect::<Vec<_>>()
}

fn groundcover_values(config: &OpenMWConfiguration) -> Vec<String> {
    config
        .groundcover_iter()
        .map(|f| f.value().clone())
        .collect::<Vec<_>>()
}

fn data_values(config: &OpenMWConfiguration) -> Vec<String> {
    config
        .data_directories_iter()
        .map(|d| d.original().clone())
        .collect::<Vec<_>>()
}

#[test]
fn test_roundtrip_complex_config_semantics_stable() {
    let cfg = "# root comment\ncontent=Morrowind.esm\nfallback=iTimescale,30\nfallback-archive=Morrowind.bsa\ngroundcover=Grass.esp\ndata=Data Files\nunknown-key=unknown-value\n";

    let config = load_from_contents("roundtrip_complex_a", cfg);
    let serialized = config.to_string();
    let reparsed = load_from_contents("roundtrip_complex_b", &serialized);

    assert_eq!(content_values(&config), content_values(&reparsed));
    assert_eq!(archive_values(&config), archive_values(&reparsed));
    assert_eq!(groundcover_values(&config), groundcover_values(&reparsed));
    assert_eq!(data_values(&config), data_values(&reparsed));
    assert_eq!(
        config.get_game_setting("iTimescale").unwrap().value(),
        reparsed.get_game_setting("iTimescale").unwrap().value()
    );
    assert!(serialized.contains("unknown-key=unknown-value"));
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn prop_roundtrip_preserves_observable_collections(
        content in prop::collection::vec("[A-Za-z]{1,8}\\.esm", 0..8),
        archives in prop::collection::vec("[A-Za-z]{1,8}\\.bsa", 0..8),
        ground in prop::collection::vec("[A-Za-z]{1,8}\\.esp", 0..8),
        data in prop::collection::vec("[A-Za-z]{1,8}", 0..6),
    ) {
        let content = unique_preserve_order(content);
        let archives = unique_preserve_order(archives);
        let ground = unique_preserve_order(ground);
        let data = unique_preserve_order(data);

        let mut cfg = String::new();
        for item in &content {
            writeln!(&mut cfg, "content={item}").expect("writing to String cannot fail");
        }
        for item in &archives {
            writeln!(&mut cfg, "fallback-archive={item}")
                .expect("writing to String cannot fail");
        }
        for item in &ground {
            writeln!(&mut cfg, "groundcover={item}").expect("writing to String cannot fail");
        }
        for item in &data {
            writeln!(&mut cfg, "data={item}").expect("writing to String cannot fail");
        }

        let loaded = load_from_contents("roundtrip_prop_a", &cfg);
        let serialized = loaded.to_string();
        let reparsed = load_from_contents("roundtrip_prop_b", &serialized);

        prop_assert_eq!(content_values(&loaded), content_values(&reparsed));
        prop_assert_eq!(archive_values(&loaded), archive_values(&reparsed));
        prop_assert_eq!(groundcover_values(&loaded), groundcover_values(&reparsed));
        prop_assert_eq!(data_values(&loaded), data_values(&reparsed));
    }
}
