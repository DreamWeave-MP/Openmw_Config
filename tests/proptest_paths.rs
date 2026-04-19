mod common;

use common::{temp_dir, write_cfg};
use openmw_config::OpenMWConfiguration;
use proptest::prelude::*;

#[cfg(windows)]
proptest! {
    #![proptest_config(ProptestConfig::with_cases(40))]

    #[test]
    fn prop_has_data_dir_separator_equivalence_windows(
        a in "[A-Za-z0-9_-]{1,10}",
        b in "[A-Za-z0-9_-](?:[A-Za-z0-9 _-]{0,8}[A-Za-z0-9_-])?",
    ) {
        let dir = temp_dir("prop_win_sep");
        let canonical = format!("C:/mods/{a}/{b}");
        write_cfg(&dir, &format!("data={canonical}\n"));
        let config = OpenMWConfiguration::new(Some(dir)).unwrap();

        let alt = canonical.replace('/', "\\");
        prop_assert!(config.has_data_dir(&canonical));
        prop_assert!(config.has_data_dir(&alt));
    }

    #[test]
    fn prop_windows_drive_paths_are_rooted(
        a in "[A-Za-z0-9_-]{1,10}",
        b in "[A-Za-z0-9_-]{1,10}",
    ) {
        let dir = temp_dir("prop_win_drive");
        let drive_path = format!("C:/games/{a}/{b}");
        write_cfg(&dir, &format!("data={drive_path}\n"));
        let config = OpenMWConfiguration::new(Some(dir)).unwrap();

        prop_assert!(config.has_data_dir(&drive_path));
    }
}

#[cfg(not(windows))]
proptest! {
    #![proptest_config(ProptestConfig::with_cases(40))]

    #[test]
    fn prop_has_data_dir_exact_absolute_query_unix(
        a in "[A-Za-z0-9_-]{1,10}",
        b in "[A-Za-z0-9_-]{1,10}",
    ) {
        let absolute = format!("/tmp/{a}/{b}");
        let dir = temp_dir("prop_unix_abs");
        write_cfg(&dir, &format!("data={absolute}\n"));
        let config = OpenMWConfiguration::new(Some(dir)).unwrap();
        prop_assert!(config.has_data_dir(&absolute));
    }
}
