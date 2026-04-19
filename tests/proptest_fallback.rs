mod common;

use common::{temp_dir, write_cfg};
use openmw_config::OpenMWConfiguration;
use proptest::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

fn fallback_key_strategy() -> impl Strategy<Value = String> {
    "[A-Za-z][A-Za-z0-9_]{0,31}"
}

fn fallback_value_strategy() -> impl Strategy<Value = String> {
    "[A-Za-z0-9_.:-][A-Za-z0-9 _.,:-]{0,31}"
        .prop_filter("fallback values must be trim-stable", |value| {
            value.trim() == value
        })
}

fn load_cfg(tag: &str, cfg: &str) -> OpenMWConfiguration {
    let dir = temp_dir(tag);
    write_cfg(&dir, cfg);
    OpenMWConfiguration::new(Some(dir)).unwrap()
}

fn snapshot_game_settings(config: &OpenMWConfiguration) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for setting in config.game_settings() {
        map.insert(setting.key().clone(), setting.value().to_string());
    }
    map
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn prop_roundtrip_fallback_last_wins_per_key(
        entries in prop::collection::vec(
            (fallback_key_strategy(), fallback_value_strategy()),
            0..24,
        )
    ) {
        let mut cfg = String::new();
        for (key, value) in &entries {
            writeln!(&mut cfg, "fallback={key},{value}").expect("writing to String cannot fail");
        }

        let loaded = load_cfg("prop_fallback_lastwins_a", &cfg);
        let serialized = loaded.to_string();
        let reparsed = load_cfg("prop_fallback_lastwins_b", &serialized);

        let expected = snapshot_game_settings(&loaded);
        for (key, value) in &expected {
            let got = reparsed
                .get_game_setting(key)
                .expect("expected generated key to exist")
                .value()
                .to_string();
            prop_assert_eq!(got.as_str(), value.as_str(), "last-wins mismatch for key={}", key);
        }
    }

    #[test]
    fn prop_game_settings_iterator_matches_getter(
        entries in prop::collection::vec(
            (fallback_key_strategy(), fallback_value_strategy()),
            0..24,
        )
    ) {
        let mut cfg = String::new();
        for (key, value) in &entries {
            writeln!(&mut cfg, "fallback={key},{value}").expect("writing to String cannot fail");
        }

        let loaded = load_cfg("prop_fallback_consistency", &cfg);
        let mut by_iter = HashMap::new();
        for setting in loaded.game_settings() {
            by_iter.insert(setting.key().clone(), setting.value().to_string());
        }

        let seen_keys: HashSet<String> = entries.into_iter().map(|(k, _)| k).collect();
        let mut by_get = HashMap::new();
        for key in &seen_keys {
            if let Some(setting) = loaded.get_game_setting(key) {
                by_get.insert(key.clone(), setting.value().to_string());
            }
        }

        prop_assert_eq!(by_iter, by_get);
    }

    #[test]
    fn prop_game_settings_emits_unique_keys_only(
        entries in prop::collection::vec(
            (fallback_key_strategy(), fallback_value_strategy()),
            0..24,
        )
    ) {
        let mut cfg = String::new();
        for (key, value) in &entries {
            writeln!(&mut cfg, "fallback={key},{value}").expect("writing to String cannot fail");
        }

        let loaded = load_cfg("prop_fallback_unique", &cfg);
        let keys: Vec<String> = loaded
            .game_settings()
            .map(|setting| setting.key().clone())
            .collect();
        let unique: HashSet<String> = keys.iter().cloned().collect();

        prop_assert_eq!(keys.len(), unique.len());
    }
}
