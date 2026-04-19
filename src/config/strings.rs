// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Dave Corley (S3kshun8)

use std::path::PathBuf;

const SEPARATORS: [char; 2] = ['/', '\\'];

/// Parses a data directory string according to `OpenMW` rules.
/// <https://openmw.readthedocs.io/en/latest/reference/modding/paths.html#openmw-cfg-syntax>
pub fn parse_data_directory<P: AsRef<std::path::Path>>(config_dir: &P, data_dir: &str) -> PathBuf {
    let mut data_dir = data_dir.to_owned();
    // Quote handling
    if data_dir.starts_with('"') {
        let mut result = String::new();
        let mut i = 1;
        let chars: Vec<char> = data_dir.chars().collect();
        while i < chars.len() {
            if chars[i] == '&' {
                i += 1; // skip the next char (escape)
            } else if chars[i] == '"' {
                break;
            }
            if i < chars.len() {
                result.push(chars[i]);
            }
            i += 1;
        }
        data_dir = result;
    }

    // Token replacement
    if data_dir.starts_with("?userdata?") {
        let suffix = data_dir["?userdata?".len()..].trim_start_matches(&SEPARATORS[..]);

        data_dir = crate::default_userdata_path()
            .join(suffix)
            .to_string_lossy()
            .to_string();
    } else if data_dir.starts_with("?userconfig?") {
        let suffix = data_dir["?userconfig?".len()..].trim_start_matches(&SEPARATORS[..]);

        data_dir = crate::default_config_path()
            .join(suffix)
            .to_string_lossy()
            .to_string();
    }

    let data_dir = data_dir.replace(SEPARATORS, std::path::MAIN_SEPARATOR_STR);

    let mut path = PathBuf::from(&data_dir);
    if !path.has_root() {
        path = config_dir.as_ref().join(path);
    }

    path
}
