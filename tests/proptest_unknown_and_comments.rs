mod common;

use common::{temp_dir, write_cfg};
use openmw_config::OpenMWConfiguration;
use proptest::prelude::*;
use std::fmt::Write as _;

const RESERVED_KEYS: &[&str] = &[
    "content",
    "fallback",
    "fallback-archive",
    "groundcover",
    "data",
    "config",
    "replace",
    "encoding",
    "resources",
    "user-data",
    "data-local",
];

fn load_cfg(tag: &str, cfg: &str) -> OpenMWConfiguration {
    let dir = temp_dir(tag);
    write_cfg(&dir, cfg);
    OpenMWConfiguration::new(Some(dir)).unwrap()
}

fn unknown_key_strategy() -> impl Strategy<Value = String> {
    "[a-z][a-z0-9_-]{2,20}".prop_filter("exclude reserved keys", |candidate| {
        !RESERVED_KEYS.contains(&candidate.as_str())
    })
}

fn unknown_value_strategy() -> impl Strategy<Value = String> {
    "[A-Za-z0-9_.:-][A-Za-z0-9 _.,:-]{0,31}"
        .prop_filter("unknown values must be trim-stable", |value| {
            value.trim() == value
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn prop_unknown_key_roundtrip_is_lossless(
        pairs in prop::collection::vec(
            (
                unknown_key_strategy(),
                unknown_value_strategy(),
            ),
            0..20,
        )
    ) {
        let mut cfg = String::new();
        for (k, v) in &pairs {
            writeln!(&mut cfg, "{k}={v}").expect("writing to String cannot fail");
        }

        let loaded = load_cfg("prop_unknown_roundtrip_a", &cfg);
        let serialized = loaded.to_string();
        let reparsed = load_cfg("prop_unknown_roundtrip_b", &serialized);
        let serialized_again = reparsed.to_string();

        for (k, v) in &pairs {
            let line = format!("{k}={v}");
            prop_assert!(serialized.contains(&line), "missing line in first serialization: {line}");
            prop_assert!(
                serialized_again.contains(&line),
                "missing line in second serialization: {line}"
            );
        }
    }

    #[test]
    fn prop_comments_attach_to_next_setting(
        c1 in "[A-Za-z0-9 .,_-]{1,24}",
        c2 in "[A-Za-z0-9 .,_-]{1,24}",
        plugin in "[A-Za-z0-9_-]{1,16}\\.esm",
        archive in "[A-Za-z0-9_-]{1,16}\\.bsa",
    ) {
        let mut cfg = String::new();
        writeln!(&mut cfg, "# {c1}").expect("writing to String cannot fail");
        writeln!(&mut cfg, "content={plugin}").expect("writing to String cannot fail");
        writeln!(&mut cfg, "# {c2}").expect("writing to String cannot fail");
        writeln!(&mut cfg, "fallback-archive={archive}").expect("writing to String cannot fail");

        let loaded = load_cfg("prop_comments_a", &cfg);
        let serialized = loaded.to_string();
        let reparsed = load_cfg("prop_comments_b", &serialized);
        let final_out = reparsed.to_string();

        let expected_a = format!("# {c1}\ncontent={plugin}");
        let expected_b = format!("# {c2}\nfallback-archive={archive}");

        prop_assert!(final_out.contains(&expected_a), "comment-to-content adjacency lost");
        prop_assert!(final_out.contains(&expected_b), "comment-to-archive adjacency lost");
    }
}
