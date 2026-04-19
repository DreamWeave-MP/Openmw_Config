// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Dave Corley (S3kshun8)

use std::path::PathBuf;

const SEPARATORS: [char; 2] = ['/', '\\'];

fn join_token_suffix(base: &std::path::Path, raw: &str, token: &str) -> String {
    let suffix = raw[token.len()..].trim_start_matches(&SEPARATORS[..]);
    base.join(suffix).to_string_lossy().to_string()
}

/// Parses a data directory string according to `OpenMW` rules.
/// <https://openmw.readthedocs.io/en/latest/reference/modding/paths.html#openmw-cfg-syntax>
pub fn parse_data_directory<P: AsRef<std::path::Path>>(config_dir: &P, data_dir: &str) -> PathBuf {
    let mut data_dir = data_dir.to_owned();
    // Quote handling
    if data_dir.starts_with('"') {
        let mut result = String::new();
        let mut escaped = false;

        for ch in data_dir.chars().skip(1) {
            if escaped {
                result.push(ch);
                escaped = false;
                continue;
            }

            if ch == '&' {
                escaped = true;
                continue;
            }

            if ch == '"' {
                break;
            }

            result.push(ch);
        }
        data_dir = result;
    }

    // Token replacement
    if data_dir.starts_with("?userdata?") {
        if let Ok(base) = crate::try_default_userdata_path() {
            data_dir = join_token_suffix(&base, &data_dir, "?userdata?");
        }
    } else if data_dir.starts_with("?userconfig?") {
        if let Ok(base) = crate::try_default_config_path() {
            data_dir = join_token_suffix(&base, &data_dir, "?userconfig?");
        }
    } else if data_dir.starts_with("?local?") {
        if let Ok(base) = crate::try_default_local_path() {
            data_dir = join_token_suffix(&base, &data_dir, "?local?");
        }
    } else if data_dir.starts_with("?global?")
        && let Ok(base) = crate::try_default_global_path()
    {
        data_dir = join_token_suffix(&base, &data_dir, "?global?");
    }

    let data_dir = data_dir.replace(SEPARATORS, std::path::MAIN_SEPARATOR_STR);

    let mut path = PathBuf::from(&data_dir);
    if !path.has_root() {
        path = config_dir.as_ref().join(path);
    }

    path
}
