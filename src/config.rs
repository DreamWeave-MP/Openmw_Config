// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Dave Corley (S3kshun8)

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

/// A single parsed entry from an `openmw.cfg` file.
///
/// Every line in the file is represented as one of these variants. The variant
/// determines both the key that appears in the file and how the value is interpreted.
/// Unknown keys are preserved as [`SettingValue::Generic`] so that round-trip
/// serialisation never silently drops unrecognised entries.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum SettingValue {
    /// A `data=` entry specifying a VFS data directory.
    DataDirectory(DirectorySetting),
    /// A `fallback=` entry containing a Morrowind.ini-style key/value pair.
    GameSetting(GameSettingType),
    /// A `user-data=` entry (singleton) specifying the user data root.
    UserData(DirectorySetting),
    /// A `data-local=` entry (singleton) specifying the highest-priority data directory.
    DataLocal(DirectorySetting),
    /// A `resources=` entry (singleton) specifying the engine resources directory.
    Resources(DirectorySetting),
    /// An `encoding=` entry (singleton) specifying the text encoding (`win1250`/`win1251`/`win1252`).
    Encoding(EncodingSetting),
    /// A `config=` entry referencing another `openmw.cfg` directory in the chain.
    SubConfiguration(DirectorySetting),
    /// Any unrecognised `key=value` line, preserved verbatim.
    Generic(GenericSetting),
    /// A `content=` entry naming an ESP/ESM plugin file.
    ContentFile(FileSetting),
    /// A `fallback-archive=` entry naming a BSA archive file.
    BethArchive(FileSetting),
    /// A `groundcover=` entry naming a groundcover plugin file.
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
                format!("{}content={}", plugin.meta().comment, plugin.value())
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
            SettingValue::BethArchive(setting)
            | SettingValue::Groundcover(setting)
            | SettingValue::ContentFile(setting) => setting.meta(),
            SettingValue::UserData(setting)
            | SettingValue::DataLocal(setting)
            | SettingValue::DataDirectory(setting)
            | SettingValue::Resources(setting)
            | SettingValue::SubConfiguration(setting) => setting.meta(),
            SettingValue::GameSetting(setting) => setting.meta(),
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

/// A fully-resolved `OpenMW` configuration chain.
///
/// Constructed by walking the `config=` chain starting from a root `openmw.cfg`, accumulating
/// every setting from every file into a flat list.  The list preserves source attribution and
/// comments so that [`save_user`](Self::save_user) can write back only the user-owned entries,
/// and [`Display`](std::fmt::Display) can reproduce a valid, comment-preserving `openmw.cfg`.
#[derive(Debug, Default, Clone)]
pub struct OpenMWConfiguration {
    root_config: PathBuf,
    settings: Vec<SettingValue>,
}

impl OpenMWConfiguration {
    /// # Errors
    /// Returns [`ConfigError`] if the path from the environment variable is invalid or if config loading fails.
    ///
    /// # Example
    /// ```no_run
    /// use openmw_config::OpenMWConfiguration;
    /// let config = OpenMWConfiguration::from_env()?;
    /// # Ok::<(), openmw_config::ConfigError>(())
    /// ```
    pub fn from_env() -> Result<Self, ConfigError> {
        if let Ok(explicit_path) = std::env::var("OPENMW_CONFIG") {
            let explicit_path: PathBuf = shellexpand::tilde(&explicit_path).into_owned().into();

            if explicit_path.as_os_str().is_empty() {
                return Err(ConfigError::NotFileOrDirectory(explicit_path));
            } else if explicit_path.is_absolute() {
                return Self::new(Some(explicit_path));
            } else if explicit_path.is_relative() {
                return Self::new(Some(std::fs::canonicalize(explicit_path)?));
            }
            return Err(ConfigError::NotFileOrDirectory(explicit_path));
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

    /// # Errors
    /// Returns [`ConfigError`] if the path does not exist, is not a valid config, or if loading the config chain fails.
    ///
    /// # Example
    /// ```no_run
    /// use std::path::PathBuf;
    /// use openmw_config::OpenMWConfiguration;
    ///
    /// // Platform default
    /// let config = OpenMWConfiguration::new(None)?;
    ///
    /// // Specific directory or file path — both are accepted
    /// let config = OpenMWConfiguration::new(Some(PathBuf::from("/home/user/.config/openmw")))?;
    /// # Ok::<(), openmw_config::ConfigError>(())
    /// ```
    pub fn new(path: Option<PathBuf>) -> Result<Self, ConfigError> {
        let mut config = OpenMWConfiguration::default();
        let root_config = match path {
            Some(path) => util::input_config_path(path)?,
            None => crate::default_config_path().join("openmw.cfg"),
        };

        config.root_config = root_config;

        if let Err(error) = config.load(&config.root_config.clone(), 0) {
            Err(error)
        } else {
            if let Some(dir) = config.data_local() {
                let path = dir.parsed();

                let path_meta = metadata(path);
                if path_meta.is_err()
                    && let Err(error) = create_dir_all(path)
                {
                    util::debug_log(&format!(
                        "WARNING: Attempted to create a data-local directory at {}, but failed: {error}",
                        path.display()
                    ));
                }

                config
                    .settings
                    .push(SettingValue::DataDirectory(dir.clone()));
            }

            if let Some(setting) = config.resources() {
                let dir = setting.parsed();

                let engine_vfs = DirectorySetting::new(
                    dir.join("vfs").to_string_lossy().to_string(),
                    setting.meta.source_config.clone(),
                    &mut setting.meta.comment.clone(),
                );

                config
                    .settings
                    .insert(0, SettingValue::DataDirectory(engine_vfs));
            }

            util::debug_log(&format!("{:#?}", config.settings));

            Ok(config)
        }
    }

    /// Path to the configuration file which is the root of the configuration chain
    /// Typically, this will be whatever is defined in the `Paths` documentation for the appropriate platform:
    /// <https://openmw.readthedocs.io/en/latest/reference/modding/paths.html#configuration-files-and-log-files>
    #[must_use]
    pub fn root_config_file(&self) -> &std::path::Path {
        &self.root_config
    }

    /// Same as `root_config_file`, but returns the directory it's in.
    /// Useful for reading other configuration files, or if assuming openmw.cfg
    /// Is always *called* openmw.cfg (which it should be)
    ///
    /// # Panics
    /// Panics if the root config path has no parent directory (i.e. it is a filesystem root).
    #[must_use]
    pub fn root_config_dir(&self) -> PathBuf {
        self.root_config
            .parent()
            .expect("root_config has no parent directory")
            .to_path_buf()
    }

    #[must_use]
    pub fn is_user_config(&self) -> bool {
        self.root_config_dir() == self.user_config_path()
    }

    /// # Errors
    /// Returns [`ConfigError`] if the user config path cannot be loaded.
    pub fn user_config(self) -> Result<Self, ConfigError> {
        let user_path = self.user_config_path();
        if self.root_config_dir() == user_path {
            Ok(self)
        } else {
            Self::new(Some(user_path))
        }
    }

    /// In order of priority, the list of all openmw.cfg files which were loaded by the configuration chain after the root.
    /// If the root openmw.cfg is different than the user one, this list will contain the user openmw.cfg as its last element.
    /// If the root and user openmw.cfg are the *same*, then this list will be empty and the root config should be considered the user config.
    /// Otherwise, if one wishes to get the contents of the user configuration specifically, construct a new `OpenMWConfiguration` from the last `sub_config`.
    ///
    /// Openmw.cfg files are added in order of the sequence in which they are defined by one openmw.cfg, and then each of *those* openmw.cfg files
    /// is then processed in their entirety, sequentially, after the first one has resolved.
    /// The highest-priority openmw.cfg loaded (the last one!) is considered the user openmw.cfg,
    /// and will be the one which is modifiable by OpenMW-Launcher and `OpenMW` proper.
    ///
    /// See <https://openmw.readthedocs.io/en/latest/reference/modding/paths.html#configuration-sources> for examples and further explanation of multiple config sources.
    ///
    /// Path to the highest-level configuration *directory*
    #[must_use]
    pub fn user_config_path(&self) -> PathBuf {
        self.sub_configs()
            .map(|setting| setting.parsed().to_path_buf())
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

    /// Content files are the actual *mods* or plugins which are created by either `OpenCS` or Bethesda's construction set
    /// These entries only refer to the names and ordering of content files.
    /// vfstool-lib should be used to derive paths
    pub fn content_files_iter(&self) -> impl Iterator<Item = &FileSetting> {
        self.settings.iter().filter_map(|setting| match setting {
            SettingValue::ContentFile(plugin) => Some(plugin),
            _ => None,
        })
    }

    /// Returns `true` if the named plugin is present in the `content=` list.
    #[must_use]
    pub fn has_content_file(&self, file_name: &str) -> bool {
        self.settings.iter().any(|setting| match setting {
            SettingValue::ContentFile(plugin) => plugin == file_name,
            _ => false,
        })
    }

    /// Returns `true` if the named plugin is present in the `groundcover=` list.
    #[must_use]
    pub fn has_groundcover_file(&self, file_name: &str) -> bool {
        self.settings.iter().any(|setting| match setting {
            SettingValue::Groundcover(plugin) => plugin == file_name,
            _ => false,
        })
    }

    /// Returns `true` if the named archive is present in the `fallback-archive=` list.
    #[must_use]
    pub fn has_archive_file(&self, file_name: &str) -> bool {
        self.settings.iter().any(|setting| match setting {
            SettingValue::BethArchive(archive) => archive == file_name,
            _ => false,
        })
    }

    /// Returns `true` if the given path is present in the `data=` list.
    ///
    /// Both `/` and `\` are normalised to the platform separator before comparison,
    /// so the query does not need to use a specific separator style.
    #[must_use]
    pub fn has_data_dir(&self, file_name: &str) -> bool {
        let query = PathBuf::from(file_name.replace(['/', '\\'], std::path::MAIN_SEPARATOR_STR));
        self.settings.iter().any(|setting| match setting {
            SettingValue::DataDirectory(data_dir) => data_dir.parsed() == query,
            _ => false,
        })
    }

    /// # Errors
    /// Returns [`ConfigError::CannotAddContentFile`] if the file is already present in the config.
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
        }

        self.settings
            .push(SettingValue::ContentFile(FileSetting::new(
                content_file,
                &self.user_config_path().join("openmw.cfg"),
                &mut String::default(),
            )));

        Ok(())
    }

    /// Iterates all `groundcover=` entries in definition order.
    pub fn groundcover_iter(&self) -> impl Iterator<Item = &FileSetting> {
        self.settings.iter().filter_map(|setting| match setting {
            SettingValue::Groundcover(grass) => Some(grass),
            _ => None,
        })
    }

    /// # Errors
    /// Returns [`ConfigError::CannotAddGroundcoverFile`] if the file is already present in the config.
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
        }

        self.settings
            .push(SettingValue::Groundcover(FileSetting::new(
                content_file,
                &self.user_config_path().join("openmw.cfg"),
                &mut String::default(),
            )));

        Ok(())
    }

    /// Removes all `content=` entries matching `file_name`.
    pub fn remove_content_file(&mut self, file_name: &str) {
        self.clear_matching(|setting| match setting {
            SettingValue::ContentFile(existing_file) => existing_file == file_name,
            _ => false,
        });
    }

    /// Removes all `groundcover=` entries matching `file_name`.
    pub fn remove_groundcover_file(&mut self, file_name: &str) {
        self.clear_matching(|setting| match setting {
            SettingValue::Groundcover(existing_file) => existing_file == file_name,
            _ => false,
        });
    }

    /// Removes all `fallback-archive=` entries matching `file_name`.
    pub fn remove_archive_file(&mut self, file_name: &str) {
        self.clear_matching(|setting| match setting {
            SettingValue::BethArchive(existing_file) => existing_file == file_name,
            _ => false,
        });
    }

    /// Removes any `data=` entry whose resolved path or original string matches `data_dir`.
    pub fn remove_data_directory(&mut self, data_dir: &PathBuf) {
        self.clear_matching(|setting| match setting {
            SettingValue::DataDirectory(existing_data_dir) => {
                existing_data_dir.parsed() == data_dir
                    || existing_data_dir.original() == data_dir.to_string_lossy().as_ref()
            }
            _ => false,
        });
    }

    /// Appends a data directory entry attributed to the user config. Does not check for duplicates.
    pub fn add_data_directory(&mut self, dir: &Path) {
        self.settings
            .push(SettingValue::DataDirectory(DirectorySetting::new(
                dir.to_string_lossy(),
                self.user_config_path().join("openmw.cfg"),
                &mut String::default(),
            )));
    }

    /// # Errors
    /// Returns [`ConfigError::CannotAddArchiveFile`] if the archive is already present in the config.
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
        }

        self.settings
            .push(SettingValue::BethArchive(FileSetting::new(
                archive_file,
                &self.user_config_path().join("openmw.cfg"),
                &mut String::default(),
            )));

        Ok(())
    }

    /// Iterates all `fallback-archive=` entries in definition order.
    pub fn fallback_archives_iter(&self) -> impl Iterator<Item = &FileSetting> {
        self.settings.iter().filter_map(|setting| match setting {
            SettingValue::BethArchive(archive) => Some(archive),
            _ => None,
        })
    }

    /// Replaces all `content=` entries with `plugins`, or clears them if `None`.
    ///
    /// Entries are attributed to the user config path. No duplicate checking is performed.
    pub fn set_content_files(&mut self, plugins: Option<Vec<String>>) {
        self.clear_matching(|setting| matches!(setting, SettingValue::ContentFile(_)));

        if let Some(plugins) = plugins {
            let cfg_path = self.user_config_path().join("openmw.cfg");
            let mut empty = String::default();
            for plugin in plugins {
                self.settings
                    .push(SettingValue::ContentFile(FileSetting::new(
                        &plugin, &cfg_path, &mut empty,
                    )));
            }
        }
    }

    /// Replaces all `fallback-archive=` entries with `archives`, or clears them if `None`.
    ///
    /// Entries are attributed to the user config path. No duplicate checking is performed.
    pub fn set_fallback_archives(&mut self, archives: Option<Vec<String>>) {
        self.clear_matching(|setting| matches!(setting, SettingValue::BethArchive(_)));

        if let Some(archives) = archives {
            let cfg_path = self.user_config_path().join("openmw.cfg");
            let mut empty = String::default();
            for archive in archives {
                self.settings
                    .push(SettingValue::BethArchive(FileSetting::new(
                        &archive, &cfg_path, &mut empty,
                    )));
            }
        }
    }

    /// Iterates all settings for which `predicate` returns `true`.
    pub fn settings_matching<'a, P>(
        &'a self,
        predicate: P,
    ) -> impl Iterator<Item = &'a SettingValue>
    where
        P: Fn(&SettingValue) -> bool + 'a,
    {
        self.settings.iter().filter(move |s| predicate(s))
    }

    /// Removes all settings for which `predicate` returns `true`.
    pub fn clear_matching<P>(&mut self, predicate: P)
    where
        P: Fn(&SettingValue) -> bool,
    {
        self.settings.retain(|s| !predicate(s));
    }

    /// Replaces all `data=` entries with `dirs`, or clears them if `None`.
    ///
    /// Entries are attributed to the user config path. No duplicate checking is performed.
    pub fn set_data_directories(&mut self, dirs: Option<Vec<PathBuf>>) {
        self.clear_matching(|setting| matches!(setting, SettingValue::DataDirectory(_)));

        if let Some(dirs) = dirs {
            let cfg_path = self.user_config_path().join("openmw.cfg");
            let mut empty = String::default();

            for dir in dirs {
                self.settings
                    .push(SettingValue::DataDirectory(DirectorySetting::new(
                        dir.to_string_lossy(),
                        cfg_path.clone(),
                        &mut empty,
                    )));
            }
        }
    }

    /// Given a string resembling a fallback= entry's value, as it would exist in openmw.cfg,
    /// Add it to the settings map.
    /// This process must be non-destructive
    ///
    /// # Errors
    /// Returns [`ConfigError`] if `base_value` cannot be parsed as a valid game setting.
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

    /// Replaces all `fallback=` entries with `settings`, or clears them if `None`.
    ///
    /// Each string must be in `Key,Value` format — the same as it would appear after the `=` in
    /// an `openmw.cfg` `fallback=` line.
    ///
    /// # Errors
    /// Returns [`ConfigError`] if any entry in `settings` cannot be parsed as a valid game setting.
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
            })?;
        }

        Ok(())
    }

    /// Iterates all `config=` sub-configuration entries in definition order.
    pub fn sub_configs(&self) -> impl Iterator<Item = &DirectorySetting> {
        self.settings.iter().filter_map(|setting| match setting {
            SettingValue::SubConfiguration(subconfig) => Some(subconfig),
            _ => None,
        })
    }

    /// Fallback entries are k/v pairs baked into the value side of k/v pairs in `fallback=` entries of openmw.cfg.
    /// They are used to express settings which are defined in Morrowind.ini for things such as:
    /// weather, lighting behaviors, UI colors, and levelup messages.
    ///
    /// Returns each key exactly once — when a key appears multiple times in the config chain, the
    /// last-defined value wins.
    ///
    /// # Example
    /// ```no_run
    /// use openmw_config::OpenMWConfiguration;
    /// let config = OpenMWConfiguration::new(None)?;
    /// for setting in config.game_settings() {
    ///     println!("{}={}", setting.key(), setting.value());
    /// }
    /// # Ok::<(), openmw_config::ConfigError>(())
    /// ```
    pub fn game_settings(&self) -> impl Iterator<Item = &GameSettingType> {
        let mut unique_settings = Vec::new();
        let mut seen: HashSet<&str> = HashSet::new();

        for setting in self.settings.iter().rev() {
            if let SettingValue::GameSetting(gs) = setting
                && seen.insert(gs.key())
            {
                unique_settings.push(gs);
            }
        }

        unique_settings.into_iter()
    }

    /// Retrieves a gamesetting according to its name.
    /// This would be whatever text comes after the equals sign `=` and before the first comma `,`
    /// Case-sensitive!
    #[must_use]
    pub fn get_game_setting(&self, key: &str) -> Option<&GameSettingType> {
        for setting in self.settings.iter().rev() {
            if let SettingValue::GameSetting(setting) = setting
                && setting == &key
            {
                return Some(setting);
            }
        }
        None
    }

    /// Data directories are the bulk of an `OpenMW` Configuration's contents,
    /// Composing the list of files from which a VFS is constructed.
    /// For a VFS implementation, see: <https://github.com/magicaldave/vfstool/tree/main/vfstool_lib>
    ///
    /// Calling this function will give the post-parsed versions of directories defined by an openmw.cfg,
    /// So the real ones may easily be iterated and loaded.
    /// There is not actually validation anywhere in the crate that `DirectorySettings` refer to a directory which actually exists.
    /// This is according to the openmw.cfg specification and doesn't technically break anything but should be considered when using these paths.
    pub fn data_directories_iter(&self) -> impl Iterator<Item = &DirectorySetting> {
        self.settings.iter().filter_map(|setting| match setting {
            SettingValue::DataDirectory(data_dir) => Some(data_dir),
            _ => None,
        })
    }

    const MAX_CONFIG_DEPTH: usize = 16;

    #[allow(clippy::too_many_lines)]
    fn load(&mut self, config_dir: &Path, depth: usize) -> Result<(), ConfigError> {
        if depth > Self::MAX_CONFIG_DEPTH {
            bail_config!(max_depth_exceeded, config_dir);
        }

        util::debug_log(&format!("BEGIN CONFIG PARSING: {}", config_dir.display()));

        if !config_dir.exists() {
            bail_config!(cannot_find, config_dir);
        }

        let cfg_file_path = if config_dir.is_dir() {
            config_dir.join("openmw.cfg")
        } else {
            config_dir.to_path_buf()
        };

        let lines = read_to_string(&cfg_file_path)?;

        let mut queued_comment = String::new();
        let mut sub_configs: Vec<(String, String)> = Vec::new();

        let mut seen_content: HashSet<String> = HashSet::new();
        let mut seen_groundcover: HashSet<String> = HashSet::new();
        let mut seen_archives: HashSet<String> = HashSet::new();

        for setting in &self.settings {
            match setting {
                SettingValue::ContentFile(f) => {
                    seen_content.insert(f.value().clone());
                }
                SettingValue::Groundcover(f) => {
                    seen_groundcover.insert(f.value().clone());
                }
                SettingValue::BethArchive(f) => {
                    seen_archives.insert(f.value().clone());
                }
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
                            config_dir,
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
                            config_dir,
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
                            config_dir,
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
                    );
                }
                "resources" => {
                    insert_dir_setting!(
                        self,
                        Resources,
                        &value,
                        (config_dir).to_path_buf(),
                        &mut queued_comment
                    );
                }
                "user-data" => {
                    insert_dir_setting!(
                        self,
                        UserData,
                        &value,
                        (config_dir).to_path_buf(),
                        &mut queued_comment
                    );
                }
                "data-local" => {
                    insert_dir_setting!(
                        self,
                        DataLocal,
                        &value,
                        (config_dir).to_path_buf(),
                        &mut queued_comment
                    );
                }
                "replace" => match value.to_lowercase().as_str() {
                    "content" => {
                        self.set_content_files(None);
                        seen_content.clear();
                    }
                    "data" => self.set_data_directories(None),
                    "fallback" => self.set_game_settings(None)?,
                    "fallback-archives" => {
                        self.set_fallback_archives(None);
                        seen_archives.clear();
                    }
                    "groundcover" => {
                        self.clear_matching(|s| matches!(s, SettingValue::Groundcover(_)));
                        seen_groundcover.clear();
                    }
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
                    self.load(Path::new(&subconfig_path), depth + 1)
                } else {
                    util::debug_log(&format!(
                        "Skipping parsing of {} As this directory does not actually contain an openmw.cfg!",
                        config_dir.display(),
                    ));

                    Ok(())
                }
            },
        )?;

        Ok(())
    }

    fn write_config(config_string: &str, path: &Path) -> Result<(), ConfigError> {
        use std::io::Write;

        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(path)?;

        file.write_all(config_string.as_bytes())?;

        Ok(())
    }

    /// Saves the currently-defined user openmw.cfg configuration.
    ///
    /// Only settings whose source is the user config file are written; settings inherited from
    /// parent configs are not affected. Modifications applied to inherited settings at runtime
    /// are therefore not persisted by this method.
    ///
    /// # Errors
    /// Returns [`ConfigError::NotWritable`] if the target path is not writable.
    /// Returns [`ConfigError::Io`] if writing the file fails.
    pub fn save_user(&self) -> Result<(), ConfigError> {
        let target_dir = self.user_config_path();
        let cfg_path = target_dir.join("openmw.cfg");

        if !util::is_writable(&cfg_path) {
            bail_config!(not_writable, &cfg_path);
        }

        let mut user_settings_string = String::new();

        for user_setting in
            self.settings_matching(|setting| setting.meta().source_config == cfg_path)
        {
            user_settings_string.push_str(&user_setting.to_string());
        }

        Self::write_config(&user_settings_string, &cfg_path)?;

        Ok(())
    }

    /// Saves the openmw.cfg belonging to a loaded sub-configuration.
    ///
    /// `target_dir` must be the directory of a `config=` entry already present in the loaded
    /// chain. This method refuses to write to arbitrary paths to prevent accidental overwrites.
    ///
    /// # Errors
    /// Returns [`ConfigError::SubconfigNotLoaded`] if `target_dir` is not part of the chain.
    /// Returns [`ConfigError::NotWritable`] if the target path is not writable.
    /// Returns [`ConfigError::Io`] if writing the file fails.
    pub fn save_subconfig(&self, target_dir: &Path) -> Result<(), ConfigError> {
        let subconfig_is_loaded = self.settings.iter().any(|setting| match setting {
            SettingValue::SubConfiguration(subconfig) => {
                subconfig.parsed() == target_dir
                    || subconfig.original() == target_dir.to_string_lossy().as_ref()
            }
            _ => false,
        });

        if !subconfig_is_loaded {
            bail_config!(subconfig_not_loaded, target_dir);
        }

        let cfg_path = target_dir.join("openmw.cfg");

        if !util::is_writable(&cfg_path) {
            bail_config!(not_writable, &cfg_path);
        }

        let mut subconfig_settings_string = String::new();

        for subconfig_setting in
            self.settings_matching(|setting| setting.meta().source_config == cfg_path)
        {
            subconfig_settings_string.push_str(&subconfig_setting.to_string());
        }

        Self::write_config(&subconfig_settings_string, &cfg_path)?;

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
            .try_for_each(|setting| write!(f, "{setting}"))?;

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
    use std::sync::{
        Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    };

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
        // Use a per-process atomic counter so concurrent tests always get distinct
        // directories.  The old `subsec_nanos()` approach could collide when two
        // tests ran at the same nanosecond offset in different seconds, causing
        // one to overwrite the other's openmw.cfg before it was read.
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, Ordering::Relaxed);
        let base = std::env::temp_dir().join(format!("openmw_cfg_test_{id}"));
        std::fs::create_dir_all(&base).unwrap();
        base
    }

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn load(cfg_contents: &str) -> OpenMWConfiguration {
        let dir = temp_dir();
        write_cfg(&dir, cfg_contents);
        OpenMWConfiguration::new(Some(dir)).unwrap()
    }

    #[cfg(unix)]
    fn symlink_dir(target: &std::path::Path, link: &std::path::Path) {
        std::os::unix::fs::symlink(target, link).unwrap();
    }

    // -----------------------------------------------------------------------
    // Content files
    // -----------------------------------------------------------------------

    #[test]
    fn test_content_files_empty_on_bare_config() {
        let config = load("");
        assert!(config.content_files_iter().next().is_none());
    }

    #[test]
    fn test_content_files_parsed_in_order() {
        let config = load("content=Morrowind.esm\ncontent=Tribunal.esm\ncontent=Bloodmoon.esm\n");
        let files: Vec<&String> = config
            .content_files_iter()
            .map(FileSetting::value)
            .collect();
        assert_eq!(
            files,
            vec!["Morrowind.esm", "Tribunal.esm", "Bloodmoon.esm"]
        );
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
        assert_eq!(
            setting.meta().source_config,
            cfg_path,
            "source_config should be the openmw.cfg file, not a directory"
        );
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
        assert!(config.content_files_iter().next().is_none());
    }

    // -----------------------------------------------------------------------
    // Fallback archives
    // -----------------------------------------------------------------------

    #[test]
    fn test_fallback_archives_parsed() {
        let config = load("fallback-archive=Morrowind.bsa\nfallback-archive=Tribunal.bsa\n");
        let archives: Vec<&String> = config
            .fallback_archives_iter()
            .map(FileSetting::value)
            .collect();
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
        let grass: Vec<&String> = config.groundcover_iter().map(FileSetting::value).collect();
        assert_eq!(grass, vec!["GrassPlugin.esp"]);
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
        assert!(
            config
                .data_directories_iter()
                .any(|d| d.parsed().ends_with("absolute/path/to/data"))
        );
    }

    #[test]
    fn test_add_data_directory() {
        let mut config = load("");
        config.add_data_directory(Path::new("/some/data/dir"));
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
        config.add_data_directory(Path::new("/remove/me"));
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
        // When the same fallback key appears more than once, game_settings() must emit only the
        // last-defined value (last-wins), matching the behavior of get_game_setting().
        let config = load("fallback=iKey,1\nfallback=iKey,2\n");
        let results: Vec<_> = config
            .game_settings()
            .filter(|s| s.key() == "iKey")
            .collect();
        assert_eq!(
            results.len(),
            1,
            "game_settings() should deduplicate by key"
        );
        assert_eq!(results[0].value(), "2", "last-defined value should win");
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

    #[test]
    fn test_replace_keeps_comment_adjacency() {
        let config = load("content=Old.esm\nreplace=content\n\n# keep me\ncontent=New.esm\n");
        let output = config.to_string();

        assert!(!output.contains("Old.esm"));
        assert!(output.contains("# keep me\ncontent=New.esm"));
    }

    // -----------------------------------------------------------------------
    // Display / serialisation
    // -----------------------------------------------------------------------

    #[test]
    fn test_display_contains_version_comment() {
        let config = load("content=Morrowind.esm\n");
        let output = config.to_string();
        assert!(
            output.contains("# OpenMW-Config Serializer Version:"),
            "Display should include version comment"
        );
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

    // -----------------------------------------------------------------------
    // save_user
    // -----------------------------------------------------------------------

    #[test]
    fn test_save_user_round_trips_content_files() {
        let dir = temp_dir();
        write_cfg(&dir, "content=Morrowind.esm\ncontent=Tribunal.esm\n");
        let mut config = OpenMWConfiguration::new(Some(dir.clone())).unwrap();
        config.add_content_file("Bloodmoon.esm").unwrap();
        config.save_user().unwrap();

        let reloaded = OpenMWConfiguration::new(Some(dir)).unwrap();
        let files: Vec<&String> = reloaded
            .content_files_iter()
            .map(FileSetting::value)
            .collect();
        assert!(files.contains(&&"Morrowind.esm".to_string()));
        assert!(files.contains(&&"Bloodmoon.esm".to_string()));
    }

    #[test]
    fn test_save_user_not_writable_returns_error() {
        // Only meaningful on Unix — skip on other platforms
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let dir = temp_dir();
            write_cfg(&dir, "content=Morrowind.esm\n");
            let config = OpenMWConfiguration::new(Some(dir.clone())).unwrap();

            // Make the directory read-only so we can't write openmw.cfg
            let cfg_path = dir.join("openmw.cfg");
            std::fs::set_permissions(&cfg_path, std::fs::Permissions::from_mode(0o444)).unwrap();

            let result = config.save_user();
            // Restore permissions before asserting so temp cleanup works
            std::fs::set_permissions(&cfg_path, std::fs::Permissions::from_mode(0o644)).unwrap();

            assert!(
                matches!(result, Err(ConfigError::NotWritable(_))),
                "expected NotWritable, got {result:?}"
            );
        }
    }

    // -----------------------------------------------------------------------
    // save_subconfig
    // -----------------------------------------------------------------------

    #[test]
    fn test_save_subconfig_rejects_unloaded_path() {
        let dir = temp_dir();
        write_cfg(&dir, "content=Morrowind.esm\n");
        let config = OpenMWConfiguration::new(Some(dir)).unwrap();

        let fake_dir = temp_dir();
        let result = config.save_subconfig(&fake_dir);
        assert!(
            matches!(result, Err(ConfigError::SubconfigNotLoaded(_))),
            "expected SubconfigNotLoaded, got {result:?}"
        );
    }

    #[test]
    fn test_save_subconfig_round_trips_settings() {
        let root_dir = temp_dir();
        let sub_dir = temp_dir();
        write_cfg(&sub_dir, "content=Plugin.esp\n");
        write_cfg(
            &root_dir,
            &format!("content=Morrowind.esm\nconfig={}\n", sub_dir.display()),
        );

        let mut config = OpenMWConfiguration::new(Some(root_dir)).unwrap();
        config.add_content_file("NewPlugin.esp").unwrap();
        config.save_subconfig(&sub_dir).unwrap();

        let sub_cfg = sub_dir.join("openmw.cfg");
        let saved = std::fs::read_to_string(sub_cfg).unwrap();
        assert!(
            saved.contains("content=Plugin.esp"),
            "sub-config content preserved"
        );
    }

    // -----------------------------------------------------------------------
    // from_env
    // -----------------------------------------------------------------------

    #[test]
    fn test_from_env_openmw_config_dir() {
        let _guard = env_lock();
        let dir = temp_dir();
        write_cfg(&dir, "content=Morrowind.esm\n");

        // SAFETY: tests that mutate env must not run concurrently with each other.
        // The test binary is single-threaded by default so this is acceptable.
        unsafe { std::env::set_var("OPENMW_CONFIG_DIR", &dir) };
        let config = OpenMWConfiguration::from_env().unwrap();
        unsafe { std::env::remove_var("OPENMW_CONFIG_DIR") };

        assert!(config.has_content_file("Morrowind.esm"));
    }

    #[test]
    fn test_from_env_openmw_config_file() {
        let _guard = env_lock();
        let dir = temp_dir();
        let cfg = write_cfg(&dir, "content=Tribunal.esm\n");

        unsafe { std::env::set_var("OPENMW_CONFIG", &cfg) };
        let config = OpenMWConfiguration::from_env().unwrap();
        unsafe { std::env::remove_var("OPENMW_CONFIG") };

        assert!(config.has_content_file("Tribunal.esm"));
    }

    // -----------------------------------------------------------------------
    // ConfigError variants
    // -----------------------------------------------------------------------

    #[test]
    fn test_error_duplicate_archive_file() {
        // The parser itself rejects duplicate fallback-archive= entries
        let dir = temp_dir();
        write_cfg(
            &dir,
            "fallback-archive=Morrowind.bsa\nfallback-archive=Morrowind.bsa\n",
        );
        let result = OpenMWConfiguration::new(Some(dir));
        assert!(matches!(
            result,
            Err(ConfigError::DuplicateArchiveFile { .. })
        ));
    }

    #[test]
    fn test_error_cannot_add_groundcover_file() {
        let mut config = load("groundcover=GrassPlugin.esp\n");
        let result = config.add_groundcover_file("GrassPlugin.esp");
        assert!(matches!(
            result,
            Err(ConfigError::CannotAddGroundcoverFile { .. })
        ));
    }

    #[test]
    fn test_error_cannot_find() {
        let result =
            OpenMWConfiguration::new(Some(PathBuf::from("/nonexistent/totally/fake/path")));
        assert!(matches!(
            result,
            Err(ConfigError::CannotFind(_) | ConfigError::NotFileOrDirectory(_))
        ));
    }

    #[test]
    fn test_error_io_from_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let config_err: ConfigError = io_err.into();
        assert!(matches!(config_err, ConfigError::Io(_)));
    }

    #[test]
    fn test_error_invalid_line() {
        // A line with no `=` separator should produce InvalidLine
        let result = OpenMWConfiguration::new(Some({
            let dir = temp_dir();
            write_cfg(&dir, "this_has_no_equals_sign\n");
            dir
        }));
        assert!(matches!(result, Err(ConfigError::InvalidLine { .. })));
    }

    #[test]
    fn test_error_max_depth_exceeded() {
        // Build a self-referencing config chain that will hit the depth limit
        let dir = temp_dir();
        write_cfg(&dir, &format!("config={}\n", dir.display()));
        let result = OpenMWConfiguration::new(Some(dir));
        assert!(matches!(result, Err(ConfigError::MaxDepthExceeded(_))));
    }

    #[test]
    fn test_error_max_depth_exceeded_for_circular_chain() {
        let a = temp_dir();
        let b = temp_dir();

        write_cfg(&a, &format!("config={}\n", b.display()));
        write_cfg(&b, &format!("config={}\n", a.display()));

        let result = OpenMWConfiguration::new(Some(a));
        assert!(matches!(result, Err(ConfigError::MaxDepthExceeded(_))));
    }

    #[cfg(unix)]
    #[test]
    fn test_symlinked_config_dir_loads_like_real_path() {
        let real_dir = temp_dir();
        write_cfg(&real_dir, "content=Morrowind.esm\n");

        let link_parent = temp_dir();
        let link_path = link_parent.join("symlinked-config");
        if link_path.exists() {
            let _ = std::fs::remove_file(&link_path);
            let _ = std::fs::remove_dir_all(&link_path);
        }
        symlink_dir(&real_dir, &link_path);

        let config = OpenMWConfiguration::new(Some(link_path.clone())).unwrap();

        assert!(config.has_content_file("Morrowind.esm"));
        assert_eq!(config.root_config_file(), link_path.join("openmw.cfg"));
        assert_eq!(config.root_config_dir(), link_path);
    }

    // -----------------------------------------------------------------------
    // settings_matching and clear_matching
    // -----------------------------------------------------------------------

    #[test]
    fn test_settings_matching_filters_correctly() {
        let config = load("content=Morrowind.esm\nfallback-archive=Morrowind.bsa\n");
        let content_count = config
            .settings_matching(|s| matches!(s, SettingValue::ContentFile(_)))
            .count();
        assert_eq!(content_count, 1);
    }

    #[test]
    fn test_clear_matching_removes_entries() {
        let mut config = load("content=Morrowind.esm\ncontent=Tribunal.esm\n");
        config.clear_matching(|s| matches!(s, SettingValue::ContentFile(_)));
        assert_eq!(config.content_files_iter().count(), 0);
    }

    // -----------------------------------------------------------------------
    // sub_configs and config chaining
    // -----------------------------------------------------------------------

    #[test]
    fn test_sub_configs_iteration() {
        let root_dir = temp_dir();
        let sub_dir = temp_dir();
        write_cfg(&sub_dir, "content=Plugin.esp\n");
        write_cfg(
            &root_dir,
            &format!("content=Morrowind.esm\nconfig={}\n", sub_dir.display()),
        );

        let config = OpenMWConfiguration::new(Some(root_dir)).unwrap();
        assert_eq!(config.sub_configs().count(), 1);
        assert!(
            config.has_content_file("Plugin.esp"),
            "sub-config content visible in root"
        );
    }

    // -----------------------------------------------------------------------
    // root_config_file / root_config_dir
    // -----------------------------------------------------------------------

    #[test]
    fn test_root_config_file_points_to_cfg() {
        let dir = temp_dir();
        write_cfg(&dir, "");
        let config = OpenMWConfiguration::new(Some(dir.clone())).unwrap();
        assert_eq!(config.root_config_file(), dir.join("openmw.cfg"));
    }

    #[test]
    fn test_root_config_dir_is_parent() {
        let dir = temp_dir();
        write_cfg(&dir, "");
        let config = OpenMWConfiguration::new(Some(dir.clone())).unwrap();
        assert_eq!(config.root_config_dir(), dir);
    }

    // -----------------------------------------------------------------------
    // Clone
    // -----------------------------------------------------------------------

    #[test]
    fn test_clone_is_independent() {
        let mut original = load("content=Morrowind.esm\n");
        let mut cloned = original.clone();
        cloned.add_content_file("Tribunal.esm").unwrap();
        original.add_content_file("Bloodmoon.esm").unwrap();
        assert!(cloned.has_content_file("Tribunal.esm"));
        assert!(!cloned.has_content_file("Bloodmoon.esm"));
        assert!(original.has_content_file("Bloodmoon.esm"));
        assert!(!original.has_content_file("Tribunal.esm"));
    }
}
