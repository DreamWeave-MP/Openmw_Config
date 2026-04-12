// This file is part of Openmw_Config.
// Openmw_Config is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.
// Openmw_Config is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.
// You should have received a copy of the GNU General Public License along with Openmw_Config. If not, see <https://www.gnu.org/licenses/>.

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
                        (Some(i), Some(value)) => self.settings[i] = SettingValue::$variant(value),
                        (None, Some(value)) => self.settings.push(SettingValue::$variant(value)),
                        (Some(i), None) => { self.settings.remove(i); }
                        (None, None) => {}
                    }
                }
            )*
    };
}
