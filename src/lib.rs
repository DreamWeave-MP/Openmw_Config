// This file is part of Openmw_Config.
// Openmw_Config is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// Openmw_Config is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
// You should have received a copy of the GNU General Public License along with Openmw_Config. If not, see <https://www.gnu.org/licenses/>.

mod config;
pub use config::{
    directorysetting::DirectorySetting,
    encodingsetting::{EncodingSetting, EncodingType},
    error::ConfigError,
    filesetting::FileSetting,
    gamesetting::GameSettingType,
    genericsetting::GenericSetting,
    OpenMWConfiguration,
};

pub(crate) trait GameSetting: std::fmt::Display {
    fn meta(&self) -> &GameSettingMeta;
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GameSettingMeta {
    source_config: std::path::PathBuf,
    comment: String,
}

const NO_CONFIG_DIR: &str = "FAILURE: COULD NOT READ CONFIG DIRECTORY";

/// Path to input bindings and core configuration
/// These functions are not expected to fail and should they fail, indicate either:
/// a severe issue with the system
/// or that an unsupported system is being used.
pub fn default_config_path() -> std::path::PathBuf {
    #[cfg(target_os = "android")]
    return std::path::PathBuf::from("/storage/emulated/0/Alpha3/config");

    #[cfg(not(target_os = "android"))]
    if cfg!(windows) {
        dirs::document_dir()
            .expect(NO_CONFIG_DIR)
            .join("My Games")
            .join("openmw")
    } else {
        dirs::preference_dir().expect(NO_CONFIG_DIR).join("openmw")
    }
}

/// Path to save storage, screenshots, navmeshdb, and data-local
/// These functions are not expected to fail and should they fail, indicate either:
/// a severe issue with the system
/// or that an unsupported system is being used.
pub fn default_userdata_path() -> std::path::PathBuf {
    #[cfg(target_os = "android")]
    return std::path::PathBuf::from("/storage/emulated/0/Alpha3");

    #[cfg(not(target_os = "android"))]
    if cfg!(windows) {
        default_config_path()
    } else {
        dirs::data_dir()
            .expect("FAILURE: COULD NOT READ USERDATA DIRECTORY")
            .join("openmw")
    }
}

/// Path to the last-loading directory of openmw.cfg,
/// As defined by the engine's defaults
/// This directory will override all others in the load order
pub fn default_data_local_path() -> std::path::PathBuf {
    default_userdata_path().join("data")
}
