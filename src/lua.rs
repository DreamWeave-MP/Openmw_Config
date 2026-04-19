// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Dave Corley (S3kshun8)

use crate::{
    ConfigChainStatus, DirectorySetting, EncodingSetting, GameSettingType, OpenMWConfiguration,
};
use mlua::{Lua, Table, UserData, UserDataMethods};
use std::path::{Path, PathBuf};

fn lua_err(error: impl std::fmt::Display) -> mlua::Error {
    mlua::Error::RuntimeError(error.to_string())
}

fn dir_setting_from_value(path: &str, source_config: &Path) -> DirectorySetting {
    DirectorySetting::new(path.to_owned(), source_config.to_path_buf(), &mut String::new())
}

fn collect_strings<I>(iter: I) -> Vec<String>
where
    I: IntoIterator,
    I::Item: ToString,
{
    iter.into_iter().map(|value| value.to_string()).collect()
}

fn game_setting_kind(setting: &GameSettingType) -> &'static str {
    match setting {
        GameSettingType::Color(_) => "Color",
        GameSettingType::String(_) => "String",
        GameSettingType::Float(_) => "Float",
        GameSettingType::Int(_) => "Int",
    }
}

fn push_game_setting_row(
    lua: &Lua,
    table: &Table,
    index: usize,
    setting: &GameSettingType,
) -> mlua::Result<()> {
    let row = lua.create_table()?;
    row.set("key", setting.key_str())?;
    row.set("value", setting.value().to_string())?;
    row.set("kind", game_setting_kind(setting))?;
    table.set(index + 1, row)?;
    Ok(())
}

#[derive(Clone)]
pub struct LuaOpenMWConfiguration {
    inner: OpenMWConfiguration,
}

impl LuaOpenMWConfiguration {
    fn new(inner: OpenMWConfiguration) -> Self {
        Self { inner }
    }
}

impl UserData for LuaOpenMWConfiguration {
    #[allow(clippy::too_many_lines)]
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_method("rootConfigFile", |_, this, ()| {
            Ok(this.inner.root_config_file().display().to_string())
        });

        methods.add_method("rootConfigDir", |_, this, ()| {
            Ok(this.inner.root_config_dir().display().to_string())
        });

        methods.add_method("isUserConfig", |_, this, ()| Ok(this.inner.is_user_config()));

        methods.add_method("userConfigPath", |_, this, ()| {
            Ok(this.inner.user_config_path().display().to_string())
        });

        methods.add_method("userConfig", |_, this, ()| {
            this.inner
                .user_config_ref()
                .map(Self::new)
                .map_err(lua_err)
        });

        methods.add_method("toString", |_, this, ()| Ok(this.inner.to_string()));

        methods.add_method("subConfigs", |_, this, ()| {
            Ok(collect_strings(
                this.inner
                    .sub_configs()
                    .map(|dir| dir.parsed().display().to_string()),
            ))
        });

        methods.add_method("configChain", |lua, this, ()| {
            let entries = lua.create_table()?;

            for (index, entry) in this.inner.config_chain().enumerate() {
                let status = match entry.status() {
                    ConfigChainStatus::Loaded => "loaded",
                    ConfigChainStatus::SkippedMissing => "skippedMissing",
                };

                let row = lua.create_table()?;
                row.set("path", entry.path().display().to_string())?;
                row.set("depth", entry.depth())?;
                row.set("status", status)?;
                entries.set(index + 1, row)?;
            }

            Ok(entries)
        });

        methods.add_method("contentFiles", |_, this, ()| {
            Ok(collect_strings(this.inner.content_files_iter().map(crate::FileSetting::value_str)))
        });

        methods.add_method("groundcoverFiles", |_, this, ()| {
            Ok(collect_strings(this.inner.groundcover_iter().map(crate::FileSetting::value_str)))
        });

        methods.add_method("fallbackArchives", |_, this, ()| {
            Ok(collect_strings(
                this.inner
                    .fallback_archives_iter()
                    .map(crate::FileSetting::value_str),
            ))
        });

        methods.add_method("dataDirectories", |_, this, ()| {
            Ok(collect_strings(
                this.inner
                    .data_directories_iter()
                    .map(|dir| dir.parsed().display().to_string()),
            ))
        });

        methods.add_method("gameSettings", |lua, this, ()| {
            let settings = lua.create_table()?;

            for (index, setting) in this.inner.game_settings().enumerate() {
                push_game_setting_row(lua, &settings, index, setting)?;
            }

            Ok(settings)
        });

        methods.add_method("getGameSetting", |lua, this, key: String| {
            if let Some(setting) = this.inner.get_game_setting(&key) {
                let row = lua.create_table()?;
                row.set("key", setting.key_str())?;
                row.set("value", setting.value().to_string())?;
                row.set("kind", game_setting_kind(setting))?;
                Ok(Some(row))
            } else {
                Ok(None::<Table>)
            }
        });

        methods.add_method("userData", |_, this, ()| {
            Ok(this
                .inner
                .userdata()
                .map(|setting| setting.parsed().display().to_string()))
        });

        methods.add_method("resources", |_, this, ()| {
            Ok(this
                .inner
                .resources()
                .map(|setting| setting.parsed().display().to_string()))
        });

        methods.add_method("dataLocal", |_, this, ()| {
            Ok(this
                .inner
                .data_local()
                .map(|setting| setting.parsed().display().to_string()))
        });

        methods.add_method("encoding", |_, this, ()| {
            Ok(this
                .inner
                .encoding()
                .map(|encoding| encoding.value().to_string().trim().to_string()))
        });

        methods.add_method("hasContentFile", |_, this, file: String| {
            Ok(this.inner.has_content_file(&file))
        });

        methods.add_method("hasGroundcoverFile", |_, this, file: String| {
            Ok(this.inner.has_groundcover_file(&file))
        });

        methods.add_method("hasArchiveFile", |_, this, file: String| {
            Ok(this.inner.has_archive_file(&file))
        });

        methods.add_method("hasDataDir", |_, this, path: String| {
            Ok(this.inner.has_data_dir(&path))
        });

        methods.add_method_mut("addContentFile", |_, this, file: String| {
            this.inner.add_content_file(&file).map_err(lua_err)
        });

        methods.add_method_mut("addGroundcoverFile", |_, this, file: String| {
            this.inner.add_groundcover_file(&file).map_err(lua_err)
        });

        methods.add_method_mut("addArchiveFile", |_, this, file: String| {
            this.inner.add_archive_file(&file).map_err(lua_err)
        });

        methods.add_method_mut("addDataDirectory", |_, this, dir: String| {
            this.inner.add_data_directory(Path::new(&dir));
            Ok(())
        });

        methods.add_method_mut("removeContentFile", |_, this, file: String| {
            this.inner.remove_content_file(&file);
            Ok(())
        });

        methods.add_method_mut("removeGroundcoverFile", |_, this, file: String| {
            this.inner.remove_groundcover_file(&file);
            Ok(())
        });

        methods.add_method_mut("removeArchiveFile", |_, this, file: String| {
            this.inner.remove_archive_file(&file);
            Ok(())
        });

        methods.add_method_mut("removeDataDirectory", |_, this, dir: String| {
            this.inner.remove_data_directory(&PathBuf::from(dir));
            Ok(())
        });

        methods.add_method_mut("setContentFiles", |_, this, files: Option<Vec<String>>| {
            this.inner.set_content_files(files);
            Ok(())
        });

        methods.add_method_mut(
            "setFallbackArchives",
            |_, this, archives: Option<Vec<String>>| {
                this.inner.set_fallback_archives(archives);
                Ok(())
            },
        );

        methods.add_method_mut("setDataDirectories", |_, this, dirs: Option<Vec<String>>| {
            let parsed = dirs.map(|items| items.into_iter().map(PathBuf::from).collect());
            this.inner.set_data_directories(parsed);
            Ok(())
        });

        methods.add_method_mut(
            "setGameSetting",
            |_, this, (value, source_path, comment): (String, Option<String>, Option<String>)| {
                let mut comment = comment.unwrap_or_default();
                let source_path = source_path.map(PathBuf::from);
                this.inner
                    .set_game_setting(&value, source_path, &mut comment)
                    .map_err(lua_err)
            },
        );

        methods.add_method_mut("setGameSettings", |_, this, settings: Option<Vec<String>>| {
            this.inner.set_game_settings(settings).map_err(lua_err)
        });

        methods.add_method_mut("setUserData", |_, this, path: Option<String>| {
            let source = this.inner.user_config_path().join("openmw.cfg");
            let setting = path.as_deref().map(|value| dir_setting_from_value(value, &source));
            this.inner.set_userdata(setting);
            Ok(())
        });

        methods.add_method_mut("setResources", |_, this, path: Option<String>| {
            let source = this.inner.user_config_path().join("openmw.cfg");
            let setting = path.as_deref().map(|value| dir_setting_from_value(value, &source));
            this.inner.set_resources(setting);
            Ok(())
        });

        methods.add_method_mut("setDataLocal", |_, this, path: Option<String>| {
            let source = this.inner.user_config_path().join("openmw.cfg");
            let setting = path.as_deref().map(|value| dir_setting_from_value(value, &source));
            this.inner.set_data_local(setting);
            Ok(())
        });

        methods.add_method_mut("setEncoding", |_, this, value: Option<String>| {
            let source = this.inner.user_config_path().join("openmw.cfg");

            let setting = match value {
                Some(value) => Some(
                    EncodingSetting::try_from((value, source, &mut String::new())).map_err(lua_err)?,
                ),
                None => None,
            };

            this.inner.set_encoding(setting);
            Ok(())
        });

        methods.add_method("saveUser", |_, this, ()| {
            this.inner.save_user().map_err(lua_err)
        });

        methods.add_method("saveSubconfig", |_, this, target_dir: String| {
            this.inner
                .save_subconfig(Path::new(&target_dir))
                .map_err(lua_err)
        });
    }
}

/// Creates the top-level Lua module table.
///
/// All outward-facing Lua methods intentionally use camelCase naming.
///
/// # Errors
/// Returns [`mlua::Error`] if Lua function/table registration fails.
pub fn create_lua_module(lua: &Lua) -> mlua::Result<Table> {
    let exports = lua.create_table()?;

    exports.set(
        "fromEnv",
        lua.create_function(|_, ()| {
            OpenMWConfiguration::from_env()
                .map(LuaOpenMWConfiguration::new)
                .map_err(lua_err)
        })?,
    )?;

    exports.set(
        "new",
        lua.create_function(|_, path: Option<String>| {
            OpenMWConfiguration::new(path.map(PathBuf::from))
                .map(LuaOpenMWConfiguration::new)
                .map_err(lua_err)
        })?,
    )?;

    exports.set(
        "defaultConfigPath",
        lua.create_function(|_, ()| Ok(crate::default_config_path().display().to_string()))?,
    )?;

    exports.set(
        "defaultUserDataPath",
        lua.create_function(|_, ()| Ok(crate::default_userdata_path().display().to_string()))?,
    )?;

    exports.set(
        "defaultDataLocalPath",
        lua.create_function(|_, ()| Ok(crate::default_data_local_path().display().to_string()))?,
    )?;

    exports.set(
        "defaultLocalPath",
        lua.create_function(|_, ()| Ok(crate::default_local_path().display().to_string()))?,
    )?;

    exports.set(
        "defaultGlobalPath",
        lua.create_function(|_, ()| Ok(crate::default_global_path().display().to_string()))?,
    )?;

    exports.set(
        "tryDefaultConfigPath",
        lua.create_function(|_, ()| match crate::try_default_config_path() {
            Ok(path) => Ok((Some(path.display().to_string()), Option::<String>::None)),
            Err(error) => Ok((Option::<String>::None, Some(error.to_string()))),
        })?,
    )?;

    exports.set(
        "tryDefaultUserDataPath",
        lua.create_function(|_, ()| match crate::try_default_userdata_path() {
            Ok(path) => Ok((Some(path.display().to_string()), Option::<String>::None)),
            Err(error) => Ok((Option::<String>::None, Some(error.to_string()))),
        })?,
    )?;

    exports.set(
        "tryDefaultLocalPath",
        lua.create_function(|_, ()| match crate::try_default_local_path() {
            Ok(path) => Ok((Some(path.display().to_string()), Option::<String>::None)),
            Err(error) => Ok((Option::<String>::None, Some(error.to_string()))),
        })?,
    )?;

    exports.set(
        "tryDefaultGlobalPath",
        lua.create_function(|_, ()| match crate::try_default_global_path() {
            Ok(path) => Ok((Some(path.display().to_string()), Option::<String>::None)),
            Err(error) => Ok((Option::<String>::None, Some(error.to_string()))),
        })?,
    )?;

    exports.set("version", env!("CARGO_PKG_VERSION"))?;

    Ok(exports)
}
