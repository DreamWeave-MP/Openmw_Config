// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Dave Corley (S3kshun8)

//! Luau bindings as an [l3i](https://github.com/DreamWeave-MP/dream-binder) extension:
//! id `dream.openmw-config`, module `@dream/openmw-config`, main type `dream.openmw.Config`.
//!
//! The crate never creates a VM. The host composes [`extension()`] into a runtime plan and
//! instantiates runtimes from it; scripts reach the module through
//! `require("@dream/openmw-config")`, or through whatever compatibility global the host's
//! policy exposes (`RuntimePolicy::compat_global("@dream/openmw-config", "openmwConfig")`).
//!
//! ```ignore
//! use l3i::extension::RuntimePlan;
//! let plan = RuntimePlan::builder().extension(openmw_config::luau::extension()).finalize()?;
//! let runtime = l3i::Runtime::from_plan(&plan)?;
//! runtime.exec("local cfg = require('@dream/openmw-config').fromEnv() cfg:saveUser()")?;
//! ```
//!
//! # Shape
//!
//! - `Config` methods are the crate's camelCase surface (loaders, reads, mutators, saves).
//!   `isUserConfig` and the counts (`contentFileCount`, `groundcoverFileCount`,
//!   `archiveFileCount`, `dataDirectoryCount`, `gameSettingCount`, `genericSettingCount`,
//!   `subConfigCount`) are direct fields, read as `cfg.isUserConfig`.
//! - The list methods (`contentFiles`, `groundcoverFiles`, `fallbackArchives`,
//!   `dataDirectories`, `subConfigs`, `gameSettings`, `genericSettings`, `configChain`) return
//!   sequence views over the live configuration: `#list`, `list[i]` (1-based, nil past the
//!   end), `for i, item in list do`, and `list:toTable()` for a plain table. Nothing is copied
//!   until an element is read, and the generated definitions type all four with the element
//!   type, so a strict script reads a view directly. `ipairs`/`pairs`/`table.*` need
//!   `:toTable()`.
//! - Rows are userdata with the field names of the old tables: `key`, `value`, `kind`,
//!   `source`, `comment` on a game setting (plus `typed`: the parsed value, a Luau integer for
//!   `Int`, a number for `Float`, a vector of the 0..255 components for `Color`, the string for
//!   `String`), `key`, `value`, `source`, `comment` on a generic setting, `path`, `depth`,
//!   `status` on a chain entry. A row reads the live configuration; after the configuration
//!   it came from is mutated, reading it raises an error rather than a shifted entry.
//! - Errors are Lua errors; the `try*` path helpers return `(path, nil)` or `(nil, message)`.
//!
//! [`Config`] is public so a host can push a configuration it loaded itself as
//! `Owned(Config::new(configuration))`.

use std::cell::{Cell, Ref, RefCell};
use std::fmt::Display;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use l3i::bind::{Call, StackResults};
use l3i::convert::{Integer, Push, Vector3};
use l3i::direct::field::{DirectField, FieldValue};
use l3i::extension::{Extension, ExtensionDescriptor, TagPolicy};
use l3i::sequence::{Sequence, SequenceItem, SequenceSource};
use l3i::source::CompileConstant;
use l3i::stack::Scope;
use l3i::userdata::{Owned, Userdata};
use l3i::value::Table;
use l3i::{Error, Result};

use crate::config::{ListKind, SettingValue};
use crate::{
    ConfigChainEntry, ConfigChainStatus, ConfigError, EncodingSetting, GameSettingType,
    GenericSetting as GenericSettingEntry, OpenMWConfiguration,
};

/// The extension id.
pub const EXTENSION_ID: &str = "dream.openmw-config";
/// The module path scripts `require`.
pub const MODULE: &str = "@dream/openmw-config";
/// The stable key of the configuration type (class `dream_openmw_Config`).
pub const CONFIG_TYPE: &str = "dream.openmw.Config";
/// The stable key of the string list view (class `dream_openmw_Strings`).
pub const STRINGS_TYPE: &str = "dream.openmw.Strings";
/// The stable key of a game setting row (class `dream_openmw_GameSetting`).
pub const GAME_SETTING_TYPE: &str = "dream.openmw.GameSetting";
/// The stable key of the game setting list view (class `dream_openmw_GameSettings`).
pub const GAME_SETTINGS_TYPE: &str = "dream.openmw.GameSettings";
/// The stable key of a generic setting row (class `dream_openmw_GenericSetting`).
pub const GENERIC_SETTING_TYPE: &str = "dream.openmw.GenericSetting";
/// The stable key of the generic setting list view (class `dream_openmw_GenericSettings`).
pub const GENERIC_SETTINGS_TYPE: &str = "dream.openmw.GenericSettings";
/// The stable key of a config chain entry (class `dream_openmw_ChainEntry`).
pub const CHAIN_ENTRY_TYPE: &str = "dream.openmw.ChainEntry";
/// The stable key of the config chain view (class `dream_openmw_ConfigChain`).
pub const CONFIG_CHAIN_TYPE: &str = "dream.openmw.ConfigChain";

/// The extension, for `RuntimePlan::builder().extension(..)`.
#[must_use]
pub fn extension() -> OpenmwConfigExtension {
    OpenmwConfigExtension
}

/// The `dream.openmw-config` extension (see the module docs).
#[derive(Debug, Default, Clone, Copy)]
pub struct OpenmwConfigExtension;

// ---------------------------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------------------------

/// One loaded configuration, shared by its `Config` handle and every view and row made from
/// it. `generation` counts mutations so a row can tell that its position went stale.
struct Shared {
    inner: RefCell<OpenMWConfiguration>,
    generation: Cell<u64>,
    /// Cached at load: the root and the chain never change from Luau.
    is_user_config: bool,
}

impl Shared {
    fn new(config: OpenMWConfiguration) -> Rc<Shared> {
        let is_user_config = config.is_user_config();
        Rc::new(Shared {
            inner: RefCell::new(config),
            generation: Cell::new(0),
            is_user_config,
        })
    }

    #[inline]
    fn read(&self) -> Ref<'_, OpenMWConfiguration> {
        self.inner.borrow()
    }

    /// Runs a mutation and marks every outstanding row stale.
    fn mutate<R>(&self, body: impl FnOnce(&mut OpenMWConfiguration) -> R) -> R {
        let result = body(&mut self.inner.borrow_mut());
        self.generation.set(self.generation.get().wrapping_add(1));
        result
    }
}

fn err(error: impl Display) -> Error {
    Error::runtime(error.to_string())
}

fn stale(what: &str) -> Error {
    Error::runtime(format!(
        "this {what} row is stale: the configuration changed after it was read; fetch it again"
    ))
}

/// Pushes borrowed text as the single result, with no intermediate `String`.
#[inline]
fn text(call: &Call<'_>, text: &str) -> Result<StackResults> {
    call.push(text)?;
    Ok(StackResults)
}

/// A path as scripts see it: `Path::display` semantics without the allocation for UTF-8.
#[inline]
fn path_text(call: &Call<'_>, path: &Path) -> Result<StackResults> {
    text(call, &path.to_string_lossy())
}

/// A count as the plain number scripts compare and add; no list here approaches 2^53.
#[allow(clippy::cast_precision_loss)]
fn count(value: usize) -> f64 {
    value as f64
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn loaded(result: std::result::Result<OpenMWConfiguration, ConfigError>) -> Result<Owned<Config>> {
    result.map(Config::new).map(Owned).map_err(err)
}

/// A `try*` helper's result: `(path, nil)` or `(nil, message)`.
fn tried(result: std::result::Result<PathBuf, ConfigError>) -> (Option<String>, Option<String>) {
    match result {
        Ok(path) => (Some(path_string(&path)), None),
        Err(error) => (None, Some(error.to_string())),
    }
}

/// The strings of a Luau array table, or `None` for nil.
fn strings(call: &Call<'_>, table: Option<Table>) -> Result<Option<Vec<String>>> {
    let Some(table) = table else {
        return Ok(None);
    };
    call.with_frame(|frame| {
        let view = table.push_to(frame)?;
        let len = view.raw_len();
        let mut out = Vec::with_capacity(len);
        for index in 1..=len {
            frame.with_frame(|step| {
                let key = i64::try_from(index).map_err(|_| Error::runtime("list too long"))?;
                let value = view.raw_get_index(step, key)?;
                let entry: &str = value
                    .read()
                    .map_err(|cause| Error::runtime(format!("list entry {index}: {cause}")))?;
                out.push(entry.to_owned());
                Ok(())
            })?;
        }
        Ok(Some(out))
    })
}

// ---------------------------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------------------------

/// A loaded configuration as scripts hold it (`dream.openmw.Config`).
#[derive(Clone)]
pub struct Config(Rc<Shared>);

// SAFETY: the payload is an `Rc` of plain Rust data; dropping it never touches the Lua API.
unsafe impl Userdata for Config {
    const NAME: &'static str = CONFIG_TYPE;
}

impl Config {
    /// Wraps a configuration the host loaded; push it as `Owned(config)`.
    #[must_use]
    pub fn new(config: OpenMWConfiguration) -> Config {
        Config(Shared::new(config))
    }

    /// Reads the configuration.
    pub fn with<R>(&self, body: impl FnOnce(&OpenMWConfiguration) -> R) -> R {
        body(&self.0.read())
    }

    /// Mutates the configuration; rows scripts hold become stale.
    pub fn with_mut<R>(&self, body: impl FnOnce(&mut OpenMWConfiguration) -> R) -> R {
        self.0.mutate(body)
    }

    fn view(&self, kind: ListKind) -> Owned<Sequence<StringList>> {
        Owned(Sequence(StringList {
            shared: Rc::clone(&self.0),
            kind,
        }))
    }

    fn user_cfg(&self) -> PathBuf {
        self.0.read().user_config_path().join("openmw.cfg")
    }
}

struct IsUserConfigField;

impl DirectField<Config> for IsUserConfigField {
    fn get(config: &Config) -> FieldValue {
        FieldValue::Boolean(config.0.is_user_config)
    }
}

macro_rules! count_field {
    ($name:ident, $method:ident) => {
        struct $name;
        impl DirectField<Config> for $name {
            fn get(config: &Config) -> FieldValue {
                FieldValue::Number(count(config.0.read().$method()))
            }
        }
    };
}

count_field!(ContentFileCountField, content_file_count);
count_field!(GroundcoverFileCountField, groundcover_file_count);
count_field!(ArchiveFileCountField, archive_file_count);
count_field!(DataDirectoryCountField, data_directory_count);
count_field!(GameSettingCountField, game_setting_count);
count_field!(GenericSettingCountField, generic_setting_count);
count_field!(SubConfigCountField, sub_config_count);

// ---------------------------------------------------------------------------------------------
// String lists
// ---------------------------------------------------------------------------------------------

/// A view over one of the configuration's string lists (`dream.openmw.Strings`).
pub struct StringList {
    shared: Rc<Shared>,
    kind: ListKind,
}

/// One element of a string list, pushed straight from the setting it names.
pub struct StringItem {
    shared: Rc<Shared>,
    position: usize,
}

impl SequenceItem for StringItem {
    fn push_item<S: Scope>(self, scope: &S) -> Result<()> {
        let config = self.shared.read();
        match config.setting_at(self.position) {
            Some(
                SettingValue::ContentFile(file)
                | SettingValue::Groundcover(file)
                | SettingValue::BethArchive(file),
            ) => file.value_str().push_only(scope),
            Some(SettingValue::DataDirectory(dir) | SettingValue::SubConfiguration(dir)) => {
                dir.parsed().to_string_lossy().as_ref().push_only(scope)
            }
            _ => ().push_only(scope),
        }
    }
}

impl SequenceSource for StringList {
    const NAME: &'static str = STRINGS_TYPE;
    type Item = StringItem;

    fn len(&self) -> usize {
        self.shared.read().list_positions(self.kind).len()
    }

    fn get(&self, index: usize) -> Option<StringItem> {
        let position = *self.shared.read().list_positions(self.kind).get(index)?;
        Some(StringItem {
            shared: Rc::clone(&self.shared),
            position,
        })
    }
}

// ---------------------------------------------------------------------------------------------
// Game setting rows
// ---------------------------------------------------------------------------------------------

/// One `fallback=` entry (`dream.openmw.GameSetting`), read live from its configuration.
pub struct GameSetting {
    shared: Rc<Shared>,
    position: usize,
    generation: u64,
}

impl GameSetting {
    fn read<R>(&self, body: impl FnOnce(&GameSettingType) -> R) -> Result<R> {
        if self.generation != self.shared.generation.get() {
            return Err(stale("game setting"));
        }
        let config = self.shared.read();
        match config.setting_at(self.position) {
            Some(SettingValue::GameSetting(setting)) => Ok(body(setting)),
            _ => Err(stale("game setting")),
        }
    }
}

// SAFETY: as `Config`.
unsafe impl Userdata for GameSetting {
    const NAME: &'static str = GAME_SETTING_TYPE;
}

/// The view over the effective game settings (`dream.openmw.GameSettings`), one row per key,
/// last-defined first, as [`OpenMWConfiguration::game_settings`] iterates.
pub struct GameSettingList {
    shared: Rc<Shared>,
}

impl SequenceSource for GameSettingList {
    const NAME: &'static str = GAME_SETTINGS_TYPE;
    type Item = Owned<GameSetting>;

    fn len(&self) -> usize {
        self.shared.read().game_setting_count()
    }

    fn get(&self, index: usize) -> Option<Owned<GameSetting>> {
        let position = self.shared.read().game_setting_position_at(index)?;
        Some(Owned(GameSetting {
            shared: Rc::clone(&self.shared),
            position,
            generation: self.shared.generation.get(),
        }))
    }
}

// ---------------------------------------------------------------------------------------------
// Generic setting rows
// ---------------------------------------------------------------------------------------------

/// One preserved `key=value` entry (`dream.openmw.GenericSetting`).
pub struct GenericSetting {
    shared: Rc<Shared>,
    position: usize,
    generation: u64,
}

impl GenericSetting {
    fn read<R>(&self, body: impl FnOnce(&GenericSettingEntry) -> R) -> Result<R> {
        if self.generation != self.shared.generation.get() {
            return Err(stale("generic setting"));
        }
        let config = self.shared.read();
        match config.setting_at(self.position) {
            Some(SettingValue::Generic(setting)) => Ok(body(setting)),
            _ => Err(stale("generic setting")),
        }
    }
}

// SAFETY: as `Config`.
unsafe impl Userdata for GenericSetting {
    const NAME: &'static str = GENERIC_SETTING_TYPE;
}

/// The view over the preserved generic entries (`dream.openmw.GenericSettings`).
pub struct GenericSettingList {
    shared: Rc<Shared>,
}

impl SequenceSource for GenericSettingList {
    const NAME: &'static str = GENERIC_SETTINGS_TYPE;
    type Item = Owned<GenericSetting>;

    fn len(&self) -> usize {
        self.shared.read().generic_setting_count()
    }

    fn get(&self, index: usize) -> Option<Owned<GenericSetting>> {
        let position = *self
            .shared
            .read()
            .list_positions(ListKind::Generic)
            .get(index)?;
        Some(Owned(GenericSetting {
            shared: Rc::clone(&self.shared),
            position,
            generation: self.shared.generation.get(),
        }))
    }
}

// ---------------------------------------------------------------------------------------------
// Config chain rows
// ---------------------------------------------------------------------------------------------

/// One observed step of the chain traversal (`dream.openmw.ChainEntry`). The chain is fixed
/// at load, so no generation check is needed.
pub struct ChainEntry {
    shared: Rc<Shared>,
    index: usize,
}

impl ChainEntry {
    fn read<R>(&self, body: impl FnOnce(&ConfigChainEntry) -> R) -> Result<R> {
        let config = self.shared.read();
        config
            .chain_entry(self.index)
            .map(body)
            .ok_or_else(|| stale("config chain"))
    }
}

// SAFETY: as `Config`.
unsafe impl Userdata for ChainEntry {
    const NAME: &'static str = CHAIN_ENTRY_TYPE;
}

/// The view over the chain traversal (`dream.openmw.ConfigChain`).
pub struct ChainList {
    shared: Rc<Shared>,
}

impl SequenceSource for ChainList {
    const NAME: &'static str = CONFIG_CHAIN_TYPE;
    type Item = Owned<ChainEntry>;

    fn len(&self) -> usize {
        self.shared.read().config_chain().count()
    }

    fn get(&self, index: usize) -> Option<Owned<ChainEntry>> {
        self.shared.read().chain_entry(index)?;
        Some(Owned(ChainEntry {
            shared: Rc::clone(&self.shared),
            index,
        }))
    }
}

fn status_name(status: &ConfigChainStatus) -> &'static str {
    match status {
        ConfigChainStatus::Loaded => "loaded",
        ConfigChainStatus::SkippedMissing => "skippedMissing",
    }
}

// ---------------------------------------------------------------------------------------------
// The extension
// ---------------------------------------------------------------------------------------------

impl Extension for OpenmwConfigExtension {
    fn id(&self) -> &'static str {
        EXTENSION_ID
    }

    fn describe(&self, d: &mut ExtensionDescriptor) -> Result<()> {
        describe_config(d);
        describe_lists(d);
        describe_game_setting(d);
        describe_generic_setting(d);
        describe_chain_entry(d);
        describe_module(d);
        Ok(())
    }
}

/// Each view names its element type, so the definitions declare `#`, `[i]`, `for`, and
/// `toTable` with it and a strict script needs no `:toTable()` to read a view.
fn describe_lists(d: &mut ExtensionDescriptor) {
    d.sequence::<StringList>(STRINGS_TYPE)
        .tag(TagPolicy::Preferred)
        .item_type("string")
        .doc("A live view over one of the configuration's string lists: #list, list[i], for i, name in list, list:toTable().");
    d.sequence::<GameSettingList>(GAME_SETTINGS_TYPE)
        .tag(TagPolicy::Preferred)
        .item_type("dream_openmw_GameSetting")
        .doc("A live view over the effective fallback= settings, one row per key, last-defined first.");
    d.sequence::<GenericSettingList>(GENERIC_SETTINGS_TYPE)
        .tag(TagPolicy::Never)
        .item_type("dream_openmw_GenericSetting")
        .doc("A live view over the preserved generic key=value entries.");
    d.sequence::<ChainList>(CONFIG_CHAIN_TYPE)
        .tag(TagPolicy::Never)
        .item_type("dream_openmw_ChainEntry")
        .doc("The chain traversal in parser order: loaded files and skipped config= targets.");
}

fn describe_game_setting(d: &mut ExtensionDescriptor) {
    let mut row = d.userdata::<GameSetting>(GAME_SETTING_TYPE);
    row.tag(TagPolicy::Preferred)
        .doc("A fallback=Key,Value entry. Reads the live configuration; stale after it changes.");
    row.getter("key", |row: &GameSetting, call: &Call| {
        row.read(|setting| text(call, setting.key_str()))?
    })
    .signature("string");
    row.getter("value", |row: &GameSetting, call: &Call| {
        row.read(|setting| text(call, setting.value_str()))?
    })
    .signature("string")
    .doc("The value text after the first comma, whatever the kind.");
    row.getter("kind", |row: &GameSetting| {
        row.read(GameSettingType::kind_name)
    })
    .signature("string")
    .doc("Color, String, Float, or Int.");
    row.getter("typed", |row: &GameSetting, call: &Call| {
        row.read(|setting| -> Result<StackResults> {
            match setting {
                GameSettingType::Int(_) => {
                    call.push(&Integer(setting.int_value().unwrap_or_default()))?;
                }
                GameSettingType::Float(_) => {
                    call.push(&setting.float_value().unwrap_or_default())?;
                }
                GameSettingType::Color(_) => {
                    let (r, g, b) = setting.color_value().unwrap_or_default();
                    call.push(&Vector3::new(f32::from(r), f32::from(g), f32::from(b)))?;
                }
                GameSettingType::String(_) => {
                    call.push(setting.value_str())?;
                }
            }
            Ok(StackResults)
        })?
    })
    .signature("integer | number | vector | string")
    .doc("The parsed value: an integer for Int, a number for Float, a vector (r, g, b) for Color, the string for String.");
    row.getter("source", |row: &GameSetting, call: &Call| {
        row.read(|setting| path_text(call, setting.meta().source_config()))?
    })
    .signature("string")
    .doc("The openmw.cfg that defined it.");
    row.getter("comment", |row: &GameSetting, call: &Call| {
        row.read(|setting| text(call, setting.meta().comment()))?
    })
    .signature("string");
    row.metamethod("__tostring", |row: &GameSetting| {
        row.read(|setting| {
            format!(
                "dream.openmw.GameSetting({}={})",
                setting.key_str(),
                setting.value_str()
            )
        })
    });
}

fn describe_generic_setting(d: &mut ExtensionDescriptor) {
    let mut row = d.userdata::<GenericSetting>(GENERIC_SETTING_TYPE);
    row.tag(TagPolicy::Never)
        .doc("A preserved key=value entry. Reads the live configuration; stale after it changes.");
    row.getter("key", |row: &GenericSetting, call: &Call| {
        row.read(|setting| text(call, setting.key()))?
    })
    .signature("string");
    row.getter("value", |row: &GenericSetting, call: &Call| {
        row.read(|setting| text(call, setting.value()))?
    })
    .signature("string");
    row.getter("source", |row: &GenericSetting, call: &Call| {
        row.read(|setting| path_text(call, setting.meta().source_config()))?
    })
    .signature("string");
    row.getter("comment", |row: &GenericSetting, call: &Call| {
        row.read(|setting| text(call, setting.meta().comment()))?
    })
    .signature("string");
    row.metamethod("__tostring", |row: &GenericSetting| {
        row.read(|setting| {
            format!(
                "dream.openmw.GenericSetting({}={})",
                setting.key(),
                setting.value()
            )
        })
    });
}

fn describe_chain_entry(d: &mut ExtensionDescriptor) {
    let mut row = d.userdata::<ChainEntry>(CHAIN_ENTRY_TYPE);
    row.tag(TagPolicy::Never)
        .doc("One step of the chain traversal: a loaded openmw.cfg or a skipped config= target.");
    row.getter("path", |row: &ChainEntry, call: &Call| {
        row.read(|entry| path_text(call, entry.path()))?
    })
    .signature("string");
    row.getter("depth", |row: &ChainEntry| {
        row.read(|entry| count(entry.depth()))
    })
    .signature("number");
    row.getter("status", |row: &ChainEntry| {
        row.read(|entry| status_name(entry.status()))
    })
    .signature("string")
    .doc("loaded or skippedMissing.");
    row.metamethod("__tostring", |row: &ChainEntry| {
        row.read(|entry| {
            format!(
                "dream.openmw.ChainEntry({}, {})",
                entry.path().display(),
                status_name(entry.status())
            )
        })
    });
}

// One declaration per member reads best as one list, however long.
#[allow(clippy::too_many_lines)]
fn describe_config(d: &mut ExtensionDescriptor) {
    let mut cfg = d.userdata::<Config>(CONFIG_TYPE);
    cfg.tag(TagPolicy::Preferred)
        .doc("A loaded openmw.cfg chain: reads, mutations, and saves.");

    // Identity and serialization.
    cfg.method("rootConfigFile", |c: &Config, call: &Call| {
        path_text(call, c.0.read().root_config_file())
    })
    .signature("(self): string");
    cfg.method("rootConfigDir", |c: &Config| {
        path_string(&c.0.read().root_config_dir())
    })
    .signature("(self): string");
    cfg.method("userConfigPath", |c: &Config| {
        path_string(&c.0.read().user_config_path())
    })
    .signature("(self): string")
    .doc("The highest-priority configuration directory.");
    cfg.method("userConfig", |c: &Config| {
        loaded(c.0.read().user_config_ref())
    })
    .signature("(self): dream_openmw_Config")
    .doc("The user's openmw.cfg (the last in the chain) as its own, independent configuration.");
    cfg.method("toString", |c: &Config| c.0.read().to_string())
        .signature("(self): string")
        .doc("Preservation-oriented serialization: original path spellings, comments, replace= lines.");
    cfg.method("toResolvedString", |c: &Config| {
        c.0.read().to_resolved_string()
    })
    .signature("(self): string")
    .doc("Flattened, relocation-safe serialization; omits config= and replace=.");
    cfg.field::<IsUserConfigField>("isUserConfig")
        .signature("boolean")
        .doc("Whether the root config is already the highest-priority one.");

    // Lists and counts.
    cfg.method("subConfigs", |c: &Config| c.view(ListKind::SubConfigs))
        .signature("(self): dream_openmw_Strings")
        .doc("The effective config= directories, resolved.");
    cfg.method("contentFiles", |c: &Config| c.view(ListKind::Content))
        .signature("(self): dream_openmw_Strings");
    cfg.method("groundcoverFiles", |c: &Config| {
        c.view(ListKind::Groundcover)
    })
    .signature("(self): dream_openmw_Strings");
    cfg.method("fallbackArchives", |c: &Config| c.view(ListKind::Archives))
        .signature("(self): dream_openmw_Strings");
    cfg.method("dataDirectories", |c: &Config| {
        c.view(ListKind::DataDirectories)
    })
    .signature("(self): dream_openmw_Strings")
    .doc("The resolved data= directories, the injected ones included.");
    cfg.method("configChain", |c: &Config| {
        Owned(Sequence(ChainList {
            shared: Rc::clone(&c.0),
        }))
    })
    .signature("(self): dream_openmw_ConfigChain");
    cfg.method("gameSettings", |c: &Config| {
        Owned(Sequence(GameSettingList {
            shared: Rc::clone(&c.0),
        }))
    })
    .signature("(self): dream_openmw_GameSettings")
    .doc("One row per fallback= key, last-defined first.");
    cfg.method("genericSettings", |c: &Config| {
        Owned(Sequence(GenericSettingList {
            shared: Rc::clone(&c.0),
        }))
    })
    .signature("(self): dream_openmw_GenericSettings");
    cfg.method(
        "getGameSetting",
        |c: &Config, key: &str| -> Option<Owned<GameSetting>> {
            let position = c.0.read().game_setting_position(key)?;
            Some(Owned(GameSetting {
                shared: Rc::clone(&c.0),
                position,
                generation: c.0.generation.get(),
            }))
        },
    )
    .signature("(self, key: string): dream_openmw_GameSetting?")
    .doc("The effective fallback= entry for a key (case-sensitive), or nil.");
    cfg.field::<ContentFileCountField>("contentFileCount")
        .signature("number");
    cfg.field::<GroundcoverFileCountField>("groundcoverFileCount")
        .signature("number");
    cfg.field::<ArchiveFileCountField>("archiveFileCount")
        .signature("number");
    cfg.field::<DataDirectoryCountField>("dataDirectoryCount")
        .signature("number");
    cfg.field::<GameSettingCountField>("gameSettingCount")
        .signature("number")
        .doc("Distinct fallback= keys.");
    cfg.field::<GenericSettingCountField>("genericSettingCount")
        .signature("number");
    cfg.field::<SubConfigCountField>("subConfigCount")
        .signature("number");

    // Singletons.
    cfg.method("userData", |c: &Config| {
        c.0.read().userdata().map(|s| path_string(s.parsed()))
    })
    .signature("(self): string?");
    cfg.method("resources", |c: &Config| {
        c.0.read().resources().map(|s| path_string(s.parsed()))
    })
    .signature("(self): string?");
    cfg.method("dataLocal", |c: &Config| {
        c.0.read().data_local().map(|s| path_string(s.parsed()))
    })
    .signature("(self): string?");
    cfg.method("encoding", |c: &Config| {
        c.0.read().encoding().map(|e| e.value().to_string())
    })
    .signature("(self): string?")
    .doc("win1250, win1251, or win1252.");

    // Lookups.
    cfg.method("hasContentFile", |c: &Config, file: &str| {
        c.0.read().has_content_file(file)
    })
    .signature("(self, file: string): boolean");
    cfg.method("hasGroundcoverFile", |c: &Config, file: &str| {
        c.0.read().has_groundcover_file(file)
    })
    .signature("(self, file: string): boolean");
    cfg.method("hasArchiveFile", |c: &Config, file: &str| {
        c.0.read().has_archive_file(file)
    })
    .signature("(self, file: string): boolean");
    cfg.method("hasDataDir", |c: &Config, path: &str| {
        c.0.read().has_data_dir(path)
    })
    .signature("(self, path: string): boolean")
    .doc("Either separator style matches.");

    // Mutators.
    cfg.method("addContentFile", |c: &Config, file: &str| {
        c.0.mutate(|cfg| cfg.add_content_file(file)).map_err(err)
    })
    .signature("(self, file: string)");
    cfg.method("addGroundcoverFile", |c: &Config, file: &str| {
        c.0.mutate(|cfg| cfg.add_groundcover_file(file))
            .map_err(err)
    })
    .signature("(self, file: string)");
    cfg.method("addArchiveFile", |c: &Config, file: &str| {
        c.0.mutate(|cfg| cfg.add_archive_file(file)).map_err(err)
    })
    .signature("(self, file: string)");
    cfg.method("addDataDirectory", |c: &Config, dir: &str| {
        c.0.mutate(|cfg| cfg.add_data_directory(Path::new(dir)));
    })
    .signature("(self, dir: string)");
    cfg.method("removeContentFile", |c: &Config, file: &str| {
        c.0.mutate(|cfg| cfg.remove_content_file(file));
    })
    .signature("(self, file: string)");
    cfg.method("removeGroundcoverFile", |c: &Config, file: &str| {
        c.0.mutate(|cfg| cfg.remove_groundcover_file(file));
    })
    .signature("(self, file: string)");
    cfg.method("removeArchiveFile", |c: &Config, file: &str| {
        c.0.mutate(|cfg| cfg.remove_archive_file(file));
    })
    .signature("(self, file: string)");
    cfg.method("removeDataDirectory", |c: &Config, dir: &str| {
        c.0.mutate(|cfg| cfg.remove_data_directory(&PathBuf::from(dir)));
    })
    .signature("(self, dir: string)");
    cfg.method(
        "setContentFiles",
        |c: &Config, call: &Call, files: Option<Table>| {
            let files = strings(call, files)?;
            c.0.mutate(|cfg| cfg.set_content_files(files));
            Ok(())
        },
    )
    .signature("(self, files: { string }?)")
    .doc("Replaces the list; nil clears it.");
    cfg.method(
        "setFallbackArchives",
        |c: &Config, call: &Call, archives: Option<Table>| {
            let archives = strings(call, archives)?;
            c.0.mutate(|cfg| cfg.set_fallback_archives(archives));
            Ok(())
        },
    )
    .signature("(self, archives: { string }?)");
    cfg.method(
        "setDataDirectories",
        |c: &Config, call: &Call, dirs: Option<Table>| {
            let dirs =
                strings(call, dirs)?.map(|dirs| dirs.into_iter().map(PathBuf::from).collect());
            c.0.mutate(|cfg| cfg.set_data_directories(dirs));
            Ok(())
        },
    )
    .signature("(self, dirs: { string }?)");
    cfg.method(
        "setGameSetting",
        |c: &Config, value: &str, source: Option<&str>, comment: Option<&str>| {
            let mut comment = comment.unwrap_or_default().to_owned();
            let source = source.map(PathBuf::from);
            c.0.mutate(|cfg| cfg.set_game_setting(value, source, &mut comment)).map_err(err)
        },
    )
    .signature("(self, value: string, sourcePath: string?, comment: string?)")
    .doc("value is a whole fallback= value (Key,Value); sourcePath defaults to the user's openmw.cfg; comment lines get a # prefix.");
    cfg.method(
        "setGameSettings",
        |c: &Config, call: &Call, settings: Option<Table>| {
            let settings = strings(call, settings)?;
            c.0.mutate(|cfg| cfg.set_game_settings(settings))
                .map_err(err)
        },
    )
    .signature("(self, settings: { string }?)");
    cfg.method(
        "setGenericSettings",
        |c: &Config, call: &Call, key: &str, values: Option<Table>| {
            let values = strings(call, values)?;
            c.0.mutate(|cfg| cfg.set_generic_settings(key, values));
            Ok(())
        },
    )
    .signature("(self, key: string, values: { string }?)");
    cfg.method("addGenericSetting", |c: &Config, key: &str, value: &str| {
        c.0.mutate(|cfg| cfg.add_generic_setting(key, value));
    })
    .signature("(self, key: string, value: string)");
    cfg.method("setUserData", |c: &Config, path: Option<&str>| {
        c.0.mutate(|cfg| match path {
            Some(path) => cfg.set_user_data_path(path),
            None => cfg.clear_user_data(),
        });
    })
    .signature("(self, path: string?)")
    .doc("nil clears the setting.");
    cfg.method("setResources", |c: &Config, path: Option<&str>| {
        c.0.mutate(|cfg| match path {
            Some(path) => cfg.set_resources_path(path),
            None => cfg.clear_resources(),
        });
    })
    .signature("(self, path: string?)");
    cfg.method("setDataLocal", |c: &Config, path: Option<&str>| {
        c.0.mutate(|cfg| match path {
            Some(path) => cfg.set_data_local_path(path),
            None => cfg.clear_data_local(),
        });
    })
    .signature("(self, path: string?)");
    cfg.method("setEncoding", |c: &Config, value: Option<&str>| {
        let setting = match value {
            Some(value) => {
                let source = c.user_cfg();
                Some(
                    EncodingSetting::try_from((value.to_owned(), source, &mut String::new()))
                        .map_err(err)?,
                )
            }
            None => None,
        };
        c.0.mutate(|cfg| cfg.set_encoding(setting));
        Ok(())
    })
    .signature("(self, encoding: string?)");

    // Saves.
    cfg.method("saveUser", |c: &Config| c.0.read().save_user().map_err(err))
        .signature("(self)")
        .doc("Writes the user's openmw.cfg atomically.");
    cfg.method("saveSubconfig", |c: &Config, dir: &str| {
        c.0.read().save_subconfig(Path::new(dir)).map_err(err)
    })
    .signature("(self, targetDir: string)");
    cfg.method("saveToPath", |c: &Config, path: &str| {
        c.0.read().save_to_path(Path::new(path)).map_err(err)
    })
    .signature("(self, path: string)");
    cfg.method("saveResolvedToPath", |c: &Config, path: &str| {
        c.0.read()
            .save_resolved_to_path(Path::new(path))
            .map_err(err)
    })
    .signature("(self, path: string)");
    cfg.metamethod("__tostring", |c: &Config| {
        format!(
            "dream.openmw.Config({})",
            c.0.read().root_config_file().display()
        )
    });
}

fn describe_module(d: &mut ExtensionDescriptor) {
    d.module(MODULE)
        .doc("openmw.cfg chains: loaders, default paths, and the Config type.")
        .function("fromEnv", || loaded(OpenMWConfiguration::from_env()))
        .signature("() -> dream_openmw_Config")
        .doc("OPENMW_CONFIG, OPENMW_CONFIG_DIR, then OpenMW's root config discovery.")
        .function("fromEnvOrUserConfig", || {
            loaded(OpenMWConfiguration::from_env_or_user_config())
        })
        .signature("() -> dream_openmw_Config")
        .doc("fromEnv, falling back to the default user openmw.cfg.")
        .function("new", |path: Option<&str>| {
            loaded(OpenMWConfiguration::new(path.map(PathBuf::from)))
        })
        .signature("(path: string?) -> dream_openmw_Config")
        .doc("An openmw.cfg path or its directory; nil loads the default config.")
        .function("newEmpty", |dir: &str| {
            loaded(OpenMWConfiguration::new_empty(dir))
        })
        .signature("(userConfigDir: string) -> dream_openmw_Config")
        .doc("An empty configuration attributed to a directory, without reading disk.")
        .function("loadOptional", |path: &str| {
            loaded(OpenMWConfiguration::load_optional(path))
        })
        .signature("(path: string) -> dream_openmw_Config")
        .doc("Loads if present; a missing path starts empty with that context.")
        .function("defaultConfigPath", || {
            path_string(&crate::default_config_path())
        })
        .signature("() -> string")
        .function("defaultUserConfigFile", || {
            path_string(&crate::default_user_config_file())
        })
        .signature("() -> string")
        .function("defaultUserDataPath", || {
            path_string(&crate::default_userdata_path())
        })
        .signature("() -> string")
        .function("defaultDataLocalPath", || {
            path_string(&crate::default_data_local_path())
        })
        .signature("() -> string")
        .function("defaultLocalPath", || {
            path_string(&crate::default_local_path())
        })
        .signature("() -> string")
        .function("defaultGlobalPath", || {
            path_string(&crate::default_global_path())
        })
        .signature("() -> string")
        .function("tryDefaultConfigPath", || {
            tried(crate::try_default_config_path())
        })
        .signature("() -> (string?, string?)")
        .doc("The path, or nil and an error message.")
        .function("tryDefaultUserConfigFile", || {
            tried(crate::try_default_user_config_file())
        })
        .signature("() -> (string?, string?)")
        .function("tryDefaultRootOrUserConfigPath", || {
            tried(crate::try_default_root_or_user_config_path())
        })
        .signature("() -> (string?, string?)")
        .function("tryDefaultUserDataPath", || {
            tried(crate::try_default_userdata_path())
        })
        .signature("() -> (string?, string?)")
        .function("tryDefaultLocalPath", || {
            tried(crate::try_default_local_path())
        })
        .signature("() -> (string?, string?)")
        .function("tryDefaultGlobalPath", || {
            tried(crate::try_default_global_path())
        })
        .signature("() -> (string?, string?)")
        .constant(
            "version",
            CompileConstant::String(env!("CARGO_PKG_VERSION").to_owned()),
        )
        .doc("The crate version.");
}
