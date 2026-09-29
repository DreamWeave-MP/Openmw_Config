+++
title = "Settings"
description = "fallback= settings, encoding=, unknown keys, and any setting by predicate."
weight = 50

[extra]
kind = "api"
+++

## Fallback settings

`fallback=Key,Value` lines carry Morrowind.ini's settings. A key can be defined many times along the
chain; the last definition is its value. Each is a [`GameSettingType`](@/docs/api/types.md#gamesettingtype).

{{ api_signature(value="fn game_settings(&self) -> impl Iterator<Item = &GameSettingType>") }}

Each key once, with its current value. The key defined most recently comes first: `A,1`, `B,2`,
`C,3`, `A,4` yields `A=4`, `C=3`, `B=2`.

{{ api_signature(value="fn get_game_setting(&self, key: &str) -> Option<&GameSettingType>") }}

The current value of `key`, the text between `fallback=` and the first comma. Case-sensitive.

{{ api_signature(value="fn set_game_setting(&mut self, base_value: &str, config_path: Option<PathBuf>, comment: &mut String) -> Result<(), ConfigError>") }}

Adds a definition, `base_value` written as `Key,Value`. Earlier definitions stay in their files;
this one wins because it is last. `config_path` is the `openmw.cfg` to attribute it to, `None`
meaning the user's. `comment` is written above the line: each line of it that is not blank and does
not start with `#` gets a `# `, and it ends with a newline. The method empties it.

Fails with `InvalidGameSetting` when `base_value` has no comma.

{{ api_signature(value="fn set_game_settings(&mut self, settings: Option<Vec<String>>) -> Result<(), ConfigError>") }}

Removes every `fallback=` entry and adds `settings`, each written as `Key,Value`, attributed to the
user's config, behind `replace=fallback` when a parent had contributed. `None` leaves none. Fails
with `InvalidGameSetting` at the first entry without a comma, leaving the old entries removed and
none of the new ones added.

## Encoding

{{ api_signature(value="fn encoding(&self) -> Option<&EncodingSetting>") }}

The `encoding=` in effect, or `None`. OpenMW uses `win1252` when there is none.

{{ api_signature(value="fn set_encoding(&mut self, new: Option<EncodingSetting>)") }}

`Some` replaces the current definition when it belongs to the same file as the new one, and
otherwise adds the new one after it; `None` removes the last. Build the setting
with `EncodingSetting::try_from`, which also decides the file it is saved to:

```rust
use openmw_config::{ConfigError, EncodingSetting, OpenMWConfiguration};

fn main() -> Result<(), ConfigError> {
    let mut config = OpenMWConfiguration::from_env_or_user_config()?;
    let user_file = config.user_config_path().join("openmw.cfg");

    let cyrillic = EncodingSetting::try_from(("win1251".to_string(), user_file, &mut String::new()))?;
    config.set_encoding(Some(cyrillic));
    config.save_user()
}
```

## Unknown keys

Every line whose key the crate does not recognize is kept as a
[`GenericSetting`](@/docs/api/types.md#genericsetting), in order, and written back.

{{ api_signature(value="fn generic_settings_iter(&self) -> impl Iterator<Item = &GenericSetting>") }}

{{ api_signature(value="fn add_generic_setting(&mut self, key: &str, value: &str)") }}

Appends `key=value`, attributed to the user's config.

{{ api_signature(value="fn set_generic_settings(&mut self, key: &str, values: Option<Vec<String>>)") }}

Removes every entry with `key` and adds one `key=value` per value, attributed to the user's
config, behind `replace=<key>` when a parent had contributed. `None` only removes.

## Any setting

{{ api_signature(value="fn settings_matching<'a, P>(&'a self, predicate: P) -> impl Iterator<Item = &'a SettingValue> where P: Fn(&SettingValue) -> bool + 'a") }}

{{ api_signature(value="fn clear_matching<P>(&mut self, predicate: P) where P: Fn(&SettingValue) -> bool") }}

Every setting the predicate accepts, and removing them from memory. A setting here is any line of
any file. Unlike the `remove_*` methods, `clear_matching` does not make the user's config replace
a list, so a parent's setting it removes is back on the next load.
It is a [`SettingValue`](@/docs/api/types.md#settingvalue), so a predicate can match on its kind.
Every setting also has:

| Call | Gives |
|---|---|
| `setting.meta().source_config()` | The `openmw.cfg` it is attributed to |
| `setting.meta().comment()` | The comment lines above it |
| `setting.to_string()` | Its line as it would be saved, comment included |

Settings loading added, such as the `resources/vfs` data directory, have an empty source.
