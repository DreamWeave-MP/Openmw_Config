// This file is part of Openmw_Config.
// Openmw_Config is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// Openmw_Config is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
// You should have received a copy of the GNU General Public License along with Openmw_Config. If not, see <https://www.gnu.org/licenses/>.

use std::{
    fmt::{self, Display},
    fs::{OpenOptions, create_dir_all, metadata, read_to_string},
    path::{Path, PathBuf},
};

use crate::{ConfigError, GameSetting, bail_config};
use std::collections::HashSet;

pub mod directorysetting;
use directorysetting::DirectorySetting;

pub mod filesetting;
use filesetting::FileSetting;

pub mod gamesetting;
use gamesetting::GameSettingType;

pub mod genericsetting;
use genericsetting::GenericSetting;

pub mod encodingsetting;
use encodingsetting::EncodingSetting;

#[macro_use]
pub mod error;
#[macro_use]
mod singletonsetting;
mod strings;
mod util;

#[derive(Clone, Debug)]
pub enum SettingValue {
    DataDirectory(DirectorySetting),
    GameSetting(GameSettingType),
    UserData(DirectorySetting),
    DataLocal(DirectorySetting),
    Resources(DirectorySetting),
    Encoding(EncodingSetting),
    SubConfiguration(DirectorySetting),
    Generic(GenericSetting),
    ContentFile(FileSetting),
    BethArchive(FileSetting),
    Groundcover(FileSetting),
}

impl Display for SettingValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = match self {
            SettingValue::Encoding(encoding_setting) => encoding_setting.to_string(),
            SettingValue::UserData(userdata_setting) => format!(
                "{}user-data={}",
                userdata_setting.meta().comment,
                userdata_setting.original()
            ),
            SettingValue::DataLocal(data_local_setting) => format!(
                "{}data-local={}",
                data_local_setting.meta().comment,
                data_local_setting.original(),
            ),
            SettingValue::Resources(resources_setting) => format!(
                "{}resources={}",
                resources_setting.meta().comment,
                resources_setting.original()
            ),
            SettingValue::GameSetting(game_setting) => game_setting.to_string(),
            SettingValue::DataDirectory(data_directory) => format!(
                "{}data={}",
                data_directory.meta().comment,
                data_directory.original()
            ),
            SettingValue::SubConfiguration(sub_config) => format!(
                "{}config={}",
                sub_config.meta().comment,
                sub_config.original()
            ),
            SettingValue::Generic(generic) => generic.to_string(),
            SettingValue::ContentFile(plugin) => {
                format!("{}content={}", plugin.meta().comment, plugin.value(),)
            }
            SettingValue::BethArchive(archive) => {
                format!(
                    "{}fallback-archive={}",
                    archive.meta().comment,
                    archive.value(),
                )
            }
            SettingValue::Groundcover(grass) => {
                format!("{}groundcover={}", grass.meta().comment, grass.value())
            }
        };

        writeln!(f, "{str}")
    }
}

impl From<GameSettingType> for SettingValue {
    fn from(g: GameSettingType) -> Self {
        SettingValue::GameSetting(g)
    }
}

impl From<DirectorySetting> for SettingValue {
    fn from(d: DirectorySetting) -> Self {
        SettingValue::DataDirectory(d)
    }
}

impl SettingValue {
    pub fn meta(&self) -> &crate::GameSettingMeta {
        match self {
            SettingValue::BethArchive(setting) => setting.meta(),
            SettingValue::Groundcover(setting) => setting.meta(),
            SettingValue::UserData(setting) => setting.meta(),
            SettingValue::DataLocal(setting) => setting.meta(),
            SettingValue::DataDirectory(setting) => setting.meta(),
            SettingValue::ContentFile(setting) => setting.meta(),
            SettingValue::GameSetting(setting) => setting.meta(),
            SettingValue::Resources(setting) => setting.meta(),
            SettingValue::SubConfiguration(setting) => setting.meta(),
            SettingValue::Encoding(setting) => setting.meta(),
            SettingValue::Generic(setting) => setting.meta(),
        }
    }
}

macro_rules! insert_dir_setting {
    ($self:ident, $variant:ident, $value:expr, $config_file:expr, $comment:expr) => {{
        $self
            .settings
            .push(SettingValue::$variant(DirectorySetting::new(
                $value,
                $config_file,
                $comment,
            )));
    }};
}

/// Core struct representing the composed OpenMW configuration,
/// After it has been fully resolved.
#[derive(Debug, Default)]
pub struct OpenMWConfiguration {
    root_config: PathBuf,
    settings: Vec<SettingValue>,
}

impl OpenMWConfiguration {
    pub fn from_env() -> Result<Self, ConfigError> {
        if let Ok(explicit_path) = std::env::var("OPENMW_CONFIG") {
            let explicit_path: PathBuf = shellexpand::tilde(&explicit_path).into_owned().into();

            if explicit_path.as_os_str().is_empty() {
                return Err(ConfigError::NotFileOrDirectory(explicit_path));
            } else if explicit_path.is_absolute() {
                return Self::new(Some(explicit_path));
            } else if explicit_path.is_relative() {
                return Self::new(Some(std::fs::canonicalize(explicit_path)?));
            } else {
                return Err(ConfigError::NotFileOrDirectory(explicit_path));
            }
        } else if let Ok(path_list) = std::env::var("OPENMW_CONFIG_DIR") {
            let path_list = if cfg!(windows) {
                path_list.split(';')
            } else {
                path_list.split(':')
            };

            for dir in path_list {
                let dir: PathBuf = shellexpand::tilde(&dir).into_owned().into();

                if dir.join("openmw.cfg").exists() {
                    return Self::new(Some(dir));
                }
            }
        }

        Self::new(None)
    }

    pub fn new(path: Option<PathBuf>) -> Result<Self, ConfigError> {
        let mut config = OpenMWConfiguration::default();
        let root_config = match path {
            Some(path) => match util::input_config_path(path) {
                Err(error) => return Err(error),
                Ok(validated_path) => validated_path,
            },
            None => crate::default_config_path().join("openmw.cfg"),
        };

        config.root_config = root_config;

        match config.load(&config.root_config.to_owned()) {
            Err(error) => Err(error),
            Ok(_) => {
                if let Some(dir) = config.data_local() {
                    let path = dir.parsed();

                    let path_meta = metadata(path);
                    if path_meta.is_err() {
                        if let Err(error) = create_dir_all(path) {
                            util::debug_log(format!(
                                "WARNING: Attempted to crete a data-local directory at {path:?}, but failed: {error}"
                            ))
                        };
                    }

                    config
                        .settings
                        .push(SettingValue::DataDirectory(dir.clone()));
                }

                if let Some(setting) = config.resources() {
                    let dir = setting.parsed();

                    let engine_vfs = DirectorySetting::new(
                        dir.join("vfs").to_string_lossy().to_string(),
                        setting.meta.source_config.to_path_buf(),
                        &mut setting.meta.comment.to_string(),
                    );

                    config
                        .settings
                        .insert(0, SettingValue::DataDirectory(engine_vfs));
                }

                util::debug_log(format!("{:#?}", config.settings));

                Ok(config)
            }
        }
    }

    /// Path to the configuration file which is the root of the configuration chain
    /// Typically, this will be whatever is defined in the `Paths` documentation for the appropriate platform:
    /// https://openmw.readthedocs.io/en/latest/reference/modding/paths.html#configuration-files-and-log-files
    pub fn root_config_file(&self) -> &PathBuf {
        &self.root_config
    }

    /// Same as root_config_file, but returns the directory it's in.
    /// Useful for reading other configuration files, or if assuming openmw.cfg
    /// Is always *called* openmw.cfg (which it should be)
    pub fn root_config_dir(&self) -> PathBuf {
        self.root_config.parent().expect("root_config has no parent directory").to_path_buf()
    }

    pub fn is_user_config(&self) -> bool {
        self.root_config_dir() == self.user_config_path()
    }

    pub fn user_config(self) -> Result<Self, ConfigError> {
        if self.is_user_config() {
            Ok(self)
        } else {
            Self::new(Some(self.user_config_path()))
        }
    }

    /// In order of priority, the list of all openmw.cfg files which were loaded by the configuration chain after the root.
    /// If the root openmw.cfg is different than the user one, this list will contain the user openmw.cfg as its last element.
    /// If the root and user openmw.cfg are the *same*, then this list will be empty and the root config should be considered the user config.
    /// Otherwise, if one wishes to get the contents of the user configuration specifically, construct a new OpenMWConfiguration from the last sub_config.
    ///
    /// Openmw.cfg files are added in order of the sequence in which they are defined by one openmw.cfg, and then each of *those* openmw.cfg files
    /// is then processed in their entirety, sequentially, after the first one has resolved.
    /// The highest-priority openmw.cfg loaded (the last one!) is considered the user openmw.cfg,
    /// and will be the one which is modifiable by OpenMW-Launcher and OpenMW proper.
    ///
    /// See https://openmw.readthedocs.io/en/latest/reference/modding/paths.html#configuration-sources for examples and further explanation of multiple config sources.

    /// Path to the highest-level configuration *directory*
    pub fn user_config_path(&self) -> PathBuf {
        self.sub_configs()
            .map(|setting| setting.parsed().clone())
            .last()
            .unwrap_or_else(|| self.root_config_dir())
    }

    impl_singleton_setting! {
        UserData => {
            get: userdata,
            set: set_userdata,
            in_type: DirectorySetting
        },
        Resources => {
            get: resources,
            set: set_resources,
            in_type: DirectorySetting
        },
        DataLocal => {
            get: data_local,
            set: set_data_local,
            in_type: DirectorySetting
        },
        Encoding => {
            get: encoding,
            set: set_encoding,
            in_type: EncodingSetting
        }
    }

    /// Content files are the actual *mods* or plugins which are created by either OpenCS or Bethesda's construction set
    /// These entries only refer to the names and ordering of content files.
    /// vfstool-lib should be used to derive paths
    pub fn content_files(&self) -> Vec<&String> {
        self.content_files_iter()
            .map(|setting| setting.value())
            .collect()
    }

    pub fn content_files_iter(&self) -> impl Iterator<Item = &FileSetting> {
        self.settings.iter().filter_map(|setting| match setting {
            SettingValue::ContentFile(plugin) => Some(plugin),
            _ => None,
        })
    }

    pub fn has_content_file(&self, file_name: &str) -> bool {
        self.settings.iter().any(|setting| match setting {
            SettingValue::ContentFile(plugin) => plugin == file_name,
            _ => false,
        })
    }

    pub fn has_groundcover_file(&self, file_name: &str) -> bool {
        self.settings.iter().any(|setting| match setting {
            SettingValue::Groundcover(plugin) => plugin == file_name,
            _ => false,
        })
    }

    pub fn has_archive_file(&self, file_name: &str) -> bool {
        self.settings.iter().any(|setting| match setting {
            SettingValue::BethArchive(archive) => archive == file_name,
            _ => false,
        })
    }

    pub fn has_data_dir(&self, file_name: &str) -> bool {
        self.settings.iter().any(|setting| match setting {
            SettingValue::DataDirectory(data_dir) => {
                data_dir.parsed().to_string_lossy() == file_name
            }
            _ => false,
        })
    }

    pub fn add_content_file(&mut self, content_file: &str) -> Result<(), ConfigError> {
        let duplicate = self.settings.iter().find_map(|setting| match setting {
            SettingValue::ContentFile(plugin) => {
                if plugin.value() == content_file {
                    Some(plugin)
                } else {
                    None
                }
            }
            _ => None,
        });

        if let Some(duplicate) = duplicate {
            bail_config!(
                content_already_defined,
                duplicate.value().to_owned(),
                duplicate.meta().source_config
            )
        };

        self.settings
            .push(SettingValue::ContentFile(FileSetting::new(
                content_file,
                &self.user_config_path().join("openmw.cfg"),
                &mut String::default(),
            )));

        Ok(())
    }

    pub fn groundcover(&self) -> Vec<&String> {
        self.groundcover_iter()
            .map(|setting| setting.value())
            .collect()
    }

    pub fn groundcover_iter(&self) -> impl Iterator<Item = &FileSetting> {
        self.settings.iter().filter_map(|setting| match setting {
            SettingValue::Groundcover(grass) => Some(grass),
            _ => None,
        })
    }

    pub fn add_groundcover_file(&mut self, content_file: &str) -> Result<(), ConfigError> {
        let duplicate = self.settings.iter().find_map(|setting| match setting {
            SettingValue::Groundcover(plugin) => {
                if plugin.value() == content_file {
                    Some(plugin)
                } else {
                    None
                }
            }
            _ => None,
        });

        if let Some(duplicate) = duplicate {
            bail_config!(
                groundcover_already_defined,
                duplicate.value().to_owned(),
                duplicate.meta().source_config
            )
        };

        self.settings
            .push(SettingValue::Groundcover(FileSetting::new(
                content_file,
                &self.user_config_path().join("openmw.cfg"),
                &mut String::default(),
            )));

        Ok(())
    }

    pub fn remove_content_file(&mut self, file_name: &str) {
        self.clear_matching(|setting| match setting {
            SettingValue::ContentFile(existing_file) => existing_file == file_name,
            _ => false,
        });
    }

    pub fn remove_groundcover_file(&mut self, file_name: &str) {
        self.clear_matching(|setting| match setting {
            SettingValue::Groundcover(existing_file) => existing_file == file_name,
            _ => false,
        });
    }

    pub fn remove_archive_file(&mut self, file_name: &str) {
        self.clear_matching(|setting| match setting {
            SettingValue::BethArchive(existing_file) => existing_file == file_name,
            _ => false,
        });
    }

    /// Removed any path matching either the relativized original version in openmw.cfg or
    /// the fully resolved absolute version the config itself relies on
    pub fn remove_data_directory(&mut self, data_dir: &PathBuf) {
        self.clear_matching(|setting| match setting {
            SettingValue::DataDirectory(existing_data_dir) => {
                existing_data_dir.parsed() == data_dir
                    || existing_data_dir.original() == &data_dir.to_string_lossy().to_string()
            }
            _ => false,
        });
    }

    /// Does not validate duplicate data directories
    /// Jest don't feel like it atm
    /// Let's add comments later after we're not super burned out on this whole config thing
    pub fn add_data_directory(&mut self, dir: PathBuf) {
        self.settings
            .push(SettingValue::DataDirectory(DirectorySetting::new(
                dir.to_string_lossy(),
                self.user_config_path().join("openmw.cfg"),
                &mut String::default(),
            )))
    }

    pub fn add_archive_file(&mut self, archive_file: &str) -> Result<(), ConfigError> {
        let duplicate = self.settings.iter().find_map(|setting| match setting {
            SettingValue::BethArchive(archive) => {
                if archive.value() == archive_file {
                    Some(archive)
                } else {
                    None
                }
            }
            _ => None,
        });

        if let Some(duplicate) = duplicate {
            bail_config!(
                duplicate_archive_file,
                duplicate.value().to_owned(),
                duplicate.meta().source_config
            )
        };

        self.settings
            .push(SettingValue::BethArchive(FileSetting::new(
                archive_file,
                &self.user_config_path().join("openmw.cfg"),
                &mut String::default(),
            )));

        Ok(())
    }

    pub fn fallback_archives(&self) -> Vec<&String> {
        self.fallback_archives_iter()
            .map(|setting| setting.value())
            .collect()
    }

    pub fn fallback_archives_iter(&self) -> impl Iterator<Item = &FileSetting> {
        self.settings.iter().filter_map(|setting| match setting {
            SettingValue::BethArchive(archive) => Some(archive),
            _ => None,
        })
    }

    /// This early iteration of the crate provides no input validation for setter functions.
    pub fn set_content_files(&mut self, plugins: Option<Vec<String>>) {
        self.clear_matching(|setting| matches!(setting, SettingValue::ContentFile(_)));

        if let Some(plugins) = plugins {
            let cfg_path = self.user_config_path().join("openmw.cfg");
            let mut empty = String::default();
            plugins.into_iter().for_each(|plugin| {
                self.settings
                    .push(SettingValue::ContentFile(FileSetting::new(
                        &plugin,
                        &cfg_path,
                        &mut empty,
                    )))
            })
        }
    }

    pub fn set_fallback_archives(&mut self, archives: Option<Vec<String>>) {
        self.clear_matching(|setting| matches!(setting, SettingValue::BethArchive(_)));

        if let Some(archives) = archives {
            let cfg_path = self.user_config_path().join("openmw.cfg");
            let mut empty = String::default();
            archives.into_iter().for_each(|archive| {
                self.settings
                    .push(SettingValue::BethArchive(FileSetting::new(
                        &archive,
                        &cfg_path,
                        &mut empty,
                    )))
            })
        }
    }

    pub fn settings_matching<'a, P>(
        &'a self,
        predicate: P,
    ) -> impl Iterator<Item = &'a SettingValue>
    where
        P: Fn(&SettingValue) -> bool + 'a,
    {
        self.settings.iter().filter(move |s| predicate(*s))
    }

    pub fn clear_matching<P>(&mut self, predicate: P)
    where
        P: Fn(&SettingValue) -> bool,
    {
        self.settings.retain(|s| !predicate(s));
    }

    /// This early iteration of the crate provides no input validation for setter functions.
    pub fn set_data_directories(&mut self, dirs: Option<Vec<PathBuf>>) {
        self.clear_matching(|setting| matches!(setting, SettingValue::DataDirectory(_)));

        if let Some(dirs) = dirs {
            let cfg_path = self.user_config_path().join("openmw.cfg");
            let mut empty = String::default();

            dirs.into_iter().for_each(|dir| {
                self.settings
                    .push(SettingValue::DataDirectory(DirectorySetting::new(
                        dir.to_string_lossy(),
                        cfg_path.clone(),
                        &mut empty,
                    )))
            })
        }
    }

    /// Given a string resembling a fallback= entry's value, as it would exist in openmw.cfg,
    /// Add it to the settings map.
    /// This process must be non-destructive
    pub fn set_game_setting(
        &mut self,
        base_value: &str,
        config_path: Option<PathBuf>,
        comment: &mut String,
    ) -> Result<(), ConfigError> {
        let new_setting = GameSettingType::try_from((
            base_value.to_owned(),
            config_path.unwrap_or_else(|| self.user_config_path().join("openmw.cfg")),
            comment,
        ))?;

        self.settings.push(SettingValue::GameSetting(new_setting));

        Ok(())
    }

    /// This early iteration of the crate provides no input validation for setter functions.
    pub fn set_game_settings(&mut self, settings: Option<Vec<String>>) -> Result<(), ConfigError> {
        self.clear_matching(|setting| matches!(setting, SettingValue::GameSetting(_)));

        if let Some(settings) = settings {
            let cfg_path = self.user_config_path().join("openmw.cfg");
            let mut empty = String::default();

            settings.into_iter().try_for_each(|setting| {
                self.settings
                    .push(SettingValue::GameSetting(GameSettingType::try_from((
                        setting,
                        cfg_path.clone(),
                        &mut empty,
                    ))?));

                Ok::<(), ConfigError>(())
            })?
        }

        Ok(())
    }

    pub fn sub_configs(&self) -> impl Iterator<Item = &DirectorySetting> {
        self.settings.iter().filter_map(|setting| match setting {
            SettingValue::SubConfiguration(subconfig) => Some(subconfig),
            _ => None,
        })
    }

    /// Fallback entries are k/v pairs baked into the value side of k/v pairs in `fallback=` entries of openmw.cfg
    /// They are used to express settings which are defined in Morrowind.ini for things such as:
    /// weather, lighting behaviors, UI Colors, and levelup messages
    pub fn game_settings(&self) -> impl Iterator<Item = &GameSettingType> {
        let mut unique_settings = Vec::new();
        let mut seen = HashSet::new();

        for setting in self.settings.iter().rev() {
            if let SettingValue::GameSetting(gs) = setting {
                if seen.insert(gs.to_string()) {
                    unique_settings.push(gs);
                }
            }
        }

        unique_settings.into_iter()
    }

    /// Retrieves a gamesetting according to its name.
    /// This would be whatever text comes after the equals sign `=` and before the first comma `,`
    /// Case-sensitive!
    pub fn get_game_setting(&self, key: &str) -> Option<&GameSettingType> {
        for setting in self.settings.iter().rev() {
            match setting {
                SettingValue::GameSetting(setting) => {
                    if setting == &key {
                        return Some(setting);
                    }
                }
                _ => continue,
            }
        }
        None
    }

    /// Data directories are the bulk of an OpenMW Configuration's contents,
    /// Composing the list of files from which a VFS is constructed.
    /// For a VFS implementation, see: https://github.com/magicaldave/vfstool/tree/main/vfstool_lib
    ///
    /// Calling this function will give the post-parsed versions of directories defined by an openmw.cfg,
    /// So the real ones may easily be iterated and loaded.
    /// There is not actually validation anywhere in the crate that DirectorySettings refer to a directory which actually exists.
    /// This is according to the openmw.cfg specification and doesn't technically break anything but should be considered when using these paths.
    pub fn data_directories(&self) -> Vec<&PathBuf> {
        self.data_directories_iter()
            .map(|setting| setting.parsed())
            .collect()
    }

    pub fn data_directories_iter(&self) -> impl Iterator<Item = &DirectorySetting> {
        self.settings.iter().filter_map(|setting| match setting {
            SettingValue::DataDirectory(data_dir) => Some(data_dir),
            _ => None,
        })
    }

    fn load(&mut self, config_dir: &Path) -> Result<(), ConfigError> {
        util::debug_log(format!("BEGIN CONFIG PARSING: {config_dir:?}"));

        if !config_dir.exists() {
            bail_config!(cannot_find, config_dir);
        }

        let cfg_file_path = match config_dir.is_dir() {
            true => config_dir.join("openmw.cfg"),
            false => config_dir.to_path_buf(),
        };

        let lines = read_to_string(&cfg_file_path)?;

        let mut queued_comment = String::new();
        let mut sub_configs: Vec<(String, String)> = Vec::new();

        let mut seen_content: HashSet<String> = HashSet::new();
        let mut seen_groundcover: HashSet<String> = HashSet::new();
        let mut seen_archives: HashSet<String> = HashSet::new();

        for setting in self.settings.iter() {
            match setting {
                SettingValue::ContentFile(f) => { seen_content.insert(f.value().clone()); }
                SettingValue::Groundcover(f) => { seen_groundcover.insert(f.value().clone()); }
                SettingValue::BethArchive(f) => { seen_archives.insert(f.value().clone()); }
                _ => {}
            }
        }

        for line in lines.lines() {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                queued_comment.push('\n');
                continue;
            } else if trimmed.starts_with('#') {
                queued_comment.push_str(line);
                queued_comment.push('\n');
                continue;
            }

            let tokens: Vec<&str> = trimmed.splitn(2, '=').collect();
            if tokens.len() < 2 {
                bail_config!(invalid_line, trimmed.into(), config_dir.to_path_buf());
            }

            let key = tokens[0].trim();
            let value = tokens[1].trim().to_string();

            match key {
                "content" => {
                    if !seen_content.insert(value.clone()) {
                        bail_config!(duplicate_content_file, value, config_dir);
                    }
                    self.settings
                        .push(SettingValue::ContentFile(FileSetting::new(
                            &value,
                            &config_dir,
                            &mut queued_comment,
                        )));
                }
                "groundcover" => {
                    if !seen_groundcover.insert(value.clone()) {
                        bail_config!(duplicate_groundcover_file, value, config_dir);
                    }
                    self.settings
                        .push(SettingValue::Groundcover(FileSetting::new(
                            &value,
                            &config_dir,
                            &mut queued_comment,
                        )));
                }
                "fallback-archive" => {
                    if !seen_archives.insert(value.clone()) {
                        bail_config!(duplicate_archive_file, value, config_dir);
                    }
                    self.settings
                        .push(SettingValue::BethArchive(FileSetting::new(
                            &value,
                            &config_dir,
                            &mut queued_comment,
                        )));
                }
                "fallback" => {
                    self.set_game_setting(
                        &value,
                        Some(config_dir.to_owned()),
                        &mut queued_comment,
                    )?;
                }
                "encoding" => self.set_encoding(Some(EncodingSetting::try_from((
                    value,
                    config_dir,
                    &mut queued_comment,
                ))?)),
                "config" => {
                    sub_configs.push((value, std::mem::take(&mut queued_comment)));
                }
                "data" => {
                    insert_dir_setting!(
                        self,
                        DataDirectory,
                        &value,
                        (config_dir).to_path_buf(),
                        &mut queued_comment
                    )
                }
                "resources" => {
                    insert_dir_setting!(
                        self,
                        Resources,
                        &value,
                        (config_dir).to_path_buf(),
                        &mut queued_comment
                    )
                }
                "user-data" => {
                    insert_dir_setting!(
                        self,
                        UserData,
                        &value,
                        (config_dir).to_path_buf(),
                        &mut queued_comment
                    )
                }
                "data-local" => {
                    insert_dir_setting!(
                        self,
                        DataLocal,
                        &value,
                        (config_dir).to_path_buf(),
                        &mut queued_comment
                    )
                }
                "replace" => match value.to_lowercase().as_str() {
                    "content" => { self.set_content_files(None); seen_content.clear(); }
                    "data" => self.set_data_directories(None),
                    "fallback" => self.set_game_settings(None)?,
                    "fallback-archives" => { self.set_fallback_archives(None); seen_archives.clear(); }
                    "groundcover" => { self.clear_matching(|s| matches!(s, SettingValue::Groundcover(_))); seen_groundcover.clear(); }
                    "data-local" => self.set_data_local(None),
                    "resources" => self.set_resources(None),
                    "user-data" => self.set_userdata(None),
                    "config" => {
                        self.settings.clear();
                        seen_content.clear();
                        seen_groundcover.clear();
                        seen_archives.clear();
                    }
                    _ => {}
                },
                _ => {
                    let setting = GenericSetting::new(key, &value, config_dir, &mut queued_comment);
                    self.settings.push(SettingValue::Generic(setting));
                }
            }
        }

        sub_configs.into_iter().try_for_each(
            |(subconfig_path, mut subconfig_comment): (String, String)| {
                let mut comment = std::mem::take(&mut subconfig_comment);

                let setting: DirectorySetting = DirectorySetting::new(subconfig_path.clone(), config_dir.to_path_buf(), &mut comment);
                let subconfig_path = setting.parsed().join("openmw.cfg");

                if std::fs::metadata(&subconfig_path).is_ok() {
                    self.settings.push(SettingValue::SubConfiguration(setting));
                    self.load(Path::new(&subconfig_path))
                } else {
                    util::debug_log(format!(
                        "Skipping parsing of {} As this directory does not actually contain an openmw.cfg!",
                        config_dir.display(),
                    ));

                    Ok(())
                }
            },
        )?;

        Ok(())
    }

    fn write_config<P: AsRef<Path> + std::fmt::Debug>(
        &self,
        config_string: String,
        path: &P,
    ) -> Result<(), String> {
        use std::io::Write;

        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&path)
            .map_err(|e| format!("Failed to open {:?} for writing: {}", path, e))?;

        file.write_all(config_string.as_bytes())
            .map_err(|e| format!("Failed to write to {:?}: {}", path, e))?;

        Ok(())
    }

    /// Saves the currently-defined user openmw.cfg configuration
    /// It should be noted that while modifications may be performed at runtime,
    /// Because of how *extensive* those modifications to a given configuration may *be*, it's more or less impossible to
    /// guarantee that saving any lower priority openmw.cfg will not *completely* destroy it.
    /// You've been warned!
    pub fn save_user(&self) -> Result<(), String> {
        let target_dir = self.user_config_path();

        // Write the config to openmw.cfg in the target directory
        let cfg_path = target_dir.join("openmw.cfg");

        if !util::is_writable(&cfg_path) {
            return Err(format!("Target path {:?} is not writable!", target_dir));
        }

        let mut user_settings_string = String::new();

        self.settings_matching(|setting| setting.meta().source_config == cfg_path)
            .for_each(|user_setting| user_settings_string.push_str(&user_setting.to_string()));

        self.write_config(user_settings_string, &cfg_path)?;

        Ok(())
    }

    /// Save the openmw.cfg to an arbitrary path, instead of the (safe) user configuration.
    /// This doesn't prevent bad usages of the configuration such as overriding an existing one with the original root configuration,
    /// So you should exercise caution when writing an openmw.cfg and be very sure you know it is going where you think it is.
    pub fn save_subconfig(&self, target_dir: PathBuf) -> Result<(), String> {
        let subconfig_is_loaded = self.settings.iter().any(|setting| match setting {
            SettingValue::SubConfiguration(subconfig) => {
                subconfig.parsed() == &target_dir
                    || subconfig.original() == &target_dir.to_string_lossy().to_string()
            }
            _ => false,
        });

        if !subconfig_is_loaded {
            return Err(format!(
                "Refusing to save a sub-configuration which is not actually loaded as a child of the current one: {}",
                target_dir.display()
            ));
        }

        let cfg_path = target_dir.join("openmw.cfg");

        if !util::is_writable(&cfg_path) {
            return Err(format!("Target path {:?} is not writable!", target_dir));
        }

        let mut subconfig_settings_string = String::new();

        self.settings_matching(|setting| setting.meta().source_config == cfg_path)
            .for_each(|subconfig_setting| {
                subconfig_settings_string.push_str(&subconfig_setting.to_string())
            });

        self.write_config(subconfig_settings_string, &cfg_path)?;

        Ok(())
    }
}

/// Keep in mind this is *not* meant to be used as a mechanism to write the openmw.cfg contents.
/// Since the openmw.cfg is a merged entity, it is impossible to distinguish the origin of one particular data directory
/// Or content file once it has been applied - this is doubly true for entries which may only exist once in openmw.cfg.
/// Thus, what this method provides is the composite configuration.
///
/// It may be safely used to write an openmw.cfg as all directories will be absolutized upon loading the config.
///
/// Token information is also lost when a config file is processed.
/// It is not necessarily recommended to write a configuration file which loads other ones or uses tokens for this reason.
///
/// Comments are also preserved.
impl fmt::Display for OpenMWConfiguration {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.settings
            .iter()
            .try_for_each(|setting| write!(f, "{}", setting))?;

        writeln!(
            f,
            "# OpenMW-Config Serializer Version: {}",
            env!("CARGO_PKG_VERSION")
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    // -----------------------------------------------------------------------
    // Helpers
    // -----------------------------------------------------------------------

    fn write_cfg(dir: &std::path::Path, contents: &str) -> PathBuf {
        let cfg = dir.join("openmw.cfg");
        let mut f = std::fs::File::create(&cfg).unwrap();
        f.write_all(contents.as_bytes()).unwrap();
        cfg
    }

    fn temp_dir() -> PathBuf {
        let base = std::env::temp_dir().join(format!(
            "openmw_cfg_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .subsec_nanos()
        ));
        std::fs::create_dir_all(&base).unwrap();
        base
    }

    fn load(cfg_contents: &str) -> OpenMWConfiguration {
        let dir = temp_dir();
        write_cfg(&dir, cfg_contents);
        OpenMWConfiguration::new(Some(dir)).unwrap()
    }

    // -----------------------------------------------------------------------
    // Content files
    // -----------------------------------------------------------------------

    #[test]
    fn test_content_files_empty_on_bare_config() {
        let config = load("");
        assert!(config.content_files().is_empty());
    }

    #[test]
    fn test_content_files_parsed_in_order() {
        let config = load("content=Morrowind.esm\ncontent=Tribunal.esm\ncontent=Bloodmoon.esm\n");
        let files = config.content_files();
        assert_eq!(files, vec!["Morrowind.esm", "Tribunal.esm", "Bloodmoon.esm"]);
    }

    #[test]
    fn test_has_content_file_found() {
        let config = load("content=Morrowind.esm\n");
        assert!(config.has_content_file("Morrowind.esm"));
    }

    #[test]
    fn test_has_content_file_not_found() {
        let config = load("content=Morrowind.esm\n");
        assert!(!config.has_content_file("Tribunal.esm"));
    }

    #[test]
    fn test_duplicate_content_file_errors_on_load() {
        let dir = temp_dir();
        write_cfg(&dir, "content=Morrowind.esm\ncontent=Morrowind.esm\n");
        assert!(OpenMWConfiguration::new(Some(dir)).is_err());
    }

    #[test]
    fn test_add_content_file_appends() {
        let mut config = load("content=Morrowind.esm\n");
        config.add_content_file("MyMod.esp").unwrap();
        assert!(config.has_content_file("MyMod.esp"));
    }

    #[test]
    fn test_add_duplicate_content_file_errors() {
        let mut config = load("content=Morrowind.esm\n");
        assert!(config.add_content_file("Morrowind.esm").is_err());
    }

    #[test]
    fn test_add_content_file_source_config_is_cfg_file() {
        let dir = temp_dir();
        let cfg_path = write_cfg(&dir, "");
        let mut config = OpenMWConfiguration::new(Some(dir)).unwrap();
        config.add_content_file("Mod.esp").unwrap();
        let setting = config.content_files_iter().next().unwrap();
        assert_eq!(setting.meta().source_config, cfg_path,
            "source_config should be the openmw.cfg file, not a directory");
    }

    #[test]
    fn test_remove_content_file() {
        let mut config = load("content=Morrowind.esm\ncontent=Tribunal.esm\n");
        config.remove_content_file("Morrowind.esm");
        assert!(!config.has_content_file("Morrowind.esm"));
        assert!(config.has_content_file("Tribunal.esm"));
    }

    #[test]
    fn test_set_content_files_replaces_all() {
        let mut config = load("content=Morrowind.esm\ncontent=Tribunal.esm\n");
        config.set_content_files(Some(vec!["NewMod.esp".to_string()]));
        assert!(!config.has_content_file("Morrowind.esm"));
        assert!(!config.has_content_file("Tribunal.esm"));
        assert!(config.has_content_file("NewMod.esp"));
    }

    #[test]
    fn test_set_content_files_none_clears_all() {
        let mut config = load("content=Morrowind.esm\n");
        config.set_content_files(None);
        assert!(config.content_files().is_empty());
    }

    // -----------------------------------------------------------------------
    // Fallback archives
    // -----------------------------------------------------------------------

    #[test]
    fn test_fallback_archives_parsed() {
        let config = load("fallback-archive=Morrowind.bsa\nfallback-archive=Tribunal.bsa\n");
        let archives = config.fallback_archives();
        assert_eq!(archives, vec!["Morrowind.bsa", "Tribunal.bsa"]);
    }

    #[test]
    fn test_has_archive_file() {
        let config = load("fallback-archive=Morrowind.bsa\n");
        assert!(config.has_archive_file("Morrowind.bsa"));
        assert!(!config.has_archive_file("Tribunal.bsa"));
    }

    #[test]
    fn test_add_duplicate_archive_errors() {
        let mut config = load("fallback-archive=Morrowind.bsa\n");
        assert!(config.add_archive_file("Morrowind.bsa").is_err());
    }

    #[test]
    fn test_remove_archive_file() {
        let mut config = load("fallback-archive=Morrowind.bsa\nfallback-archive=Tribunal.bsa\n");
        config.remove_archive_file("Morrowind.bsa");
        assert!(!config.has_archive_file("Morrowind.bsa"));
        assert!(config.has_archive_file("Tribunal.bsa"));
    }

    // -----------------------------------------------------------------------
    // Groundcover
    // -----------------------------------------------------------------------

    #[test]
    fn test_groundcover_parsed() {
        let config = load("groundcover=GrassPlugin.esp\n");
        assert_eq!(config.groundcover(), vec!["GrassPlugin.esp"]);
    }

    #[test]
    fn test_has_groundcover_file() {
        let config = load("groundcover=Grass.esp\n");
        assert!(config.has_groundcover_file("Grass.esp"));
        assert!(!config.has_groundcover_file("Other.esp"));
    }

    #[test]
    fn test_duplicate_groundcover_errors_on_load() {
        let dir = temp_dir();
        write_cfg(&dir, "groundcover=Grass.esp\ngroundcover=Grass.esp\n");
        assert!(OpenMWConfiguration::new(Some(dir)).is_err());
    }

    // -----------------------------------------------------------------------
    // Data directories
    // -----------------------------------------------------------------------

    #[test]
    fn test_data_directories_absolute_paths_parsed() {
        let config = load("data=/absolute/path/to/data\n");
        let dirs = config.data_directories();
        assert!(dirs.iter().any(|d| d.ends_with("absolute/path/to/data")));
    }

    #[test]
    fn test_add_data_directory() {
        let mut config = load("");
        config.add_data_directory(PathBuf::from("/some/data/dir"));
        assert!(config.has_data_dir("/some/data/dir"));
    }

    #[test]
    fn test_set_data_directories_replaces_all() {
        let mut config = load("data=/old/dir\n");
        config.set_data_directories(Some(vec![PathBuf::from("/new/dir")]));
        assert!(!config.has_data_dir("/old/dir"));
        assert!(config.has_data_dir("/new/dir"));
    }

    #[test]
    fn test_remove_data_directory() {
        let mut config = load("data=/keep/me\n");
        config.add_data_directory(PathBuf::from("/remove/me"));
        config.remove_data_directory(&PathBuf::from("/remove/me"));
        assert!(!config.has_data_dir("/remove/me"));
        assert!(config.has_data_dir("/keep/me"));
    }

    // -----------------------------------------------------------------------
    // Fallback (game) settings
    // -----------------------------------------------------------------------

    #[test]
    fn test_game_settings_parsed() {
        let config = load("fallback=iMaxLevel,100\n");
        let setting = config.get_game_setting("iMaxLevel").unwrap();
        assert_eq!(setting.value(), "100");
    }

    #[test]
    fn test_game_settings_last_wins() {
        let config = load("fallback=iKey,1\nfallback=iKey,2\n");
        let setting = config.get_game_setting("iKey").unwrap();
        assert_eq!(setting.value(), "2");
    }

    #[test]
    fn test_game_settings_deduplicates_by_key() {
        // Known bug (#18): game_settings() deduplicates by full serialized string rather than by
        // key alone, so the same key with different values appears multiple times.
        // get_game_setting() correctly applies last-wins by key; game_settings() does not.
        // This test documents current (buggy) behavior so regressions are visible.
        // When #18 is fixed, change the assertion to `count == 1`.
        let config = load("fallback=iKey,1\nfallback=iKey,2\n");
        let count = config.game_settings().filter(|s| s.key() == "iKey").count();
        assert_eq!(count, 2, "BUG #18: game_settings() does not deduplicate by key");
    }

    #[test]
    fn test_get_game_setting_missing_returns_none() {
        let config = load("fallback=iKey,1\n");
        assert!(config.get_game_setting("iMissing").is_none());
    }

    #[test]
    fn test_game_setting_color_roundtrip() {
        let config = load("fallback=iSkyColor,100,149,237\n");
        let setting = config.get_game_setting("iSkyColor").unwrap();
        assert_eq!(setting.value(), "100,149,237");
    }

    #[test]
    fn test_game_setting_float_roundtrip() {
        let config = load("fallback=fGravity,9.81\n");
        let setting = config.get_game_setting("fGravity").unwrap();
        assert_eq!(setting.value(), "9.81");
    }

    // -----------------------------------------------------------------------
    // Encoding
    // -----------------------------------------------------------------------

    #[test]
    fn test_encoding_parsed() {
        use crate::config::encodingsetting::EncodingType;
        let config = load("encoding=win1252\n");
        assert_eq!(config.encoding().unwrap().value(), EncodingType::WIN1252);
    }

    #[test]
    fn test_invalid_encoding_errors_on_load() {
        let dir = temp_dir();
        write_cfg(&dir, "encoding=utf8\n");
        assert!(OpenMWConfiguration::new(Some(dir)).is_err());
    }

    // -----------------------------------------------------------------------
    // Replace semantics
    // -----------------------------------------------------------------------

    #[test]
    fn test_replace_content_clears_prior_plugins() {
        let config = load("content=Old.esm\nreplace=content\ncontent=New.esm\n");
        assert!(!config.has_content_file("Old.esm"));
        assert!(config.has_content_file("New.esm"));
    }

    #[test]
    fn test_replace_data_clears_prior_dirs() {
        let config = load("data=/old\nreplace=data\ndata=/new\n");
        assert!(!config.has_data_dir("/old"));
        assert!(config.has_data_dir("/new"));
    }

    // -----------------------------------------------------------------------
    // Display / serialisation
    // -----------------------------------------------------------------------

    #[test]
    fn test_display_contains_version_comment() {
        let config = load("content=Morrowind.esm\n");
        let output = config.to_string();
        assert!(output.contains("# OpenMW-Config Serializer Version:"),
            "Display should include version comment");
    }

    #[test]
    fn test_display_preserves_content_entries() {
        let config = load("content=Morrowind.esm\ncontent=Tribunal.esm\n");
        let output = config.to_string();
        assert!(output.contains("content=Morrowind.esm"));
        assert!(output.contains("content=Tribunal.esm"));
    }

    #[test]
    fn test_display_preserves_comments() {
        let config = load("# This is a comment\ncontent=Morrowind.esm\n");
        let output = config.to_string();
        assert!(output.contains("# This is a comment"));
    }

    // -----------------------------------------------------------------------
    // Generic settings
    // -----------------------------------------------------------------------

    #[test]
    fn test_generic_setting_preserved() {
        let config = load("some-unknown-key=some-value\n");
        let output = config.to_string();
        assert!(output.contains("some-unknown-key=some-value"));
    }
}
