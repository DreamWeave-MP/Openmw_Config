// This file is part of Openmw_Config.
// Openmw_Config is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// Openmw_Config is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
// You should have received a copy of the GNU General Public License along with Openmw_Config. If not, see <https://www.gnu.org/licenses/>.

use std::path::PathBuf;

const SEPARATORS: [char; 2] = ['/', '\\'];

/// Parses a data directory string according to OpenMW rules.
/// https://openmw.readthedocs.io/en/latest/reference/modding/paths.html#openmw-cfg-syntax
pub fn parse_data_directory<P: AsRef<std::path::Path>>(
    config_dir: &P,
    mut data_dir: String,
) -> PathBuf {
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

    let data_dir = data_dir.replace(SEPARATORS, &std::path::MAIN_SEPARATOR.to_string());

    let mut path = PathBuf::from(&data_dir);
    if !path.is_absolute() {
        path = config_dir.as_ref().join(path);
    }

    path
}
