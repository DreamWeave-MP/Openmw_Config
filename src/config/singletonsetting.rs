// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2025 Dave Corley (S3kshun8)

#[macro_export]
macro_rules! impl_singleton_setting {
    ($($variant:ident => {
        get: $get_fn:ident,
        set: $set_fn:ident,
        in_type: $in_type:ident
    }),* $(,)?) => {
            $(
                pub fn $get_fn(&self) -> Option<&$in_type> {
                    self.settings.iter().rev().find_map(|setting| {
                        match setting {
                            SettingValue::$variant(value) => Some(value),
                            _ => None,
                        }
                    })
                }

                pub fn $set_fn(&mut self, new: Option<$in_type>) {
                    let index = self
                        .settings
                        .iter()
                        .rposition(|setting| matches!(setting, SettingValue::$variant(_)));

                    match (index, new) {
                        // The last definition is replaced when the new value belongs to the same
                        // file. Another file's stays, as it does in that file, and the new one
                        // follows it and wins.
                        (Some(i), Some(value))
                            if util::paths_equivalent(
                                self.settings[i].meta().source_config(),
                                value.meta().source_config(),
                            ) =>
                        {
                            self.settings[i] = SettingValue::$variant(value);
                        }
                        (_, Some(value)) => self.settings.push(SettingValue::$variant(value)),
                        (Some(i), None) => { self.settings.remove(i); }
                        (None, None) => return,
                    }
                    // Positions in the indexes shift with the list.
                    self.rebuild_indexes();
                }
            )*
    };
}
